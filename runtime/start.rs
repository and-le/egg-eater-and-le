use std::collections::HashSet;
use std::env;

/**
 * Rust functions that are linked at runtime with the compiler
 */

const I63_MIN: i64 = -4611686018427387904;
const I63_MAX: i64 = 4611686018427387903;
const TRUE: i64 = 7;
const FALSE: i64 = 3;
const NIL: i64 = 1;

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
        4 => eprintln!("an error occurred: invalid vector address"),
        5 => eprintln!("an error occurred: invalid vector offset"),
        6 => eprintln!("an error occurred: vector address out of bounds"),
        _ => eprintln!("Unknown error code: {errcode}"),
    }
    std::process::exit(errcode as i32);
}

// Recursive structural equality. Permits comparison of values with different types
#[export_name = "\x01snek_equals"]
pub unsafe extern "C" fn snek_equals(val1: i64, val2: i64) -> i64 {
    snek_equals_helper(val1, val2, &mut HashSet::<(i64, i64)>::new())
}

unsafe fn snek_equals_helper(val1: i64, val2: i64, seen: &mut HashSet<(i64, i64)>) -> i64 {
    if val1 & 3 == 1 && val2 & 3 == 1 {
        if val1 == val2 {
            // println!("Pointers are equal");
            seen.remove(&(val1, val2));
            return TRUE;
        }
        if val1 == NIL || val2 == NIL {
            // println!("Pointer is NIL");
            seen.remove(&(val1, val2));
            return FALSE;
        }
        if !seen.insert((val1, val2)) {
            // println!("Pointers seen before");
            seen.remove(&(val1, val2));
            return TRUE;
        }

        let addr1 = (val1 - 1) as *const u64;
        let addr2 = (val2 - 1) as *const u64;
        let size1 = addr1.read();
        let size2 = addr2.read();
        if size1 != size2 {
            return FALSE;
        }
        // Compare each of the values of val1 and val2
        for i in 0..size1 {
            let elem1 = addr1.add(1 + i as usize).read() as i64;
            let elem2 = addr2.add(1 + i as usize).read() as i64;
            if snek_equals_helper(elem1, elem2, seen) == FALSE {
                return FALSE;
            }
        }
        seen.remove(&(val1, val2));
        // println!("Structurally equal");
        TRUE
    } else {
        // If val1 and val2 aren't pointers, use reference equality
        seen.remove(&(val1, val2));
        if val1 == val2 {
            // println!("Referentially equal");
            return TRUE;
        } else {
            // println!("Referentially unequal");
            return FALSE;
        }
    }
}

// Prints the formatted representation of the value and returns the original input value.
#[export_name = "\x01snek_print"]
pub unsafe extern "C" fn snek_print(val: i64) -> i64 {
    let print_val = snek_str(val, &mut HashSet::<i64>::new());
    println!("{print_val}");
    val
}

// Converts the internal representation of the value to its true value, formatted as a string.
unsafe fn snek_str(val: i64, seen: &mut HashSet<i64>) -> String {
    if val == 7 {
        String::from("true")
    } else if val == 3 {
        String::from("false")
    } else if val % 2 == 0 {
        (val >> 1).to_string()
    } else if val == 1 {
        String::from("nil")
    } else if val & 1 == 1 {
        if !seen.insert(val) {
            return String::from("[...]");
        }
        let addr = (val - 1) as *const u64;
        let size = addr.read() as usize;
        let mut result_str = String::from("[");
        for i in 1..size + 1 {
            let elem = addr.add(i).read() as i64;
            result_str = result_str + &snek_str(elem, seen);
            if i < size {
                result_str = result_str + ", ";
            }
        }
        seen.remove(&val);
        result_str + "]"
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
    let mut heap_mem = Vec::<i64>::with_capacity(HEAP_CAPACITY);
    let heap_start: *mut i64 = heap_mem.as_mut_ptr();
    let heap_end: *mut i64 = unsafe { heap_start.offset(HEAP_CAPACITY as isize) };

    // Run the compiled code
    let output: i64 = unsafe { our_code_starts_here(input, heap_start, heap_end) };
    // Print the output
    unsafe {
        let _ = snek_print(output);
    }
}
