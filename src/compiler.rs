/**
 * Main compiler file.
 */
use im::HashMap;
use im::HashSet;

use crate::abstract_syntax::*;
use crate::assembly::*;
use crate::constants::*;

const WORD_SIZE: i64 = 8;

const I63_MIN: i64 = -4611686018427387904;
const I63_MAX: i64 = 4611686018427387903;

const FALSE_INT: i64 = 1;
const TRUE_INT: i64 = 3;

static mut LABEL_CTR: usize = 0;

// Contains contextual information the compiler uses to compile each expression.
struct Context<'a> {
    si: i64,
    env: &'a HashMap<String, i64>, // maps identifiers to their stack offsets
    break_label: &'a str,
    indentation: usize,
    function_env: &'a HashMap<String, Vec<String>>,
    compiling_main: bool, // whether this context is being used to compile the main expression
}

// A collection of instructions for a compiled program
pub struct CompiledProgram {
    pub error_instrs: Vec<FInstr>,
    pub fun_instrs: Vec<FInstr>,
    pub main_instrs: Vec<FInstr>,
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
        si: 2,
        env: &HashMap::new(),
        break_label: "",
        indentation: indentation,
        function_env: &fun_env,
        compiling_main: false,
    };

    // On the second pass, we compile the body of each function.
    let mut fun_instrs: Vec<FInstr> = Vec::new();
    for def in prog.defs.iter() {
        // These values need to be set up by the caller.
        // For now, we use the same ENV variable, since there is no concept of
        // global variables yet. If we introduce global variables, we would need
        // to differentiate between function scopes and global scopes, likely
        // by using a separate ENV variable for functions and globals.
        let mut new_env = ctxt.env.clone();
        for (index, param) in def.params.iter().enumerate() {
            // We use *negative* stack index values, starting from -1, so that
            // functions access their argument values at *positive offsets* from their current location.
            let arg_offset = -1 * ((index + 1) as i64) * WORD_SIZE;
            new_env = new_env.update(param.to_string(), arg_offset);
        }
        let new_ctxt = Context {
            env: &new_env,
            indentation: indentation + 1,
            si: ctxt.si + 1,
            ..ctxt
        };

        // Insert function label
        fun_instrs.push(FInstr {
            instr: Instr::Label(def.name.to_string()),
            indentation,
        });

        // Save RBX
        fun_instrs.push(FInstr {
            instr: Instr::Mov(Val::Reg(Reg::R10), Val::Reg(Reg::RBX)),
            indentation: indentation + 1,
        });

        // Insert body
        fun_instrs.append(&mut compile_expr(&def.body, &new_ctxt));

        // Restore RBX
        fun_instrs.push(FInstr {
            instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::R10)),
            indentation: indentation + 1,
        });

        // Insert return
        fun_instrs.push(FInstr {
            instr: Instr::Ret(),
            indentation: indentation + 1,
        });
    }

    // Compile the main expr
    let mut main_instrs: Vec<FInstr> = Vec::new();
    main_instrs.push(FInstr {
        instr: Instr::Label(start_label.to_string()),
        indentation,
    });
    main_instrs.append(&mut compile_expr(
        &prog.main,
        &Context {
            compiling_main: true,
            indentation: indentation + 1,
            ..ctxt
        },
    ));
    // Final return
    main_instrs.push(FInstr {
        instr: Instr::Ret(),
        indentation: indentation + 1,
    });

    return CompiledProgram {
        error_instrs: compile_error_instrs(indentation),
        fun_instrs,
        main_instrs,
    };
}

// Recursively compiles an expression into a list of assembly instruction
fn compile_expr(expr: &Expr, ctxt: &Context) -> Vec<FInstr> {
    // The generated instructions. We push/append instructions to this vector.
    let mut instrs: Vec<FInstr> = Vec::new();

    match expr {
        Expr::Number(num) => {
            if int_overflow(*num) {
                panic!("Invalid: number must be in the range of a 63-bit signed integer");
            } else {
                // Convert the number to our internal representation
                let num = *num << 1;
                instrs.push(FInstr {
                    instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(num)),
                    indentation: ctxt.indentation,
                });
            }
        }
        Expr::Boolean(false) => instrs.push(FInstr {
            instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
            indentation: ctxt.indentation,
        }),
        Expr::Boolean(true) => instrs.push(FInstr {
            instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(TRUE_INT)),
            indentation: ctxt.indentation,
        }),

        Expr::Id(keyword) if keyword == "input" => {
            if !ctxt.compiling_main {
                panic!("Invalid: input can only be used in the main expression");
            }
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Reg(Reg::RDI)),
                indentation: ctxt.indentation,
            });
        }
        Expr::Id(s) => {
            // If the identifier is unbound in its scope, report an error.
            let id_stack_offset = match ctxt.env.get(s) {
                Some(offset) => offset,
                None => panic!("Unbound variable identifier {s}"),
            };
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RAX), Val::RegOff(Reg::RSP, *id_stack_offset)),
                indentation: ctxt.indentation,
            });
        }
        Expr::UnOp(Op1::Add1, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.append(&mut get_number_type_check_instrs(ctxt));
            instrs.push(FInstr {
                instr: Instr::Add(Val::Reg(Reg::RAX), Val::Imm(1 << 1)),
                indentation: ctxt.indentation,
            });
            instrs.append(&mut get_num_overflow_instrs(ctxt));
        }
        Expr::UnOp(Op1::Sub1, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            instrs.append(&mut get_number_type_check_instrs(ctxt));
            instrs.push(FInstr {
                instr: Instr::Sub(Val::Reg(Reg::RAX), Val::Imm(1 << 1)),
                indentation: ctxt.indentation,
            });
            instrs.append(&mut get_num_overflow_instrs(ctxt));
        }
        Expr::UnOp(Op1::IsNum, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a number
            instrs.append(&mut is_number_type_instrs(ctxt));
            // Move false into RAX by default. Conditionally move true into RAX if e is a number
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                indentation: ctxt.indentation,
            });
        }
        Expr::UnOp(Op1::IsBool, e) => {
            instrs.append(&mut compile_expr(e, ctxt));
            // Set condition codes for whether e is a Boolean
            instrs.append(&mut is_boolean_type_instrs(ctxt));
            // Move false into RAX by default. Conditionally move true into RAX if e is a Boolean
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                indentation: ctxt.indentation,
            });
        }
        Expr::UnOp(Op1::Print, e) => {
            instrs.append(&mut compile_expr(e, ctxt));

            // Allocate an extra word of space if needed to keep RSP 16-byte aligned
            let alignment_offset: i64;
            if (ctxt.si + 1) % 2 != 0 {
                alignment_offset = WORD_SIZE;
            } else {
                alignment_offset = 0;
            }

            // Save current value in RDI on stack
            let rdi_offset = ctxt.si * WORD_SIZE + alignment_offset;
            instrs.push(FInstr {
                instr: Instr::Mov(Val::RegOff(Reg::RSP, rdi_offset), Val::Reg(Reg::RDI)),
                indentation: ctxt.indentation,
            });

            // Move expression result into RDI
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RDI), Val::Reg(Reg::RAX)),
                indentation: ctxt.indentation,
            });

            let rsp_offset = (ctxt.si * WORD_SIZE) + alignment_offset;

            // Move stack pointer
            instrs.push(FInstr {
                instr: Instr::Sub(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)),
                indentation: ctxt.indentation,
            });

            // Call the print function
            instrs.push(FInstr {
                instr: Instr::Call("snek_print".to_string()),
                indentation: ctxt.indentation,
            });

            // Reset stack pointer
            instrs.push(FInstr {
                instr: Instr::Add(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)),
                indentation: ctxt.indentation,
            });

            // Restore saved value of RDI
            instrs.push(FInstr {
                instr: Instr::Mov(Val::Reg(Reg::RDI), Val::RegOff(Reg::RSP, rdi_offset)),
                indentation: ctxt.indentation,
            });
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
            // If e1 didn't evaluate to a number, jump to error code
            instrs.push(FInstr {
                instr: Instr::Test(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::JumpNotZero(INVALID_TYPE_LABEL.to_string()),
                indentation: ctxt.indentation,
            });

            // Save result of e1 on stack
            instrs.push(FInstr {
                instr: Instr::Mov(Val::RegOff(Reg::RSP, stack_offset), Val::Reg(Reg::RAX)),
                indentation: ctxt.indentation,
            });

            // e2 instructions
            instrs.append(&mut compile_expr(e2, e2_ctxt));

            // If e2 didn't evaluate to a number, jump to error code
            instrs.push(FInstr {
                instr: Instr::Test(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::JumpNotZero(INVALID_TYPE_LABEL.to_string()),
                indentation: ctxt.indentation,
            });

            // Add the appropriate instruction based on the arithmetic operator
            match op {
                Op2::Plus => {
                    instrs.push(FInstr {
                        instr: Instr::Add(Val::Reg(Reg::RAX), Val::RegOff(Reg::RSP, stack_offset)),
                        indentation: ctxt.indentation,
                    });
                }
                Op2::Minus => {
                    // Move result of e2 from rax into rbx
                    instrs.push(FInstr {
                        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)),
                        indentation: ctxt.indentation,
                    });

                    // Move result of e1 from stack into rax
                    instrs.push(FInstr {
                        instr: Instr::Mov(Val::Reg(Reg::RAX), Val::RegOff(Reg::RSP, stack_offset)),
                        indentation: ctxt.indentation,
                    });
                    // Do [rax] - [rbx]
                    instrs.push(FInstr {
                        instr: Instr::Sub(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                        indentation: ctxt.indentation,
                    });
                }
                Op2::Times => {
                    // For multiplication, shift the result of e2 right 1 bit.
                    instrs.push(FInstr {
                        instr: Instr::Sar(Val::Reg(Reg::RAX), Val::Imm(1)),
                        indentation: ctxt.indentation,
                    });

                    instrs.push(FInstr {
                        instr: Instr::Mul(Val::Reg(Reg::RAX), Val::RegOff(Reg::RSP, stack_offset)),
                        indentation: ctxt.indentation,
                    });
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
            instrs.push(FInstr {
                instr: Instr::Mov(Val::RegOff(Reg::RSP, stack_offset), Val::Reg(Reg::RAX)),
                indentation: ctxt.indentation,
            });

            instrs.append(&mut compile_expr(e2, e2_ctxt));

            // Insert instructions based on the type of logical operator
            match op {
                Op2::Equal => {
                    instrs.append(&mut get_same_type_check_instrs(stack_offset, ctxt));
                    // Compare the results of e1 and e2
                    instrs.push(FInstr {
                        instr: Instr::Cmp(Val::Reg(Reg::RAX), Val::RegOff(Reg::RSP, stack_offset)),
                        indentation: ctxt.indentation,
                    });

                    // Move true into RBX for the conditional move below
                    instrs.push(FInstr {
                        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_INT)),
                        indentation: ctxt.indentation,
                    });
                    // By default, move false into RAX.
                    // If the equality comparison was true, we conditionally move true into RAX.
                    instrs.push(FInstr {
                        instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
                        indentation: ctxt.indentation,
                    });
                    instrs.push(FInstr {
                        instr: Instr::CMove(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                        indentation: ctxt.indentation,
                    });
                }
                Op2::Greater => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(FInstr {
                        instr: Instr::CMovg(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                        indentation: ctxt.indentation,
                    })
                }
                Op2::GreaterEqual => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(FInstr {
                        instr: Instr::CMovge(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                        indentation: ctxt.indentation,
                    })
                }
                Op2::Less => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(FInstr {
                        instr: Instr::CMovl(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                        indentation: ctxt.indentation,
                    });
                }
                Op2::LessEqual => {
                    instrs.append(&mut get_inequality_instrs(ctxt));
                    instrs.push(FInstr {
                        instr: Instr::CMovle(Val::Reg(Reg::RAX), Val::Reg(Reg::RBX)),
                        indentation: ctxt.indentation,
                    });
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

                let id_stack_offset = i64::try_from(index).unwrap();
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
                instrs.push(FInstr {
                    instr: Instr::Mov(
                        Val::RegOff(Reg::RSP, id_stack_index * WORD_SIZE),
                        Val::Reg(Reg::RAX),
                    ),
                    indentation: ctxt.indentation,
                });

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
            instrs.push(FInstr {
                instr: Instr::Cmp(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::JmpEqual(else_label.clone()),
                indentation: ctxt.indentation,
            });

            // If the condition evaluated to any other value, continue on with the then branch.
            instrs.append(&mut compile_expr(
                then_ex,
                &Context {
                    indentation: ctxt.indentation + 1,
                    ..*ctxt
                },
            ));
            // Jump to the end of the if statement
            instrs.push(FInstr {
                instr: Instr::Jmp(end_label.clone()),
                indentation: ctxt.indentation + 1,
            });

            // Insert the else branch label
            instrs.push(FInstr {
                instr: Instr::Label(else_label.clone()),
                indentation: ctxt.indentation,
            });
            instrs.append(&mut compile_expr(
                else_ex,
                &Context {
                    indentation: ctxt.indentation + 1,
                    ..*ctxt
                },
            ));

            // Insert the end of the if statement label
            instrs.push(FInstr {
                instr: Instr::Label(end_label.clone()),
                indentation: ctxt.indentation,
            });
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
            instrs.push(FInstr {
                instr: Instr::Mov(Val::RegOff(Reg::RSP, variable_loc), Val::Reg(Reg::RAX)),
                indentation: ctxt.indentation,
            });
        }

        Expr::Loop(e) => {
            let start_label = get_new_label("loop");
            let end_label = get_new_label("endloop");
            instrs.push(FInstr {
                instr: Instr::Label(start_label.clone()),
                indentation: ctxt.indentation,
            });
            instrs.append(&mut compile_expr(
                e,
                &Context {
                    break_label: &end_label,
                    indentation: ctxt.indentation + 1,
                    ..*ctxt
                },
            ));
            instrs.push(FInstr {
                instr: Instr::Jmp(start_label.clone()),
                indentation: ctxt.indentation,
            });
            instrs.push(FInstr {
                instr: Instr::Label(end_label.clone()),
                indentation: ctxt.indentation,
            });
            // The result is in RAX
        }
        Expr::Break(e) => {
            // If the break label isn't defined, report an error
            if ctxt.break_label.is_empty() {
                panic!("Error: break without surrounding loop");
            }

            instrs.append(&mut compile_expr(e, ctxt));
            // Jump to endloop label
            instrs.push(FInstr {
                instr: Instr::Jmp(ctxt.break_label.to_string()),
                indentation: ctxt.indentation,
            });
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
            let alignment_offset: i64;
            if (ctxt.si + args.len() as i64 + 1) % 2 != 0 {
                alignment_offset = WORD_SIZE;
            } else {
                alignment_offset = 0;
            }

            // Save RDI on stack
            instrs.push(FInstr {
                instr: Instr::Mov(
                    Val::RegOff(Reg::RSP, (ctxt.si * WORD_SIZE) + alignment_offset),
                    Val::Reg(Reg::RDI),
                ),
                indentation: ctxt.indentation,
            });

            // Evaluate the argument expressions in reverse order and store the results
            // at decreasing memory addresses on the stack.
            for (index, arg) in args.iter().rev().enumerate() {
                let mut arg_si = ctxt.si + 1 + index as i64;
                if alignment_offset > 0 {
                    arg_si += 1;
                }
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
                instrs.push(FInstr {
                    instr: Instr::Mov(Val::RegOff(Reg::RSP, arg_offset), Val::Reg(Reg::RAX)),
                    indentation: ctxt.indentation,
                });
            }

            // Move the stack pointer to the correct location
            let rsp_offset = ((ctxt.si + args.len() as i64) * WORD_SIZE) + alignment_offset;
            instrs.push(FInstr {
                instr: Instr::Sub(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)),
                indentation: ctxt.indentation,
            });

            // Call function
            instrs.push(FInstr {
                instr: Instr::Call(funname.to_string()),
                indentation: ctxt.indentation,
            });

            // Restore RDI
            instrs.push(FInstr {
                instr: Instr::Mov(
                    Val::Reg(Reg::RDI),
                    Val::RegOff(Reg::RSP, -(args.len() as i64 * WORD_SIZE)),
                ),
                indentation: ctxt.indentation,
            });

            // Reset stack pointer
            instrs.push(FInstr {
                instr: Instr::Add(Val::Reg(Reg::RSP), Val::Imm(rsp_offset)),
                indentation: ctxt.indentation,
            });
        }
    }
    return instrs;
}

// Returns error labels and instructions
fn compile_error_instrs(indentation: usize) -> Vec<FInstr> {
    let mut error_instrs: Vec<FInstr> = Vec::new();

    error_instrs.append(&mut get_error_instrs(ERR_NUM_OVERFLOW, indentation));
    error_instrs.append(&mut get_error_instrs(ERR_INVALID_TYPE, indentation));

    return error_instrs;
}

// Get the instructions for the error handler for the given error code
fn get_error_instrs(errcode: i64, indentation: usize) -> Vec<FInstr> {
    let mut instrs: Vec<FInstr> = Vec::new();

    match errcode {
        ERR_NUM_OVERFLOW => {
            instrs.push(FInstr {
                instr: Instr::Label(NUM_OVERFLOW_LABEL.to_string()),
                indentation,
            });
        }
        ERR_INVALID_TYPE => {
            instrs.push(FInstr {
                instr: Instr::Label(INVALID_TYPE_LABEL.to_string()),
                indentation,
            });
        }

        _ => panic!("Unknown error code: {errcode}"),
    }

    // Pass error code as first function argument to snek_error
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RDI), Val::Imm(errcode)),
        indentation: indentation + 1,
    });

    // Save stack pointer of current function onto stack
    instrs.push(FInstr {
        instr: Instr::Push(Val::Reg(Reg::RSP)),
        indentation: indentation + 1,
    });

    // Call snek_error
    instrs.push(FInstr {
        instr: Instr::Call("snek_error".to_string()),
        indentation: indentation + 1,
    });

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
fn get_inequality_instrs(ctxt: &Context) -> Vec<FInstr> {
    let mut instrs: Vec<FInstr> = Vec::new();
    let stack_offset = ctxt.si * WORD_SIZE;

    // Move the result of e2 into RBX for the type check
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)),
        indentation: ctxt.indentation,
    });

    // Check that both operands are of the integer type.
    // e1 OR e2 has a 0 as the LSB if both are integers, 1 otherwise.
    instrs.push(FInstr {
        instr: Instr::Or(Val::Reg(Reg::RBX), Val::RegOff(Reg::RSP, stack_offset)),
        indentation: ctxt.indentation,
    });
    // Test if the LSB is 0
    instrs.push(FInstr {
        instr: Instr::Test(Val::Reg(Reg::RBX), Val::Imm(1)),
        indentation: ctxt.indentation,
    });
    // If the tag bits are not both 0 (i.e. the operands weren't both integers), jump to the error handler
    instrs.push(FInstr {
        instr: Instr::JumpNotEqual(INVALID_TYPE_LABEL.to_string()),
        indentation: ctxt.indentation,
    });

    // Compare the results of e1 and e2.
    instrs.push(FInstr {
        instr: Instr::Cmp(Val::RegOff(Reg::RSP, stack_offset), Val::Reg(Reg::RAX)),
        indentation: ctxt.indentation,
    });

    // Move true into RBX
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Imm(TRUE_INT)),
        indentation: ctxt.indentation,
    });

    // Move false into RAX
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RAX), Val::Imm(FALSE_INT)),
        indentation: ctxt.indentation,
    });

    return instrs;
}

// Returns a vector of instructions that jumps to the numerical overflow error label
fn get_num_overflow_instrs(ctxt: &Context) -> Vec<FInstr> {
    let mut instrs: Vec<FInstr> = Vec::new();
    instrs.push(FInstr {
        instr: Instr::JumpOverflow(NUM_OVERFLOW_LABEL.to_string()),
        indentation: ctxt.indentation,
    });
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a number.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_number_type_instrs(ctxt: &Context) -> Vec<FInstr> {
    let mut instrs = Vec::new();
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)),
        indentation: ctxt.indentation,
    });
    instrs.push(FInstr {
        instr: Instr::Not(Val::Reg(Reg::RBX)),
        indentation: ctxt.indentation,
    });
    instrs.push(FInstr {
        instr: Instr::And(Val::Reg(Reg::RBX), Val::Imm(1)),
        indentation: ctxt.indentation,
    });
    instrs.push(FInstr {
        instr: Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(1)),
        indentation: ctxt.indentation,
    });
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX is a Boolean.
// Uses RBX for intermediate computation, and does a CMP that sets condition codes.
fn is_boolean_type_instrs(ctxt: &Context) -> Vec<FInstr> {
    let mut instrs = Vec::new();
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)),
        indentation: ctxt.indentation,
    });
    // and rax, 1 = 1 only if rax is a Boolean
    instrs.push(FInstr {
        instr: Instr::And(Val::Reg(Reg::RBX), Val::Imm(1)),
        indentation: ctxt.indentation,
    });
    instrs.push(FInstr {
        instr: Instr::Cmp(Val::Reg(Reg::RBX), Val::Imm(1)),
        indentation: ctxt.indentation,
    });
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// is a number. Throws an error if this value is not a number, otherwise continues.
fn get_number_type_check_instrs(ctxt: &Context) -> Vec<FInstr> {
    let mut instrs = Vec::new();
    instrs.append(&mut is_number_type_instrs(ctxt));
    instrs.push(FInstr {
        instr: Instr::JumpNotEqual(INVALID_TYPE_LABEL.to_string()),
        indentation: ctxt.indentation,
    });
    return instrs;
}

// Returns a vector of instructions that checks whether the current value in RAX
// and another value on the stack are the same type. If the types are different,
// jumps to error code; otherwise, continues.
fn get_same_type_check_instrs(stack_offset: i64, ctxt: &Context) -> Vec<FInstr> {
    let mut instrs = Vec::new();
    // Move the contents of RAX into RBX
    instrs.push(FInstr {
        instr: Instr::Mov(Val::Reg(Reg::RBX), Val::Reg(Reg::RAX)),
        indentation: ctxt.indentation,
    });

    // Compare the tag bits of RBX and the value on the stack
    instrs.push(FInstr {
        instr: Instr::Xor(Val::Reg(Reg::RBX), Val::RegOff(Reg::RSP, stack_offset)),
        indentation: ctxt.indentation,
    });

    // If the tag bits are unequal, jump to the error handler
    instrs.push(FInstr {
        instr: Instr::Test(Val::Reg(Reg::RBX), Val::Imm(FALSE_INT)),
        indentation: ctxt.indentation,
    });
    instrs.push(FInstr {
        instr: Instr::JumpNotEqual(INVALID_TYPE_LABEL.to_string()),
        indentation: ctxt.indentation,
    });
    return instrs;
}
