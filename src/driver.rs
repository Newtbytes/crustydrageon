use std::{
    error::Error,
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{self, Stdio},
};

use crate::{
    ast::Token,
    error::{CompilerError, CompilerResult},
    ir, lexer, parser,
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

    pub fn preprocess_file<'a>(
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

    pub fn preprocess(&self, src: &str) -> Result<String, Box<dyn Error>> {
        let mut child = self
            .command()
            .arg("-E")
            .arg("-P")
            .arg("-xc")
            .arg("-")
            .arg("-o-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let child_stdin = child.stdin.as_mut().unwrap();

        child_stdin.write_all(src.as_bytes())?;

        Ok(format!(
            "{}",
            str::from_utf8(&child.wait_with_output()?.stdout)?
        ))
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerStage {
    Lex,
    Parse,
    IR,
    Codegen,
}

pub enum CompilationOutput {
    Completed(x86::Program),
    Stopped(CompilerStage, String),
}

pub fn compile(
    program: String,
    stop_at: Option<CompilerStage>,
) -> CompilerResult<CompilationOutput> {
    let sys_cc = SysCompiler::try_new().map_err(CompilerError::SysCompilerNotFound)?;

    let src = sys_cc.preprocess(&program).unwrap();
    let src = Source::new(src);

    let tokens = lexer::tokenize(&src);

    if stop_at == Some(CompilerStage::Lex) {
        let output = format!("{:#?}", tokens.collect::<Vec<Token>>());
        return Ok(CompilationOutput::Stopped(CompilerStage::Lex, output));
    }

    let ast =
        parser::parse(tokens.peekable()).map_err(|e| CompilerError::ParserError(src.clone(), e))?;

    if stop_at == Some(CompilerStage::Parse) {
        let output = format!("{ast:#?}");
        return Ok(CompilationOutput::Stopped(CompilerStage::Parse, output));
    }

    let ir = ir::lower_program(ast);

    if stop_at == Some(CompilerStage::IR) {
        let output = format!("{ir}");
        return Ok(CompilationOutput::Stopped(CompilerStage::IR, output));
    }

    let asm: x86::Program = x86::lower_program(ir);

    if stop_at == Some(CompilerStage::Codegen) {
        let output = format!("{asm}");
        return Ok(CompilationOutput::Stopped(CompilerStage::Codegen, output));
    }

    Ok(CompilationOutput::Completed(asm))
}

pub fn compile_file(
    filename: &str,
    stop_at: Option<CompilerStage>,
) -> CompilerResult<Option<PathBuf>> {
    let sys_cc = SysCompiler::try_new().map_err(CompilerError::SysCompilerNotFound)?;

    let source_file = CompilerFile::from_path(Path::new(filename));
    let source = fs::read_to_string(source_file.filename()).map_err(|_| CompilerError::IoError)?;

    match compile(source, stop_at)? {
        CompilationOutput::Completed(asm) => {
            let mut asm_file: CompilerFile = source_file.with_kind(FileKind::ASM);

            asm_file
                .write(asm.to_string())
                .expect("Writing to intermediate file shouldn't fail");

            let out = sys_cc
                .assemble(asm_file)
                .map_err(CompilerError::sys_cc_err)?;

            Ok(Some(out.filename()))
        }
        CompilationOutput::Stopped(_, output) => {
            println!("{}", output);
            Ok(None)
        }
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
