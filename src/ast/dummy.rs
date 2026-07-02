//! Constructors for test dummies of nodes which don't include [`Span`] information, for example.
//!
//! Used for easing the definition of test cases.

use crate::src::{Source, Span};

use super::{BinOp, BinOpKind, Expr, ExprKind, Identifier, Token, TokenKind, UnOp, UnOpKind};

impl Span {
    pub fn dummy(value: &str) -> Self {
        Source::new(value.to_owned()).into()
    }
}

#[must_use]
pub fn ident(value: &str) -> Identifier {
    Identifier {
        names: vec![value.to_owned()],
        span: Span::dummy(value),
    }
}

impl Token {
    pub fn ident(value: &str) -> Self {
        Self::new(TokenKind::Ident, Span::dummy(value))
    }
}

#[must_use]
pub fn expr(kind: ExprKind) -> Expr {
    Expr {
        kind: kind.clone(),
        span: Span::dummy(&kind.to_string()),
    }
}

#[must_use]
pub fn unop(kind: UnOpKind) -> UnOp {
    UnOp {
        kind: kind.clone(),
        span: Span::dummy(&kind.to_string()),
    }
}

#[must_use]
pub fn binop(kind: BinOpKind) -> BinOp {
    BinOp {
        kind: kind.clone(),
        span: Span::dummy(&kind.to_string()),
    }
}

impl Expr {
    #[must_use]
    pub fn constant(value: i32) -> Self {
        expr(ExprKind::Const(value))
    }

    #[must_use]
    pub fn var(name: &str) -> Self {
        expr(ExprKind::Var(ident(name)))
    }

    #[must_use]
    pub fn unary(kind: UnOpKind, operand: Expr) -> Self {
        expr(ExprKind::Unary(unop(kind), operand.into()))
    }

    #[must_use]
    pub fn binary(kind: BinOpKind, a: Expr, b: Expr) -> Self {
        expr(ExprKind::Binary(binop(kind), a.into(), b.into()))
    }

    #[must_use]
    pub fn cond(cond: Expr, if_true: Expr, if_false: Expr) -> Self {
        expr(ExprKind::Cond(cond.into(), if_true.into(), if_false.into()))
    }
}
