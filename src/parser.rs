use std::{fmt, iter};

use crate::{
    ast::{
        BinaryOp, Expr, Function, Identifier, Precedence, Program, Stmt, Token, TokenKind, UnaryOp,
    },
    src,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserError {
    ExpectedToken {
        expected: TokenKind,
        actual: Token,
    },
    ExpectedString {
        expected: &'static str,
        actual: Token,
    },
    ErrorToken(Token, &'static str),
    UnexpectedEOF,
    ExpectedEOF(Token),
}

type ParseResult<T> = Result<T, ParserError>;

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::ExpectedToken { expected, actual } => {
                write!(f, "Expected a {:?} but got a {:?}", expected, actual.kind())
            }
            ParserError::ExpectedString { expected, actual } => {
                write!(f, "Expected a {:?} but got a {:?}", expected, actual.kind())
            }
            ParserError::ErrorToken(_tok, msg) => write!(f, "{msg}"),
            ParserError::UnexpectedEOF => write!(f, "Unexpectedly reached end of file"),
            ParserError::ExpectedEOF(tok) => {
                write!(f, "Expected end of file but got a {:?}", tok.kind())
            }
        }
    }
}

impl ParserError {
    #[must_use]
    pub fn span(&self) -> Option<&src::Span> {
        match self {
            ParserError::ExpectedToken {
                expected: _,
                actual,
            }
            | ParserError::ExpectedString {
                expected: _,
                actual,
            }
            | ParserError::ExpectedEOF(actual) => Some(actual.span()),

            ParserError::ErrorToken(token, _) => Some(token.span()),

            ParserError::UnexpectedEOF => None,
        }
    }
}

struct Parser<I: Iterator<Item = Token>> {
    tokens: iter::Peekable<I>,
}

impl<I: iter::Iterator<Item = Token>> Parser<I> {
    fn take(&mut self) -> ParseResult<Token> {
        let token = self.tokens.next().ok_or(ParserError::UnexpectedEOF)?;

        if let TokenKind::Error(msg) = token.kind() {
            Err(ParserError::ErrorToken(token, msg))
        } else {
            Ok(token)
        }
    }

    fn peek(&mut self) -> &Token {
        use std::sync;

        // FIXME: hacky way to return a ref without creating a temporary
        static EOF: sync::LazyLock<Token> =
            sync::LazyLock::new(|| Token::new(TokenKind::EOF, src::Span::default()));

        match self.tokens.peek() {
            Some(t) => t,
            None => &EOF,
        }
    }

    fn expect(&mut self, expected: TokenKind) -> ParseResult<Token> {
        match self.take()? {
            token if token.kind() == expected => Ok(token),
            token if token.kind() == TokenKind::EOF => Err(ParserError::UnexpectedEOF),
            actual => Err(ParserError::ExpectedToken { expected, actual }),
        }
    }

    fn parse_unary_op(&mut self) -> ParseResult<UnaryOp> {
        let tok = self.take()?;

        match tok.kind() {
            TokenKind::Complement => Ok(UnaryOp::Complement),
            TokenKind::Minus => Ok(UnaryOp::Negate),
            TokenKind::LogicNot => Ok(UnaryOp::Not),
            kind if kind.is_unary_op() => {
                todo!("parsing unary operator of kind {:?}", kind)
            }
            _ => Err(ParserError::ExpectedString {
                expected: "unary operator",
                actual: tok,
            }),
        }
    }

    fn parse_binary_op(&mut self) -> ParseResult<BinaryOp> {
        let tok = self.take()?;

        Ok(match tok.kind() {
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Divide => BinaryOp::Divide,
            TokenKind::Modulo => BinaryOp::Modulo,
            TokenKind::And => BinaryOp::And,
            TokenKind::Or => BinaryOp::Or,
            TokenKind::Equal => BinaryOp::Equal,
            TokenKind::NotEqual => BinaryOp::NotEqual,
            TokenKind::LT => BinaryOp::LessThan,
            TokenKind::LTE => BinaryOp::LessOrEqual,
            TokenKind::GT => BinaryOp::GreaterThan,
            TokenKind::GTE => BinaryOp::GreaterOrEqual,
            kind if kind.is_binary_op() => {
                todo!("parsing binary operator of kind {:?}", kind)
            }
            _ => {
                return Err(ParserError::ExpectedString {
                    expected: "binary operator",
                    actual: tok,
                });
            }
        })
    }

    fn parse_expr(&mut self, min_prec: Precedence) -> ParseResult<Expr> {
        let mut left = self.parse_factor()?;

        let mut next_kind = self.peek().kind();

        while next_kind.is_binary_op() && next_kind.precedence() >= Some(min_prec) {
            let op = self.parse_binary_op()?;
            let right = self.parse_expr(next_kind.precedence().unwrap() + 1)?;

            left = Expr::Binary(op, Box::new(left), Box::new(right));

            next_kind = self.peek().kind();
        }

        if min_prec == Precedence::default() {
            cov_mark::hit!(parser_expr_parsed)
        } else {
            cov_mark::hit!(parser_sub_expr_parsed)
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> ParseResult<Expr> {
        let expr =
            match self.peek().kind() {
                TokenKind::Constant => {
                    let constant = self.expect(TokenKind::Constant)?;
                    let expr =
                        Expr::Const(constant.value().to_string().parse().expect(
                            "Constant token should always contain a parseable integer value",
                        ));

                    cov_mark::hit!(parser_constant_expr_parsed);

                    expr
                }
                kind if kind.is_unary_op() => {
                    let op = self.parse_unary_op()?;
                    Expr::Unary(op, Box::new(self.parse_factor()?))
                }
                TokenKind::LParen => {
                    self.expect(TokenKind::LParen)?;
                    let inner_expr = self.parse_expr(Precedence::default())?;
                    self.expect(TokenKind::RParen)?;

                    cov_mark::hit!(parser_paren_pair_parsed);

                    inner_expr
                }

                _ => {
                    return Err(ParserError::ExpectedString {
                        expected: "factor",
                        actual: self.take()?,
                    });
                }
            };

        cov_mark::hit!(parser_factor_parsed);

        Ok(expr)
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        self.expect(TokenKind::Return)?;
        let ret_val = self.parse_expr(Precedence::default())?;
        self.expect(TokenKind::Semicolon)?;

        Ok(Stmt::Return(ret_val))
    }

    fn parse_identifier(&mut self) -> ParseResult<Identifier> {
        let tok = self.expect(TokenKind::Ident)?;

        Ok(Identifier {
            value: tok.lexeme().to_string(),
            span: tok.span().clone(),
        })
    }

    fn parse_function(&mut self) -> ParseResult<Function> {
        self.expect(TokenKind::Int)?;
        let name = self.parse_identifier()?;

        self.expect(TokenKind::LParen)?;
        self.expect(TokenKind::Void)?;
        self.expect(TokenKind::RParen)?;

        self.expect(TokenKind::LBrace)?;
        let body = self.parse_stmt()?;
        self.expect(TokenKind::RBrace)?;

        Ok(Function::new(name, body))
    }

    fn parse_program(&mut self) -> ParseResult<Program> {
        let func = self.parse_function()?;

        match self.tokens.next() {
            Some(token) => Err(ParserError::ExpectedEOF(token)),
            None => Ok(Program { body: func }),
        }
    }
}

pub fn parse<'src>(
    tokens: iter::Peekable<impl iter::Iterator<Item = Token> + 'src>,
) -> ParseResult<Program> {
    let mut parser = Parser { tokens };
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use crate::src::Source;

    use super::*;

    use BinaryOp::*;
    use Expr::*;
    use TokenKind as tk;
    use UnaryOp::*;

    use proptest::prelude::*;
    use rstest::{fixture, rstest};
    use rstest_reuse::{self, *};

    fn tok(kind: TokenKind, lexeme: &'static str) -> Token {
        Token::new(kind, Source::new(lexeme.to_owned()).into())
    }

    #[fixture]
    fn parser(#[default(&[])] tokens: &[Token]) -> Parser<impl Iterator<Item = Token>> {
        Parser {
            tokens: tokens.iter().cloned().peekable(),
        }
    }

    proptest! {
        #[test]
        fn test_parse_unary_op(token_kind: TokenKind) {
            let toks = &[tok(token_kind, "operator")];
            let mut parser = parser(toks);
            let actual_op = parser.parse_unary_op();

            prop_assert_eq!(actual_op.is_ok(), token_kind.is_unary_op());
        }
    }

    proptest! {
        #[test]
        fn test_parse_binary_op(token_kind: TokenKind) {
            let toks = &[tok(token_kind, "operator")];
            let mut parser = parser(toks);
            let actual_op = parser.parse_binary_op();

            prop_assert_eq!(actual_op.is_ok(), token_kind.is_binary_op());
        }
    }

    #[template]
    #[rstest]
    #[case(
        &[tok(tk::Complement, "~"), tok(tk::Constant, "5")],
        Unary(Complement, Const(5).into())
    )]
    #[case(
        &[tok(tk::Complement, "~"), tok(tk::Complement, "~"), tok(tk::Constant, "42")],
        Unary(Complement, Unary(Complement, Const(42).into()).into())
    )]
    #[case(
        &[tok(tk::Minus, "-"), tok(tk::LParen, "("), tok(tk::Constant, "69"), tok(tk::RParen, ")")],
        Unary(Negate, Const(69).into())
    )]
    #[case(
        &[tok(TokenKind::LogicNot, "!"), tok(TokenKind::Constant, "0")],
        Expr::Unary(UnaryOp::Not, Box::new(Expr::Const(0)))
    )]
    fn factors(#[case] tokens: &[Token], #[case] expected_expr: Expr) {}

    // TODO: unit test comparison operators

    #[apply(factors)]
    fn test_parse_factor_matches_expected(#[case] tokens: &[Token], #[case] expected_expr: Expr) {
        cov_mark::check!(parser_factor_parsed);

        let mut parser = parser(tokens);
        let actual_expr = parser.parse_factor().unwrap();

        assert_eq!(expected_expr, actual_expr);
    }

    #[apply(factors)]
    fn test_parse_expr_parses_factors(#[case] tokens: &[Token], _expected_expr: Expr) {
        cov_mark::check!(parser_factor_parsed);
        cov_mark::check!(parser_expr_parsed);

        let mut parser = parser(tokens);
        let _ = parser.parse_expr(Precedence::default());
    }

    #[apply(factors)]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Plus, "+"), tok(tk::Constant, "2")],
        Binary(Add, Const(4).into(), Const(2).into())
    )]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Plus, "+"), tok(tk::Constant, "2"), tok(tk::Minus, "+"), tok(tk::Constant, "6")],
        Binary(
            Subtract,
            Binary(Add, Const(4).into(), Const(2).into()).into(),
            Const(6).into(),
        ),
    )]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Plus, "+"), tok(tk::Constant, "2"), tok(tk::Star, "*"), tok(tk::Constant, "3")],
        Binary(
            Add,
            Const(4).into(),
            Binary(Multiply, Const(2).into(), Const(3).into()).into(),
        )
    )]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Star, "*"), tok(tk::Constant, "2"), tok(tk::Plus, "+"), tok(tk::Constant, "3")],
        Binary(
            Add,
            Binary(Multiply, Const(4).into(), Const(2).into()).into(),
            Const(3).into(),
        )
    )]
    #[case(
        &[tok(tk::Constant, "7"), tok(tk::Star, "*"), tok(tk::Constant, "3"), tok(tk::Minus, "-"), tok(tk::Constant, "1")],
        Binary(
            Subtract,
            Binary(Multiply, Const(7).into(), Const(3).into()).into(),
            Const(1).into(),
        )
    )]
    fn test_parse_expr_matches_expected(
        #[case] _tokens: &[Token],
        #[with(_tokens)] mut parser: Parser<impl Iterator<Item = Token>>,
        #[case] expected_expr: Expr,
    ) {
        let actual_expr = parser.parse_expr(Precedence::default()).unwrap();

        assert_eq!(expected_expr, actual_expr);
    }

    #[rstest]
    #[case(&[tok(tk::Complement, "~"), tok(tk::LParen, ")")])]
    #[case(&[tok(tk::Complement, "-"), tok(tk::RParen, "("), tok(tk::RParen, "(")])]
    #[case(&[tok(tk::Complement, "-"), tok(tk::RParen, "("), tok(tk::LParen, ")")])]
    #[case(&[tok(tk::Complement, "~"), tok(tk::RParen, "("), tok(tk::RParen, "(")])]
    #[case(&[tok(tk::Complement, "~"), tok(tk::RParen, "("), tok(tk::LParen, ")")])]
    #[case(&[tok(tk::Complement, "~"), tok(tk::RParen, "("), 
        tok(tk::Complement, "-"), tok(tk::RParen, "("), tok(tk::LParen, ")"), 
    tok(tk::LParen, ")")])]
    #[case(&[tok(tk::Complement, "~"), tok(tk::RParen, "("), 
        tok(tk::Complement, "-"), tok(tk::RParen, "("), tok(tk::RParen, "("), 
    tok(tk::RParen, "(")])]
    fn test_parse_expr_err(
        #[case] _tokens: &[Token],
        #[with(_tokens)] mut parser: Parser<impl Iterator<Item = Token>>,
    ) {
        parser.parse_expr(Precedence::default()).unwrap_err();
    }
}
