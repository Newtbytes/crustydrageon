use std::fmt::{self, Debug};

use crate::src::{self, Source};

#[derive(Debug)]
pub enum DiagLevel {
    Error,
    Warn,
    Info,
}

#[derive(Debug)]
pub struct Diag {
    level: DiagLevel,
    annotations: Vec<Annotation>,
}

impl Diag {
    pub fn new(level: DiagLevel) -> Self {
        Diag {
            level,
            annotations: Vec::new(),
        }
    }

    pub fn annotate(&mut self, annotation: Annotation) -> &mut Self {
        self.annotations.push(annotation);
        self
    }

    pub fn fmt_for(&self, f: &mut std::fmt::Formatter<'_>, src: &Source) -> fmt::Result {
        writeln!(f, "{:?}:", self.level)?;

        for (i, annotation) in self.annotations.iter().enumerate() {
            if i != 0 {
                write!(f, "\n\n")?;
            }
            annotation.fmt_for(f, src)?;
        }

        Ok(())
    }
}

pub trait Diagnostic {
    fn into_diag(self) -> Diag;
}

#[derive(Debug)]
pub struct Annotation {
    pub span: src::Span,
    pub msg: String,
}

impl Annotation {
    fn fmt_for(&self, f: &mut std::fmt::Formatter<'_>, src: &Source) -> fmt::Result {
        let span = &self.span;

        writeln!(
            f,
            "at line {}, column {}:",
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
                write!(f, "{} ", "^".repeat(marker_end - marker_start))?;
            }
        }

        write!(f, "{}", self.msg)
    }
}
