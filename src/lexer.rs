use std::{
    iter::{self, Peekable},
    str::Chars,
};

use contracts::ensures;
#[allow(unused_imports)]
use itertools::Itertools;

use crate::{
    ast::{Identifier, Token, TokenKind},
    src::{Source, Span},
};

/// Corresponds to the word character class, or the `\w` regex pattern.
fn is_word(c: &char) -> bool {
    matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '_')
}

/// Tokenize an identifier string as a keyword or identifier [`TokenKind`].
///
/// Returns [`None`] if the input is not an identifier.
fn classify_ident(s: &str) -> Option<TokenKind> {
    Some(match s {
        "int" => TokenKind::Int,
        "void" => TokenKind::Void,
        "return" => TokenKind::Return,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "do" => TokenKind::Do,
        "while" => TokenKind::While,
        "for" => TokenKind::For,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        s if Identifier::is_ident(s) => TokenKind::Ident,
        _ => return None,
    })
}

pub struct Lexer<'src> {
    src: &'src Source,
    chars: Peekable<Chars<'src>>,
    consumed: Span,
}

impl<'src> Lexer<'src> {
    #[must_use]
    pub fn new(src: &'src Source) -> Self {
        Lexer {
            src,
            chars: src.chars().peekable(),
            consumed: Span::empty_at(src, 0).unwrap(),
        }
    }

    /// Reset the consumed token, setting it to start at the next character.
    fn end_token(&mut self) {
        if self.one_ahead().is_some() {
            self.consumed = self.src.empty_at(self.consumed.end_index()).unwrap();
        }
    }

    fn one_ahead(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Eat one [`char`] from the source, extending the consumed token by one character.
    #[ensures(
        ret.is_some() -> old(self.consumed.len()) < self.consumed.len(),
        "eating one [`char`] increases the length of the consumed [`Token`]"
    )]
    #[ensures(
        old(self.consumed.len()) == self.consumed.len() -> ret.is_none(),
        "if the [`consumed`](Lexer::consumed) length is unchanged, then [`None`] was returned"
    )]
    fn eat(&mut self) -> Option<char> {
        let c = self.chars.next()?;

        self.consumed = self
            .src
            .subspan(
                self.consumed.start_index(),
                self.consumed.end_index() + c.len_utf8(),
            )
            .unwrap();

        Some(c)
    }

    fn eat_if<P>(&mut self, mut predicate: P) -> Option<char>
    where
        P: FnMut(&char) -> bool,
    {
        if predicate(self.one_ahead()?) {
            self.eat()
        } else {
            None
        }
    }

    fn eat_while<P>(&mut self, mut predicate: P)
    where
        P: FnMut(&char) -> bool,
    {
        while self.eat_if(&mut predicate).is_some() {
            continue;
        }
    }

    fn eat_until<P>(&mut self, mut predicate: P)
    where
        P: FnMut(&char) -> bool,
    {
        self.eat_while(|c| !predicate(c));
    }

    fn eat_identifier(&mut self) {
        self.eat_while(is_word);
        cov_mark::hit!(lex_identifier_eaten);
    }

    fn eat_int_literal(&mut self) {
        self.eat_while(char::is_ascii_digit);
        cov_mark::hit!(lex_int_literal_eaten);
    }

    fn emit(&mut self, kind: TokenKind) -> Token {
        let tok = Token::new(kind, self.consumed.clone());

        self.end_token();

        cov_mark::hit!(lex_token_emitted);

        tok
    }

    fn at_word_bound(&mut self) -> bool {
        match self.one_ahead() {
            Some(c) => !is_word(c),
            None => true,
        }
    }

    /// After finding one character, tokenize based on the next character.
    /// If it does, return one token kind, otherwise return a second token kind.
    fn based_on_next(&mut self, expected: char, single: TokenKind, double: TokenKind) -> TokenKind {
        match self.eat_if(|&c| c == expected) {
            Some(_) => double,
            None => single,
        }
    }

    fn error(&self, msg: &'static str) -> TokenKind {
        TokenKind::Error(msg)
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        use TokenKind as tk;

        // skip whitespace
        self.eat_while(|&c| c.is_whitespace());
        self.end_token();

        let kind = match self.eat() {
            Some(c) => match c {
                // structural tokens
                '(' => tk::LParen,
                ')' => tk::RParen,
                '{' => tk::LBrace,
                '}' => tk::RBrace,
                ';' => tk::Semicolon,

                // operators
                '~' => tk::Complement,
                '*' => tk::Star,
                '/' => tk::Divide,
                '%' => tk::Modulo,
                '^' => tk::UpArrow,
                '+' => self.based_on_next('+', tk::Plus, tk::Increment),
                '-' => self.based_on_next('-', tk::Minus, tk::Decrement),
                '!' => self.based_on_next('=', tk::Not, tk::NotEqual),
                '&' => self.based_on_next('&', tk::Ampersand, tk::And),
                '|' => self.based_on_next('|', tk::Pipe, tk::Or),
                '=' => self.based_on_next('=', tk::Assign, tk::Equal),
                '<' => match self.eat_if(|c| matches!(c, '=' | '<')) {
                    Some('=') => tk::LTE,
                    Some('<') => tk::LShift,
                    Some(_) => unreachable!(),
                    None => tk::LT,
                },
                '>' => match self.eat_if(|c| matches!(c, '=' | '>')) {
                    Some('=') => tk::GTE,
                    Some('>') => tk::RShift,
                    Some(_) => unreachable!(),
                    _ => tk::GT,
                },
                ':' => tk::Colon,
                '?' => tk::Question,

                // identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => {
                    self.eat_identifier();

                    if self.at_word_bound() {
                        // handle keywords
                        classify_ident(self.consumed.as_str()).unwrap()
                    } else {
                        // if the next character isn't \b
                        self.error("Invalid identifier")
                    }
                }

                // integer literals
                c if c.is_ascii_digit() => {
                    self.eat_int_literal();

                    if self.at_word_bound() {
                        tk::Constant
                    } else {
                        self.error("Invalid constant")
                    }
                }

                _ => self.error("Unexpected character"),
            },
            None => return None,
        };

        // synchronize by eating until synchronization point
        if let tk::Error(_) = kind {
            self.eat_until(|&c| !is_word(&c));
        }

        Some(self.emit(kind))
    }
}

/// Tokenize a [`Source`] into an iterator of [`Token`]s using the frontend's lexical analysis
///
/// # Returns
/// An iterator of tokens, or an empty iterator if src is empty.
#[must_use]
pub fn tokenize(src: &Source) -> Box<dyn Iterator<Item = Token> + '_> {
    if src.is_empty() {
        Box::new(iter::empty::<Token>())
    } else {
        Box::new(Lexer::new(src))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ast::{Identifier, TokenKind as tk};

    use proptest::prelude::*;
    use rstest::rstest;

    #[test]
    fn test_tokenize_returns_empty() {
        let src = Source::new(String::new());
        let tokens = tokenize(&src);

        assert_eq!(tokens.count(), 0);
    }

    #[rstest]
    //# single tokens
    //## operators
    //### arithmetic
    #[case::minus("-", [tk::Minus])]
    #[case::plus("+", [tk::Plus])]
    #[case::star("*", [tk::Star])]
    #[case::divide("/", [tk::Divide])]
    #[case::modulo("%", [tk::Modulo])]
    //### conditional
    #[case::question("?", [tk::Question])]
    #[case::colon(":", [tk::Colon])]
    //### mutating
    #[case::assign("=", [tk::Assign])]
    #[case::decrement("--", [tk::Decrement])]
    #[case::increment("++", [tk::Increment])]
    //### logical
    #[case::not("!", [tk::Not])]
    #[case::and("&&", [tk::And])]
    #[case::or("||", [tk::Or])]
    #[case::equal("==", [tk::Equal])]
    #[case::not_equal("!=", [tk::NotEqual])]
    #[case::lt("<", [tk::LT])]
    #[case::gt(">", [tk::GT])]
    #[case::lte("<=", [tk::LTE])]
    #[case::gte(">=", [tk::GTE])]
    //### bitwise
    #[case::ampersand("&", [tk::Ampersand])]
    #[case::pipe("|", [tk::Pipe])]
    #[case::up_arrow("^", [tk::UpArrow])]
    //## identifiers & keywords
    #[case::ident_a("a", [tk::Ident])]
    #[case::void("void", [tk::Void])]
    #[case::int("int", [tk::Int])]
    #[case::return_kw("return", [tk::Return])]
    #[case::if_kw("if", [tk::If])]
    #[case::else_kw("else", [tk::Else])]
    #[case::do_kw("do", [tk::Do])]
    #[case::while_kw("while", [tk::While])]
    #[case::for_kw("for", [tk::For])]
    #[case::break_kw("break", [tk::Break])]
    #[case::continue_kw("continue", [tk::Continue])]
    #[case::else_kw("else", [tk::Else])]
    //## literals
    //### int
    #[case::const_1("1", [tk::Constant])]
    #[case::const_2("2", [tk::Constant])]
    #[case::const_3("3", [tk::Constant])]
    #[case::const_10("10", [tk::Constant])]
    #[case::const_42("42", [tk::Constant])]
    #[case::const_69("69", [tk::Constant])]
    #[case::const_360("360", [tk::Constant])]
    #[case::const_720("720", [tk::Constant])]
    //#### error cases
    #[case::error_0aaaaa("0aaaaa", [tk::Error("Invalid constant")])]
    //# combinations
    //## plus, minus, increment, decrement
    #[case::minus_a("-a", [tk::Minus, tk::Ident])]
    #[case::minus_space_a("- a", [tk::Minus, tk::Ident])]
    #[case::plus_a("+a", [tk::Plus, tk::Ident])]
    #[case::decrement_a("--a", [tk::Decrement, tk::Ident])]
    #[case::increment_b("++b", [tk::Increment, tk::Ident])]
    #[case::a_decrement("a--", [tk::Ident, tk::Decrement])]
    #[case::b_increment("b++", [tk::Ident, tk::Increment])]
    #[case::five_plus("+++++", [
        tk::Increment,
        tk::Increment,
        tk::Plus,
    ])]
    #[case::five_plus_dup("+++++", [
        tk::Increment,
        tk::Increment,
        tk::Plus,
    ])]
    #[case::plus_space_plus_space_increment_plus("+ + +++", [
        tk::Plus,
        tk::Plus,
        tk::Increment,
        tk::Plus,
    ])]
    #[case::plus_space_plus_space_plus_space_plus("+ + + +", [
        tk::Plus,
        tk::Plus,
        tk::Plus,
        tk::Plus,
    ])]
    #[case::five_minus("-----", [
        tk::Decrement,
        tk::Decrement,
        tk::Minus,
    ])]
    #[case::minus_space_minus_space_decrement_minus("- - ---", [
        tk::Minus,
        tk::Minus,
        tk::Decrement,
        tk::Minus,
    ])]
    #[case::minus_space_minus_space_minus_space_minus("- - - -", [
        tk::Minus,
        tk::Minus,
        tk::Minus,
        tk::Minus,
    ])]
    #[case::mixed_operators("++++%-*---", [
        tk::Increment,
        tk::Increment,
        tk::Modulo,
        tk::Minus,
        tk::Star,
        tk::Decrement,
        tk::Minus,
    ])]
    //## multiply, divide, and modulo and their combinations
    #[case::star_a("*a", [tk::Star, tk::Ident])]
    #[case::divide_b("/b", [tk::Divide, tk::Ident])]
    #[case::modulo_c("%c", [tk::Modulo, tk::Ident])]
    //## repeated operators
    #[case::four_stars("****", [tk::Star, tk::Star, tk::Star, tk::Star])]
    #[case::four_divides("////", [tk::Divide, tk::Divide, tk::Divide, tk::Divide])]
    #[case::four_modulos("%%%%", [tk::Modulo, tk::Modulo, tk::Modulo, tk::Modulo])]
    //## identifiers and keywords
    #[case::voidx("voidx", [tk::Ident])]
    #[case::int_("int_", [tk::Ident])]
    #[case::a0aaaa("a0aaaa", [tk::Ident])]
    fn test_tokenize_operators<const S: usize>(#[case] src: &str, #[case] expected: [tk; S]) {
        let src = Source::new(src.to_owned());
        let tokens = tokenize(&src).collect::<Vec<Token>>();

        assert_eq!(
            tokens.len(),
            S,
            "expected {} tokens, got {}: {:?}",
            tokens.len(),
            S,
            tokens.iter().map(Token::kind).collect::<Vec<tk>>()
        );

        // check that the token kinds match the expected kinds
        for (tok, &kind) in tokens.iter().zip(expected.iter()) {
            assert_eq!(tok.kind(), kind);
        }
    }

    fn is_keyword(s: &str) -> bool {
        match classify_ident(s) {
            Some(kind) => !matches!(kind, tk::Ident),
            None => false,
        }
    }

    proptest! {
        #[test]
        fn test_is_word(s in r"[a-zA-Z_0-9]" /* FIXME: should be \w but unicode isn't supported yet */) {
            let c = s.chars().next().unwrap();
            assert!(is_word(&c));
        }

        #[test]
        fn test_tokenize_identifiers(s: Identifier) {
            // skip keywords
            prop_assume!(!is_keyword(&s.to_string()));

            let src = Source::new(s.to_string());
            let tokens = tokenize(&src).collect::<Vec<Token>>();

            prop_assert_eq!(tokens.len(), 1);
            prop_assert_eq!(tokens[0].kind(), tk::Ident);
            prop_assert_eq!(tokens[0].span().to_string(), s.to_string());
        }

        /// Test that identifier tokenization in the [`Lexer`] and [`Identifier::is_ident`] agree.
        #[test]
        fn test_is_ident_equivalence(s: String) {
            // skip keywords
            prop_assume!(!is_keyword(&s.clone()));
            prop_assume!(!s.is_empty());

            // the lexer skips whitespace, but Identifier::is_ident does not, so we trim the input
            let s = s.trim();

            fn lexer_is_ident(s: &str) -> bool {
                let src = Source::new(s.to_string());
                let tokens = tokenize(&src).collect::<Vec<Token>>();

                tokens.len() == 1 && tokens[0].kind() == tk::Ident
            }

            prop_assert_eq!(Identifier::is_ident(s), lexer_is_ident(s));
        }

        #[test]
        fn test_tokenize_round_trip(
            tokens in prop::collection::vec(
                any::<Token>().prop_filter("is not EOF or error token", |tok| {
                    !matches!(tok.kind(), tk::Error(_))
                }),
                1..=10,
            )
        ) {
            let src: Source = tokens.iter().join(" ").into();

            prop_assert_eq!(
                tokenize(&src).join(" "),
                tokens.iter().join(" "),
            );
        }

        #[test]
        fn test_classify_keyword_equivalent_to_ident_tok_kind(s in "[a-zA-Z_][0-9a-zA-Z_]") {
            let id = Identifier::from(Span::from(Source::new(s.clone())));

            prop_assert_eq!(Some(id.tok_kind()), classify_ident(&s));
        }
    }
}
