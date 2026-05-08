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
    }

    fn eat_int_literal(&mut self) {
        self.eat_while(char::is_ascii_digit);
    }

    fn emit(&mut self, kind: TokenKind) -> Token {
        let tok = Token::new(kind, self.consumed.clone());

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

impl Iterator for Lexer<'_> {
    type Item = Token;

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
                '~' => tk::Complement,
                '-' => {
                    if self.eat_if(|c| matches!(c, '-')).is_some() {
                        tk::Decrement
                    } else {
                        tk::Negate
                    }
                }

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

    #[test]
    fn test_tokenize_returns_empty() {
        let src = Source::new("".to_owned());
        let tokens = tokenize(&src);

        assert_eq!(tokens.count(), 0);
    }
}
