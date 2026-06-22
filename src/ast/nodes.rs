use super::tok::Identifier;
use crate::src::Span;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
};

use itertools::Itertools;
#[cfg(test)]
use proptest_derive::Arbitrary;

/// A C program
///
/// Currently can only contain a single function.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Program {
    pub body: Function,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.body, f)
    }
}

/// Node representing a function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Function {
    pub name: Identifier,
    pub body: Block,
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "int {}(void) {}", self.name, self.body)
    }
}

impl Function {
    #[must_use]
    pub fn new(name: Identifier, body: Block) -> Self {
        Function { name, body }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum UnOpKind {
    /// Negation: `-`
    Negate,

    /// Bitwise complement: `~`
    Complement,

    /// Logical not: `!`
    Not,
}

impl Display for UnOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Negate => "-",
                Self::Complement => "~",
                Self::Not => "!",
            }
        )
    }
}

/// Unary operator node.
///
/// Wraps a [`UnOpKind`] with a [`Span`].
#[derive(Debug, Eq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct UnOp {
    pub kind: UnOpKind,
    pub span: Span,
}

impl PartialEq for UnOp {
    /// Tests if `self` and `other` are equal, excluding this node's source [`span`](UnOp::span).
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Display for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.kind, f)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BinOpKind {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Logical
    And,
    Or,
    Equal,
    NotEqual,
    LessThan,
    LessOrEqual,
    GreaterThan,
    GreaterOrEqual,

    // Bitwise
    BitAnd,
    BitOr,
    Xor,
    LShift,
    RShift,

    // Assignment
    Assign,
}

impl Display for BinOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Add => "+",
                Self::Subtract => "-",
                Self::Multiply => "*",
                Self::Divide => "/",
                Self::Modulo => "%",
                Self::And => "&&",
                Self::Or => "||",
                Self::Equal => "==",
                Self::NotEqual => "!=",
                Self::LessThan => "<",
                Self::LessOrEqual => "<=",
                Self::GreaterThan => ">",
                Self::GreaterOrEqual => ">=",
                Self::BitAnd => "&",
                Self::BitOr => "|",
                Self::Xor => "^",
                Self::LShift => "<<",
                Self::RShift => ">>",
                Self::Assign => "=",
            }
        )
    }
}

/// Binary operator node.
///
/// Wraps a [`BinOpKind`] with a [`Span`].
#[derive(Debug, Eq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct BinOp {
    pub kind: BinOpKind,
    pub span: Span,
}

impl PartialEq for BinOp {
    /// Tests if `self` and `other` are equal, excluding this node's source [`span`](BinOp::span).
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.kind, f)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExprKind {
    Const(i32),
    Var(Identifier),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Display for ExprKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Const(val) => write!(f, "{val}"),
            Self::Var(id) => write!(f, "{}", id.source_name()),
            Self::Unary(op, expr) => write!(f, "{op}{expr}"),
            Self::Binary(op, a, b) => write!(f, "{a} {op} {b}"),
            Self::Cond(cond, if_true, if_false) => write!(f, "{cond} ? {if_true} : {if_false}"),
        }
    }
}

/// Expression node.
///
/// Wraps a [`ExprKind`] with a [`Span`].
#[derive(Debug, Eq, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl PartialEq for Expr {
    /// Tests if `self` and `other` are equal, excluding this node's source [`span`](Expr::span).
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.kind, f)
    }
}

/// Statement node.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Stmt {
    Expr(Expr),
    Return(Expr),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    Compound(Block),
    /// Null statement
    Null,
}

impl Display for Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expr(expr) => write!(f, "{expr};"),
            Self::Return(expr) => write!(f, "return {expr};"),
            Self::If(cond, if_true, if_false) => {
                write!(f, "if {cond} {if_true}")?;
                if let Some(if_false) = if_false {
                    write!(f, " {if_false}")?;
                }
                Ok(())
            }
            Self::Compound(block) => write!(f, "{block}"),
            Self::Null => write!(f, ";"),
        }
    }
}

/// Declaration node.
#[derive(Debug, Eq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Decl {
    pub name: Identifier,
    pub init: Option<Expr>,
    pub span: Span,
}

impl PartialEq for Decl {
    /// Tests if `self` and `other` are equal, excluding this node's source [`span`](Decl::span).
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.init == other.init
    }
}

impl Display for Decl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "int {}", self.name.source_name())?;

        if let Some(init) = &self.init {
            write!(f, " = {init}")?;
        }

        write!(f, ";")
    }
}

/// Node representing a single element of a block.
#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BlockItem {
    Stmt(Stmt),
    Decl(Decl),
}

impl Display for BlockItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stmt(stmt) => Display::fmt(stmt, f),
            Self::Decl(decl) => Display::fmt(decl, f),
        }
    }
}

#[derive(Debug, Eq, Clone, Default)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Block {
    pub items: Vec<BlockItem>,
    pub span: Span,
}

impl Block {
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            span: Default::default(),
        }
    }

    /// Append a [`BlockItem`] to the end of the block.
    ///
    /// Equivalent to [`Vec<BlockItem>::push`].
    pub fn push(&mut self, item: BlockItem) {
        self.items.push(item);
    }

    /// Check if the block is empty.
    ///
    /// Equivalent to [`Vec<BlockItem>::is_empty`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Deref for Block {
    type Target = Vec<BlockItem>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for Block {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

impl IntoIterator for Block {
    type Item = BlockItem;
    type IntoIter = <Vec<BlockItem> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl From<Block> for Vec<BlockItem> {
    fn from(block: Block) -> Self {
        block.items
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{ {} }}", self.items.iter().join(" "))
    }
}

// TODO: Implement block! macro

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::TokenKind;
    use proptest::prelude::*;
    use rstest::rstest;

    mod ident {
        use super::*;

        #[rstest]
        #[case(" A", false)]
        fn test_is_ident(#[case] given: &'static str, #[case] expected: bool) {
            assert_eq!(Identifier::is_ident(given), expected);
        }

        proptest! {
            #[test]
            fn test_rename(mut id: Identifier, new_name: String) {
                let old_name = id.source_name().to_owned();

                prop_assume!(old_name != new_name);

                prop_assert_eq!(id.value(), &old_name);

                id.rename(new_name.clone());

                prop_assert_eq!(id.value(), &new_name);
                prop_assert_eq!(id.source_name(), &old_name);
            }
        }
    }

    mod op {
        use super::*;

        proptest! {
            #[test]
            fn unary_binary_ternary_are_separate(kind: TokenKind) {
                prop_assume!(kind.is_op());

                prop_assert_eq!(
                    kind.is_ternary_op(), !kind.is_binary_op() && !kind.is_unary_op(),
                    "if {:?} is a ternary operator, it shouldn't be a binary or unary operator",
                    kind
                );
            }

            #[test]
            fn ops_are_unop_binop_or_ternary(kind: TokenKind) {
                prop_assert_eq!(
                    kind.is_op(), kind.is_unary_op() || kind.is_binary_op() || kind.is_ternary_op(),
                    "if {:?} is an operator, it is either a unary, binary, or ternary operator",
                    kind
                );
            }
        }
    }
}
