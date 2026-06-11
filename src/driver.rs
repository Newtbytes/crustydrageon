use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process,
};

use crate::{
    ast::Token,
    error::{CompilerError, CompilerResult},
    ir, lexer, parser, sema,
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

#[derive(PartialEq, Eq)]
enum SysCompiler {
    CC,
    Gcc,
    Clang,
}

impl SysCompiler {
    pub fn try_new() -> Result<Self, &'static str> {
        if SysCompiler::Clang.installed() {
            Ok(SysCompiler::Clang)
        } else if SysCompiler::Gcc.installed() {
            Ok(SysCompiler::Gcc)
        } else if SysCompiler::CC.installed() {
            Ok(SysCompiler::CC)
        } else {
            Err(
                "Neither `clang`, `gcc`, nor `cc` commands were able to be executed. Are they installed and in a directory in PATH?",
            )
        }
    }

    fn name(&self) -> &'static str {
        match self {
            SysCompiler::CC => "cc",
            SysCompiler::Gcc => "gcc",
            SysCompiler::Clang => "clang",
        }
    }

    fn installed(&self) -> bool {
        match process::Command::new(self.name())
            .arg("-v")
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status()
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    pub fn command(&self) -> process::Command {
        process::Command::new(self.name())
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

    fn target_triple(&self) -> String {
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

        format!("x86_64-{vendor}-{sys}-{env}")
    }

    pub fn assemble<'a>(
        &self,
        file: CompilerFile<'a>,
    ) -> Result<CompilerFile<'a>, process::ExitStatus> {
        assert!(self.can_assemble(file.kind));

        let compiled = file.with_kind(FileKind::Out);

        let status = {
            let mut status = &mut self.command();

            if !cfg!(target_arch = "x86_64") && self == &SysCompiler::Clang {
                status = status.args(["-target", &self.target_triple()]);
            }

            status
                .arg(file.filename())
                .arg("-o")
                .arg(compiled.filename())
                .status()
                .expect("command should successfully run to completion")
        };

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
    Validate,
    IR,
    Codegen,
}

pub fn compile(
    program: String,
    stop_at: Option<CompilerStage>,
    verbose: bool,
) -> CompilerResult<Option<x86::Program>> {
    cov_mark::hit!(compilation);

    let src = Source::new(program);
    let tokens = lexer::tokenize(&src);
    cov_mark::hit!(tokenize);

    if stop_at == Some(CompilerStage::Lex) {
        println!("{:#?}", tokens.collect::<Vec<Token>>());
        return Ok(None);
    }

    let mut ast = parser::parse(src.clone(), tokens.peekable())
        .map_err(|e| CompilerError::SourceDiagnostic(src.clone(), Box::new(e)))?;
    cov_mark::hit!(parse);

    if verbose || stop_at == Some(CompilerStage::Parse) {
        println!("{ast:#?}");
        return Ok(None);
    }

    sema::resolve(&mut ast).map_err(|e| CompilerError::SourceDiagnostic(src, Box::new(e)))?;

    if verbose || stop_at == Some(CompilerStage::Validate) {
        println!("{ast:#?}");
        return Ok(None);
    }

    let ir = ir::lower_program(ast);
    cov_mark::hit!(lower_to_ir);

    if verbose || stop_at == Some(CompilerStage::IR) {
        println!("{ir}");
        return Ok(None);
    }

    let asm: x86::Program = x86::lower_program(ir);
    cov_mark::hit!(lower_to_x86);

    if verbose || stop_at == Some(CompilerStage::Codegen) {
        println!("{asm}");
        return Ok(None);
    }

    Ok(Some(asm))
}

pub fn compile_file(
    filename: &str,
    stop_at: Option<CompilerStage>,
    verbose: bool,
) -> CompilerResult<Option<PathBuf>> {
    let sys_cc = SysCompiler::try_new().map_err(CompilerError::SysCompilerNotFound)?;

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
