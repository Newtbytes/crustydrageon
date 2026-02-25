use std::iter;

use crate::ast::{self, Token, TokenKind};

#[derive(Debug)]
pub enum ParserError {
    UnexpectedToken {
        expected: TokenKind,
        actual: TokenKind,
    },
    ReachedEOF,
    ErrorToken(String),
}

type ParseResult<T> = Result<T, ParserError>;

struct Parser<'a, I: Iterator<Item = Token>> {
    tokens: &'a mut iter::Peekable<I>,
}

impl<I: iter::Iterator<Item = Token>> Parser<'_, I> {
    fn take(&mut self) -> ParseResult<Token> {
        let token = self.tokens.next().ok_or(ParserError::ReachedEOF)?;

        if let TokenKind::Error = token.kind() {
            Err(ParserError::ErrorToken(token.value().to_string()))
        } else {
            Ok(token)
        }
    }

    fn expect(&mut self, expected: TokenKind) -> ParseResult<Token> {
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

        return Ok(ast::Stmt::Return(ret_val));
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

        Ok(ast::Program { body: func })
    }
}

pub fn parse(
    tokens: &mut iter::Peekable<impl iter::Iterator<Item = Token>>,
) -> ParseResult<ast::Program> {
    let mut parser = Parser { tokens };
    parser.parse_program()
}
