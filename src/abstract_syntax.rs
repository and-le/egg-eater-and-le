/**
 * The abstract syntax used by the compiler.
 */

// Unary operators
#[derive(Debug)]
pub enum Op1 {
    Add1,
    Sub1,
    IsNum,
    IsBool,
    Print,
}

// Binary operators
#[derive(Debug)]
pub enum Op2 {
    Plus,
    Minus,
    Times,
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

// Expressions
#[derive(Debug)]
pub enum Expr {
    // Numbers are 63-bit signed integers. The LSB is reserved for typing.
    // The LSB is 0 if the value represents a Number; 1 if the value represents a Boolean.
    Number(i64),
    Boolean(bool),
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
    FunCall(String, Vec<Expr>),  // function call
    Tuple(Vec<Expr>),            // tuple of heap-allocated values
    Index(Box<Expr>, Box<Expr>), // (index e1 e2) returns the element at an offset of e2 words away from the value of e1
}

// A function consists of a name, 0 or more named parameters (arguments), and a body
#[derive(Debug)]
pub struct Definition {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
}
// A program consits of a list of function definitions and a main expression
#[derive(Debug)]
pub struct Program {
    pub defs: Vec<Definition>,
    pub main: Box<Expr>,
}
