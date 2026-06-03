use std::ops;

use cov_mark;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use test_strategy::Arbitrary;

use crate::src::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum TokenKind {
    // Literals
    Constant,
    Ident,

    // Operators
    Complement, // ~
    Minus,      // -
    Plus,       // +
    Divide,     // /
    Star,       // *
    Modulo,     // %
    LogicNot,   // !
    And,        // &&
    Or,         // ||
    Assign,     // =
    Equal,      // ==
    NotEqual,   // !=
    LT,         // <
    GT,         // >
    LTE,        // <=
    GTE,        // >=

    Decrement, // ++
    Increment, // --

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

    EOF,
    // TODO: Arbitrary Error TokenKinds
    #[cfg_attr(test, weight(0))]
    Error(&'static str),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Precedence(usize);

impl ops::Add<usize> for Precedence {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl TokenKind {
    #[must_use]
    pub fn is_unary_op(&self) -> bool {
        use TokenKind::{Complement, LogicNot, Minus};
        matches!(self, Minus | Complement | LogicNot)
    }

    #[must_use]
    pub fn is_binary_op(&self) -> bool {
        use TokenKind as tk;
        matches!(
            self,
            tk::Plus
                | tk::Minus
                | tk::Divide
                | tk::Star
                | tk::Modulo
                | tk::And
                | tk::Or
                | tk::Equal
                | tk::NotEqual
                | tk::LT
                | tk::LTE
                | tk::GT
                | tk::GTE
                | tk::Assign
        )
    }

    // TODO: this really should be a method of BinaryOp
    #[must_use]
    pub fn precedence(&self) -> Option<Precedence> {
        use TokenKind as tk;

        cov_mark::hit!(binary_op_precedence);

        Some(Precedence(match self {
            tk::Star | tk::Divide | tk::Modulo => 50,
            tk::Plus | tk::Minus => 45,
            tk::LT | tk::LTE | tk::GT | tk::GTE => 35,
            tk::Equal | tk::NotEqual => 30,
            tk::And => 10,
            tk::Or => 5,
            tk::Assign => 1,
            kind if self.is_binary_op() => todo!("precedence value of {:?} binary operator", kind),
            _ => return None,
        }))
    }
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
#[cfg_attr(test, derive(Arbitrary))]
pub struct Program {
    pub body: Function,
}

/// User-defined identifier (function names, variable names, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Identifier {
    pub value: String,
    pub span: Span,
}

/// Function definition
#[derive(Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Function {
    pub name: Identifier,
    pub body: Vec<BlockItem>,
}

impl Function {
    #[must_use]
    pub fn new(name: Identifier, body: Vec<BlockItem>) -> Self {
        Function { name, body }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum UnaryOp {
    // Arithmetic
    Negate,

    // Bitwise
    Complement,

    // Logical
    Not,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BinaryOp {
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

    Assign,
}

/// Expression
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr {
    Const(i32),
    Var(Identifier),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
}

/// Statement
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum Stmt {
    Expr(Expr),
    Return(Expr),
    Null,
}

#[derive(Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Decl {
    pub name: Identifier,
    pub init: Option<Expr>,
}

#[derive(Debug)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BlockItem {
    Stmt(Stmt),
    Decl(Decl),
}

#[cfg(test)]
pub mod strategy {
    use super::*;

    // This is a manual implementation as the Arbitrary derive impl overflows its stack because Expr is a recursive data structure
    impl Arbitrary for Expr {
        type Parameters = ();
        type Strategy = proptest::prelude::BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            let leaf = prop_oneof![
                any::<i32>().prop_map(Expr::Const),
                any::<Identifier>().prop_map(Expr::Var),
            ];

            leaf.prop_recursive(
                3,  // max depth
                16, // max total size
                2,  // max branching factor
                |inner| {
                    prop_oneof![
                        (any::<UnaryOp>(), inner.clone())
                            .prop_map(|(op, expr)| Expr::Unary(op, Box::new(expr))),
                        (any::<BinaryOp>(), inner.clone(), inner).prop_map(|(op, lhs, rhs)| {
                            Expr::Binary(op, Box::new(lhs), Box::new(rhs))
                        }),
                    ]
                },
            )
            .boxed()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod precedence {
        use super::*;

        #[test]
        fn test_groups() {
            cov_mark::check!(binary_op_precedence);

            // Check that the precedence of *, /, % are equal
            assert_eq!(TokenKind::Star.precedence(), TokenKind::Divide.precedence());
            assert_eq!(
                TokenKind::Divide.precedence(),
                TokenKind::Modulo.precedence()
            );

            // Check that the precedence of the above precedence group is greater than the below group
            assert!(TokenKind::Star.precedence() > TokenKind::Plus.precedence());

            // Check that the precedence of +, - are equal
            assert_eq!(TokenKind::Minus.precedence(), TokenKind::Plus.precedence());

            // Check that the precedence of the above precedence group is greater than the below group
            assert!(TokenKind::Plus.precedence() > TokenKind::LT.precedence());

            // Check precedence of comparison operators
            assert_eq!(TokenKind::LT.precedence(), TokenKind::LTE.precedence());
            assert_eq!(TokenKind::LTE.precedence(), TokenKind::GT.precedence());
            assert_eq!(TokenKind::GT.precedence(), TokenKind::GTE.precedence());

            // Check that the precedence of && and || is less than the above
            assert!(TokenKind::GTE.precedence() > TokenKind::And.precedence());
            assert!(TokenKind::And.precedence() > TokenKind::Or.precedence());
        }

        proptest! {
            #[test]
            fn test_binop_has_precedence(kind: TokenKind) {
                prop_assert_eq!(kind.precedence() > Some(Precedence::default()), kind.is_binary_op());
            }
        }

        proptest! {
            #[test]
            fn test_symmetric(a: TokenKind, b: TokenKind) {
                prop_assert_eq!(a == b, b == a);
            }
        }

        proptest! {
            #[test]
            fn test_reflexive(kind: TokenKind) {
                prop_assert_eq!(kind.precedence(), kind.precedence());
            }
        }

        proptest! {
            #[test]
            fn test_substitution(a: TokenKind, b: TokenKind, c: TokenKind) {
                if (a == c) && (a == b) {
                    prop_assert_eq!( b , c);
                }
            }
        }
    }
}
