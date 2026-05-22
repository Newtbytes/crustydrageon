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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    pub value: String,
    pub span: Span,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryOp {
    Complement,
    Negate,
}

/// Expression
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr {
    Const(i32),
    Unary(UnaryOp, Box<Expr>),
}

/// Statement
#[derive(Debug)]
pub enum Stmt {
    Return(Expr),
}

#[cfg(test)]
mod strategy {
    use super::*;

    use crate::src::{Source, Span};

    use proptest::prelude::*;

    fn dummy_span() -> Span {
        let src = Source::new(" ".to_owned());
        Span::empty_at(&src, 0).unwrap()
    }

    fn arb_unary_op() -> impl Strategy<Value = UnaryOp> {
        prop_oneof![Just(UnaryOp::Complement), Just(UnaryOp::Negate),]
    }

    pub fn arb_expr() -> impl Strategy<Value = Expr> {
        let leaf = any::<i32>().prop_map(Expr::Const);

        leaf.prop_recursive(
            3,  // max depth
            16, // max total size
            2,  // max branching factor
            |inner| (arb_unary_op(), inner).prop_map(|(op, expr)| Expr::Unary(op, Box::new(expr))),
        )
    }

    pub fn arb_stmt() -> impl Strategy<Value = Stmt> {
        arb_expr().prop_map(Stmt::Return)
    }

    pub fn arb_identifier() -> impl Strategy<Value = Identifier> {
        "[a-zA-Z_][a-zA-Z0-9_]*"
            .prop_map(|s| Identifier {
                value: s,
                span: dummy_span(),
            })
            .boxed()
    }

    pub fn arb_function() -> impl Strategy<Value = Function> {
        (arb_identifier(), arb_stmt()).prop_map(|(name, body)| Function::new(name, body))
    }

    pub fn arb_program() -> impl Strategy<Value = Program> {
        arb_function().prop_map(|body| Program { body })
    }
}

#[cfg(test)]
pub use strategy::*;
