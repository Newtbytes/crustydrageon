use crustydrageon::{
    cli,
    driver::{self, CompilerStage},
};

fn main() -> Result<(), String> {
    let src_fn = cli::positional_arg(0).expect("source filename should be first argument");

    let lex = cli::flag("--lex");
    let parse = cli::flag("--parse");
    let codegen = cli::flag("--codegen") || cli::flag("--assemble") || cli::flag("-S");

    let verbose = cli::flag("--verbose");

    let stop_at = if lex {
        Some(CompilerStage::Lex)
    } else if parse {
        Some(CompilerStage::Parse)
    } else if codegen {
        Some(CompilerStage::Codegen)
    } else {
        None
    };

    if let Some(filename) = driver::compile_file(&src_fn, stop_at, verbose)
        .inspect_err(|e| println!("{e}"))
        .unwrap_or(None)
    {
        println!("{}", filename.display());
    }

    Ok(())
}
