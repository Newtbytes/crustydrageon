use std::{fmt::Display, ops};

use contracts::{debug_requires, requires};
use cov_mark;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

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
    Not,        // !
    Ampersand,  // &
    And,        // &&
    Pipe,       // |
    Or,         // ||
    Assign,     // =
    Equal,      // ==
    NotEqual,   // !=
    LT,         // <
    GT,         // >
    LTE,        // <=
    GTE,        // >=
    UpArrow,    // ^
    LShift,     // <<
    RShift,     // >>

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
    #[cfg_attr(test, proptest(value = "TokenKind::Error(\"test error\")"))]
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
        use TokenKind::{Complement, Minus, Not};
        matches!(self, Minus | Complement | Not)
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
                | tk::Ampersand
                | tk::Pipe
                | tk::UpArrow
                | tk::LShift
                | tk::RShift
                | tk::Assign
        )
    }

    // TODO: this really should be a method of BinaryOp
    #[must_use]
    pub fn precedence(&self) -> Option<Precedence> {
        //! See https://en.cppreference.com/c/language/operator_precedence

        use TokenKind as tk;

        cov_mark::hit!(binary_op_precedence);

        Some(Precedence(match self {
            tk::Star | tk::Divide | tk::Modulo => 50,
            tk::Plus | tk::Minus => 45,
            tk::LShift | tk::RShift => 40,
            tk::LT | tk::LTE | tk::GT | tk::GTE => 35,
            tk::Equal | tk::NotEqual => 30,
            tk::Ampersand => 16,
            tk::UpArrow => 14,
            tk::Pipe => 12,
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

/// Keywords and user-defined identifiers (e.g. function names or variable names).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Identifier {
    names: Vec<String>,
    span: Span,
}

impl Identifier {
    /// Return the current value of the identifier, including all mangling that has been done so far.
    #[requires(self.names.len() >0)]
    pub fn value(&self) -> &str {
        self.names.last().unwrap()
    }

    pub fn rename(&mut self, name: String) {
        self.names.push(name);
    }

    /// Get the original, demangled name from the name history.
    #[requires(self.names.len() >0)]
    pub fn source_name(&self) -> &str {
        self.names.first().unwrap()
    }

    /// Check if a given string is a valid identifier.
    ///
    /// # Examples
    ///
    /// Valid identifiers can contain alphabetic characters, numeric characters, or the character '_'.
    ///
    /// ```rust
    /// # use crustydrageon::ast::Identifier;
    /// assert_eq!(Identifier::is_ident("abcdef"), true);
    /// assert_eq!(Identifier::is_ident("abc_def"), true);
    /// assert_eq!(Identifier::is_ident("hello_world"), true);
    /// assert_eq!(Identifier::is_ident("x1"), true);
    /// assert_eq!(Identifier::is_ident("x2"), true);
    /// ```
    ///
    /// Other characters, such as special characters or whitespace, are not allowed.
    ///
    /// ```rust
    /// # use crustydrageon::ast::Identifier;
    /// assert_eq!(Identifier::is_ident("hello world"), false);
    /// assert_eq!(Identifier::is_ident("Hello, world!"), false);
    /// ```
    ///
    /// Empty strings are not identifiers.
    ///
    /// ```rust
    /// # use crustydrageon::ast::Identifier;
    /// assert_eq!(Identifier::is_ident(""), false);
    /// ```
    pub fn is_ident(s: &str) -> bool {
        cov_mark::hit!(ast_is_ident);

        s.chars().next().is_some_and(|c: char| !c.is_ascii_digit())
            && s.chars()
                .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '_' | '0'..='9'))
    }

    /// Get the [`TokenKind`] of this [`Identifier`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::{src, ast::{Identifier, TokenKind}};
    ///
    /// let mut id = Identifier { value: "hello_world".to_owned(), span: src::Span::default() };
    ///
    /// assert_eq!(id.tok_kind(), TokenKind::Ident);
    ///
    /// id.value = "int".to_owned();
    /// assert_eq!(id.tok_kind(), TokenKind::Int);
    ///
    /// id.value = "return".to_owned();
    /// assert_eq!(id.tok_kind(), TokenKind::Return);
    /// ```
    #[debug_requires(Self::is_ident(&self.source_name()))]
    pub fn tok_kind(&self) -> TokenKind {
        match self.source_name() {
            "int" => TokenKind::Int,
            "void" => TokenKind::Void,
            "return" => TokenKind::Return,
            _ => TokenKind::Ident,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.span)
    }
}

impl From<Span> for Identifier {
    #[requires(Identifier::is_ident(&span.to_string()))]
    fn from(span: Span) -> Self {
        let value = span.to_string();
        Self {
            span,
            names: vec![value],
        }
    }
}

/// Function definition
#[derive(Debug, Clone)]
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

    // Bitwise
    BitAnd,
    BitOr,
    Xor,
    LShift,
    RShift,

    // Assignment
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

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct Decl {
    pub name: Identifier,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BlockItem {
    Stmt(Stmt),
    Decl(Decl),
}

#[cfg(test)]
pub mod strategy {

    use crate::src::Source;

    use super::*;

    impl Arbitrary for Identifier {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            "[a-zA-Z_][0-9a-zA-Z_]"
                .prop_map(Source::new)
                .prop_map(Span::from)
                .prop_map(Identifier::from)
                .boxed()
        }
    }

    // This is a manual implementation as the Arbitrary derive impl overflows its stack because Expr is a recursive data structure
    impl Arbitrary for Expr {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

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

    use rstest::{fixture, rstest};

    proptest! {
        #[test]
        fn test_identifier_rename(mut id: Identifier, new_name: String) {
            let old_name = id.source_name().to_owned();

            prop_assume!(old_name != new_name);

            prop_assert_eq!(id.value(), &old_name);

            id.rename(new_name.clone());

            prop_assert_eq!(id.value(), &new_name);
            prop_assert_eq!(id.source_name(), &old_name);
        }
    }

    /// See https://en.cppreference.com/c/language/operator_precedence
    mod precedence {
        use super::*;

        use TokenKind::*;

        #[fixture]
        fn precedence_groups() -> Vec<Vec<TokenKind>> {
            vec![
                vec![Star, Divide, Modulo],
                vec![Minus, Plus],
                vec![LShift, RShift],
                vec![LT, LTE, GT, GTE],
                vec![Equal, NotEqual],
                vec![Ampersand],
                vec![Pipe],
                vec![And],
                vec![Or],
                vec![Assign],
            ]
        }

        #[rstest]
        fn test_prec_group_equality(#[from(precedence_groups)] groups: Vec<Vec<TokenKind>>) {
            for group in groups {
                if let Some(first) = group.first() {
                    assert!(
                        group
                            .iter()
                            .all(|item| item.precedence() == first.precedence())
                    )
                }
            }
        }

        #[rstest]
        fn test_prec_group_relativity(#[from(precedence_groups)] groups: Vec<Vec<TokenKind>>) {
            let precedences: Vec<Precedence> = groups
                .iter()
                .filter_map(|g| {
                    g.first()
                        .map(|k| k.precedence().expect("should have precedence"))
                })
                .rev()
                .collect();

            assert!(precedences.is_sorted(), "not sorted: {precedences:?}");
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
