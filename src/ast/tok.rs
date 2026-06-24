use std::{fmt::Display, ops};

use contracts::{debug_requires, requires};
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

/// Keywords and user-defined identifiers (e.g. function names or variable names).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Identifier {
    pub(super) names: Vec<String>,
    pub(super) span: Span,
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

#[cfg(test)]
mod tests {
    use super::*;

    use proptest::prelude::*;
    use rstest::{fixture, rstest};

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
