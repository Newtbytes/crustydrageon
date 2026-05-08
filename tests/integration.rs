use std::fs;
use std::path::PathBuf;

use proptest::prelude::*;
use rstest::rstest;

use crustydrageon::{
    ast::{Token, TokenKind},
    driver,
    lexer::tokenize,
    parser::parse,
    src::Source,
};

const FAIL_PARSE_VALID: &str = "parsing a valid program should never fail";
const FAIL_PARSE_INVALID: &str = "parsing an invalid program should never succeed";

#[rstest]
fn test_driver(#[files("tests/valid/**/*.c")] path: PathBuf) {
    let filename = driver::compile_file(path.to_str().unwrap(), None, false)
        .expect("Compilation should succeed for valid programs");

    if let Some(filename) = filename {
        fs::remove_file(filename)
            .expect("Failed to remove temporary binay. This test may be broken");
    }
}

#[rstest]
fn test_parse_valid(
    #[files("tests/valid/**/*.c")]
    #[mode = str]
    program: &str,
) {
    let src = Source::new(program.to_owned());
    let tokens = tokenize(&src);
    parse(&mut tokens.peekable()).expect(FAIL_PARSE_VALID);
}

#[rstest]
fn test_lex_invalid(
    #[files("tests/invalid/lex/**/*.c")]
    #[mode = str]
    program: &str,
) {
    let src = Source::new(program.to_owned());
    let mut tokens = tokenize(&src);
    assert!(
        tokens.any(|tok| {
            match tok.kind() {
                TokenKind::Error(_) => true,
                _ => false,
            }
        }),
        "{}:\n{:#?}",
        FAIL_PARSE_INVALID,
        tokens.collect::<Vec<Token>>()
    );
}

#[rstest]
fn test_parse_invalid(
    #[files("tests/invalid/parse/**/*.c")]
    #[mode = str]
    program: &str,
) {
    let src = Source::new(program.to_owned());
    let tokens = tokenize(&src).collect::<Vec<Token>>();
    let ast = parse(&mut tokens.iter().cloned().peekable());
    assert!(
        ast.is_err(),
        "{}:\nTOKENS:\n{:#?}\nAST:\n{:#?}",
        FAIL_PARSE_INVALID,
        tokens,
        ast
    );
}

proptest! {
    #[test]
    fn doesnt_panic(program: String) {
        let _ = driver::compile(program, None, false);
    }
}
