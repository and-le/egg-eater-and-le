/**
 * Main compiler file.
 * Value representation:
 * Numbers have a 0 as the LSB.
 * Booleans have a 11 as the LSBs.
 * Tuples (pointers) have a 1 as the LSB.
 */
use im::HashMap;
use im::HashSet;

use crate::assembly::*;
use crate::constants::*;
use crate::syntax::*;

static mut LABEL_CTR: usize = 0;

// Contains contextual information the compiler uses to compile each expression.
struct Context<'a> {
    si: i64,
    env: &'a HashMap<String, i64>, // maps identifiers to their stack offsets
    break_label: &'a str,
    function_env: &'a HashMap<String, Vec<String>>, // maps each function name to its parameters
    compiling_main: bool, // whether this context is being used to compile the main expression
}

// A collection of instructions for a compiled program
pub struct CompiledProgram {
    pub error_instrs: Vec<Instr>,
    pub fun_instrs: Vec<Instr>,
    pub main_instrs: Vec<Instr>,
}

// Returns a tuple of (instructions for function definitions, instructions for main expression)
pub fn compile_program(prog: &Program, start_label: String) -> CompiledProgram {
    // Indentation level used for formatting
    let indentation = 1;

    // Maps each function name to its parameters
    let mut fun_env: HashMap<String, Vec<String>> = HashMap::new();

    // On this first pass, we add function names to the function env.
    // We do this pre-processing to later detect if we try to call a function
    // that doesn't exist or with the wrong number of arguments.
    for def in prog.defs.iter() {
        // Check if a function with the same name has already been defined
        if fun_env.contains_key(&def.name) {
            panic!("Function {} already defined", def.name);
        }

        // Check for duplicate-named parameters
        let mut seen_params: HashSet<String> = HashSet::new();
        for param in def.params.iter() {
            if seen_params.contains(param) {
                panic!("Duplicate parameter {param}");
            }
            seen_params = seen_params.update(param.to_string());
        }
        // Update the function env
        fun_env = fun_env.update(def.name.to_string(), def.params.to_vec());
    }

    let ctxt = Context {
        si: 1,
        env: &HashMap::new(),
        break_label: "",
        function_env: &fun_env,
        compiling_main: false,
    };

    // On the second pass, we compile the body of each function.
    let mut fun_instrs: Vec<Instr> = Vec::new();
    const NUM_SAVED_VALUES: i64 = 2;
    for def in prog.defs.iter() {
        // These values need to be set up by the caller.
        // For now, we use the same ENV variable, since there is no concept of
        // global variables yet. If we introduce global variables, we would need
        // to differentiate between function scopes and global scopes, likely
        // by using a separate ENV variable for functions and globals.
        let mut new_env = ctxt.env.clone();
        for (index, param) in def.params.iter().enumerate() {
            // Functions access arguments at positive offsets from the base of the current stack frame.
            // All saved values followed by the return address are located after the base.
            let arg_offset = (index as i64 + NUM_SAVED_VALUES + 1) * WORD_SIZE;
            new_env = new_env.update(param.to_string(), -arg_offset);
        }
        let new_ctxt = Context {
            env: &new_env,
            si: ctxt.si + 1,
            ..ctxt
        };

        // Insert function label
        fun_instrs.push(Instr::Label(def.name.to_string()));

        // TODO: If needed, allocate an extra word of space to keep stack pointer aligned.

        // Save RBX
        fun_instrs.push(Instr::Push(Val::Reg(Reg::RBX)));

        // Save base pointer
        fun_instrs.push(Instr::Push(Val::Reg(Reg::RBP)));

        // Set base pointer to bottom of stack frame
        fun_instrs.push(Instr::Mov(Val::Reg(Reg::RBP), Val::Reg(Reg::RSP)));

        // Insert body
        fun_instrs.append(&mut compile_expr(&def.body, &new_ctxt));

        // Restore base pointer
        fun_instrs.push({ Instr::Pop(Val::Reg(Reg::RBP)) });

        // Restore RBP
        fun_instrs.push(Instr::Pop(Val::Reg(Reg::RBX)));

        // Insert return
        fun_instrs.push(Instr::Ret());
    }

    // Compile the main expr
    let mut main_instrs: Vec<Instr> = Vec::new();
    main_instrs.push(Instr::Label(start_label.to_string()));

    // Move the heap start address into R15
    main_instrs.push(Instr::Mov(Val::Reg(Reg::R15), Val::Reg(Reg::RSI)));

    // Move the heap start address into R13
    main_instrs.push(Instr::Mov(Val::Reg(Reg::R13), Val::Reg(Reg::RSI)));
    // Move the heap end address into R14
    main_instrs.push(Instr::Mov(Val::Reg(Reg::R14), Val::Reg(Reg::RDX)));

    // Subtract a word from stack pointer to maintain alignment
    main_instrs.push(Instr::Sub(Val::Reg(Reg::RSP), Val::Imm(WORD_SIZE)));

    // Save the current base pointer
    main_instrs.push(Instr::Push(Val::Reg(Reg::RBP)));

    // Move the current stack pointer into the base pointer
    main_instrs.push(Instr::Mov(Val::Reg(Reg::RBP), Val::Reg(Reg::RSP)));

    // Main body
    main_instrs.append(&mut compile_expr(
        &prog.main,
        &Context {
            compiling_main: true,
            ..ctxt
        },
    ));

    // Restore base pointer
    main_instrs.push(Instr::Pop(Val::Reg(Reg::RBP)));

    // Reset stack pointer
    main_instrs.push(Instr::Add(Val::Reg(Reg::RSP), Val::Imm(WORD_SIZE)));

    // Final return
    main_instrs.push(Instr::Ret());

    return CompiledProgram {
        error_instrs: compile_error_instrs(indentation),
        fun_instrs,
        main_instrs,
    };
}

// Recursively compiles an expression into a list of assembly instruction
fn compile_expr(expr: &Expr, ctxt: &Context) -> Vec<Instr> {
    // The generated instructions. We push/append instructions to this vector.
    let mut instrs: Vec<Instr> = Vec::new();

    match expr {
        Expr::Number(num) => {
            if int_overflow(*num) {
                panic!("Invalid: number must be in the range of a 63-bit signed integer");
            } else {
                // Convert the number to our internal representation
                let num = *num << 1;
                instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(num)));
            }
        }
        Expr::Boolean(false) => instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL))),
        Expr::Boolean(true) => instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(TRUE_VAL))),
        Expr::Input => {
            if !ctxt.compiling_main {
                panic!("Invalid: input can only be used in the main expression");
            }
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Reg(Reg::RDI)));
        }
        Expr::Nil => instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(NIL_VAL))),

        Expr::Id(s) => {
            // If the identifier is unbound in its scope, report an error.
            let id_stack_offset = match ctxt.env.get(s) {
                Some(offset) => offset,
                None => panic!("Unbound variable identifier {s}"),
            };
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RAX),
                Val::RegOff(Reg::RBP, *id_stack_offset),
            ));
        }
        Expr::UnOp(Op1::Add1, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.append(&mut is_number_with_error(ctxt));
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1 << 1)));
            instrs.append(&mut get_num_overflow_instrs(ctxt));
        }
        Expr::UnOp(Op1::Sub1, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.append(&mut is_number_with_error(ctxt));
            instrs.push(Instr::Sub(Val::Reg(Reg::RAX), Val::Imm(1 << 1)));
            instrs.append(&mut get_num_overflow_instrs(ctxt));
        }
        Expr::UnOp(Op1::IsNum, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a number
            instrs.append(&mut is_number(ctxt));
            // Move false into RAX by default. Conditionally move true into RAX if e is a number
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));
            instrs.push(Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
        }
        Expr::UnOp(Op1::IsBool, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a Boolean
            instrs.append(&mut is_boolean(ctxt));
            // Move false into RAX by default. Conditionally move true into RAX if e is a Boolean
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));
            instrs.push(Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
        }
        Expr::UnOp(Op1::Print, e) => {
            instrs.append(&mut compile_expr(e, ctxt));

            // Allocate an extra word of space if needed to keep stack pointer aligned
            let num_saved_words = 3;
            let stack_index: i64;
            let alloc_extra_word = (ctxt.si + num_saved_words) % 2 == 1;
            if alloc_extra_word {
                stack_index = ctxt.si + 1;
            } else {
                stack_index = ctxt.si;
            }

            // Save RDI on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, stack_index * WORD_SIZE),
                Val::Reg(Reg::RDI),
            ));

            // Save RSI on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, (stack_index + 1) * WORD_SIZE),
                Val::Reg(Reg::RSI),
            ));

            // Save RDX on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, (stack_index + 2) * WORD_SIZE),
                Val::Reg(Reg::RDX),
            ));

            // Move expression result into RDI
            instrs.push(Instr::Mov(Val::Reg(Reg::RDI), Val::Reg(Reg::RAX)));

            let rsp_offset = (stack_index + num_saved_words - 1) * WORD_SIZE;
            // Move stack pointer
            instrs.push(Instr::Sub(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)));

            // Call the print function
            instrs.push(Instr::Call("snek_print".to_string()));

            // Reset stack pointer
            instrs.push(Instr::Add(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)));

            // Restore RDI
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RDI),
                Val::RegOff(Reg::RBP, stack_index * WORD_SIZE),
            ));
            // Restore RSI
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RSI),
                Val::RegOff(Reg::RBP, (stack_index + 1) * WORD_SIZE),
            ));
            // Restore RDX
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RDX),
                Val::RegOff(Reg::RBP, (stack_index + 2) * WORD_SIZE),
            ));

            // The return value of print function is carried over from evaluating the expression
        }

        // Arithmetic binary operations
        Expr::BinOp(op @ (Op2::Plus | Op2::Minus | Op2::Times), e1, e2) => {
            let stack_offset: i64 = ctxt.si * WORD_SIZE;
            let e2_ctxt = &Context {
                si: ctxt.si + 1,
                ..*ctxt
            };

            instrs.append(&mut compile_expr(e1, ctxt));
            // If e1 didn't evaluate to a number (LSB is not 0), jump to error code
            instrs.push(Instr::Test(Val::Reg(Reg::RAX), Val::Imm(1)));
            instrs.push(Instr::JumpNotZero(INVALID_TYPE_LABEL.to_string()));

            // Save result of e1 on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, stack_offset),
                Val::Reg(Reg::RAX),
            ));

            // e2 instructions
            instrs.append(&mut compile_expr(e2, e2_ctxt));

            // If e2 didn't evaluate to a number (LSB is not 0), jump to error code
            instrs.push(Instr::Test(Val::Reg(Reg::RAX), Val::Imm(1)));
            instrs.push(Instr::JumpNotZero(INVALID_TYPE_LABEL.to_string()));

            // Add the appropriate instruction based on the arithmetic operator
            match op {
                Op2::Plus => {
                    instrs.push(Instr::Add(
                        Val::Reg(Reg::RAX),
                        Val::RegOff(Reg::RBP, stack_offset),
                    ));
                }
                Op2::Minus => {
                    // Move result of e2 from rax into rbx
                    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));

                    // Move result of e1 from stack into rax
                    instrs.push(Instr::Mov(
                        Val::Reg(Reg::RAX),
                        Val::RegOff(Reg::RBP, stack_offset),
                    ));
                    // Do [rax] - [rbx]
                    instrs.push(Instr::Sub(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
                }
                Op2::Times => {
                    // For multiplication, shift the result of e2 right 1 bit.
                    instrs.push(Instr::Sar(Val::Reg(Reg::RAX), Val::Imm(1)));

                    instrs.push(Instr::Mul(
                        Val::Reg(Reg::RAX),
                        Val::RegOff(Reg::RBP, stack_offset),
                    ));
                }
                _ => panic!("Should not panic here: {op:?}"),
            }

            // Check for overflow
            instrs.append(&mut get_num_overflow_instrs(ctxt));
        }

        // Logical binary operators
        Expr::BinOp(
            op @ (Op2::Equal | Op2::Greater | Op2::GreaterEqual | Op2::Less | Op2::LessEqual),
            e1,
            e2,
        ) => {
            let stack_offset: i64 = ctxt.si * WORD_SIZE;
            let e2_ctxt = &Context {
                si: ctxt.si + 1,
                ..*ctxt
            };

            instrs.append(&mut compile_expr(e1, ctxt));

            // Save result of e1_instrs on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, stack_offset),
                Val::Reg(Reg::RAX),
            ));

            instrs.append(&mut compile_expr(e2, e2_ctxt));

            // Insert instructions based on the type of logical operator
            match op {
                Op2::Equal => {
                    instrs.append(&mut are_same_types(stack_offset, ctxt));
                    // Compare the results of e1 and e2
                    instrs.push(Instr::Cmp(
                        Val::Reg(Reg::RAX),
                        Val::RegOff(Reg::RBP, stack_offset),
                    ));

                    // Move true into RBX for the conditional move below
                    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));
                    // By default, move false into RAX.
                    // If the equality comparison was true, we conditionally move true into RAX.
                    instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
                    instrs.push(Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
                }
                Op2::Greater => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(Instr::CMovg(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)))
                }
                Op2::GreaterEqual => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(Instr::CMovge(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)))
                }
                Op2::Less => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(Instr::CMovl(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
                }
                Op2::LessEqual => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(Instr::CMovle(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
                }
                _ => panic!("Should never panic here: {op:?}"),
            }
        }

        Expr::Let(bindings, body) => {
            let mut new_env: HashMap<String, i64> = ctxt.env.clone();
            let mut locally_bound_ids: HashSet<String> = HashSet::new();

            for (index, (id, e)) in bindings.iter().enumerate() {
                if locally_bound_ids.contains(id) {
                    panic!("Duplicate binding");
                }

                let id_stack_offset = index as i64;
                let id_stack_index = ctxt.si + id_stack_offset;

                // Compile the instructions of the let binding.
                let new_ctxt = Context {
                    si: id_stack_index,
                    env: &new_env,
                    ..*ctxt
                };
                let mut e_instrs = compile_expr(e, &new_ctxt);
                instrs.append(&mut e_instrs);

                // Store the let-binded variable on the stack
                instrs.push(Instr::Mov(
                    Val::RegOff(Reg::RBP, id_stack_index * WORD_SIZE),
                    Val::Reg(Reg::RAX),
                ));

                // Track which identifiers have been bound locally.
                locally_bound_ids = locally_bound_ids.update(id.to_string());

                // Update the environment mapping of identifier -> memory location.
                // IMPORTANT: This must be done after compiling the let expression.
                new_env = new_env.update(id.to_string(), id_stack_index * WORD_SIZE);
            }

            // The body is offset by the number of let bindings at the top level.
            let body_stack_index = ctxt.si + i64::try_from(bindings.len()).unwrap();
            let new_ctxt = Context {
                si: body_stack_index,
                env: &new_env,
                ..*ctxt
            };

            instrs.append(&mut compile_expr(body, &new_ctxt));
        }

        Expr::If(cond, then_ex, else_ex) => {
            let end_label = get_new_label("ifend");
            let else_label = get_new_label("ifelse");

            // Evaluate the condition
            instrs.append(&mut compile_expr(cond, ctxt));

            // If the condition evaluated to false, jump to the else branch.
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
            instrs.push(Instr::JumpEqual(else_label.clone()));

            // If the condition evaluated to any other value, continue on with the then branch.
            instrs.append(&mut compile_expr(then_ex, &Context { ..*ctxt }));
            // Jump to the end of the if statement
            instrs.push(Instr::Jump(end_label.clone()));

            // Insert the else branch label
            instrs.push(Instr::Label(else_label.clone()));
            instrs.append(&mut compile_expr(else_ex, &Context { ..*ctxt }));

            // Insert the end of the if statement label
            instrs.push(Instr::Label(end_label.clone()));
        }
        Expr::Block(exprs) => {
            for e in exprs.iter() {
                instrs.append(&mut compile_expr(e, ctxt));
            }
        }
        Expr::Set(name, e) => {
            let variable_loc = match ctxt.env.get(name) {
                Some(offset) => *offset,
                None => panic!("Unbound variable identifier {name}"),
            };

            // Evaluate expression
            instrs.append(&mut compile_expr(e, ctxt));
            // Update value of variable
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, variable_loc),
                Val::Reg(Reg::RAX),
            ));
        }

        Expr::Loop(e) => {
            let start_label = get_new_label("loop");
            let end_label = get_new_label("endloop");
            instrs.push(Instr::Label(start_label.clone()));
            instrs.append(&mut compile_expr(
                e,
                &Context {
                    break_label: &end_label,
                    ..*ctxt
                },
            ));
            instrs.push(Instr::Jump(start_label.clone()));
            instrs.push(Instr::Label(end_label.clone()));
            // The result is in RAX
        }
        Expr::Break(e) => {
            // If the break label isn't defined, report an error
            if ctxt.break_label.is_empty() {
                panic!("Error: break without surrounding loop");
            }

            instrs.append(&mut compile_expr(e, ctxt));
            // Jump to endloop label
            instrs.push(Instr::Jump(ctxt.break_label.to_string()));
        }

        // Function call
        Expr::FunCall(funname, args) => {
            // Check for undefined functions
            if !ctxt.function_env.contains_key(funname) {
                panic!("Invalid: undefined function {funname}");
            }
            // Check for incorrect number of args
            let expected_num = ctxt.function_env.get(funname).unwrap().len();
            if expected_num != args.len() {
                panic!(
                    "Invalid: function {funname} called with {} args, expected {}",
                    args.len(),
                    expected_num
                );
            }

            // Allocate an extra word of space if needed to keep RSP 16-byte aligned
            let stack_index;
            let num_saved_words = 3;
            let allocate_extra_word = (ctxt.si + args.len() as i64 + num_saved_words) % 2 == 1;
            if allocate_extra_word {
                stack_index = ctxt.si + 1;
            } else {
                stack_index = ctxt.si;
            }

            // Save RDI on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, stack_index * WORD_SIZE),
                Val::Reg(Reg::RDI),
            ));

            // Save RSI on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, (stack_index + 1) * WORD_SIZE),
                Val::Reg(Reg::RSI),
            ));

            // Save RDX on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, (stack_index + 2) * WORD_SIZE),
                Val::Reg(Reg::RDX),
            ));

            // Evaluate the argument expressions in reverse order and store the results
            // at decreasing memory addresses on the stack.
            for (index, arg) in args.iter().rev().enumerate() {
                let arg_si = stack_index + num_saved_words + index as i64;
                let arg_offset = arg_si * WORD_SIZE;

                let mut arg_is = compile_expr(
                    arg,
                    &Context {
                        si: arg_si,
                        ..*ctxt
                    },
                );
                instrs.append(&mut arg_is);
                // Save argument value on stack
                instrs.push(Instr::Mov(
                    Val::RegOff(Reg::RBP, arg_offset),
                    Val::Reg(Reg::RAX),
                ));
            }

            // Move the stack pointer to the correct location
            let rsp_offset = (stack_index + (num_saved_words - 1) + args.len() as i64) * WORD_SIZE;
            instrs.push(Instr::Sub(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)));

            // Call function
            instrs.push(Instr::Call(funname.to_string()));

            // Restore RDI
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RDI),
                Val::RegOff(Reg::RBP, stack_index * WORD_SIZE),
            ));

            // Restore RSI
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RSI),
                Val::RegOff(Reg::RBP, (stack_index + 1) * WORD_SIZE),
            ));

            // Restore RDX
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RDX),
                Val::RegOff(Reg::RBP, (stack_index + 2) * WORD_SIZE),
            ));

            // Reset stack pointer
            instrs.push(Instr::Add(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)));
        }
        // Tuples are represented as [tuple size] [first element] [second element] ... [last element]
        // The return value of a tuple expression is 63-bit address to a word in memory
        // containing the size of the tuple. The word after this size metadata is the first element of the tuple.
        Expr::Tuple(args) => {
            // Save the current value of the heap pointer on the stack; this is the return value.
            let tup_addr_offset = ctxt.si * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, tup_addr_offset),
                Val::Reg(Reg::R15),
            ));
            // Store the size of the tuple on the heap
            instrs.push(Instr::Mov(
                Val::Reg(Reg::R12),
                Val::Imm((args.len() as i64) << 1),
            ));
            instrs.push(Instr::Mov(Val::RegOff(Reg::R15, 0), Val::Reg(Reg::R12)));
            // Update the heap pointer
            instrs.push(Instr::Add(Val::Reg(Reg::R15), Val::Imm(WORD_SIZE)));
            // Incrementally evaluate each argument and store it in the heap
            for arg in args.iter() {
                instrs.append(&mut compile_expr(
                    arg,
                    &Context {
                        si: ctxt.si + 1,
                        ..*ctxt
                    },
                ));
                instrs.push(Instr::Mov(Val::RegOff(Reg::R15, 0), Val::Reg(Reg::RAX)));
                // Update heap pointer
                instrs.push(Instr::Add(Val::Reg(Reg::R15), Val::Imm(WORD_SIZE)));
            }
            // Tag the start address of the heap pointer before returning it
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RAX),
                Val::RegOff(Reg::RBP, tup_addr_offset),
            ));
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1)));
        }
        Expr::Index(addr, offset) => {
            instrs.append(&mut compile_expr(addr, ctxt));
            // If the address expression did not actually evaluate to an address, error
            instrs.append(&mut is_heap_address_with_error(ctxt));

            // If the address is out of bounds of the heap, error
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Reg(Reg::RSI)));
            instrs.push(Instr::JumpLess(
                HEAP_ADDRESS_OUT_OF_BOUNDS_LABEL.to_string(),
            ));

            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Reg(Reg::RDX)));
            instrs.push(Instr::JumpGreaterEqual(
                HEAP_ADDRESS_OUT_OF_BOUNDS_LABEL.to_string(),
            ));

            // Save the address on the stack
            let addr_offset = ctxt.si * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, addr_offset),
                Val::Reg(Reg::RAX),
            ));

            instrs.append(&mut compile_expr(
                offset,
                &Context {
                    si: ctxt.si + 1,
                    ..*ctxt
                },
            ));
            // If the offset expression did not actually evaluate to a number, error.
            instrs.append(&mut is_number(ctxt));
            instrs.push(Instr::JumpNotEqual(NOT_INDEX_OFFSET_LABEL.to_string()));

            // Unmask the address by clearing the LSB
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, addr_offset),
            ));
            instrs.push(Instr::Sub(Val::Reg(Reg::RBX), Val::Imm(1)));

            // Get the tuple size at the address
            instrs.push(Instr::Mov(Val::Reg(Reg::R12), Val::RegOff(Reg::RBX, 0)));
            // If offset size >= tuple size, jump to error
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Reg(Reg::R12)));
            instrs.push(Instr::JumpGreaterEqual(
                INDEX_OUT_OF_BOUNDS_LABEL.to_string(),
            ));
            // If offset size < 0, jump to error
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Imm(0)));
            instrs.push(Instr::JumpLess(INDEX_OUT_OF_BOUNDS_LABEL.to_string()));

            // Convert the offset to its actual value
            instrs.push(Instr::Sar(Val::Reg(Reg::RAX), Val::Imm(1)));
            // Add 1 because the address is currently at the tuple size, not the first element.
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1)));
            // Multiply the offset by the word size
            instrs.push(Instr::Mul(Val::Reg(Reg::RAX), Val::Imm(WORD_SIZE)));
            // Add the offset to the base address
            instrs.push(Instr::Add(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
            // Load the value from the heap
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::RegOff(Reg::RBX, 0)));
        }
    }
    return instrs;
}

// Returns error labels and instructions
fn compile_error_instrs(indentation: usize) -> Vec<Instr> {
    let mut error_instrs: Vec<Instr> = Vec::new();

    error_instrs.append(&mut get_error_instrs(ERR_NUM_OVERFLOW));
    error_instrs.append(&mut get_error_instrs(ERR_INVALID_TYPE));
    error_instrs.append(&mut get_error_instrs(ERR_INDEX_OUT_OF_BOUNDS));
    error_instrs.append(&mut get_error_instrs(ERR_NOT_HEAP_ADDRESS));
    error_instrs.append(&mut get_error_instrs(ERR_NOT_INDEX_OFFSET));
    error_instrs.append(&mut get_error_instrs(ERR_HEAP_ADDRESS_OUT_OF_BOUNDS));

    return error_instrs;
}

// Get the instructions for the error handler for the given error code
fn get_error_instrs(errcode: i64) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();

    match errcode {
        ERR_NUM_OVERFLOW => {
            instrs.push(Instr::Label(NUM_OVERFLOW_LABEL.to_string()));
        }
        ERR_INVALID_TYPE => {
            instrs.push(Instr::Label(INVALID_TYPE_LABEL.to_string()));
        }
        ERR_INDEX_OUT_OF_BOUNDS => instrs.push(Instr::Label(INDEX_OUT_OF_BOUNDS_LABEL.to_string())),
        ERR_NOT_HEAP_ADDRESS => instrs.push(Instr::Label(NOT_HEAP_ADDRESS_LABEL.to_string())),
        ERR_NOT_INDEX_OFFSET => instrs.push(Instr::Label(NOT_INDEX_OFFSET_LABEL.to_string())),
        ERR_HEAP_ADDRESS_OUT_OF_BOUNDS => {
            instrs.push(Instr::Label(HEAP_ADDRESS_OUT_OF_BOUNDS_LABEL.to_string()))
        }

        _ => panic!("Unknown error code: {errcode}"),
    }

    // Pass error code as first function argument to snek_error
    instrs.push(Instr::Mov(Val::Reg(Reg::RDI), Val::Imm(errcode)));

    // Save stack pointer of current function onto stack
    instrs.push(Instr::Push(Val::Reg(Reg::RSP)));

    // Call snek_error
    instrs.push(Instr::Call("snek_error".to_string()));

    return instrs;
}

// Returns true if the the given integer is outside the range of a 63-bit signed integer,
// false otherwise.
fn int_overflow(i: i64) -> bool {
    return i < I63_MIN || i > I63_MAX;
}

// Get an incremented label. Increments the global variable LABEL_CTR each time it is called.
fn get_new_label(s: &str) -> String {
    unsafe {
        let current = LABEL_CTR;
        LABEL_CTR += 1;
        return format!("{s}_{current}");
    }
}

// Return instructions that are common to all implementations of inequality operators.
// Sets condition codes with a CMP indicating the result of the inequality comparison.
// RAX contains false and RBX contains true.
fn get_inequality_instrs(ctxt: &Context) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();
    let stack_offset = ctxt.si * WORD_SIZE;

    // Move the result of e2 into RBX for the type check
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));

    // Check that both operands are of the integer type.
    // e1 OR e2 has a 0 as the LSB if both are integers, 1 otherwise.
    instrs.push(Instr::Or(
        Val::Reg(Reg::RBX),
        Val::RegOff(Reg::RBP, stack_offset),
    ));
    // Test if the LSB is 0
    instrs.push(Instr::Test(Val::Reg(Reg::RBX), Val::Imm(1)));
    // If the tag bits are not both 0 (i.e. the operands weren't both integers), jump to the error handler
    instrs.push(Instr::JumpNotEqual(INVALID_TYPE_LABEL.to_string()));

    // Compare the results of e1 and e2.
    instrs.push(Instr::Cmp(
        Val::RegOff(Reg::RBP, stack_offset),
        Val::Reg(Reg::RAX),
    ));

    // Move true into RBX
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));

    // Move false into RAX
    instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));

    return instrs;
}

// Returns a vector of instructions that jumps to the numerical overflow error label
fn get_num_overflow_instrs(ctxt: &Context) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();
    instrs.push(Instr::JumpOverflow(NUM_OVERFLOW_LABEL.to_string()));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a number.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_number(ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
    instrs.push(Instr::Not(Val::Reg(Reg::RBX)));
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(1)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(1)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// is a number. Throws an error if this value is not a number, otherwise continues.
fn is_number_with_error(ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.append(&mut is_number(ctxt));
    instrs.push(Instr::JumpNotEqual(INVALID_TYPE_LABEL.to_string()));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a Boolean.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_boolean(ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));

    // Clear all bits except the lower two
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(0b11)));
    // and rax, 0b11 = 0b11 only if rax is a Boolean
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(BOOLEAN_LSB)));

    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(BOOLEAN_LSB)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a heap address.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_heap_address(ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(1)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(1)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// is a heap address. Throws an error if not, otherwise continues.
fn is_heap_address_with_error(ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.append(&mut is_heap_address(ctxt));
    instrs.push(Instr::JumpNotEqual(NOT_HEAP_ADDRESS_LABEL.to_string()));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// and the next value on the stack at lower memory are the same type. If the types are different,
// jumps to error code; otherwise, continues.
fn are_same_types(stack_offset: i64, ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    // Move the contents of RAX into RBX
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));

    // These are the possible type comparisons in the current value representation:
    // 1. number XOR number -> LSB = 0
    // 2. boolean XOR boolean -> LSB = 0
    // 3. pointer XOR pointer -> LSB = 0
    // 4. number XOR boolean -> LSB = 1
    // 5. number XOR pointer -> LSB = 1
    // 6. boolean XOR pointer -> LSB = 10
    // To check if two values A, B have the same type:
    // Do result = A XOR B.
    // If LSB(result) != 0, type error (Cases 4, 5)
    // Else if LSB(result) XOR LSB(a) = 0b11, type error (Case 6)
    // Else no error (Cases 1, 2, 3)

    // Compare the tag bits of RBX and the value on the stack. Store the result in RBX.
    instrs.push(Instr::Xor(
        Val::Reg(Reg::RBX),
        Val::RegOff(Reg::RBP, stack_offset),
    ));

    // If the LSB is not 0, then the arguments were different types
    instrs.push(Instr::Test(Val::Reg(Reg::RBX), Val::Imm(1)));
    instrs.push(Instr::JumpNotZero(INVALID_TYPE_LABEL.to_string()));

    // If LSB(rax) XOR LSB(result) = 0b11, type error
    // Get the LSB of RAX into R11
    instrs.push(Instr::Mov(Val::Reg(Reg::R12), Val::Reg(Reg::RAX)));
    instrs.push(Instr::And(Val::Reg(Reg::R12), Val::Imm(1)));
    // Clear all but the lower two bits of RBX
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(0b11)));
    instrs.push(Instr::Xor(Val::Reg(Reg::R12), Val::Reg(Reg::RBX)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::R12), Val::Imm(0b11)));
    instrs.push(Instr::JumpEqual(INVALID_TYPE_LABEL.to_string()));

    return instrs;
}

// Calculates the number of stack words that must be allocated for the given expression.
fn depth(expr: &Expr) -> u32 {
    match expr {
        Expr::Number(_) | Expr::Boolean(_) | Expr::Input | Expr::Nil | Expr::Id(_) => 0,
        Expr::UnOp(_, e) | Expr::Loop(e) | Expr::Break(e) | Expr::Set(_, e) => depth(e),
        Expr::BinOp(_, e1, e2) => depth(e1).max(depth(e2) + 1),
        Expr::If(e1, e2, e3) => depth(e1).max(depth(e2)).max(depth(e3)),
        Expr::Block(es) => es.iter().map(depth).max().unwrap_or(0),
        Expr::Let(bindings, body) => bindings
            .iter()
            .enumerate()
            .map(|(i, (_, e))| depth(e) + (i as u32))
            .max()
            .unwrap_or(0)
            .max(depth(body) + bindings.len() as u32),
        Expr::FunCall(_, args) | Expr::Tuple(args) => args
            .iter()
            .enumerate()
            .map(|(i, e)| depth(e) + (i as u32))
            .max()
            .unwrap_or(0)
            .max(args.len() as u32),
        Expr::Index(vec, offset) => depth(vec).max(depth(offset) + 1),
    }
}
