use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process,
};

use crate::{
    ast::Token,
    error::{CompilerError, CompilerResult},
    lexer, parser,
    src::Source,
    x86,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileKind {
    Source,
    Preprocessed,
    ASM,
    Out,
}

impl FileKind {
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            FileKind::Source => "c",
            FileKind::Preprocessed => "i",
            FileKind::ASM => "S",
            FileKind::Out => "",
        }
    }
}

impl From<&str> for FileKind {
    fn from(value: &str) -> Self {
        match value {
            "c" => FileKind::Source,
            "i" => FileKind::Preprocessed,
            "S" => FileKind::ASM,
            &_ => FileKind::Out,
        }
    }
}

pub struct CompilerFile<'a> {
    dir: Option<&'a Path>,
    name: &'a OsStr,
    kind: FileKind,
}

impl<'a> CompilerFile<'a> {
    pub fn write(&mut self, txt: String) -> Result<(), std::io::Error> {
        fs::write(self.filename(), txt)
    }

    #[must_use]
    pub fn from_path(path: &'a Path) -> Self {
        CompilerFile {
            dir: path.parent(),
            name: path.file_stem().expect("Compiler files should have a name"),
            kind: FileKind::from(
                path.extension()
                    .expect("Filename should have an extension")
                    .to_str()
                    .expect("Filename should be valid unicode"),
            ),
        }
    }

    #[must_use]
    pub fn filename(&self) -> PathBuf {
        if let Some(dir) = self.dir {
            dir.join(self.name).with_extension(self.kind.extension())
        } else {
            PathBuf::new()
                .with_file_name(self.name)
                .with_extension(self.kind.extension())
        }
    }

    #[must_use]
    pub fn with_kind(&self, kind: FileKind) -> Self {
        CompilerFile {
            dir: self.dir,
            name: self.name,
            kind,
        }
    }
}

impl Drop for CompilerFile<'_> {
    // Delete intermediate files (and not input/output files)
    fn drop(&mut self) {
        let result = match self.kind {
            FileKind::Source | FileKind::Out => Ok(()),
            _ => fs::remove_file(self.filename()),
        };

        if let Err(e) = result {
            unreachable!(
                "Intermediate file should always successfully be deleted: {}",
                e
            )
        }
    }
}

enum SysCompiler {
    CC,
    GCC,
    Clang,
}

impl SysCompiler {
    pub fn command(&self) -> process::Command {
        process::Command::new(match self {
            SysCompiler::CC => "cc",
            SysCompiler::GCC => "gcc",
            SysCompiler::Clang => "clang",
        })
    }

    fn can_preprocess(&self, kind: FileKind) -> bool {
        kind == FileKind::Source
    }

    fn can_assemble(&self, kind: FileKind) -> bool {
        kind == FileKind::ASM
    }

    pub fn preprocess<'a>(
        &self,
        file: CompilerFile<'a>,
    ) -> Result<CompilerFile<'a>, process::ExitStatus> {
        assert!(self.can_preprocess(file.kind));

        let preprocessed = file.with_kind(FileKind::Preprocessed);

        let status = self
            .command()
            .arg("-E")
            .arg("-P")
            .arg(file.filename())
            .arg("-o")
            .arg(preprocessed.filename())
            .status()
            .expect("command should successfully run to completion");

        if status.success() {
            Ok(preprocessed)
        } else {
            Err(status)
        }
    }

    pub fn assemble<'a>(
        &self,
        file: CompilerFile<'a>,
    ) -> Result<CompilerFile<'a>, process::ExitStatus> {
        assert!(self.can_assemble(file.kind));

        let compiled = file.with_kind(FileKind::Out);

        #[cfg(target_os = "linux")]
        let sys = "linux";

        #[cfg(target_os = "macos")]
        let sys = "darwin";

        #[cfg(target_vendor = "unknown")]
        let vendor = "unknown";

        #[cfg(target_vendor = "apple")]
        let vendor = "apple";

        #[cfg(target_env = "gnu")]
        let env = "gnu";

        #[cfg(target_env = "")]
        let env = "";

        let target_triple = format!("x86_64-{vendor}-{sys}-{env}");

        let status = self
            .command()
            .args(["-target", &target_triple])
            .arg(file.filename())
            .arg("-o")
            .arg(compiled.filename())
            .status()
            .expect("command should successfully run to completion");

        if status.success() {
            Ok(compiled)
        } else {
            Err(status)
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum CompilerStage {
    Lex,
    Parse,
    Codegen,
}

pub fn compile(
    program: String,
    stop_at: Option<CompilerStage>,
    verbose: bool,
) -> CompilerResult<Option<x86::Program>> {
    let src = Source::new(program);
    let tokens = lexer::tokenize(&src);

    if stop_at == Some(CompilerStage::Lex) {
        println!("{:#?}", tokens.collect::<Vec<Token>>());
        return Ok(None);
    }

    let ast = parser::parse(&mut tokens.peekable())
        .map_err(|e| CompilerError::ParserError(src.clone(), e))?;

    if verbose || stop_at == Some(CompilerStage::Parse) {
        println!("{ast:#?}");

        return Ok(None);
    }

    let asm: x86::Program = x86::lower_program(ast);

    if verbose || stop_at == Some(CompilerStage::Codegen) {
        println!("{asm}");
    }

    Ok(Some(asm))
}

pub fn compile_file(
    filename: &str,
    stop_at: Option<CompilerStage>,
    verbose: bool,
) -> CompilerResult<Option<PathBuf>> {
    let sys_cc = SysCompiler::CC;

    let source = CompilerFile::from_path(Path::new(filename));

    let preprocessed = sys_cc
        .preprocess(source)
        .map_err(CompilerError::sys_cc_err)?;

    let preprocessed_src =
        fs::read_to_string(preprocessed.filename()).map_err(|_| CompilerError::IoError)?;

    if let Some(asm) = compile(preprocessed_src, stop_at, verbose)? {
        let mut asm_file: CompilerFile = preprocessed.with_kind(FileKind::ASM);

        asm_file
            .write(asm.to_string())
            .expect("Writing to intermediate file shouldn't fail");

        let out = sys_cc
            .assemble(asm_file)
            .map_err(CompilerError::sys_cc_err)?;

        Ok(Some(out.filename()))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_file_fn() {
        let file = CompilerFile {
            dir: None,
            name: OsStr::new("test"),
            kind: FileKind::Source,
        };

        let filename = file.filename();

        assert_eq!(filename.file_name().unwrap(), "test.c");
        assert_eq!(filename.extension().unwrap(), "c");
    }

    #[test]
    fn test_compiler_file_fn_in_dir() {
        let dir: &str = "some/directory/that/doesnt/exist/";

        let file = CompilerFile {
            dir: Some(Path::new(dir)),
            name: OsStr::new("test"),
            kind: FileKind::Source,
        };

        let filename = file.filename();

        assert_eq!(filename.file_name().unwrap(), "test.c");
        assert_eq!(filename.extension().unwrap(), "c");
        assert!(
            filename.starts_with(dir),
            "{} should start with dir: {}",
            filename.display(),
            dir
        );
    }
}
