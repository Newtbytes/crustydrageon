use std::{fmt::Display, ops};

use contracts::{debug_requires, requires};
use cov_mark;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::src::{self, Span};

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

    #[cfg_attr(test, proptest(skip))]
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

impl From<Token> for Span {
    fn from(tok: Token) -> Self {
        tok.lexeme
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lexeme)
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
    #[requires(!self.names.is_empty())]
    pub fn value(&self) -> &str {
        self.names.last().unwrap()
    }

    pub fn rename(&mut self, name: String) {
        self.names.push(name);
    }

    /// Get the original, demangled name from the name history.
    #[requires(!self.names.is_empty())]
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
    #[debug_requires(Self::is_ident(self.source_name()))]
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

impl From<Identifier> for Span {
    fn from(id: Identifier) -> Self {
        id.span
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
pub enum UnOpKind {
    // Arithmetic
    Negate,

    // Bitwise
    Complement,

    // Logical
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

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct UnOp {
    pub kind: UnOpKind,
    pub span: Span,
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

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub struct BinOp {
    pub kind: BinOpKind,
    pub span: Span,
}

/// Expression
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExprKind {
    Const(i32),
    Var(Identifier),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

impl Display for ExprKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Const(val) => write!(f, "{val}"),
            Self::Var(id) => write!(f, "{}", id.source_name()),
            Self::Unary(op, expr) => write!(f, "{op}{expr}"),
            Self::Binary(op, a, b) => write!(f, "{a} {op} {b}"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Const(val) => write!(f, "{val}"),
            Self::Var(id) => write!(f, "{}", id.source_name()),
            Self::Unary(op, expr) => write!(f, "{op}{expr}"),
            Self::Binary(op, a, b) => write!(f, "{a} {op} {b}"),
        }
    }
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
    pub span: src::Span,
}

#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum BlockItem {
    Stmt(Stmt),
    Decl(Decl),
}

impl Display for Stmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Expr(expr) => write!(f, "{};", expr),
            Self::Return(expr) => write!(f, "return {};", expr),
            Self::Null => write!(f, ";"),
        }
    }
}

/// Constructors for test dummies of AST nodes which don't include [`Span`] information, for example.
///
/// Used for easing the definition of test cases.
#[cfg(test)]
pub mod dummy {
    use super::*;

    pub fn ident(value: &str) -> Identifier {
        Identifier {
            names: vec![value.to_owned()],
            span: Span::default(),
        }
    }

    pub fn expr(kind: ExprKind) -> Expr {
        Expr {
            kind,
            span: Span::default(),
        }
    }

    pub fn unop(kind: UnOpKind) -> UnOp {
        UnOp {
            kind,
            span: Span::default(),
        }
    }

    pub fn binop(kind: BinOpKind) -> BinOp {
        BinOp {
            kind,
            span: Span::default(),
        }
    }

    impl Expr {
        pub fn constant(value: i32) -> Self {
            expr(ExprKind::Const(value))
        }

        pub fn var(name: &str) -> Self {
            expr(ExprKind::Var(ident(name)))
        }

        pub fn unary(kind: UnOpKind, operand: Expr) -> Self {
            expr(ExprKind::Unary(unop(kind), operand.into()))
        }

        pub fn binary(kind: BinOpKind, a: Expr, b: Expr) -> Self {
            expr(ExprKind::Binary(binop(kind), a.into(), b.into()))
        }
    }
}

#[cfg(test)]
pub mod strategy {
    use crate::src::Source;
    use proptest::string::string_regex;

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
                any::<i32>().prop_map(ExprKind::Const),
                any::<Identifier>().prop_map(ExprKind::Var),
            ];

            (leaf, any::<Span>())
                .prop_map(|(kind, span)| Expr { kind, span })
                .prop_recursive(
                    3,  // max depth
                    16, // max total size
                    2,  // max branching factor
                    |inner| {
                        prop_oneof![
                            (any::<UnOp>(), inner.clone(), any::<src::Span>()).prop_map(
                                |(op, expr, span)| Expr {
                                    kind: ExprKind::Unary(op, Box::new(expr)),
                                    span
                                }
                            ),
                            (any::<BinOp>(), inner.clone(), inner, any::<src::Span>()).prop_map(
                                |(op, lhs, rhs, span)| {
                                    Expr {
                                        kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                                        span,
                                    }
                                }
                            ),
                        ]
                    },
                )
                .boxed()
        }
    }

    impl Arbitrary for Token {
        type Parameters = ();
        type Strategy = proptest::prelude::BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            any::<TokenKind>()
                .prop_flat_map(|k| {
                    let regex_str = match k {
                        TokenKind::Constant => "[0-9]",
                        // FIXME: ensure keywords are never generated here
                        TokenKind::Ident => "[_a-zA-Z][_a-zA-Z0-9]+",
                        TokenKind::Complement => "~",
                        TokenKind::Minus => "-",
                        TokenKind::Plus => "\\+",
                        TokenKind::Divide => "/",
                        TokenKind::Star => "\\*",
                        TokenKind::Modulo => "%",
                        TokenKind::Not => "!",
                        TokenKind::And => "&&",
                        TokenKind::Or => "\\|\\|",
                        TokenKind::Equal => "==",
                        TokenKind::NotEqual => "!=",
                        TokenKind::LT => "<",
                        TokenKind::GT => ">",
                        TokenKind::LTE => "<=",
                        TokenKind::GTE => ">=",
                        TokenKind::Decrement => "--",
                        TokenKind::Increment => "\\+\\+",
                        TokenKind::LParen => "\\(",
                        TokenKind::RParen => "\\)",
                        TokenKind::LBrace => "\\{",
                        TokenKind::RBrace => "\\}",
                        TokenKind::Semicolon => ";",
                        TokenKind::Int => "int",
                        TokenKind::Void => "void",
                        TokenKind::Return => "return",
                        TokenKind::Ampersand => "&",
                        TokenKind::Pipe => "\\|",
                        TokenKind::UpArrow => "\\^",
                        TokenKind::LShift => "<<",
                        TokenKind::RShift => ">>",
                        TokenKind::Assign => "=",
                        TokenKind::EOF => "",
                        TokenKind::Error(_) => "",
                    };
                    (
                        Just(k),
                        string_regex(regex_str)
                            .expect("valid regex")
                            .prop_map(Source::new)
                            .prop_map(Span::from),
                    )
                })
                .prop_map(|(kind, lexeme)| Token::new(kind, lexeme))
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
