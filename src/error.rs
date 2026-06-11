use std::{fmt, process};

use crate::{diag::Annotation, src::Source};

#[derive(Debug)]
pub enum CompilerError {
    SysCompilerNotFound(&'static str),
    SysCompilerError(process::ExitStatus),
    SourceDiagnostic(Source, Box<dyn Annotation>),
    IoError,
}

impl CompilerError {
    #[must_use]
    pub fn sys_cc_err(status: process::ExitStatus) -> Self {
        CompilerError::SysCompilerError(status)
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SysCompilerError(exit_status) => write!(
                f,
                "A system compiler exited with status code {exit_status} during the compilation"
            ),
            Self::SourceDiagnostic(src, annotation) => annotation.fmt_for(f, src),
            Self::IoError => todo!(),
            Self::SysCompilerNotFound(msg) => write!(f, "{msg}"),
        }
    }
}

pub type CompilerResult<T> = Result<T, CompilerError>;
