use std::{fs, path::Path};

use test_each_file::{test_each_file, test_each_path};

use crustydrageon::{
    ast::{Token, TokenKind},
    driver,
    lexer::tokenize,
    parser::parse,
    src::Source,
};

const FAIL_PARSE_VALID: &str = "parsing a valid program should never fail";
const FAIL_PARSE_INVALID: &str = "parsing an invalid program should never succeed";

test_each_path! { in "tests/valid/" as compile => test_driver }
fn test_driver(path: &Path) {
    let filename = driver::compile(
        path.to_str().unwrap(),
        driver::FinalCompilerStage::new(false, false, false),
        false,
    )
    .expect("Compilation should succeed for valid programs");

    if let Some(filename) = filename {
        fs::remove_file(filename);
    }
}

test_each_file! { in "tests/valid/" => test_parse_valid }
fn test_parse_valid(program: &str) {
    let src = Source::new(program.to_owned());
    let tokens = tokenize(&src);
    parse(&mut tokens.peekable()).expect(FAIL_PARSE_VALID);
}

test_each_file! { in "tests/invalid/lex" => test_lex_invalid }
fn test_lex_invalid(program: &str) {
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

test_each_file! { in "tests/invalid/parse" => test_parse_invalid }
fn test_parse_invalid(program: &str) {
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
