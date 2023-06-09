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
#[derive(Debug, Clone)]
struct Context<'a> {
    si: i64,                                   // stack index
    env: &'a HashMap<String, i64>, // maps ids to their (positive) stack offsets relative to the stack pointer
    break_label: &'a str,          // current label to break to
    fun_map: &'a HashMap<String, Vec<String>>, // maps each function name to its parameters
    compiling_main: bool, // whether this context is being used to compile the main expression
}

// Returns a tuple of (instructions for function definitions, instructions for main expression)
pub fn compile_program(prog: &Program, start_label: String) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();
    instrs.append(&mut compile_error_instrs());

    // Maps each function name to its parameters.
    // This map enables checking for:
    // 1. calling undefined functions
    // 2. calling a function with the wrong number of arguments
    let mut fun_map: HashMap<String, Vec<String>> = HashMap::new();

    for def in prog.defs.iter() {
        if fun_map.contains_key(&def.name) {
            panic!("Function {} already defined", def.name);
        }

        let mut seen_params: HashSet<String> = HashSet::new();
        for param in def.params.iter() {
            if seen_params.contains(param) {
                panic!("Duplicate parameter {param}");
            }
            seen_params = seen_params.update(param.to_string());
        }
        fun_map = fun_map.update(def.name.to_string(), def.params.to_vec());
    }

    instrs.append(&mut compile_funs(&prog.defs, &fun_map));
    instrs.push(Instr::Label(start_label.to_string()));

    let locals = depth(&prog.main);
    let callee_saved = [
        Val::Reg(Reg::RBP),
        Val::Reg(Reg::R11),
        Val::Reg(Reg::R12),
        Val::Reg(Reg::R13),
    ];

    instrs.append(&mut fun_entry(locals, &callee_saved));
    instrs.push(Instr::Mov(Val::Reg(Reg::R15), Val::Reg(Reg::RSI)));
    instrs.push(Instr::Mov(Val::Reg(Reg::R13), Val::Reg(Reg::RDI)));
    instrs.push(Instr::Mov(Val::Reg(Reg::R14), Val::Reg(Reg::RDX)));
    instrs.push(Instr::Mov(Val::Reg(Reg::R11), Val::Reg(Reg::RSI)));

    // Main body
    instrs.append(&mut compile_expr(
        &prog.main,
        &Context {
            si: 0,
            env: &HashMap::default(),
            break_label: "",
            fun_map: &fun_map,
            compiling_main: true,
        },
    ));
    instrs.append(&mut fun_exit(locals, &callee_saved));

    return instrs;
}

// Compile all functions
fn compile_funs(funs: &Vec<FunDef>, fun_map: &HashMap<String, Vec<String>>) -> Vec<Instr> {
    let mut instrs = Vec::new();
    for fun in funs.iter() {
        instrs.append(&mut compile_fun(fun, fun_map));
    }
    return instrs;
}

// Compile given function
fn compile_fun(fun: &FunDef, fun_map: &HashMap<String, Vec<String>>) -> Vec<Instr> {
    let mut instrs = Vec::new();
    let locals = depth(&fun.body);
    let callee_saved = &[Val::Reg(Reg::RBP)];

    instrs.push(Instr::Label(fun.name.to_string()));
    instrs.append(&mut fun_entry(locals, callee_saved));

    // The " + 2 " skips over the saved RBP and return address
    let env: HashMap<String, i64> = fun
        .params
        .iter()
        .enumerate()
        .map(|(i, param)| (param.to_string(), (-1) * WORD_SIZE * (i as i64 + 2)))
        .collect();
    let ctxt = Context {
        si: 0,
        env: &env,
        break_label: "",
        fun_map: fun_map,
        compiling_main: false,
    };
    instrs.append(&mut compile_expr(&fun.body, &ctxt));
    instrs.append(&mut fun_exit(locals, callee_saved));

    return instrs;
}

// Instructions for the beginning of every function.
fn fun_entry(locals: u32, callee_saved: &[Val]) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();

    for reg in callee_saved {
        instrs.push(Instr::Push(*reg));
    }
    instrs.push(Instr::Mov(Val::Reg(Reg::RBP), Val::Reg(Reg::RSP)));

    let size = frame_size(locals, callee_saved);
    instrs.push(Instr::Sub(
        Val::Reg(Reg::RSP),
        Val::Imm(WORD_SIZE * (size as i64)),
    ));

    // Set all of the allocated stack space words to NIL; this ensures we don't
    // try to process garbage "heap" values in garbage collection
    // for i in 0..size {
    //     instrs.push(Instr::Mov(
    //         Val::RegOff(Reg::RBP, WORD_SIZE * (1 + i as i64)),
    //         Val::Imm(NIL_VAL),
    //     ));
    // }

    return instrs;
}

// Instructions for the end of every function
fn fun_exit(locals: u32, callee_saved: &[Val]) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();
    let size = frame_size(locals, callee_saved);
    instrs.push(Instr::Add(
        Val::Reg(Reg::RSP),
        Val::Imm(WORD_SIZE * size as i64),
    ));
    for reg in callee_saved.iter().rev() {
        instrs.push(Instr::Pop(*reg));
    }
    instrs.push(Instr::Ret());

    return instrs;
}

// Returns amount of words to subtract for RSP
fn frame_size(locals: u32, callee_saved: &[Val]) -> u32 {
    // frame size = #locals + #callee saved + return address
    let n = locals + callee_saved.len() as u32 + 1;
    if n % 2 == 0 {
        locals
    } else {
        // Adjust for alignment
        locals + 1
    }
}

// Recursively compiles an expression into a list of assembly instruction
fn compile_expr(expr: &Expr, ctxt: &Context) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();
    // println!("Expr is {:?}, si is {}", expr, ctxt.si);

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
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Reg(Reg::R13)));
        }
        Expr::Nil => instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(NIL_VAL))),

        Expr::Id(s) => {
            let stack_offset = match ctxt.env.get(s) {
                Some(offset) => offset,
                None => panic!("Unbound variable identifier {s}"),
            };
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RAX),
                Val::RegOff(Reg::RBP, *stack_offset),
            ));
        }
        Expr::UnOp(op, e) => instrs.append(&mut compile_unary_op(*op, e, ctxt)),
        Expr::BinOp(op, e1, e2) => instrs.append(&mut compile_binary_op(*op, e1, e2, ctxt)),

        Expr::Let(bindings, body) => {
            let mut new_env: HashMap<String, i64> = ctxt.env.clone();
            let mut locally_bound_ids: HashSet<String> = HashSet::new();

            for (index, (id, e)) in bindings.iter().enumerate() {
                if locally_bound_ids.contains(id) {
                    panic!("Duplicate binding");
                }

                let stack_index = ctxt.si + 1 + index as i64;
                let stack_offset = stack_index * WORD_SIZE;

                // Compile the instructions of the let binding.
                let new_ctxt = Context {
                    si: ctxt.si + index as i64,
                    env: &new_env,
                    ..*ctxt
                };
                let mut e_instrs = compile_expr(e, &new_ctxt);
                instrs.append(&mut e_instrs);

                // Store the let-binded variable on the stack
                instrs.push(Instr::Mov(
                    Val::RegOff(Reg::RBP, stack_offset),
                    Val::Reg(Reg::RAX),
                ));

                // Track which identifiers have been bound locally.
                locally_bound_ids = locally_bound_ids.update(id.to_string());

                // Update the environment mapping of identifier -> memory location.
                // IMPORTANT: This must be done after compiling the let expression.
                new_env = new_env.update(id.to_string(), stack_offset);
            }

            // The body is offset by the number of let bindings at the top level.
            let body_stack_index = ctxt.si + bindings.len() as i64;
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
            let stack_offset = match ctxt.env.get(name) {
                Some(offset) => *offset,
                None => panic!("Unbound variable identifier {name}"),
            };

            // Evaluate expression
            instrs.append(&mut compile_expr(e, ctxt));
            // Update value of variable
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, stack_offset),
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
        }
        Expr::Break(e) => {
            if ctxt.break_label.is_empty() {
                panic!("Error: break without surrounding loop");
            }
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.push(Instr::Jump(ctxt.break_label.to_string()));
        }

        // Function call
        Expr::Call(name, args) => {
            if !ctxt.fun_map.contains_key(name) {
                panic!("Invalid: undefined function {name}");
            }
            let expected_num = ctxt.fun_map.get(name).unwrap().len();
            if expected_num != args.len() {
                panic!(
                    "Invalid: function {name} called with {} args, expected {}",
                    args.len(),
                    expected_num
                );
            }

            let mut curr_ctxt = ctxt.clone();
            for arg in args {
                let next_ctxt = Context {
                    si: curr_ctxt.si + 1,
                    ..curr_ctxt
                };
                instrs.append(&mut compile_expr(arg, &next_ctxt));
                instrs.push(Instr::Mov(
                    Val::RegOff(Reg::RBP, WORD_SIZE * next_ctxt.si),
                    Val::Reg(Reg::RAX),
                ));
                curr_ctxt = next_ctxt;
            }

            let stack_offsets: Vec<i64> = (ctxt.si..ctxt.si + args.len() as i64)
                .map(|i| WORD_SIZE * (i + 1))
                .collect();

            let mut fun_args: Vec<Val> = stack_offsets
                .iter()
                .map(|off| Val::RegOff(Reg::RBP, *off))
                .collect();

            // Maintain RSP alignment if needed by pushing an extra value
            if fun_args.len() % 2 != 0 {
                fun_args.push(Val::Imm(NIL_VAL));
            }

            // Push computed arguments onto stack for function call
            for fun_arg in fun_args.iter().rev() {
                instrs.push(Instr::Push(*fun_arg));
            }

            // Call function
            instrs.push(Instr::Call(name.to_string()));
            // Reset stack pointer
            instrs.push(Instr::Add(
                Val::Reg(Reg::RSP),
                Val::Imm(WORD_SIZE * fun_args.len() as i64),
            ));
        }
        Expr::Vec(args) => {
            // Save the current value of the heap pointer on the stack; this is the return value.
            let vec_stack_offset = (ctxt.si + 1) * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, vec_stack_offset),
                Val::Reg(Reg::R15),
            ));

            // Allocate space for the vector on the heap
            instrs.push(Instr::Add(
                Val::Reg(Reg::R15),
                Val::Imm(WORD_SIZE * (1 + args.len() as i64)),
            ));

            // Store the size of the vector
            instrs.push(Instr::Mov(Val::Reg(Reg::R10), Val::Imm(args.len() as i64)));
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Mov(Val::RegOff(Reg::RBX, 0), Val::Reg(Reg::R10)));

            // Evaluate each argument and store it in the allocated heap space
            for (i, arg) in args.iter().enumerate() {
                instrs.append(&mut compile_expr(
                    arg,
                    &Context {
                        si: ctxt.si + 1,
                        ..*ctxt
                    },
                ));
                instrs.push(Instr::Mov(
                    Val::Reg(Reg::RBX),
                    Val::RegOff(Reg::RBP, vec_stack_offset),
                ));
                instrs.push(Instr::Add(
                    Val::Reg(Reg::RBX),
                    Val::Imm(WORD_SIZE * (1 + i as i64)),
                ));
                instrs.push(Instr::Mov(Val::RegOff(Reg::RBX, 0), Val::Reg(Reg::RAX)));
            }
            // Tag the start address of the heap pointer before returning it
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RAX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1)));
        }
        Expr::VecGet(vec, index) => {
            instrs.append(&mut compile_expr(vec, ctxt));
            instrs.append(&mut is_non_nil_vector());

            // Save the address on the stack
            let vec_stack_offset = (ctxt.si + 1) * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, vec_stack_offset),
                Val::Reg(Reg::RAX),
            ));

            instrs.append(&mut compile_expr(
                index,
                &Context {
                    si: ctxt.si + 1,
                    ..*ctxt
                },
            ));
            // If the offset expression did not actually evaluate to a number, error.
            instrs.append(&mut is_number());
            instrs.push(Instr::JumpNotEqual(String::from(INVALID_TYPE_LABEL)));

            // Convert the offset to its actual number representation
            instrs.push(Instr::Sar(Val::Reg(Reg::RAX), Val::Imm(1)));

            // Unmask the address by clearing the LSB
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Sub(Val::Reg(Reg::RBX), Val::Imm(1)));

            // Get the vector size at the address
            instrs.push(Instr::Mov(Val::Reg(Reg::R10), Val::RegOff(Reg::RBX, 0)));
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Reg(Reg::R10)));
            instrs.push(Instr::JumpGreaterEqual(
                INDEX_OUT_OF_BOUNDS_LABEL.to_string(),
            ));
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Imm(0)));
            instrs.push(Instr::JumpLess(INDEX_OUT_OF_BOUNDS_LABEL.to_string()));

            // Add 1 because the address is currently at the tuple size, not the first element.
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1)));
            // Multiply the offset by the word size
            instrs.push(Instr::Shl(Val::Reg(Reg::RAX), Val::Imm(WORD_SIZE_SHIFT)));
            // Add the offset to the base address
            instrs.push(Instr::Add(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
            // Load the value from the heap
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::RegOff(Reg::RBX, 0)));
        }
        Expr::VecSet(vec, index, value) => {
            instrs.append(&mut compile_expr(vec, ctxt));
            instrs.append(&mut is_non_nil_vector());

            // Save vector address on stack
            let vec_stack_offset = (ctxt.si + 1) * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, vec_stack_offset),
                Val::Reg(Reg::RAX),
            ));

            let index_ctxt = Context {
                si: ctxt.si + 1,
                ..*ctxt
            };
            instrs.append(&mut compile_expr(index, &index_ctxt));
            instrs.append(&mut is_number());
            instrs.push(Instr::JumpNotEqual(String::from(INVALID_TYPE_LABEL)));

            // Convert the offset to its actual number representation
            instrs.push(Instr::Sar(Val::Reg(Reg::RAX), Val::Imm(1)));

            // Unmask the address by clearing the LSB
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Sub(Val::Reg(Reg::RBX), Val::Imm(1)));
            // Get the vector size at the address
            instrs.push(Instr::Mov(Val::Reg(Reg::R10), Val::RegOff(Reg::RBX, 0)));
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Reg(Reg::R10)));
            instrs.push(Instr::JumpGreaterEqual(
                INDEX_OUT_OF_BOUNDS_LABEL.to_string(),
            ));
            instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Imm(0)));
            instrs.push(Instr::JumpLess(INDEX_OUT_OF_BOUNDS_LABEL.to_string()));

            // Save index on stack
            let index_stack_offset = (ctxt.si + 2) * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, index_stack_offset),
                Val::Reg(Reg::RAX),
            ));

            let value_ctxt = Context {
                si: ctxt.si + 2,
                ..*ctxt
            };
            instrs.append(&mut compile_expr(value, &value_ctxt));

            // Get vector address from stack
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Sub(Val::Reg(Reg::RBX), Val::Imm(1)));

            // Get index from stack
            instrs.push(Instr::Mov(
                Val::Reg(Reg::R10),
                Val::RegOff(Reg::RBP, index_stack_offset),
            ));
            instrs.push(Instr::Add(Val::Reg(Reg::R10), Val::Imm(1)));
            instrs.push(Instr::Shl(Val::Reg(Reg::R10), Val::Imm(WORD_SIZE_SHIFT)));
            instrs.push(Instr::Add(Val::Reg(Reg::R10), Val::Reg(Reg::RBX)));
            // Set value
            instrs.push(Instr::Mov(Val::RegOff(Reg::R10, 0), Val::Reg(Reg::RAX)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
        }
        Expr::VecLen(vec) => {
            instrs.append(&mut compile_expr(vec, ctxt));
            instrs.append(&mut is_non_nil_vector());
            instrs.push(Instr::Sub(Val::Reg(Reg::RAX), Val::Imm(1)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::RegOff(Reg::RAX, 0)));
            instrs.push(Instr::Shl(Val::Reg(Reg::RAX), Val::Imm(1)));
        }
        Expr::MakeVec(size, elem) => {
            // Allocate a vector with the given size
            instrs.append(&mut compile_expr(size, ctxt));
            instrs.append(&mut is_positive_int());
            instrs.push(Instr::Shl(
                Val::Reg(Reg::RAX),
                Val::Imm(SNEK_NUMBER_TO_OFFSET_SHIFT),
            ));

            // Save the current value of the heap pointer on the stack; this is the return value.
            let vec_stack_offset = (ctxt.si + 1) * WORD_SIZE;
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, vec_stack_offset),
                Val::Reg(Reg::R15),
            ));

            // Allocate space for the vector on the heap
            instrs.push(Instr::Add(Val::Reg(Reg::R15), Val::Reg(Reg::RAX)));

            // Store the size of the vector
            instrs.push(Instr::Sar(
                Val::Reg(Reg::RAX),
                Val::Imm(OFFSET_TO_NUMBER_SHIFT),
            ));
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Mov(Val::RegOff(Reg::RBX, 0), Val::Reg(Reg::RAX)));

            // Compute the element to fill the vector with
            let elem_ctxt = Context {
                si: ctxt.si + 2,
                ..*ctxt
            };
            instrs.append(&mut compile_expr(elem, &elem_ctxt));

            // Loop to fill vector
            let make_vec_start = get_new_label("make_vec_start");
            let make_vec_end = get_new_label("make_vec_end");
            // R10 serves as the loop index
            instrs.push(Instr::Mov(Val::Reg(Reg::R10), Val::Imm(0)));
            instrs.push(Instr::Label(make_vec_start.clone()));
            // Check the loop index against the size of the vector
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::RegOff(Reg::RBX, 0)));
            instrs.push(Instr::Cmp(Val::Reg(Reg::R10), Val::Reg(Reg::RBX)));
            instrs.push(Instr::JumpEqual(make_vec_end.clone()));
            // Increment loop index
            instrs.push(Instr::Add(Val::Reg(Reg::R10), Val::Imm(1)));
            // Compute address to store element at
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RBX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            // Store the element
            instrs.push(Instr::Shl(Val::Reg(Reg::R10), Val::Imm(WORD_SIZE_SHIFT)));
            instrs.push(Instr::Add(Val::Reg(Reg::RBX), Val::Reg(Reg::R10)));
            instrs.push(Instr::Mov(Val::RegOff(Reg::RBX, 0), Val::Reg(Reg::RAX)));
            instrs.push(Instr::Sar(Val::Reg(Reg::R10), Val::Imm(WORD_SIZE_SHIFT)));
            instrs.push(Instr::Jump(make_vec_start));
            instrs.push(Instr::Label(make_vec_end));
            // Return the vector address
            instrs.push(Instr::Mov(
                Val::Reg(Reg::RAX),
                Val::RegOff(Reg::RBP, vec_stack_offset),
            ));
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1)));
        }
    }
    return instrs;
}

// Returns error labels and instructions
fn compile_error_instrs() -> Vec<Instr> {
    let mut error_instrs: Vec<Instr> = Vec::new();

    error_instrs.append(&mut get_error_instrs(ErrCode::Overflow));
    error_instrs.append(&mut get_error_instrs(ErrCode::InvalidType));
    error_instrs.append(&mut get_error_instrs(ErrCode::IndexOutOfBounds));
    error_instrs.append(&mut get_error_instrs(ErrCode::InvalidVecSize));

    return error_instrs;
}

// Helper for unary operators
fn compile_unary_op(op: Op1, e: &Expr, ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    match op {
        Op1::Add1 => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.append(&mut is_number_with_error());
            instrs.push(Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1 << 1)));
            instrs.append(&mut get_num_overflow_instrs());
        }
        Op1::Sub1 => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.append(&mut is_number_with_error());
            instrs.push(Instr::Sub(Val::Reg(Reg::RAX), Val::Imm(1 << 1)));
            instrs.append(&mut get_num_overflow_instrs());
        }
        Op1::IsNum => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a number
            instrs.append(&mut is_number());
            // Move false into RAX by default. Conditionally move true into RAX if e is a number
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));
            instrs.push(Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
        }
        Op1::IsBool => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a Boolean
            instrs.append(&mut is_boolean());
            // Move false into RAX by default. Conditionally move true into RAX if e is a Boolean
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));
            instrs.push(Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
        }
        Op1::IsVec => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a vector
            instrs.append(&mut is_vector());
            // Move false into RAX by default. Conditionally move true into RAX if e is a vector
            instrs.push(Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_VAL)));
            instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_VAL)));
            instrs.push(Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)));
        }
        Op1::Print => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.push(Instr::Mov(Val::Reg(Reg::RDI), Val::Reg(Reg::RAX)));
            instrs.push(Instr::Call(String::from("snek_print")));
            // The return value of print function is carried over from evaluating the expression
        }
    }
    return instrs;
}

// Helper for binary operators
fn compile_binary_op(op: Op2, e1: &Expr, e2: &Expr, ctxt: &Context) -> Vec<Instr> {
    let mut instrs = Vec::new();
    match op {
        // Arithmetic binary operations
        Op2::Plus | Op2::Minus | Op2::Times => {
            let stack_offset: i64 = (ctxt.si + 1) * WORD_SIZE;
            let next_ctxt = &Context {
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
            instrs.append(&mut compile_expr(e2, next_ctxt));

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
                    instrs.push(Instr::JumpOverflow(String::from(NUM_OVERFLOW_LABEL)));
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
                    instrs.push(Instr::JumpOverflow(String::from(NUM_OVERFLOW_LABEL)));
                }
                Op2::Times => {
                    // For multiplication, shift the result of e2 right 1 bit.
                    instrs.push(Instr::Sar(Val::Reg(Reg::RAX), Val::Imm(1)));

                    instrs.push(Instr::Mul(
                        Val::Reg(Reg::RAX),
                        Val::RegOff(Reg::RBP, stack_offset),
                    ));
                    instrs.push(Instr::JumpOverflow(String::from(NUM_OVERFLOW_LABEL)));
                }
                _ => panic!("Should not panic here: {op:?}"),
            }
            // Check for overflow
            instrs.append(&mut get_num_overflow_instrs());
        }

        // Logical binary operators
        Op2::Equal
        | Op2::Greater
        | Op2::GreaterEqual
        | Op2::Less
        | Op2::LessEqual
        | Op2::StructEqual => {
            let stack_offset: i64 = (ctxt.si + 1) * WORD_SIZE;
            let next_ctxt = &Context {
                si: ctxt.si + 1,
                ..*ctxt
            };

            instrs.append(&mut compile_expr(e1, ctxt));

            // Save result of e1_instrs on stack
            instrs.push(Instr::Mov(
                Val::RegOff(Reg::RBP, stack_offset),
                Val::Reg(Reg::RAX),
            ));

            instrs.append(&mut compile_expr(e2, next_ctxt));

            // Insert instructions based on the type of logical operator
            match op {
                Op2::Equal => {
                    instrs.append(&mut are_same_types(stack_offset));
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
                Op2::StructEqual => {
                    instrs.append(&mut are_same_types(stack_offset));
                    instrs.push(Instr::Mov(
                        Val::Reg(Reg::RDI),
                        Val::RegOff(Reg::RBP, stack_offset),
                    ));
                    instrs.push(Instr::Mov(Val::Reg(Reg::RSI), Val::Reg(Reg::RAX)));
                    instrs.push(Instr::Call(String::from("snek_equals")));
                    // Return value will be in RAX
                }
                _ => panic!("Should never panic here: {op:?}"),
            }
        }
    }
    return instrs;
}

// Get the instructions for the error handler for the given error code
fn get_error_instrs(errcode: ErrCode) -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();

    match errcode {
        ErrCode::Overflow => instrs.push(Instr::Label(String::from(NUM_OVERFLOW_LABEL))),
        ErrCode::InvalidType => instrs.push(Instr::Label(String::from(INVALID_TYPE_LABEL))),
        ErrCode::IndexOutOfBounds => {
            instrs.push(Instr::Label(String::from(INDEX_OUT_OF_BOUNDS_LABEL)))
        }
        ErrCode::InvalidVecSize => instrs.push(Instr::Label(String::from(INVALID_VEC_SIZE_LABEL))),
    }

    // Pass error code as first function argument to snek_error
    instrs.push(Instr::Mov(Val::Reg(Reg::EDI), Val::Imm(errcode as i64)));

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
    let stack_offset = (ctxt.si + 1) * WORD_SIZE;

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
fn get_num_overflow_instrs() -> Vec<Instr> {
    let mut instrs: Vec<Instr> = Vec::new();
    instrs.push(Instr::JumpOverflow(NUM_OVERFLOW_LABEL.to_string()));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a number.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_number() -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
    instrs.push(Instr::Not(Val::Reg(Reg::RBX)));
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(1)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(1)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// is a number. Throws an error if this value is not a number, otherwise continues.
fn is_number_with_error() -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.append(&mut is_number());
    instrs.push(Instr::JumpNotEqual(String::from(INVALID_TYPE_LABEL)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// is a positive integer. Throws an error if not, otherwise continues.
fn is_positive_int() -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.append(&mut &mut is_number_with_error());
    instrs.push(Instr::Cmp(Val::Reg(Reg::RAX), Val::Imm(1)));
    instrs.push(Instr::JumpLess(String::from(INVALID_VEC_SIZE_LABEL)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a Boolean.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_boolean() -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));

    // Clear all bits except the lower two
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(0b11)));
    // and rax, 0b11 = 0b11 only if rax is a Boolean
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(BOOLEAN_LSB)));

    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(BOOLEAN_LSB)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a non-nil vector.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_vector() -> Vec<Instr> {
    let mut instrs = Vec::new();
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(0b11)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(0b1)));
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// is a non-nil vector. Throws an error if not, otherwise continues.
fn is_non_nil_vector() -> Vec<Instr> {
    let mut instrs = Vec::new();

    // Check that value is not nil
    instrs.push(Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(NIL_VAL)));
    instrs.push(Instr::JumpEqual(String::from(INVALID_TYPE_LABEL)));

    // Check that value is vector
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(1)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(1)));
    instrs.push(Instr::JumpNotEqual(String::from(INVALID_TYPE_LABEL)));

    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// and the next value on the stack at lower memory are the same type. If the types are different,
// jumps to error code; otherwise, continues.
fn are_same_types(stack_offset: i64) -> Vec<Instr> {
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
    instrs.push(Instr::Mov(Val::Reg(Reg::R10), Val::Reg(Reg::RAX)));
    instrs.push(Instr::And(Val::Reg(Reg::R10), Val::Imm(1)));
    // Clear all but the lower two bits of RBX
    instrs.push(Instr::And(Val::Reg(Reg::RBX), Val::Imm(0b11)));
    instrs.push(Instr::Xor(Val::Reg(Reg::R10), Val::Reg(Reg::RBX)));
    instrs.push(Instr::Cmp(Val::Reg(Reg::R10), Val::Imm(0b11)));
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
        Expr::Call(_, args) | Expr::Vec(args) => args
            .iter()
            .enumerate()
            .map(|(i, e)| depth(e) + (i as u32))
            .max()
            .unwrap_or(0)
            .max(args.len() as u32),
        Expr::VecLen(e) => depth(e),
        Expr::VecGet(vec, offset) => depth(vec).max(depth(offset) + 1),
        Expr::VecSet(vec, index, value) => depth(vec)
            .max(depth(index) + 1)
            .max(depth(value) + 2)
            .max(2),
        Expr::MakeVec(size, elem) => depth(size).max(depth(elem) + 1).max(2),
    }
}
