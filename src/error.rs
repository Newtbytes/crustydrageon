use std::{fmt, process};

use crate::parser::ParserError;

#[derive(Debug)]
pub enum CompilerError {
    SysCompilerError(process::ExitStatus),
    ParserError(ParserError),
    IoError,
}

impl CompilerError {
    pub fn sys_cc_err(status: process::ExitStatus) -> Self {
        CompilerError::SysCompilerError(status)
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::SysCompilerError(exit_status) => write!(
                f,
                "A system compiler exited with status code {} during the compilation",
                exit_status
            ),
            CompilerError::ParserError(parser_error) => write!(f, "{}", parser_error),
            CompilerError::IoError => todo!(),
        }
    }
}

pub type CompilerResult<T> = Result<T, CompilerError>;
