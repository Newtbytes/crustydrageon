use std::{fmt, process};

use crate::{parser::ParserError, sema::ResolveError, src::Source};

#[derive(Debug)]
pub enum CompilerError {
    SysCompilerNotFound(&'static str),
    SysCompilerError(process::ExitStatus),
    ParserError(Source, Box<ParserError>),
    ResolutionError(Source, ResolveError),
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
            Self::ParserError(src, parser_error) => {
                writeln!(f, "Error while parsing:")?;

                if let Some(span) = parser_error.span() {
                    writeln!(
                        f,
                        " at line {}, column {}:",
                        span.start().line(),
                        span.start().column()
                    )?;

                    let start_line = src.find_line(span.start_index()).unwrap();
                    let end_line = src.find_line(span.end_index()).unwrap();
                    let start_col = span.start().column();
                    let end_col = src.find_column(span.end_index()).unwrap();

                    for i in start_line..=end_line {
                        if let Some(line) = src.lines().nth(i) {
                            writeln!(f, "{line}")?;

                            let marker_start: usize;
                            let marker_end: usize;

                            if i == start_line && i == end_line {
                                marker_start = start_col;
                                marker_end = end_col;
                            } else if i == start_line {
                                marker_start = start_col;
                                marker_end = 0;
                            } else if i == end_line {
                                marker_start = 0;
                                marker_end = end_col;
                            } else if start_line < i && i < end_line {
                                marker_start = 0;
                                marker_end = line.len() - 1;
                            } else {
                                marker_start = 0;
                                marker_end = 0;
                            }

                            write!(f, "{}", " ".repeat(marker_start))?;
                            writeln!(f, "{}", "^".repeat(marker_end - marker_start))?;
                        }
                    }
                }
                write!(f, "{parser_error}")
            }
            Self::IoError => todo!(),
            Self::SysCompilerNotFound(msg) => write!(f, "{msg}"),
            Self::ResolutionError(_, msg) => {
                write!(f, "Error during variable resolution {msg}")
            }
        }
    }
}

pub type CompilerResult<T> = Result<T, CompilerError>;
