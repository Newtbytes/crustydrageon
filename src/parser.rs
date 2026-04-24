use std::{fmt, iter};

use crate::ast::{self, Token, TokenKind};

#[derive(Debug, Clone)]
pub enum ParserError {
    UnexpectedToken {
        expected: TokenKind,
        actual: TokenKind,
    },
    ErrorToken(&'static str, String),
    ReachedEOF,
    ExpectedEOF(String),
}

type ParseResult<T> = Result<T, ParserError>;

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::UnexpectedToken { expected, actual } => {
                write!(f, "Expected a {:?} but got a {:?}", expected, actual)
            }
            ParserError::ErrorToken(msg, value) => write!(f, "{}: {}", msg, value),
            ParserError::ReachedEOF => write!(f, "Unexpectedly reached end of file"),
            ParserError::ExpectedEOF(s) => write!(f, "Expected end of file but got {}", s),
        }
    }
}

struct Parser<'src, 'iter, I: Iterator<Item = Token<'src>> + 'iter> {
    tokens: &'iter mut iter::Peekable<I>,
}

impl<'src, 'iter, I: iter::Iterator<Item = Token<'src>> + 'iter> Parser<'src, 'iter, I> {
    fn take(&mut self) -> ParseResult<Token<'src>> {
        let token = self.tokens.next().ok_or(ParserError::ReachedEOF)?;

        if let TokenKind::Error(msg) = token.kind() {
            Err(ParserError::ErrorToken(msg, token.value().to_string()))
        } else {
            Ok(token)
        }
    }

    fn expect(&mut self, expected: TokenKind) -> ParseResult<Token<'src>> {
        match self.take()? {
            token if token.kind() == expected => Ok(token),
            actual => Err(ParserError::UnexpectedToken {
                expected,
                actual: actual.kind(),
            }),
        }
    }

    fn parse_expr(&mut self) -> ParseResult<ast::Expr> {
        let constant = self.expect(TokenKind::Constant)?;
        Ok(ast::Expr::Const(
            constant
                .value()
                .to_string()
                .parse()
                .expect("Constant token should always contain a parseable integer value"),
        ))
    }

    fn parse_stmt(&mut self) -> ParseResult<ast::Stmt> {
        self.expect(TokenKind::Return)?;
        let ret_val = self.parse_expr()?;
        self.expect(TokenKind::Semicolon)?;

        Ok(ast::Stmt::Return(ret_val))
    }

    fn parse_function(&mut self) -> ParseResult<ast::Function> {
        self.expect(TokenKind::Int)?;
        let name = self.expect(TokenKind::Ident)?;

        self.expect(TokenKind::LParen)?;
        self.expect(TokenKind::Void)?;
        self.expect(TokenKind::RParen)?;

        self.expect(TokenKind::LBrace)?;
        let body = self.parse_stmt()?;
        self.expect(TokenKind::RBrace)?;

        Ok(ast::Function::new(
            ast::Identifier {
                value: name.value().to_string(),
            },
            body,
        ))
    }

    fn parse_program(&mut self) -> ParseResult<ast::Program> {
        let func = self.parse_function()?;

        match self.tokens.next() {
            Some(token) => Err(ParserError::ExpectedEOF(token.value().to_string())),
            None => Ok(ast::Program { body: func }),
        }
    }
}

pub fn parse<'src>(
    tokens: &mut iter::Peekable<impl iter::Iterator<Item = Token<'src>> + 'src>,
) -> ParseResult<ast::Program> {
    let mut parser = Parser { tokens };
    parser.parse_program()
}
