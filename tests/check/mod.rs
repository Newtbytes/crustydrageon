use std::{collections::HashSet, error::Error, path::Path, process};

use const_format::concatcp;
use crustydrageon::driver::{self, CompilerStage};

const COMMENT: &str = "//";
const DIRECTIVE: &str = concatcp!(COMMENT, "$ ");

const OUTPUT_DIRECTIVE: &str = concatcp!(DIRECTIVE, "OUTPUT");
const CHECK_DIRECTIVE: &str = concatcp!(DIRECTIVE, "CHECK");
const STATUS_DIRECTIVE: &str = concatcp!(DIRECTIVE, "STATUS");

#[derive(PartialEq, Eq, Hash)]
enum OutputToCheck {
    CompilerStage(driver::CompilerStage),
}

impl From<driver::CompilerStage> for OutputToCheck {
    fn from(stage: driver::CompilerStage) -> Self {
        Self::CompilerStage(stage)
    }
}

impl OutputToCheck {
    pub fn acquire(&self, src: String) -> String {
        match &self {
            OutputToCheck::CompilerStage(stage) => {
                match driver::compile(src, Some(*stage)).unwrap() {
                    driver::CompilationOutput::Stopped(_, output) => output,
                    driver::CompilationOutput::Completed(_) => unreachable!(),
                }
            }
        }
    }
}

/// Find the values of all instances of a directive in a source file
fn find_directive_values<'a>(
    src: &'a str,
    directive: &'static str,
) -> impl Iterator<Item = &'a str> {
    src.match_indices(directive)
        .filter_map(|(idx, _)| src[idx..].lines().next())
        .map(move |s| {
            s.trim_start_matches(directive)
                .trim()
                .trim_start_matches(':')
                .trim()
        })
}

/// Parse the expected status given the STATUS directives in a program
fn parse_expected_status(src: &str) -> Result<Option<i32>, Box<dyn Error>> {
    // parse the expected status
    if let Some(idx) = src.find(STATUS_DIRECTIVE) {
        let expected_status = {
            let status = src[idx..].trim_start_matches(STATUS_DIRECTIVE);
            let status = status.lines().next().unwrap().trim();
            let status = status.trim_start_matches(":").trim();
            status.parse::<i32>()?
        };

        Ok(Some(expected_status))
    } else {
        Ok(None)
    }
}

/// Parse the output the CHECK directives are intended to be checked against
fn parse_checked_outputs(src: &str) -> HashSet<OutputToCheck> {
    let mut output_kinds = HashSet::new();

    for output in find_directive_values(src, OUTPUT_DIRECTIVE) {
        output_kinds.insert(match output.trim().to_lowercase().as_str() {
            "ast" | "parse" | "parsing" => CompilerStage::Parse.into(),
            "tacky" | "ir" => CompilerStage::IR.into(),
            "x86" => CompilerStage::Codegen.into(),
            _ => panic!("unknown output type: '{}'", output),
        });
    }

    output_kinds
}

/// Parse the program for CHECK directives
fn parse_checks(src: &str) -> impl Iterator<Item = &str> {
    find_directive_values(src, CHECK_DIRECTIVE)
}

pub fn assert_status(program: &Path, expected_status: i32) -> Result<(), Box<dyn Error>> {
    let actual_status = process::Command::new(program).status()?.code();
    assert_eq!(actual_status, Some(expected_status));

    Ok(())
}

pub fn assert_checks<'src>(outputs: &'src Vec<String>, checks: impl Iterator<Item = &'src str>) {
    for check in checks {
        for output in outputs {
            assert!(output.contains(check));
        }
    }
}

pub fn check_status(src: &str, out: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(expected_status) = parse_expected_status(&src)? {
        assert_status(out, expected_status)?;
    }

    Ok(())
}

pub fn check_outputs(src: &str) {
    let output_kinds = parse_checked_outputs(src);
    let outputs: Vec<String> = output_kinds
        .iter()
        .map(|k| k.acquire(src.to_owned()))
        .collect();

    let checks = parse_checks(&src);

    assert_checks(&outputs, checks);
}
