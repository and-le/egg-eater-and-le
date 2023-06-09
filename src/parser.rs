/**
 * The parser for the compiler.
 */
use sexp::Atom::*;
use sexp::*;

use crate::syntax::*;

// Parses an S-expression into a Snek program
pub fn parse_program(sexpr: &Sexp) -> Program {
    match sexpr {
        // Program or S-expression surrounded by parentheses
        Sexp::List(vec) => {
            let mut parsing_main_only = true;
            let mut defs: Vec<FunDef> = Vec::new();
            for def_or_expr in vec {
                // println!("def_or_expr: {def_or_expr}");
                if is_fundef(def_or_expr) {
                    // println!("Parsing definition: {def_or_expr:?}");
                    parsing_main_only = false;
                    defs.push(parse_definition(def_or_expr));
                } else {
                    // println!("Parsing expr: {def_or_expr:?}");
                    let main;
                    if parsing_main_only {
                        main = Box::new(parse_sexpr(sexpr))
                    } else {
                        main = Box::new(parse_sexpr(def_or_expr))
                    }
                    return Program {
                        defs,
                        main,
                    };
                }
            }
            panic!("Invalid: Only found definitions, no main expression");
        }

        // S-expression without parentheses
        Sexp::Atom(I(_)) => {
            return Program {
                defs: vec![],
                main: Box::new(parse_sexpr(sexpr)),
            };
        }
        Sexp::Atom(S(_)) => {
            return Program {
                defs: vec![],
                main: Box::new(parse_sexpr(sexpr)),
            };
        }
        _ => panic!(
            "Invalid: Program should be a list of 0 or more function definitions and a main expression"
        ),
    }
}

// Converts an Sexp to an Expr. Panics if there was an error.
fn parse_sexpr(sexpr: &Sexp) -> Expr {
    match sexpr {
        Sexp::Atom(I(num)) => {
            // Allow overflow
            let parsed_num = i64::try_from(*num).ok();
            return Expr::Number(match parsed_num {
                Some(n) => n,
                None => 0,
            });
        }

        // Literals
        Sexp::Atom(S(name)) if name == "false" => Expr::Boolean(false),
        Sexp::Atom(S(name)) if name == "true" => Expr::Boolean(true),
        Sexp::Atom(S(name)) if name == "input" => Expr::Input,
        Sexp::Atom(S(name)) if name == "nil" => Expr::Nil,

        // Identifier
        Sexp::Atom(S(name)) => {
            if is_keyword(name) {
                panic!("Invalid: {name} cannot be used as a variable identifier");
            } else {
                Expr::Id(name.to_string())
            }
        }

        // List pattern
        Sexp::List(vec) => match &vec[..] {
            // Type checks
            [Sexp::Atom(S(op)), e] if op == "isnum" => {
                Expr::UnOp(Op1::IsNum, Box::new(parse_sexpr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "isbool" => {
                Expr::UnOp(Op1::IsBool, Box::new(parse_sexpr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "isvec" => {
                Expr::UnOp(Op1::IsVec, Box::new(parse_sexpr(e)))
            }

            // Unary operators
            [Sexp::Atom(S(op)), e] if op == "add1" => {
                Expr::UnOp(Op1::Add1, Box::new(parse_sexpr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "sub1" => {
                Expr::UnOp(Op1::Sub1, Box::new(parse_sexpr(e)))
            }
            [Sexp::Atom(S(op)), e] if op == "print" => {
                Expr::UnOp(Op1::Print, Box::new(parse_sexpr(e)))
            }

            // If
            [Sexp::Atom(S(keyword)), cond, thn, els] if keyword == "if" => Expr::If(
                Box::new(parse_sexpr(cond)),
                Box::new(parse_sexpr(thn)),
                Box::new(parse_sexpr(els)),
            ),

            // Arithmetic
            [Sexp::Atom(S(op)), e1, e2] if op == "+" => Expr::BinOp(
                Op2::Plus,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "-" => Expr::BinOp(
                Op2::Minus,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "*" => Expr::BinOp(
                Op2::Times,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            // Comparison
            [Sexp::Atom(S(op)), e1, e2] if op == "=" => Expr::BinOp(
                Op2::Equal,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == ">" => Expr::BinOp(
                Op2::Greater,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == ">=" => Expr::BinOp(
                Op2::GreaterEqual,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "<" => Expr::BinOp(
                Op2::Less,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "<=" => Expr::BinOp(
                Op2::LessEqual,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),
            [Sexp::Atom(S(op)), e1, e2] if op == "==" => Expr::BinOp(
                Op2::StructEqual,
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
            ),

            // let
            [Sexp::Atom(S(keyword)), bindings, body] if keyword == "let" => {
                let bindings = parse_bindings(bindings);
                if bindings.is_empty() {
                    panic!("Invalid: no bindings");
                }
                Expr::Let(bindings, Box::new(parse_sexpr(body)))
            }

            // Set!
            [Sexp::Atom(S(op)), Sexp::Atom(S(name)), e] if op == "set!" => {
                if is_keyword(name) {
                    panic!("Invalid: {name} cannot be used as a variable identifier");
                } else {
                    Expr::Set(name.to_string(), Box::new(parse_sexpr(e)))
                }
            }

            // Block
            [Sexp::Atom(S(op)), exprs @ ..] if op == "block" => {
                let parsed_exprs: Vec<Expr> = exprs.iter().map(parse_sexpr).collect();
                if parsed_exprs.is_empty() {
                    panic!("Invalid: no expressions for block");
                }
                Expr::Block(parsed_exprs)
            }

            // Loop
            [Sexp::Atom(S(op)), e] if op == "loop" => Expr::Loop(Box::new(parse_sexpr(e))),
            // Break
            [Sexp::Atom(S(op)), e] if op == "break" => Expr::Break(Box::new(parse_sexpr(e))),

            // Vector construction with N expressions
            [Sexp::Atom(S(keyword)), arg1, remaining_args @ ..] if keyword == "vec" => {
                let mut args = Vec::new();
                args.push(parse_sexpr(arg1));
                for arg in remaining_args.iter() {
                    args.push(parse_sexpr(arg));
                }
                Expr::Vec(args)
            }

            // Vector indexing
            [Sexp::Atom(S(keyword)), e1, e2] if keyword == "vec-get" => {
                Expr::VecGet(Box::new(parse_sexpr(e1)), Box::new(parse_sexpr(e2)))
            }

            // Vector mutability
            [Sexp::Atom(S(keyword)), e1, e2, e3] if keyword == "vec-set!" => Expr::VecSet(
                Box::new(parse_sexpr(e1)),
                Box::new(parse_sexpr(e2)),
                Box::new(parse_sexpr(e3)),
            ),

            // Vector length
            [Sexp::Atom(S(keyword)), e] if keyword == "vec-len" => {
                Expr::VecLen(Box::new(parse_sexpr(e)))
            }

            // Vector length
            [Sexp::Atom(S(keyword)), e1, e2] if keyword == "make-vec" => {
                Expr::MakeVec(Box::new(parse_sexpr(e1)), Box::new(parse_sexpr(e2)))
            }

            // Function call
            [Sexp::Atom(S(funname)), args @ ..] => {
                if is_keyword(funname) {
                    panic!("Invalid: function {funname} is a reserved keyword");
                }
                // Parse each of the argument expressions
                Expr::Call(funname.to_string(), args.iter().map(parse_sexpr).collect())
            }

            // Unrecognized list pattern
            _ => {
                panic!("Invalid: {sexpr:?}")
            }
        },

        _ => {
            panic!("Invalid");
        }
    }
}

// Parses 1 or more let bindings
fn parse_bindings(sexpr: &Sexp) -> Vec<(String, Expr)> {
    match sexpr {
        Sexp::List(vec) => {
            return vec.iter().map(parse_bind).collect();
        }
        _ => {
            panic!("Invalid");
        }
    }
}

// Parses a single let binding
fn parse_bind(sexpr: &Sexp) -> (String, Expr) {
    match sexpr {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), e] => {
                if is_keyword(name) {
                    panic!("Invalid: let binding variable {name} is a reserved keyword");
                } else {
                    return (name.to_string(), parse_sexpr(e));
                }
            }
            _ => {
                panic!("Invalid");
            }
        },
        _ => {
            panic!("Invalid: {sexpr:?}")
        }
    }
}

// Returns true if the S-expression is a function definition; false otherwise
fn is_fundef(sexpr: &Sexp) -> bool {
    match sexpr {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(keyword)), Sexp::List(_), _] if keyword == "fun" => true,
            _ => false,
        },
        _ => false,
    }
}

// Parses the parameter.
fn parse_param(sexpr: &Sexp) -> String {
    match sexpr {
        Sexp::Atom(S(name)) => {
            if is_keyword(name) {
                panic!("Invalid: parameter {name} is a reserved keyword");
            }
            name.to_string()
        }
        _ => panic!("Invalid function parameter"),
    }
}

// Parses the function definition.
fn parse_definition(s: &Sexp) -> FunDef {
    match s {
        Sexp::List(def_vec) => match &def_vec[..] {
            [Sexp::Atom(S(keyword)), Sexp::List(name_and_params), body] if keyword == "fun" => {
                match &name_and_params[..] {
                    [Sexp::Atom(S(funname)), params @ ..] => {
                        if is_keyword(funname) {
                            panic!("Invalid: function {funname} is a reserved keyword")
                        }
                        let parsed_params = params.iter().map(parse_param).collect();
                        return FunDef {
                            name: funname.to_string(),
                            params: parsed_params,
                            body: Box::new(parse_sexpr(body)),
                        };
                    }
                    _ => panic!("Invalid function definition syntax"),
                }
            }
            _ => panic!("fun keyword not found"),
        },
        _ => panic!("Definition is a not a List"),
    }
}

// Returns true if the given string is a language keyword, false otherwise
fn is_keyword(s: &str) -> bool {
    match s {
        "true" | "false" | "input" | "nil"  // literals
        | "add1" | "sub1" | "isnum" | "isbool" | "print" // unary operators
        | "let" | "set!" // variable identifiers
        | "if" | "block" | "loop" | "break" // control flow
        | "fun" // functions
        |  "vec" | "vec-get" | "vec-set!" // vectors
        |  "+" | "-" | "*" | "<" | "=" | "<=" | ">=" | "==" // binary operators
        => true,
        _ => false,
    }
}
