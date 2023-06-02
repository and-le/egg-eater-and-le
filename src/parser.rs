/**
 * The parser for the compiler.
 */
use sexp::Atom::*;
use sexp::*;

use crate::syntax::*;

use im::HashSet;

// Parses an S-expression into a Snek program
pub fn parse_program(sexpr: &Sexp) -> Program {
    match sexpr {
        // Program or S-expression surrounded by parentheses
        Sexp::List(vec) => {
            let mut parsing_main_only = true;
            let mut defs: Vec<Definition> = Vec::new();
            for def_or_expr in vec {
                // println!("def_or_expr: {def_or_expr}");
                if is_definition(def_or_expr) {
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
    let reserved_strings: HashSet<String> = [
        "add1", "sub1", "let", "if", "block", "loop", "break", "set!", "isnum", "isbool", "print",
        "+", "-", "*", "<", "<=", ">=", ">", "=", "index", "tuple", "nil",
    ]
    .iter()
    .cloned()
    .collect();
    match sexpr {
        Sexp::Atom(I(num)) => {
            // Allow overflow
            let parsed_num = i64::try_from(*num).ok();
            return Expr::Number(match parsed_num {
                Some(n) => n,
                None => 0,
            });
        }
        Sexp::Atom(S(name)) if name == "false" => Expr::Boolean(false),
        Sexp::Atom(S(name)) if name == "true" => Expr::Boolean(true),
        Sexp::Atom(S(name)) if name == "input" => Expr::Input,
        Sexp::Atom(S(name)) if name == "nil" => Expr::Nil,

        // Identifier
        Sexp::Atom(S(name)) => {
            if reserved_strings.contains(name) {
                panic!("Invalid: reserved keyword {name}");
            }
            Expr::Id(name.to_string())
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

            // let
            [Sexp::Atom(S(keyword)), bindings, body] if keyword == "let" => {
                let bindings = parse_bindings(bindings);
                // Check for no bindings
                if bindings.is_empty() {
                    panic!("Invalid: no bindings");
                }
                Expr::Let(bindings, Box::new(parse_sexpr(body)))
            }

            // Set!
            [Sexp::Atom(S(op)), Sexp::Atom(S(name)), e] if op == "set!" => {
                if reserved_strings.contains(name)
                    || name == "true"
                    || name == "false"
                    || name == "input"
                    || name == "print"
                {
                    panic!("Invalid: reserved keyword");
                }
                Expr::Set(name.to_string(), Box::new(parse_sexpr(e)))
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

            // Tuple
            [Sexp::Atom(S(keyword)), arg1, remaining_args @ ..] if keyword == "tuple" => {
                let mut args = Vec::new();
                args.push(parse_sexpr(arg1));
                for arg in remaining_args.iter() {
                    args.push(parse_sexpr(arg));
                }
                Expr::Tuple(args)
            }

            // Index
            [Sexp::Atom(S(keyword)), e1, e2] if keyword == "index" => {
                Expr::Index(Box::new(parse_sexpr(e1)), Box::new(parse_sexpr(e2)))
            }

            // Function call
            [Sexp::Atom(S(funname)), args @ ..] => {
                if reserved_strings.contains(funname) {
                    panic!("Invalid: reserved keyword {funname}");
                }
                // Parse each of the argument expressions
                Expr::FunCall(funname.to_string(), args.iter().map(parse_sexpr).collect())
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
    let reserved_strings: HashSet<String> = [
        "add1", "sub1", "let", "if", "block", "loop", "break", "set!", "isnum", "isbool", "true",
        "false", "input", "print", "+", "-", "*", "<", "<=", ">=", ">", "=", "index", "tuple",
        "nil",
    ]
    .iter()
    .cloned()
    .collect();
    match sexpr {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(name)), e] => {
                if reserved_strings.contains(name) {
                    panic!("Invalid: reserved keyword {name}");
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
fn is_definition(sexpr: &Sexp) -> bool {
    match sexpr {
        Sexp::List(vec) => match &vec[..] {
            [Sexp::Atom(S(keyword)), Sexp::List(_), _] if keyword == "fun" => true,
            _ => false,
        },
        _ => false,
    }
}

// Parses the S-expression into a String; panics if the S-expression is not a String Atom or is a reserved keyword
fn parse_param(sexpr: &Sexp) -> String {
    let reserved_strings: HashSet<String> = [
        "add1", "sub1", "let", "if", "block", "loop", "break", "set!", "isnum", "isbool", "true",
        "false", "input", "print", "+", "-", "*", "<", "<=", ">=", ">", "=", "index", "tuple",
        "nil",
    ]
    .iter()
    .cloned()
    .collect();
    match sexpr {
        Sexp::Atom(S(name)) => {
            if reserved_strings.contains(name) {
                panic!("Invalid: reserved keyword");
            }
            name.to_string()
        }
        _ => panic!("Invalid function parameter"),
    }
}

// Parses the S-expression into a definition; panics if the syntax is invalid or the function name or params use reserved words
fn parse_definition(s: &Sexp) -> Definition {
    let reserved_strings: HashSet<String> = [
        "add1", "sub1", "let", "if", "block", "loop", "break", "set!", "isnum", "isbool", "true",
        "false", "input", "print", "+", "-", "*", "<", "<=", ">=", ">", "=", "index", "tuple",
        "nil",
    ]
    .iter()
    .cloned()
    .collect();
    match s {
        Sexp::List(def_vec) => match &def_vec[..] {
            [Sexp::Atom(S(keyword)), Sexp::List(name_and_params), body] if keyword == "fun" => {
                match &name_and_params[..] {
                    [Sexp::Atom(S(funname)), params @ ..] => {
                        if reserved_strings.contains(funname) {
                            panic!("Invalid: reserved keyword {funname}")
                        }
                        let parsed_params = params.iter().map(parse_param).collect();
                        return Definition {
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
