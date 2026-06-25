//! Constructors for test dummies of nodes which don't include [`Span`] information, for example.
//!
//! Used for easing the definition of test cases.

use crate::src::Span;

use super::{BinOp, BinOpKind, Expr, ExprKind, Identifier, UnOp, UnOpKind};

#[must_use]
pub fn ident(value: &str) -> Identifier {
    Identifier {
        names: vec![value.to_owned()],
        span: Span::default(),
    }
}

#[must_use]
pub fn expr(kind: ExprKind) -> Expr {
    Expr {
        kind,
        span: Span::default(),
    }
}

#[must_use]
pub fn unop(kind: UnOpKind) -> UnOp {
    UnOp {
        kind,
        span: Span::default(),
    }
}

#[must_use]
pub fn binop(kind: BinOpKind) -> BinOp {
    BinOp {
        kind,
        span: Span::default(),
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
