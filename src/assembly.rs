// Assembly values
#[derive(Debug)]
pub enum Val {
    Reg(Reg),
    Imm(i64),
    // Offsets are always subtracted from the register.
    // A positive offset corresponds to moving to lower memory; a negative offset corresponds to moving to higher memory.
    RegOff(Reg, i64),
}

// Registers
#[derive(Debug, Clone, Copy)]
pub enum Reg {
    RAX, // return value, caller-saved

    RDI, // first function arg, caller-saved
    RSI, // second function arg, caller-saved

    RSP, // stack pointer, callee-saved

    RBX, // local variable, callee-saved
    R12, // local variable, callee-saved
    R13, // local variable, callee-saved

    R15, // heap pointer, callee-saved
}

// Assembly instructions
#[derive(Debug)]
pub enum Instr {
    // Move
    Mov(Val, Val),

    // Arithmetic
    Add(Val, Val),
    Sub(Val, Val),
    Mul(Val, Val),

    // Comparison
    Cmp(Val, Val),
    Test(Val, Val),

    // Conditional Move
    CMove(Val, Val),
    CMovg(Val, Val),
    CMovge(Val, Val),
    CMovl(Val, Val),
    CMovle(Val, Val),

    // Shifts
    Sar(Val, Val),

    // Bitwise
    And(Val, Val),
    Or(Val, Val),
    Xor(Val, Val),
    Not(Val),

    // Label
    Label(String),

    // Jumps
    Jump(String),
    JumpEqual(String),
    JumpNotEqual(String),
    JumpNotZero(String),
    JumpGreaterEqual(String),
    JumpLess(String),
    JumpOverflow(String),

    // Function conventions
    Push(Val),
    Pop(Val),
    Call(String),
    Ret(),
}

// Formats the assembly instruction as a string
pub fn instr_to_str(instr: &Instr) -> String {
    match instr {
        // Mov
        Instr::Mov(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("mov {str_val1}, {str_val2}")
        }

        // Arithmetic
        Instr::Add(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("add {str_val1}, {str_val2}")
        }
        Instr::Sub(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("sub {str_val1}, {str_val2}")
        }
        Instr::Mul(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("imul {str_val1}, {str_val2}")
        }

        // Comparison
        Instr::Cmp(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("cmp {str_val1}, {str_val2}")
        }

        Instr::Test(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("test {str_val1}, {str_val2}")
        }

        // Conditional move
        Instr::CMove(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("cmove {str_val1}, {str_val2}")
        }
        Instr::CMovg(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("cmovg {str_val1}, {str_val2}")
        }
        Instr::CMovge(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("cmovge {str_val1}, {str_val2}")
        }
        Instr::CMovl(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("cmovl {str_val1}, {str_val2}")
        }
        Instr::CMovle(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("cmovle {str_val1}, {str_val2}")
        }

        // Bitwise
        Instr::And(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("and {str_val1}, {str_val2}")
        }
        Instr::Or(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("or {str_val1}, {str_val2}")
        }

        Instr::Xor(val1, val2) => {
            let str_val1 = val_to_str(val1);
            let str_val2 = val_to_str(val2);
            format!("xor {str_val1}, {str_val2}")
        }
        Instr::Not(val) => {
            format!("not {}", val_to_str(val))
        }

        // Label
        Instr::Label(label) => {
            format!("{label}:")
        }

        // Jumps
        Instr::Jump(label) => {
            format!("jmp {label}")
        }
        Instr::JumpEqual(label) => {
            format!("je {label}")
        }
        Instr::JumpNotEqual(label) => {
            format!("jne {label}")
        }
        Instr::JumpNotZero(label) => {
            format!("jnz {label}")
        }
        Instr::JumpGreaterEqual(label) => {
            format!("jge {label}")
        }
        Instr::JumpLess(label) => {
            format!("jl {label}")
        }
        Instr::JumpOverflow(label) => {
            format!("jo {label}")
        }

        // Shifts
        Instr::Sar(src, shift_amount) => {
            let str_src = val_to_str(src);
            let str_amount = val_to_str(shift_amount);
            format!("sar {str_src}, {str_amount}")
        }

        // Function calling
        Instr::Push(val) => format!("push {}", val_to_str(val)),
        Instr::Pop(val) => format!("pop {}", val_to_str(val)),
        Instr::Call(label) => format!("call {label}"),
        Instr::Ret() => format!("ret"),
    }
}

// Formats an assembly value as a String
fn val_to_str(val: &Val) -> String {
    match val {
        Val::Imm(num) => format!("{num}"),
        Val::Reg(Reg::RAX) => format!("rax"),
        Val::Reg(Reg::RBX) => format!("rbx"),
        Val::Reg(Reg::RDI) => format!("rdi"),
        Val::Reg(Reg::RSI) => format!("rsi"),
        Val::Reg(Reg::RSP) => format!("rsp"),
        Val::Reg(Reg::R12) => format!("r12"),
        Val::Reg(Reg::R13) => format!("r13"),
        Val::Reg(Reg::R15) => format!("r15"),

        Val::RegOff(Reg::RAX, offset) => {
            if *offset > 0 {
                format!("[rax - {offset}]")
            } else {
                format!("[rax + {}]", -1 * offset)
            }
        }
        Val::RegOff(Reg::RSP, offset) => {
            if *offset > 0 {
                format!("[rsp - {offset}]")
            } else {
                format!("[rsp + {}]", -1 * offset)
            }
        }
        Val::RegOff(Reg::R12, offset) => {
            if *offset > 0 {
                format!("[r12 - {offset}]")
            } else {
                format!("[r12 + {}]", -1 * offset)
            }
        }
        Val::RegOff(Reg::R15, offset) => {
            if *offset > 0 {
                format!("[r15 - {offset}]")
            } else {
                format!("[r15 + {}]", -1 * offset)
            }
        }

        _ => panic!("Unhandled Instruction value: {val:?}"),
    }
}

// A formatted assembly instruction for generating nicer-looking assembly code
#[derive(Debug)]
pub struct FInstr {
    pub instr: Instr,       // the instruction itself
    pub indentation: usize, // number of tabs this instruction is indented by
}

// Formats the vector of instructions
pub fn format_instructions(finstrs: Vec<FInstr>) -> String {
    let mut strs: Vec<String> = Vec::new();
    for fis in finstrs.iter() {
        let indentation = "\t".repeat(fis.indentation);
        let s_is = instr_to_str(&fis.instr);
        strs.push(format!("{indentation}{s_is}"));
    }
    return strs.join("\n");
}
