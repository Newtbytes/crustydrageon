use std::{fmt, process};

use crate::{diag::Diag, src::Source};

#[derive(Debug)]
pub enum CompilerError {
    SysCompilerNotFound(&'static str),
    SysCompilerRaised(process::ExitStatus),
    SourceDiagnostic(Source, Diag),
    IoError,
}

impl CompilerError {
    #[must_use]
    pub fn sys_cc_err(status: process::ExitStatus) -> Self {
        CompilerError::SysCompilerRaised(status)
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SysCompilerRaised(exit_status) => write!(
                f,
                "A system compiler exited with status code {exit_status} during the compilation"
            ),
            Self::SourceDiagnostic(src, diag) => diag.fmt_for(f, src),
            Self::IoError => todo!(),
            Self::SysCompilerNotFound(msg) => write!(f, "{msg}"),
        }
    }
}

pub type CompilerResult<T> = Result<T, CompilerError>;
