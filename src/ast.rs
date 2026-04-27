use crate::src::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Structural
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,

    // Literals
    Constant,
    Ident,

    // Keywords
    Int,
    Void,
    Return,

    Error(&'static str),
}

#[derive(Debug, Clone)]
pub struct Token {
    kind: TokenKind,
    lexeme: Span,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: Span) -> Self {
        Self { kind, lexeme }
    }

    /// Return the TokenKind of this Token
    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    pub fn value(&self) -> &Span {
        &self.lexeme
    }

    pub fn lexeme(&self) -> &Span {
        &self.lexeme
    }

    pub fn span(&self) -> &Span {
        &self.lexeme
    }
}

/// A C program
/// Currently can only contain a single function.
#[derive(Debug)]
pub struct Program {
    pub body: Function,
}

/// User-defined identifier (function names, variable names, etc.)
#[derive(Debug)]
pub struct Identifier {
    pub value: String,
}

/// Function definition
#[derive(Debug)]
pub struct Function {
    pub name: Identifier,
    pub body: Stmt,
}

impl Function {
    pub fn new(name: Identifier, body: Stmt) -> Self {
        Function { name, body }
    }
}

/// Expression
#[derive(Debug)]
pub enum Expr {
    Const(i32),
}

/// Statement
#[derive(Debug)]
pub enum Stmt {
    Return(Expr),
}
