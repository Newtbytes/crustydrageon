use std::{iter::Peekable, str::Chars};

#[derive(Debug)]
pub enum TokenKind {
    // Structural
    LParen,
    RParen,
    LBrace,
    RBrace,
    Semicolon,

    // Literals
    Constant,
    Ident,

    // Keywords
    Int,
    Void,
    Return,

    Error,
}

#[derive(Debug)]
pub struct Token {
    kind: TokenKind,
    lexeme: String,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String) -> Self {
        Self { kind, lexeme }
    }
}

pub struct Lexer<'src> {
    src: Peekable<Chars<'src>>,
    consumed: String,
    offset: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: Chars<'src>) -> Self {
        Lexer {
            src: src.peekable(),
            consumed: String::new(),
            offset: 0,
        }
    }

    fn clear_consumed(&mut self) {
        self.offset += self.consumed.len();
        self.consumed.clear();
    }

    fn one_ahead(&mut self) -> Option<&char> {
        self.src.peek()
    }

    fn eat(&mut self) -> Option<char> {
        let c = self.src.next()?;

        self.consumed.push(c);

        Some(c)
    }

    fn eat_if<P>(&mut self, mut predicate: P) -> Option<char>
    where
        P: FnMut(&char) -> bool,
    {
        let c = self.src.next_if(&mut predicate)?;

        self.consumed.push(c);

        Some(c)
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

    fn emit(&mut self, token: TokenKind) -> Token {
        let tok = Token {
            kind: token,
            lexeme: self.consumed.clone(),
        };

        self.clear_consumed();

        tok
    }

    fn at_word_bound(&mut self) -> bool {
        match self.one_ahead() {
            Some(c) => !is_word(c),
            None => true,
        }
    }

    fn error(&mut self, msg: &'static str) -> TokenKind {
        self.consumed = msg.to_owned();
        TokenKind::Error
    }
}

fn is_word(c: &char) -> bool {
    matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '_')
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        use TokenKind as tk;

        // skip whitespace
        self.eat_while(|&c| c.is_whitespace());
        self.clear_consumed();

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

                _ => todo!(),
            },
            None => return None,
        };

        // synchronize by eating until synchronization point
        if let tk::Error = kind {
            self.eat_until(|&c| !is_word(&c));
        }

        return Some(self.emit(kind));
    }
}

pub fn tokenize(src: Chars) -> impl Iterator<Item = Token> {
    Lexer::new(src)
}
