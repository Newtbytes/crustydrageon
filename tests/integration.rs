use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process,
};

use proptest::prelude::*;
use rstest::rstest;

use crustydrageon::driver;

fn cleanup_out(out: Option<PathBuf>) {
    if let Some(out) = out {
        fs::remove_file(out)
            .expect("Failed to remove temporary output file; this test might be broken");
    }
}

struct TestOutput(Option<PathBuf>);

impl Drop for TestOutput {
    fn drop(&mut self) {
        cleanup_out(self.0.clone())
    }
}

/// Parse the expected status given the CHECK STATUS directives in a program
fn expected_status(src: &str) -> Result<Option<i32>, Box<dyn Error>> {
    let directive = "//$ CHECK STATUS";

    // parse the expected status
    if let Some(idx) = src.find(directive) {
        let expected_status = {
            let status = src[idx..].trim_start_matches(directive);
            let status = status.lines().next().unwrap().trim();
            let status = status.trim_start_matches(":").trim();
            status.parse::<i32>()?
        };

        Ok(Some(expected_status))
    } else {
        Ok(None)
    }
}

/// Run a program and compare its output to the CHECK directives defined in its source code
fn check_program(src: String, out: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(expected_status) = expected_status(&src)? {
        let actual_status = process::Command::new(out).status()?.code();

        assert_eq!(actual_status, Some(expected_status));
    }

    Ok(())
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

    let out = driver::compile_file(path.clone().to_str().unwrap(), Some(final_stage), false);

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
    let out = TestOutput(
        driver::compile_file(path.to_str().unwrap(), None, false)
            .expect("Compilation should succeed for valid programs"),
    );

    if let Some(ref out) = out.0 {
        let src = fs::read_to_string(&path).unwrap();
        check_program(src, out).unwrap();
    }
}

proptest! {
    #[test]
    // FIXME: this tests only ASCII input programs
    fn doesnt_panic(program in "[ -~]" /* all printable ASCII characters */) {
        let _ = driver::compile(program, None, false);
    }
}
