use std::{
    iter::{self, Peekable},
    str::Chars,
};

use crate::{
    ast::{Token, TokenKind},
    src::{Source, Span},
};

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

    fn get_consumed(&self) -> &str {
        self.consumed
            .get(self.src)
            .expect("Consumed span should always be valid for the source string")
    }

    /// Reset the consumed token, setting it to start at the next character
    fn end_token(&mut self) {
        if self.one_ahead().is_some() {
            self.consumed
                .point_to(self.src, self.consumed.end_index())
                .unwrap();
        } else {
            self.consumed.clear();
        }
    }

    fn one_ahead(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn eat(&mut self) -> Option<char> {
        let c = self.chars.next()?;

        self.consumed.push_char(c);

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

fn is_word(c: &char) -> bool {
    matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '_')
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        use TokenKind as tk;

        // skip whitespace
        self.eat_while(|&c| c.is_whitespace());
        self.end_token();

        macro_rules! todo_token {
            ($msg:literal) => {
                self.error(concat!("not yet implemented: ", $msg))
            };
        }

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
                '-' | '+' => {
                    if self.eat_if(|c| matches!(c, '-' | '+')).is_some() {
                        match c {
                            '-' => tk::Decrement,
                            '+' => tk::Increment,
                            _ => unreachable!(),
                        }
                    } else {
                        match c {
                            '-' => tk::Minus,
                            '+' => tk::Plus,
                            _ => unreachable!(),
                        }
                    }
                }
                '!' => self.based_on_next('=', tk::LogicNot, tk::NotEqual),
                '&' => self.based_on_next(
                    '&',
                    todo_token!("tokenizing bitwise operators: ampersand token"),
                    tk::And,
                ),
                '|' => self.based_on_next(
                    '|',
                    todo_token!("tokenizing bitwise operators: pipe token"),
                    tk::Or,
                ),
                '=' => self.based_on_next('=', tk::Assign, tk::Equal),
                '<' => self.based_on_next('=', tk::LT, tk::LTE),
                '>' => self.based_on_next('=', tk::GT, tk::GTE),

                // identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => {
                    self.eat_identifier();

                    if self.at_word_bound() {
                        // handle keywords
                        match self.get_consumed() {
                            "void" => tk::Void,
                            "int" => tk::Int,
                            "return" => tk::Return,

                            _ => tk::Ident,
                        }
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

    use rstest::rstest;

    #[test]
    fn test_tokenize_returns_empty() {
        let src = Source::new("".to_owned());
        let tokens = tokenize(&src);

        assert_eq!(tokens.count(), 0);
    }

    #[rstest]
    // plus, minus, increment, decrement, and their combinations
    #[case("-", [TokenKind::Minus])]
    #[case("+", [TokenKind::Plus])]
    #[case("--", [TokenKind::Decrement])]
    #[case("++", [TokenKind::Increment])]
    #[case("-a", [TokenKind::Minus, TokenKind::Ident])]
    #[case("- a", [TokenKind::Minus, TokenKind::Ident])]
    #[case("+a", [TokenKind::Plus, TokenKind::Ident])]
    #[case("--a", [TokenKind::Decrement, TokenKind::Ident])]
    #[case("++b", [TokenKind::Increment, TokenKind::Ident])]
    #[case("a--", [TokenKind::Ident, TokenKind::Decrement])]
    #[case("b++", [TokenKind::Ident, TokenKind::Increment])]
    // more complex combinations
    #[case("+++++", [
        TokenKind::Increment,
        TokenKind::Increment,
        TokenKind::Plus,
    ])]
    #[case("+++++", [
        TokenKind::Increment,
        TokenKind::Increment,
        TokenKind::Plus,
    ])]
    #[case("+ + +++", [
        TokenKind::Plus,
        TokenKind::Plus,
        TokenKind::Increment,
        TokenKind::Plus,
    ])]
    #[case("+ + + +", [
        TokenKind::Plus,
        TokenKind::Plus,
        TokenKind::Plus,
        TokenKind::Plus,
    ])]
    #[case("-----", [
        TokenKind::Decrement,
        TokenKind::Decrement,
        TokenKind::Minus,
    ])]
    #[case("- - ---", [
        TokenKind::Minus,
        TokenKind::Minus,
        TokenKind::Decrement,
        TokenKind::Minus,
    ])]
    #[case("- - - -", [
        TokenKind::Minus,
        TokenKind::Minus,
        TokenKind::Minus,
        TokenKind::Minus,
    ])]
    #[case("++++%-*---", [
        TokenKind::Increment,
        TokenKind::Increment,
        TokenKind::Modulo,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Decrement,
        TokenKind::Minus,
    ])]
    // multiply, divide, and modulo
    #[case("*", [TokenKind::Star])]
    #[case("/", [TokenKind::Divide])]
    #[case("%", [TokenKind::Modulo])]
    #[case("*a", [TokenKind::Star, TokenKind::Ident])]
    #[case("/b", [TokenKind::Divide, TokenKind::Ident])]
    #[case("%c", [TokenKind::Modulo, TokenKind::Ident])]
    // repeated operators
    #[case("****", [TokenKind::Star, TokenKind::Star, TokenKind::Star, TokenKind::Star])]
    #[case("////", [TokenKind::Divide, TokenKind::Divide, TokenKind::Divide, TokenKind::Divide])]
    #[case("%%%%", [TokenKind::Modulo, TokenKind::Modulo, TokenKind::Modulo, TokenKind::Modulo])]
    // logical operators
    #[case("!", [TokenKind::LogicNot])]
    #[case("&&", [TokenKind::And])]
    #[case("||", [TokenKind::Or])]
    #[case("==", [TokenKind::Equal])]
    #[case("!=", [TokenKind::NotEqual])]
    #[case("<", [TokenKind::LT])]
    #[case(">", [TokenKind::GT])]
    #[case("<=", [TokenKind::LTE])]
    #[case(">=", [TokenKind::GTE])]
    // assignment operator
    #[case("=", [TokenKind::Assign])]
    fn test_tokenize_operators<const S: usize>(
        #[case] src: &str,
        #[case] expected: [TokenKind; S],
    ) {
        let src = Source::new(src.to_owned());
        let tokens = tokenize(&src).collect::<Vec<Token>>();

        assert_eq!(tokens.len(), S);

        // check that the token kinds match the expected kinds
        for (tok, &kind) in tokens.iter().zip(expected.iter()) {
            assert_eq!(tok.kind(), kind);
        }
    }
}
