/**
 * The abstract syntax used by the compiler.
 */

// Unary operators
#[derive(Debug, Copy, Clone)]
pub enum Op1 {
    Add1,
    Sub1,
    IsNum,
    IsBool,
    IsVec,
    Print,
}

// Binary operators
#[derive(Debug, Copy, Clone)]
pub enum Op2 {
    Plus,
    Minus,
    Times,
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    StructEqual,
}

// Expressions
#[derive(Debug)]
pub enum Expr {
    Number(i64),
    Boolean(bool),
    Input,
    Nil,
    Id(String),
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(Op1, Box<Expr>),
    BinOp(Op2, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Loop(Box<Expr>),
    Break(Box<Expr>),
    Set(String, Box<Expr>),
    Block(Vec<Expr>),
    Call(String, Vec<Expr>), // function call
    Vec(Vec<Expr>),          // vector of heap-allocated values
    VecGet(Box<Expr>, Box<Expr>),
    VecSet(Box<Expr>, Box<Expr>, Box<Expr>),
    VecLen(Box<Expr>),
    MakeVec(Box<Expr>, Box<Expr>),
}

// A function consists of a name, 0 or more named parameters (arguments), and a body
#[derive(Debug)]
pub struct FunDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
}
// A program consits of a list of function definitions and a main expression
#[derive(Debug)]
pub struct Program {
    pub defs: Vec<FunDef>,
    pub main: Box<Expr>,
}
