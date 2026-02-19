use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileKind {
    Source,
    Preprocessed,
    ASM,
    Out,
}

impl FileKind {
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

    pub fn filename(&self) -> PathBuf {
        if let Some(dir) = self.dir {
            dir.with_file_name(self.name)
                .with_extension(self.kind.extension())
        } else {
            PathBuf::new()
                .with_file_name(self.name)
                .with_extension(self.kind.extension())
        }
    }

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

    fn can_compile(&self, kind: FileKind) -> bool {
        kind == FileKind::Source || kind == FileKind::Preprocessed || kind == FileKind::ASM
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

        if !status.success() {
            Err(status)
        } else {
            Ok(preprocessed)
        }
    }

    pub fn compile<'a>(
        &self,
        file: CompilerFile<'a>,
    ) -> Result<CompilerFile<'a>, process::ExitStatus> {
        assert!(self.can_compile(file.kind));

        let compiled = file.with_kind(FileKind::Out);

        let status = self
            .command()
            .arg(file.filename())
            .arg("-o")
            .arg(compiled.filename())
            .status()
            .expect("command should successfully run to completion");

        if !status.success() {
            Err(status)
        } else {
            Ok(compiled)
        }
    }
}

pub enum CompilerError {
    SysCompilerError(process::ExitStatus),
    IoError,
}

impl CompilerError {
    pub fn sys_cc_err(status: process::ExitStatus) -> Self {
        CompilerError::SysCompilerError(status)
    }
}

pub fn compile(filename: &String) -> Result<PathBuf, CompilerError> {
    let sys_cc = SysCompiler::CC;

    let source = CompilerFile::from_path(Path::new(filename));

    let preprocessed = sys_cc
        .preprocess(source)
        .map_err(CompilerError::sys_cc_err)?;

    let preprocessed_src =
        fs::read_to_string(preprocessed.filename()).map_err(|_| CompilerError::IoError)?;

    std::println!("{}", preprocessed_src);

    let compiled = sys_cc
        .compile(preprocessed)
        .map_err(CompilerError::sys_cc_err)?;

    Ok(compiled.filename())
}
