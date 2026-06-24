use proptest::{prelude::*, string::string_regex};

use crate::src::{Source, Span};

use super::{BinOp, Expr, ExprKind, Identifier, Stmt, Token, TokenKind, UnOp};

impl Arbitrary for Identifier {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        "[a-zA-Z_][0-9a-zA-Z_]"
            .prop_map(Source::new)
            .prop_map(Span::from)
            .prop_map(Identifier::from)
            .prop_filter("identifier should have an Ident TokenKind", |id| {
                id.tok_kind() == TokenKind::Ident
            })
            .boxed()
    }
}

// This is a manual implementation as the Arbitrary derive impl overflows its stack because Expr is a recursive data structure
impl Arbitrary for Expr {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        let leaf = prop_oneof![
            any::<i32>().prop_map(ExprKind::Const),
            any::<Identifier>().prop_map(ExprKind::Var),
        ];

        (leaf, any::<Span>())
            .prop_map(|(kind, span)| Expr { kind, span })
            .prop_recursive(
                3,  // max depth
                16, // max total size
                2,  // max branching factor
                |inner| {
                    prop_oneof![
                        (any::<UnOp>(), inner.clone(), any::<Span>()).prop_map(
                            |(op, expr, span)| Expr {
                                kind: ExprKind::Unary(op, Box::new(expr)),
                                span
                            }
                        ),
                        (any::<BinOp>(), inner.clone(), inner.clone(), any::<Span>()).prop_map(
                            |(op, lhs, rhs, span)| {
                                Expr {
                                    kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                                    span,
                                }
                            }
                        ),
                        (inner.clone(), inner.clone(), inner, any::<Span>()).prop_map(
                            |(cond, if_true, if_false, span)| {
                                Expr {
                                    kind: ExprKind::Cond(
                                        cond.into(),
                                        if_true.into(),
                                        if_false.into(),
                                    ),
                                    span,
                                }
                            }
                        )
                    ]
                },
            )
            .boxed()
    }
}

impl Arbitrary for Stmt {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        let leaf = prop_oneof![any::<Expr>().prop_map(Stmt::Expr), Just(Stmt::Null),];

        leaf.prop_recursive(
            3,  // max depth
            16, // max total size
            2,  // max branching factor
            |inner| {
                prop_oneof![
                    any::<Expr>().prop_map(Stmt::Return),
                    (any::<Expr>(), inner.clone(), any::<bool>(), inner).prop_map(
                        |(cond, if_true, has_else, if_false)| {
                            let else_branch = if has_else {
                                Some(Box::new(if_false))
                            } else {
                                None
                            };
                            Stmt::If(cond, Box::new(if_true), else_branch)
                        }
                    ),
                ]
            },
        )
        .boxed()
    }
}

impl Arbitrary for Token {
    type Parameters = ();
    type Strategy = proptest::prelude::BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        any::<TokenKind>()
            .prop_flat_map(|k| {
                let regex_str = match k {
                    TokenKind::Constant => "[0-9]",
                    TokenKind::Ident => "[_a-zA-Z][_a-zA-Z0-9]+",
                    TokenKind::Complement => "~",
                    TokenKind::Minus => "-",
                    TokenKind::Plus => "\\+",
                    TokenKind::Divide => "/",
                    TokenKind::Star => "\\*",
                    TokenKind::Modulo => "%",
                    TokenKind::Not => "!",
                    TokenKind::And => "&&",
                    TokenKind::Or => "\\|\\|",
                    TokenKind::Equal => "==",
                    TokenKind::NotEqual => "!=",
                    TokenKind::LT => "<",
                    TokenKind::GT => ">",
                    TokenKind::LTE => "<=",
                    TokenKind::GTE => ">=",
                    TokenKind::Decrement => "--",
                    TokenKind::Increment => "\\+\\+",
                    TokenKind::LParen => "\\(",
                    TokenKind::RParen => "\\)",
                    TokenKind::LBrace => "\\{",
                    TokenKind::RBrace => "\\}",
                    TokenKind::Semicolon => ";",
                    TokenKind::Int => "int",
                    TokenKind::Void => "void",
                    TokenKind::Return => "return",
                    TokenKind::Ampersand => "&",
                    TokenKind::Pipe => "\\|",
                    TokenKind::UpArrow => "\\^",
                    TokenKind::LShift => "<<",
                    TokenKind::RShift => ">>",
                    TokenKind::Assign => "=",
                    TokenKind::Question => "\\?",
                    TokenKind::Colon => ":",
                    TokenKind::If => "if",
                    TokenKind::Else => "else",
                    TokenKind::Error(_) => "",
                };
                (
                    Just(k),
                    string_regex(regex_str)
                        .expect("valid regex")
                        .prop_map(Source::new)
                        .prop_map(Span::from),
                )
            })
            // ensure that we never end up with an identifier token that should be a keyword
            .prop_map(|(kind, lexeme)| {
                if matches!(kind, TokenKind::Ident) {
                    (Identifier::from(lexeme.clone()).tok_kind(), lexeme)
                } else {
                    (kind, lexeme)
                }
            })
            .prop_map(|(kind, lexeme)| Token::new(kind, lexeme))
            .boxed()
    }
}
