use std::process;

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

type CompilerResult<T> = Result<T, CompilerError>;
