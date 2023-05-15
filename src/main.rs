use std::env;
use std::fs::File;
use std::io::prelude::*;

use assembly::format_instructions;
use sexp::*;

// Modules
mod abstract_syntax;
mod assembly;
mod compiler;
mod constants;
mod parser;

use compiler::compile_program;
use parser::parse_program;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Input file path
    let in_name = &args[1];

    // Output file path
    let out_name = &args[2];

    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    // First try to parse the file contents as a single main expression.
    // If that fails, try to parse the file contents as a Program
    let parsed_sexpr = parse(&in_contents);
    let sexpr = match parsed_sexpr {
        Ok(sexpr) => sexpr,
        Err(_) => {
            // Try to parse the file as a program
            let parsed_sexpr = parse(&format!("({in_contents})"));
            match parsed_sexpr {
                Ok(sexpr) => sexpr,
                Err(_) => panic!("Invalid S-expression format"),
            }
        }
    };

    let program = parse_program(&sexpr);
    let compiled_program = compile_program(&program, "our_code_starts_here".to_string());
    let error_code = format_instructions(compiled_program.error_instrs);
    let function_code = format_instructions(compiled_program.fun_instrs);
    let main_code = format_instructions(compiled_program.main_instrs);

    let asm_program = format!(
        "
    section .text
    global our_code_starts_here
    extern snek_error
    extern snek_print
{error_code}
{function_code}
{main_code}
    "
    );

    // Write the generated assembly into the output file
    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;

    Ok(())
}
