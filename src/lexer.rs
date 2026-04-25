use std::{iter::Peekable, str::Chars};

use crate::{
    ast::{Token, TokenKind},
    src::{Source, Span},
};

pub struct Lexer<'src> {
    src: Peekable<Chars<'src>>,
    consumed: Span<'src>,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src Source) -> Self {
        Lexer {
            src: src.chars().peekable(),
            consumed: Span::empty(src),
        }
    }

    /// Reset the consumed token, setting it to start at the next character
    fn end_token(&mut self) {
        if self.one_ahead().is_some() {
            self.consumed.point_to(self.consumed.end_index()).unwrap();
        } else {
            self.consumed.reset();
        }
    }

    fn one_ahead(&mut self) -> Option<&char> {
        self.src.peek()
    }

    fn eat(&mut self) -> Option<char> {
        let c = self.src.next()?;

        self.consumed.advance_by(1)
            .expect("Extending the consumed token should not fail as we should be guaranteed to be within bounds here");

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
    }

    fn eat_int_literal(&mut self) {
        self.eat_while(char::is_ascii_digit);
    }

    fn emit(&mut self, kind: TokenKind) -> Token<'src> {
        let tok = Token::new(kind, self.consumed);

        self.end_token();

        tok
    }

    fn at_word_bound(&mut self) -> bool {
        match self.one_ahead() {
            Some(c) => !is_word(c),
            None => true,
        }
    }

    fn error(&mut self, msg: &'static str) -> TokenKind {
        TokenKind::Error(msg)
    }
}

fn is_word(c: &char) -> bool {
    matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '_')
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        use TokenKind as tk;

        // skip whitespace
        self.eat_while(|&c| c.is_whitespace());
        self.end_token();

        let kind = match self.eat() {
            Some(c) => match c {
                '(' => tk::LParen,
                ')' => tk::RParen,
                '{' => tk::LBrace,
                '}' => tk::RBrace,
                ';' => tk::Semicolon,

                'a'..='z' | 'A'..='Z' | '_' => {
                    self.eat_identifier();

                    if self.at_word_bound() {
                        // handle keywords
                        match self.consumed.as_str() {
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

pub fn tokenize<'src>(src: &'src Source) -> impl Iterator<Item = Token<'src>> {
    Lexer::new(src)
}
