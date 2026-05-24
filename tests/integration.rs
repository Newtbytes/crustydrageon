use std::{fs, path::PathBuf};

use proptest::prelude::*;
use rstest::rstest;

use crustydrageon::{driver, error};

fn cleanup_out(out: Option<PathBuf>) {
    if let Some(out) = out {
        fs::remove_file(out)
            .expect("Failed to remove temporary output file; this test might be broken");
    }
}

#[rstest]
fn test_invalid(
    #[files("tests/invalid/**/*.c")]
    #[mode = path]
    path: PathBuf,
) {
    let stage_str = path
        .components()
        .find(|c| {
            let s = c.as_os_str().to_str().unwrap();
            s == "lex" || s == "parse" || s == "ir" || s == "codegen"
        })
        .expect("Invalid test file path: should contain a stage component")
        .as_os_str()
        .to_str()
        .unwrap();

    let final_stage = match stage_str {
        // the lexer stage has error tokens, so we want to run the parser to see those errors
        "lex" | "parse" => driver::CompilerStage::Parse,
        "ir" => driver::CompilerStage::IR,
        "codegen" => driver::CompilerStage::Codegen,
        _ => panic!("Invalid stage component in test file path: {stage_str}"),
    };

    let out = driver::compile_file(path.to_str().unwrap(), Some(final_stage), false);

    match out {
        Ok(out) => {
            cleanup_out(out);
            panic!("Compilation should fail for invalid programs, but it succeeded");
        }
        Err(e) => {
            if stage_str == "lex" {
                // for lex stage tests, we expect a parser error due to the presence of error tokens
                let msg = format!("{:?}", e).to_lowercase();
                assert!(
                    matches!(msg, _ if
                        msg.contains("errortoken")
                    ),
                    "Expected a parser error for lex stage tests, but got: {e}"
                );
            }
        } // Expected error
    }
}

#[rstest]
fn test_valid(
    #[files("tests/valid/**/*.c")]
    #[mode = path]
    path: PathBuf,
) {
    let out = driver::compile_file(path.to_str().unwrap(), None, false)
        .expect("Compilation should succeed for valid programs");

    cleanup_out(out);
}

proptest! {
    #[test]
    fn doesnt_panic(program: String) {
        let _ = driver::compile(program, None, false);
    }
}
