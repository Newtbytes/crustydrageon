use test_each_file::test_each_file;

use crustydrageon::{ast::TokenKind, lexer::tokenize, parser::parse};

const FAIL_VALID: &str = "parsing a valid program should never fail";
const FAIL_INVALID: &str = "parsing an invalid program should never succeed";

test_each_file! { in "tests/valid/" => test_parse_valid }
fn test_parse_valid(program: &str) {
    let tokens = tokenize(program.chars());
    parse(&mut tokens.peekable()).expect(FAIL_VALID);
}

test_each_file! { in "tests/invalid/lex" => test_lex_invalid }
fn test_lex_invalid(program: &str) {
    let mut tokens = tokenize(program.chars());
    assert!(
        tokens.any(|tok| tok.kind() == TokenKind::Error),
        "{}",
        FAIL_INVALID
    );
}

test_each_file! { in "tests/invalid/parse" => test_parse_invalid }
fn test_parse_invalid(program: &str) {
    let tokens = tokenize(program.chars());
    assert!(parse(&mut tokens.peekable()).is_err(), "{}", FAIL_INVALID);
}
