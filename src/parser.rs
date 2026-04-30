use std::{fmt, iter};

use crate::{
    ast::{self, Token, TokenKind},
    src,
};

#[derive(Debug, Clone)]
pub enum ParserError {
    UnexpectedToken { expected: TokenKind, actual: Token },
    ErrorToken(Token, &'static str),
    UnexpectedEOF,
    ExpectedEOF(Token),
}

type ParseResult<T> = Result<T, ParserError>;

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::UnexpectedToken { expected, actual } => {
                write!(f, "Expected a {:?} but got a {:?}", expected, actual.kind())
            }
            ParserError::ErrorToken(tok, msg) => write!(f, "{}", msg),
            ParserError::UnexpectedEOF => write!(f, "Unexpectedly reached end of file"),
            ParserError::ExpectedEOF(tok) => {
                write!(f, "Expected end of file but got a {:?}", tok.kind())
            }
        }
    }
}

impl ParserError {
    pub fn span(&self) -> Option<&src::Span> {
        match self {
            ParserError::UnexpectedToken {
                expected: _,
                actual,
            } => Some(actual.span()),
            ParserError::ErrorToken(token, _) => Some(token.span()),
            ParserError::ExpectedEOF(token) => Some(token.span()),

            ParserError::UnexpectedEOF => None,
        }
    }
}

struct Parser<'iter, I: Iterator<Item = Token> + 'iter> {
    tokens: &'iter mut iter::Peekable<I>,
}

impl<'iter, I: iter::Iterator<Item = Token> + 'iter> Parser<'iter, I> {
    fn take(&mut self) -> ParseResult<Token> {
        let token = self.tokens.next().ok_or(ParserError::UnexpectedEOF)?;

        if let TokenKind::Error(msg) = token.kind() {
            Err(ParserError::ErrorToken(token, msg))
        } else {
            Ok(token)
        }
    }

    fn expect(&mut self, expected: TokenKind) -> ParseResult<Token> {
        match self.take()? {
            token if token.kind() == expected => Ok(token),
            actual => Err(ParserError::UnexpectedToken {
                expected,
                actual: actual,
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
            Some(token) => Err(ParserError::ExpectedEOF(token)),
            None => Ok(ast::Program { body: func }),
        }
    }
}

pub fn parse<'src>(
    tokens: &mut iter::Peekable<impl iter::Iterator<Item = Token> + 'src>,
) -> ParseResult<ast::Program> {
    let mut parser = Parser { tokens };
    parser.parse_program()
}
