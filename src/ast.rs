use crate::src::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Constant,
    Ident,

    // Operators
    Complement,
    Negate,
    Decrement,

    // Structural
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,

    // Keywords
    Int,
    Void,
    Return,

    Error(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    lexeme: Span,
}

impl Token {
    #[must_use]
    pub fn new(kind: TokenKind, lexeme: Span) -> Self {
        Self { kind, lexeme }
    }

    /// Return the `TokenKind` of this Token
    #[must_use]
    pub fn kind(&self) -> TokenKind {
        self.kind
    }

    #[must_use]
    pub fn value(&self) -> &Span {
        &self.lexeme
    }

    #[must_use]
    pub fn lexeme(&self) -> &Span {
        &self.lexeme
    }

    #[must_use]
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
    #[must_use]
    pub fn new(name: Identifier, body: Stmt) -> Self {
        Function { name, body }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Complement,
    Negate,
}

/// Expression
#[derive(Debug, PartialEq, Eq)]
pub enum Expr {
    Const(i32),
    Unary(UnaryOp, Box<Expr>),
}

/// Statement
#[derive(Debug)]
pub enum Stmt {
    Return(Expr),
}
