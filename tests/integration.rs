use std::{fs, path::PathBuf};

use proptest::prelude::*;
use rstest::rstest;

use crustydrageon::driver;

mod check;

pub struct FileCleanupGuard {
    filename: PathBuf,
}

impl FileCleanupGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { filename: path }
    }
}

impl Drop for FileCleanupGuard {
    fn drop(&mut self) {
        if fs::exists(&self.filename).unwrap_or(false) {
            fs::remove_file(&self.filename).expect(&format!(
                "Failed to cleanup file: {}",
                self.filename.to_string_lossy()
            ));
        }
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

    let out = driver::compile_file(path.clone().to_str().unwrap(), Some(final_stage));
    let _cleanup = out.as_ref().map(|o| o.clone().map(FileCleanupGuard::new));

    match out {
        Ok(_) => {
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
fn test_return_code(
    #[files("tests/valid/**/*.c")]
    #[mode = path]
    path: PathBuf,
) {
    let out = driver::compile_file(path.to_str().unwrap(), None)
        .expect("Compilation should succeed for valid programs");
    let _cleanup = out.clone().map(FileCleanupGuard::new);

    if let Some(ref out) = out {
        let src = fs::read_to_string(&path).unwrap();
        check::check_status(&src, out).unwrap();
    }
}

#[rstest]
fn test_check_output(
    #[files("tests/valid/**/*.c")]
    #[mode = path]
    path: PathBuf,
) {
    let src = fs::read_to_string(&path).unwrap();
    check::check_outputs(&src);
}

proptest! {
    #[test]
    fn doesnt_panic(program: String) {
        let _ = driver::compile(program, None);
    }
}
