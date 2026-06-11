use std::iter;

use crate::{
    ast::{
        BinOp, BinOpKind, BlockItem, Decl, Expr, ExprKind, Function, Identifier, Precedence,
        Program, Stmt, Token, TokenKind, UnOp, UnOpKind,
    },
    diag::{Annotation, Diag, DiagLevel, Diagnostic},
    src::{self, Source},
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

impl Diagnostic for ParserError {
    fn into_diag(self) -> crate::diag::Diag {
        let mut diag = Diag::new(DiagLevel::Error);

        let (span, msg) = match self {
            Self::ExpectedToken { expected, actual } => (
                actual.clone().into(),
                format!("Expected a {:?} but got a {:?}", expected, actual.kind()),
            ),
            Self::ExpectedString { expected, actual } => (
                actual.clone().into(),
                format!("Expected a {:?} but got a {:?}", expected, actual.kind()),
            ),
            Self::ErrorToken(tok, msg) => (tok.into(), msg.to_owned()),
            Self::UnexpectedEOF => todo!(),
            Self::ExpectedEOF(tok) => (
                tok.clone().into(),
                format!("Expected end of file but got a {:?}", tok.kind()),
            ),
        };

        diag.annotate(Annotation::new(span, msg));

        diag
    }
}

type ParseResult<T> = Result<T, ParserError>;

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
    src: Source,
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

    fn check(&mut self, kind: TokenKind) -> bool {
        self.peek().kind() == kind
    }

    fn parse_unary_op(&mut self) -> ParseResult<UnOp> {
        let tok = self.take()?;

        Ok(UnOp {
            kind: match tok.kind() {
                TokenKind::Complement => UnOpKind::Complement,
                TokenKind::Minus => UnOpKind::Negate,
                TokenKind::Not => UnOpKind::Not,
                kind if kind.is_unary_op() => {
                    todo!("parsing unary operator of kind {:?}", kind)
                }
                _ => {
                    return Err(ParserError::ExpectedString {
                        expected: "unary operator",
                        actual: tok,
                    });
                }
            },
            span: tok.into(),
        })
    }

    fn parse_binary_op(&mut self) -> ParseResult<BinOp> {
        let tok = self.take()?;

        Ok(BinOp {
            kind: match tok.kind() {
                TokenKind::Plus => BinOpKind::Add,
                TokenKind::Minus => BinOpKind::Subtract,
                TokenKind::Star => BinOpKind::Multiply,
                TokenKind::Divide => BinOpKind::Divide,
                TokenKind::Modulo => BinOpKind::Modulo,
                TokenKind::And => BinOpKind::And,
                TokenKind::Or => BinOpKind::Or,
                TokenKind::Equal => BinOpKind::Equal,
                TokenKind::NotEqual => BinOpKind::NotEqual,
                TokenKind::LT => BinOpKind::LessThan,
                TokenKind::LTE => BinOpKind::LessOrEqual,
                TokenKind::GT => BinOpKind::GreaterThan,
                TokenKind::GTE => BinOpKind::GreaterOrEqual,
                TokenKind::Ampersand => BinOpKind::BitAnd,
                TokenKind::Pipe => BinOpKind::BitOr,
                TokenKind::UpArrow => BinOpKind::Xor,
                TokenKind::LShift => BinOpKind::LShift,
                TokenKind::RShift => BinOpKind::RShift,
                TokenKind::Assign => BinOpKind::Assign,
                kind if kind.is_binary_op() => {
                    todo!("parsing binary operator of kind {:?}", kind)
                }
                _ => {
                    return Err(ParserError::ExpectedString {
                        expected: "binary operator",
                        actual: tok,
                    });
                }
            },
            span: tok.into(),
        })
    }

    fn parse_block_item(&mut self) -> ParseResult<BlockItem> {
        Ok(if self.peek().kind() == TokenKind::Int {
            BlockItem::Decl(self.parse_decl()?)
        } else {
            BlockItem::Stmt(self.parse_stmt()?)
        })
    }

    fn parse_expr(&mut self, min_prec: Precedence) -> ParseResult<Expr> {
        let mut left = self.parse_factor()?;

        let mut next_kind = self.peek().kind();

        while next_kind.is_binary_op() && next_kind.precedence() >= Some(min_prec) {
            let right: Expr;
            let op = self.parse_binary_op()?;

            if next_kind == TokenKind::Assign {
                // parse assignment operators as right-associative
                assert_eq!(op.kind, BinOpKind::Assign);
                right = self.parse_expr(next_kind.precedence().unwrap())?;
            } else {
                right = self.parse_expr(next_kind.precedence().unwrap() + 1)?;
            }

            left.kind = ExprKind::Binary(op, Box::new(left.clone()), Box::new(right.clone()));

            left.span = self
                .src
                .subspan(left.span.start_index(), right.span.end_index())
                .unwrap();

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
        let expr = match self.peek().kind() {
            TokenKind::Constant => {
                let constant = self.expect(TokenKind::Constant)?;
                let expr =
                    Expr {
                        kind: ExprKind::Const(constant.value().to_string().parse().expect(
                            "Constant token should always contain a parseable integer value",
                        )),
                        span: constant.into(),
                    };

                cov_mark::hit!(parser_constant_expr_parsed);

                expr
            }
            TokenKind::Ident => {
                let id = self.parse_identifier()?;
                Expr {
                    kind: ExprKind::Var(id.clone()),
                    span: id.into(),
                }
            }
            kind if kind.is_unary_op() => {
                let op = self.parse_unary_op()?;
                let factor = self.parse_factor()?;
                let span = self
                    .src
                    .subspan(op.span.start_index(), factor.span.end_index())
                    .expect("should have spans derived from the source");

                Expr {
                    kind: ExprKind::Unary(op, Box::new(factor)),
                    span,
                }
            }
            TokenKind::LParen => {
                let start_tok = self.expect(TokenKind::LParen)?;
                let mut inner_expr = self.parse_expr(Precedence::default())?;
                let end_tok = self.expect(TokenKind::RParen)?;

                inner_expr.span = self
                    .src
                    .subspan(start_tok.span().start_index(), end_tok.span().end_index())
                    .expect("Should have valid spans for this source");

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
        if self.check(TokenKind::Return) {
            self.expect(TokenKind::Return)?;
            let ret_val = self.parse_expr(Precedence::default())?;
            self.expect(TokenKind::Semicolon)?;
            Ok(Stmt::Return(ret_val))
        } else if self.check(TokenKind::Semicolon) {
            self.expect(TokenKind::Semicolon)?;
            Ok(Stmt::Null)
        } else {
            let expr = self.parse_expr(Precedence::default())?;
            self.expect(TokenKind::Semicolon)?;
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_decl(&mut self) -> ParseResult<Decl> {
        let start_tok = self.expect(TokenKind::Int)?;

        let name = self.parse_identifier()?;

        let init = if self.peek().kind() == TokenKind::Semicolon {
            None
        } else {
            self.expect(TokenKind::Assign)?;
            let expr = self.parse_expr(Precedence::default())?;
            Some(expr)
        };

        let end_tok = self.expect(TokenKind::Semicolon)?;

        Ok(Decl {
            name,
            init,
            span: self
                .src
                .subspan(start_tok.span().start_index(), end_tok.span().end_index())
                .expect("start and end tokens should have valid spans in the source"),
        })
    }

    fn parse_identifier(&mut self) -> ParseResult<Identifier> {
        Ok(self.expect(TokenKind::Ident)?.span().clone().into())
    }

    fn parse_function(&mut self) -> ParseResult<Function> {
        self.expect(TokenKind::Int)?;

        let name = self.parse_identifier()?;

        self.expect(TokenKind::LParen)?;
        self.expect(TokenKind::Void)?;
        self.expect(TokenKind::RParen)?;

        self.expect(TokenKind::LBrace)?;

        let mut body = Vec::new();
        while self.peek().kind() != TokenKind::RBrace {
            body.push(self.parse_block_item()?);
        }

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
    src: Source,
    tokens: iter::Peekable<impl iter::Iterator<Item = Token> + 'src>,
) -> ParseResult<Program> {
    let mut parser = Parser { tokens, src };
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use itertools::Itertools;

    use BinOpKind::*;
    use UnOpKind::*;
    use proptest::prelude::*;
    use rstest::{fixture, rstest};
    use rstest_reuse::{self, *};

    use crate::{lexer, src::Source};
    use TokenKind as tk;

    fn tok(kind: TokenKind, lexeme: &'static str) -> Token {
        Token::new(kind, Source::new(lexeme.to_owned()).into())
    }

    #[fixture]
    fn parser(#[default(&[])] tokens: &[Token]) -> Parser<impl Iterator<Item = Token>> {
        let src = Source::new(tokens.iter().map(Token::to_string).join(" "));
        let tokens = tokens.iter().cloned().peekable();
        Parser { tokens, src }
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
        Expr::unary(Complement, Expr::constant(5))
    )]
    #[case(
        &[tok(tk::Complement, "~"), tok(tk::Complement, "~"), tok(tk::Constant, "42")],
        Expr::unary(Complement, Expr::unary(Complement, Expr::constant(42)))
    )]
    #[case(
        &[tok(tk::Minus, "-"), tok(tk::LParen, "("), tok(tk::Constant, "69"), tok(tk::RParen, ")")],
        Expr::unary(Negate, Expr::constant(69))
    )]
    #[case(
        &[tok(TokenKind::Not, "!"), tok(TokenKind::Constant, "0")],
        Expr::unary(Not, Expr::constant(0))
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
        Expr::binary(Add, Expr::constant(4), Expr::constant(2))
    )]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Plus, "+"), tok(tk::Constant, "2"), tok(tk::Minus, "+"), tok(tk::Constant, "6")],
        Expr::binary(
            Subtract,
            Expr::binary(Add, Expr::constant(4), Expr::constant(2)),
            Expr::constant(6),
        ),
    )]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Plus, "+"), tok(tk::Constant, "2"), tok(tk::Star, "*"), tok(tk::Constant, "3")],
        Expr::binary(
            Add,
            Expr::constant(4),
            Expr::binary(Multiply, Expr::constant(2), Expr::constant(3)),
        )
    )]
    #[case(
        &[tok(tk::Constant, "4"), tok(tk::Star, "*"), tok(tk::Constant, "2"), tok(tk::Plus, "+"), tok(tk::Constant, "3")],
        Expr::binary(
            Add,
            Expr::binary(Multiply, Expr::constant(4), Expr::constant(2)),
            Expr::constant(3),
        )
    )]
    #[case(
        &[tok(tk::Constant, "7"), tok(tk::Star, "*"), tok(tk::Constant, "3"), tok(tk::Minus, "-"), tok(tk::Constant, "1")],
        Expr::binary(
            Subtract,
            Expr::binary(Multiply, Expr::constant(7), Expr::constant(3)),
            Expr::constant(1),
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

    fn contains_unimplemented(tokens: &[Token]) -> bool {
        // Parsing increment / decrement operators is not yet implemented
        tokens
            .iter()
            .any(|t| matches!(t.kind(), TokenKind::Increment | TokenKind::Decrement))
    }

    proptest! {
        #[test]
        fn test_parse_expr_roundtrip(expr: Expr) {
            let src: Source = expr.to_string().into();
            let tokens: Vec<Token> = lexer::tokenize(&src).collect();
            let mut parser = parser(&tokens);

            prop_assume!(!contains_unimplemented(&tokens));

            let parsed = parser.parse_expr(Precedence::default()).unwrap();
            prop_assert_eq!(parsed.to_string(), expr.to_string());
            prop_assert!(parsed.span.len() > 1);
        }

        #[test]
        fn test_parse_decl_roundtrip(decl: Decl) {
            let src: Source = decl.to_string().into();
            let tokens: Vec<Token> = lexer::tokenize(&src).collect();
            let mut parser = parser(&tokens);

            prop_assume!(!contains_unimplemented(&tokens));

            let parsed = parser.parse_decl().unwrap();
            prop_assert_eq!(parsed.to_string(), decl.to_string());
            prop_assert!(parsed.span.len() > 1);
        }
    }
}
