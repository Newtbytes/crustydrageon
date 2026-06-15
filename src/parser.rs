use std::iter;

use crate::{
    ast::{
        BinOp, BinOpKind, Block, BlockItem, Decl, Expr, ExprKind, Function, Identifier, Precedence,
        Program, Stmt, Token, TokenKind, UnOp, UnOpKind,
    },
    diag::Annotation,
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

impl Annotation for ParserError {
    fn span(&self) -> &src::Span {
        match self {
            Self::ExpectedToken {
                expected: _,
                actual: tok,
            }
            | Self::ExpectedString {
                expected: _,
                actual: tok,
            }
            | Self::ErrorToken(tok, _)
            | Self::ExpectedEOF(tok) => tok.span(),
            Self::UnexpectedEOF => todo!(),
        }
    }

    fn message(&self) -> String {
        match self {
            ParserError::ExpectedToken { expected, actual } => {
                format!("Expected a {:?} but got a {:?}", expected, actual.kind())
            }
            ParserError::ExpectedString { expected, actual } => {
                format!("Expected a {:?} but got a {:?}", expected, actual.kind())
            }
            ParserError::ErrorToken(_, msg) => msg.to_string(),
            ParserError::UnexpectedEOF => "Unexpectedly reached end of file".to_string(),
            ParserError::ExpectedEOF(tok) => {
                format!("Expected end of file but got a {:?}", tok.kind())
            }
        }
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

        while (next_kind.is_binary_op() || next_kind.is_ternary_op())
            && next_kind.precedence() >= Some(min_prec)
        {
            if next_kind == TokenKind::Assign {
                // parse assignment operators as right-associative
                let op = self.parse_binary_op()?;
                assert_eq!(op.kind, BinOpKind::Assign);

                let right = self.parse_expr(next_kind.precedence().unwrap())?;

                left.kind = ExprKind::Binary(op, Box::new(left.clone()), Box::new(right));
            } else if next_kind == TokenKind::Question {
                let middle = self.parse_ternary_middle()?;
                let right = self.parse_expr(next_kind.precedence().unwrap())?;

                left.kind =
                    ExprKind::Cond(Box::new(left.clone()), Box::new(middle), Box::new(right))
            } else {
                let op = self.parse_binary_op()?;
                let right = self.parse_expr(next_kind.precedence().unwrap() + 1)?;

                left.kind = ExprKind::Binary(op, Box::new(left.clone()), Box::new(right));
            }
            next_kind = self.peek().kind();
        }

        if min_prec == Precedence::default() {
            cov_mark::hit!(parser_expr_parsed)
        } else {
            cov_mark::hit!(parser_sub_expr_parsed)
        }

        Ok(left)
    }

    fn parse_ternary_middle(&mut self) -> ParseResult<Expr> {
        self.expect(TokenKind::Question)?;
        let middle = self.parse_expr(Default::default())?;
        self.expect(TokenKind::Colon)?;

        Ok(middle)
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
        if self.check(TokenKind::Return) {
            self.expect(TokenKind::Return)?;
            let ret_val = self.parse_expr(Precedence::default())?;
            self.expect(TokenKind::Semicolon)?;
            Ok(Stmt::Return(ret_val))
        } else if self.check(TokenKind::If) {
            self.expect(TokenKind::If)?;

            self.expect(TokenKind::LParen)?;
            let cond = self.parse_expr(Default::default())?;
            self.expect(TokenKind::RParen)?;

            let if_true = Box::new(self.parse_stmt()?);
            let if_false = if self.check(TokenKind::Else) {
                self.expect(TokenKind::Else)?;
                Some(Box::new(self.parse_stmt()?))
            } else {
                None
            };

            Ok(Stmt::If(cond, if_true, if_false))
        } else if self.check(TokenKind::LBrace) {
            let block = self.parse_block()?;
            Ok(Stmt::Compound(block))
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

    fn parse_block(&mut self) -> ParseResult<Block> {
        let block_start = self.expect(TokenKind::LBrace)?;

        let mut block = Block::new();
        while self.peek().kind() != TokenKind::RBrace {
            block.push(self.parse_block_item()?);
        }

        let block_end = self.expect(TokenKind::RBrace)?;

        block.span = self
            .src
            .subspan(
                block_start.span().start_index(),
                block_end.span().end_index(),
            )
            .unwrap();

        Ok(block)
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

        let body = self.parse_block()?;

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

    mod expr {
        use super::*;

        // TODO: unit test comparison operators

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

        #[apply(factors)]
        fn test_parse_factor_matches_expected(
            #[case] tokens: &[Token],
            #[case] expected_expr: Expr,
        ) {
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
        #[case(
            &[tok(tk::Constant, "7"), tok(tk::Question, "?"), 
                tok(tk::Constant, "1"),
                tok(tk::Colon, ":"),
                tok(tk::Constant, "5")],
            Expr::cond(Expr::constant(7), Expr::constant(1), Expr::constant(5))
        )]
        fn test_parse_matches_expected(
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
        fn test_parse_err(
            #[case] _tokens: &[Token],
            #[with(_tokens)] mut parser: Parser<impl Iterator<Item = Token>>,
        ) {
            parser.parse_expr(Precedence::default()).unwrap_err();
        }

        proptest! {
            #[test]
            fn test_parse_roundtrip(expr: Expr) {
                let src: Source = expr.to_string().into();
                let tokens: Vec<Token> = lexer::tokenize(&src).collect();
                let mut parser = parser(&tokens);

                // Parsing increment / decrement operators is not yet implemented
                prop_assume!(!tokens.iter().any(|t| matches!(t.kind(), TokenKind::Increment | TokenKind::Decrement)));

                let parsed = parser.parse_expr(Precedence::default());
                prop_assert_eq!(parsed.unwrap().to_string(), expr.to_string());
            }
        }
    }

    mod stmt {
        use super::*;

        #[rstest]
        #[case(
            &[tok(tk::Return, "return"), tok(tk::Constant, "5"), tok(tk::Semicolon, ";")],
            Stmt::Return(Expr::constant(5))
        )]
        #[case(
            &[tok(tk::If, "if"), tok(tk::LParen, "("), tok(tk::Constant, "1"), tok(tk::RParen, ")"),
                tok(tk::Return, "return"), tok(tk::Constant, "42"), tok(tk::Semicolon, ";")],
            Stmt::If(Expr::constant(1), Stmt::Return(Expr::constant(42)).into(), None)
        )]
        #[case(
            &[tok(tk::Constant, "1"), tok(tk::Plus, "+"), tok(tk::Constant, "2"), tok(tk::Semicolon, ";")],
            Stmt::Expr(Expr::binary(BinOpKind::Add, Expr::constant(1), Expr::constant(2)))
        )]
        fn test_parse_matches_expected(
            #[case] _tokens: &[Token],
            #[with(_tokens)] mut parser: Parser<impl Iterator<Item = Token>>,
            #[case] expected_stmt: Stmt,
        ) {
            let actual_stmt = parser.parse_stmt().unwrap();

            assert_eq!(expected_stmt, actual_stmt);
        }
    }
}
