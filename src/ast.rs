//! Type definitions for the AST.

use std::{
    fmt::{Debug, Display},
    ops::{self, Deref, DerefMut},
};

use contracts::{debug_requires, requires};
use cov_mark;
use itertools::Itertools;
#[cfg(test)]
use proptest::prelude::*;
#[cfg(test)]
use proptest_derive::Arbitrary;

use crate::src::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(Arbitrary))]
pub enum TokenKind {
    // Literals
    /// An [integer constant](https://cppreference.net/c/language/integer_constant.html) literal.
    Constant,
    /// An [identifier](https://cppreference.net/c/language/identifiers.html).
    Ident,

    // Operators
    /// `~`
    Complement,
    /// `-`
    Minus,
    /// `+`
    Plus,
    /// `/`
    Divide,
    /// `*`
    Star,
    /// `%`
    Modulo,
    /// `!`
    Not,
    /// `&`
    Ampersand,
    /// `&&`
    And,
    /// `|`
    Pipe,
    /// `||`
    Or,
    /// `=`
    Assign,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LT,
    /// `>`
    GT,
    /// `<=`
    LTE,
    /// `>=`
    GTE,
    /// `^`
    UpArrow,
    /// `<<`
    LShift,
    /// `>>`
    RShift,

    /// `?`
    Question,
    /// `:`
    Colon,

    /// `--`
    Decrement,
    /// `++`
    Increment,

    // Statement
    If,
    Else,

    // Structural
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `;`
    Semicolon,

    // Keywords
    /// `int`
    Int,
    /// `void`
    Void,
    /// `return`
    Return,

    #[cfg_attr(test, proptest(skip))]
    EOF,
    #[cfg_attr(test, proptest(value = "TokenKind::Error(\"test error\")"))]
    Error(&'static str),
}

/// Represents the [precedence](https://en.cppreference.com/c/language/operator_precedence) of a binary operator.
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
    /// Returns `true` if this `TokenKind` can be parsed into a [`UnOp`]
    ///
    /// # Examples
    /// ```
    /// # use crustydrageon::ast::TokenKind;
    /// assert_eq!(TokenKind::Minus.is_unary_op(), true);
    /// assert_eq!(TokenKind::Complement.is_unary_op(), true);
    /// assert_eq!(TokenKind::Not.is_unary_op(), true);
    ///
    /// assert_eq!(TokenKind::Semicolon.is_unary_op(), false);
    /// assert_eq!(TokenKind::LBrace.is_unary_op(), false);
    /// assert_eq!(TokenKind::RBrace.is_unary_op(), false);
    ///
    /// // binary operators which are not unary operators
    /// assert_eq!(TokenKind::Plus.is_unary_op(), false);
    /// assert_eq!(TokenKind::Star.is_unary_op(), false);
    /// assert_eq!(TokenKind::Divide.is_unary_op(), false);
    /// ```
    #[must_use]
    pub fn is_unary_op(&self) -> bool {
        use TokenKind::{Complement, Minus, Not};
        matches!(self, Minus | Complement | Not)
    }

    /// Returns `true` if this `TokenKind` can be parsed into a [`BinOp`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use crustydrageon::ast::TokenKind;
    /// assert_eq!(TokenKind::Plus.is_binary_op(), true);
    /// assert_eq!(TokenKind::Minus.is_binary_op(), true);
    /// assert_eq!(TokenKind::Star.is_binary_op(), true);
    /// assert_eq!(TokenKind::Divide.is_binary_op(), true);
    ///
    /// assert_eq!(TokenKind::Semicolon.is_binary_op(), false);
    /// assert_eq!(TokenKind::LBrace.is_binary_op(), false);
    /// assert_eq!(TokenKind::RBrace.is_binary_op(), false);
    ///
    /// // unary operators which are not binary operators
    /// assert_eq!(TokenKind::Complement.is_binary_op(), false);
    /// assert_eq!(TokenKind::Not.is_binary_op(), false);
    /// ```
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

    #[must_use]
    pub fn is_ternary_op(&self) -> bool {
        matches!(self, Self::Question)
    }

    #[must_use]
    pub fn is_op(&self) -> bool {
        self.is_unary_op() || self.is_binary_op() || self.is_ternary_op()
    }

    // TODO: this really should be a method of BinaryOp
    /// Get the [`Precedence`] of a binary operator token.
    ///
    /// Returns [`None`] for tokens which are not binary operators.
    ///
    /// ```
    /// # use crustydrageon::ast::TokenKind;
    ///
    /// assert_eq!(TokenKind::Plus.precedence(), TokenKind::Minus.precedence());
    /// assert!(TokenKind::RShift.precedence() > TokenKind::Equal.precedence());
    ///
    /// assert!(TokenKind::Semicolon.precedence().is_none());
    /// ```
    #[must_use]
    pub fn precedence(&self) -> Option<Precedence> {
        //! See <https://en.cppreference.com/c/language/operator_precedence>

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
            tk::Question => 3,
            tk::Assign => 1,
            kind if self.is_binary_op() => todo!("precedence value of {:?} binary operator", kind),
            _ => return None,
        }))
    }
}

/// A single lexical unit, comprised of a [kind](Token::kind) and a [lexeme](Token::lexeme).
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

    /// Rename (or 'mangle') this identifier, storing the original name in the name history.
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
    #[must_use] 
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
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            _ => TokenKind::Ident,
        }
    }

    #[must_use] 
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

/// Constructors for test dummies of nodes which don't include [`Span`] information, for example.
///
/// Used for easing the definition of test cases.
#[cfg(test)]
pub mod dummy {
    use super::*;

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
}

#[cfg(test)]
pub mod strategy {
    use crate::src::Source;
    use proptest::string::string_regex;

    use super::*;

    impl Arbitrary for Identifier {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            "[a-zA-Z_][0-9a-zA-Z_]"
                .prop_map(Source::new)
                .prop_map(Span::from)
                .prop_map(Identifier::from)
                .prop_filter("identifier should have an Ident TokenKind", |id| {
                    id.tok_kind() == TokenKind::Ident
                })
                .boxed()
        }
    }

    // This is a manual implementation as the Arbitrary derive impl overflows its stack because Expr is a recursive data structure
    impl Arbitrary for Expr {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
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
                            (any::<UnOp>(), inner.clone(), any::<Span>()).prop_map(
                                |(op, expr, span)| Expr {
                                    kind: ExprKind::Unary(op, Box::new(expr)),
                                    span
                                }
                            ),
                            (any::<BinOp>(), inner.clone(), inner.clone(), any::<Span>()).prop_map(
                                |(op, lhs, rhs, span)| {
                                    Expr {
                                        kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                                        span,
                                    }
                                }
                            ),
                            (inner.clone(), inner.clone(), inner, any::<Span>()).prop_map(
                                |(cond, if_true, if_false, span)| {
                                    Expr {
                                        kind: ExprKind::Cond(
                                            cond.into(),
                                            if_true.into(),
                                            if_false.into(),
                                        ),
                                        span,
                                    }
                                }
                            )
                        ]
                    },
                )
                .boxed()
        }
    }

    impl Arbitrary for Stmt {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            let leaf = prop_oneof![any::<Expr>().prop_map(Stmt::Expr), Just(Stmt::Null),];

            leaf.prop_recursive(
                3,  // max depth
                16, // max total size
                2,  // max branching factor
                |inner| {
                    prop_oneof![
                        any::<Expr>().prop_map(Stmt::Return),
                        (any::<Expr>(), inner.clone(), any::<bool>(), inner).prop_map(
                            |(cond, if_true, has_else, if_false)| {
                                let else_branch = if has_else {
                                    Some(Box::new(if_false))
                                } else {
                                    None
                                };
                                Stmt::If(cond, Box::new(if_true), else_branch)
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

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<TokenKind>()
                .prop_flat_map(|k| {
                    let regex_str = match k {
                        TokenKind::Constant => "[0-9]",
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
                        TokenKind::Question => "\\?",
                        TokenKind::Colon => ":",
                        TokenKind::If => "if",
                        TokenKind::Else => "else",
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
                // ensure that we never end up with an identifier token that should be a keyword
                .prop_map(|(kind, lexeme)| {
                    if matches!(kind, TokenKind::Ident) {
                        (Identifier::from(lexeme.clone()).tok_kind(), lexeme)
                    } else {
                        (kind, lexeme)
                    }
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

    /// See <https://en.cppreference.com/c/language/operator_precedence>
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
                vec![Question],
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
                    );
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
            fn test_binop_and_ternary_has_precedence(kind: TokenKind) {
                prop_assert_eq!(
                    kind.precedence() > Some(Precedence::default()),
                    kind.is_binary_op() || kind.is_ternary_op()
                );
                prop_assert_eq!(kind.precedence().is_some(), kind.is_binary_op() || kind.is_ternary_op());
            }

            #[test]
            fn test_symmetric(a: TokenKind, b: TokenKind) {
                prop_assert_eq!(a == b, b == a);
            }

            #[test]
            fn test_reflexive(kind: TokenKind) {
                prop_assert_eq!(kind.precedence(), kind.precedence());
            }

            #[test]
            fn test_substitution(a: TokenKind, b: TokenKind, c: TokenKind) {
                if (a == c) && (a == b) {
                    prop_assert_eq!( b , c);
                }
            }
        }
    }
}
