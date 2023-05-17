/**
 * Rust functions that are linked at runtime with the compiler
 */
use std::env;

const I63_MIN: i64 = -4611686018427387904;
const I63_MAX: i64 = 4611686018427387903;

// The \x01 used below is an undocumented feature of LLVM that ensures
// it does not add an underscore in front of the name.
// Courtesy of Max New (https://maxsnew.com/teaching/eecs-483-fa22/hw_adder_assignment.html)

#[link(name = "our_code")]
extern "C" {
    // input is the value provided by the "input" operator. Its value is in RDI.
    // heap_start is the starting address of the heap. Its value is in RSI.
    // heap_end is the end address of the heap. Its value is in RDX.
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here(input: i64, heap_start: *mut i64, heap_end: *mut i64) -> i64;
}

// Prints an error message to standard error and then exits the process with a nonzero exit code
#[export_name = "\x01snek_error"]
pub extern "C" fn snek_error(errcode: i64) {
    match errcode {
        1 => eprintln!("an error occurred: numeric overflow"),
        2 => eprintln!("an error occurred: invalid argument (incompatible types)"),
        3 => eprintln!("an error occurred: index out of bounds"),
        _ => eprintln!("Unknown error code: {errcode}"),
    }
    std::process::exit(1);
}

// Prints the formatted representation of the value and returns the original input value.
#[export_name = "\x01snek_print"]
pub extern "C" fn snek_print(val: i64) -> i64 {
    let print_val = snek_str(val);
    println!("{print_val}");
    return val;
}

// Parse "input" values into their internal representations
fn parse_input(input: &str) -> i64 {
    match input {
        "false" => 3,
        "true" => 7,
        _ => match input.parse::<i64>() {
            Ok(num) => {
                if num < I63_MIN || num > I63_MAX {
                    panic!("Invalid: input overflows a 63-bit signed integer");
                }
                return num << 1;
            }
            Err(_) => {
                panic!("Invalid: error occurred parsing input");
            }
        },
    }
}

// Converts the internal representation of the value to a user-facing String value.
fn snek_str(val: i64) -> String {
    if val == 7 {
        "true".to_string()
    } else if val == 3 {
        "false".to_string()
    } else if val % 2 == 0 {
        (val >> 1).to_string()
    } else if val == 1 {
        "nil".to_string()
    } else if val & 1 == 1 {
        let mut strings: Vec<String> = Vec::new();
        let addr = (val - 1) as *const i64;
        let tuple_size = unsafe { *addr };
        for i in 1..tuple_size + 1 {
            let elem = unsafe { *addr.offset(i as isize) };
            strings.push(snek_str(elem));
        }
        format!("tuple {}", strings.join(" "))
    } else {
        format!("Unknown value: {}", val)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    // The default value for the "input" operator is false
    let input = if args.len() == 2 { &args[1] } else { "false" };
    let input = parse_input(&input);

    // Allocate a large chunk of memory for the heap
    const HEAP_CAPACITY: usize = 1000000;
    let mut heap_mem = Vec::<i64>::with_capacity(1000000);
    let heap_start: *mut i64 = heap_mem.as_mut_ptr();
    let heap_end: *mut i64 = unsafe { heap_start.offset(HEAP_CAPACITY as isize) };

    // Run the compiled code
    let output: i64 = unsafe { our_code_starts_here(input, heap_start, heap_end) };
    // Print the output
    let _ = snek_print(output);
}
