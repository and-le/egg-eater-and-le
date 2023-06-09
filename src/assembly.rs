// Assembly values
#[derive(Debug, Copy, Clone)]
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
    RDX, // third function arg, caller-saved
    RSP, // stack pointer
    RBP, // base pointer
    RBX, // scratch register
    R10, // scratch register
    R11, // heap start
    R12, // stack base
    R13, // stores "input"
    R14, // heap end
    R15, // heap pointer
    EDI, // first input for snek_error
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
    Shl(Val, Val),

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
        Instr::Mov(Val::Reg(Reg::EDI), Val::Imm(i)) => format!("mov edi, {i}"),
        Instr::Mov(val1, val2) => match val2 {
            Val::Imm(_) => format!("mov qword {}, {}", val_to_str(val1), val_to_str(val2)),
            _ => format!("mov {}, {}", val_to_str(val1), val_to_str(val2)),
        },
        // Arithmetic
        Instr::Add(val1, val2) => format!("add {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::Sub(val1, val2) => format!("sub {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::Mul(val1, val2) => format!("imul {}, {}", val_to_str(val1), val_to_str(val2)),
        // Comparison
        Instr::Cmp(val1, val2) => format!("cmp {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::Test(val1, val2) => format!("test {}, {}", val_to_str(val1), val_to_str(val2)),
        // Conditional move
        Instr::CMove(val1, val2) => format!("cmove {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::CMovg(val1, val2) => format!("cmovg {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::CMovge(val1, val2) => format!("cmovge {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::CMovl(val1, val2) => format!("cmovl {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::CMovle(val1, val2) => format!("cmovle {}, {}", val_to_str(val1), val_to_str(val2)),
        // Bitwise
        Instr::And(val1, val2) => format!("and {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::Or(val1, val2) => format!("or {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::Xor(val1, val2) => format!("xor {}, {}", val_to_str(val1), val_to_str(val2)),
        Instr::Not(val) => format!("not {}", val_to_str(val)),
        // Label
        Instr::Label(label) => format!("{label}:"),
        // Jumps
        Instr::Jump(label) => format!("jmp {label}"),
        Instr::JumpEqual(label) => format!("je {label}"),
        Instr::JumpNotEqual(label) => format!("jne {label}"),
        Instr::JumpNotZero(label) => format!("jnz {label}"),
        Instr::JumpGreaterEqual(label) => format!("jge {label}"),
        Instr::JumpLess(label) => format!("jl {label}"),
        Instr::JumpOverflow(label) => format!("jo {label}"),
        // Shifts
        Instr::Sar(src, shift_amount) => {
            format!("sar {}, {}", val_to_str(src), val_to_str(shift_amount))
        }
        Instr::Shl(src, shift_amount) => {
            format!("shl {}, {}", val_to_str(src), val_to_str(shift_amount))
        }
        // Function calling
        Instr::Push(val) => format!("push qword {}", val_to_str(val)),
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
        Val::Reg(Reg::RDX) => format!("rdx"),
        Val::Reg(Reg::RSP) => format!("rsp"),
        Val::Reg(Reg::RBP) => format!("rbp"),
        Val::Reg(Reg::R10) => format!("r10"),
        Val::Reg(Reg::R11) => format!("r11"),
        Val::Reg(Reg::R12) => format!("r12"),
        Val::Reg(Reg::R13) => format!("r13"),
        Val::Reg(Reg::R14) => format!("r14"),
        Val::Reg(Reg::R15) => format!("r15"),
        Val::Reg(Reg::EDI) => format!("edi"),

        Val::RegOff(Reg::RAX, offset) => {
            if *offset > 0 {
                format!("[rax - {offset}]")
            } else if *offset < 0 {
                format!("[rax + {}]", -1 * offset)
            } else {
                format!("[rax]")
            }
        }
        Val::RegOff(Reg::RSP, offset) => {
            if *offset > 0 {
                format!("[rsp - {offset}]")
            } else if *offset < 0 {
                format!("[rsp + {}]", -1 * offset)
            } else {
                format!("[rsp]")
            }
        }
        Val::RegOff(Reg::RBP, offset) => {
            if *offset > 0 {
                format!("[rbp - {offset}]")
            } else if *offset < 0 {
                format!("[rbp + {}]", -1 * offset)
            } else {
                format!("[rbp]")
            }
        }
        Val::RegOff(Reg::RBX, offset) => {
            if *offset > 0 {
                format!("[rbx - {offset}]")
            } else if *offset < 0 {
                format!("[rbx + {}]", -1 * offset)
            } else {
                format!("[rbx]")
            }
        }
        Val::RegOff(Reg::R10, offset) => {
            if *offset > 0 {
                format!("[r10 - {offset}]")
            } else if *offset < 0 {
                format!("[r10 + {}]", -1 * offset)
            } else {
                format!("[r10]")
            }
        }
        Val::RegOff(Reg::R15, offset) => {
            if *offset > 0 {
                format!("[r15 - {offset}]")
            } else if *offset < 0 {
                format!("[r15 + {}]", -1 * offset)
            } else {
                format!("[r15]")
            }
        }

        _ => panic!("Unhandled Instruction value: {val:?}"),
    }
}

// Converts the instructions to a String
pub fn instructions_to_string(instrs: Vec<Instr>) -> String {
    let mut strs: Vec<String> = Vec::new();
    for instr in instrs.iter() {
        let s_is = instr_to_str(&instr);
        strs.push(format!("{s_is}"));
    }
    return strs.join("\n");
}
