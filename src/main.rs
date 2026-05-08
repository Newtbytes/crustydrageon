use crustydrageon::{cli, driver};

fn main() -> Result<(), String> {
    let src_fn = cli::positional_arg(0).expect("source filename should be first argument");

    let lex = cli::flag("lex");
    let parse = cli::flag("parse");
    let codegen = cli::flag("codegen");
    let verbose = cli::flag("verbose");

    if let Some(filename) = driver::compile_file(
        &src_fn,
        driver::FinalCompilerStage::new(lex, parse, codegen),
        verbose,
    )
    .inspect_err(|e| println!("{e}"))
    .unwrap_or(None)
    {
        println!("{}", filename.display());
    }

    Ok(())
}
