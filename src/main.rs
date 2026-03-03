use crustydrageon::{cli, driver, error};

fn main() -> Result<(), error::CompilerError> {
    let src_fn = cli::positional_arg(0).expect("source filename should be first argument");

    let lex = cli::flag("lex");
    let parse = cli::flag("parse");
    let codegen = cli::flag("codegen");
    let verbose = cli::flag("verbose");

    if let Some(filename) = driver::compile(
        &src_fn,
        driver::FinalCompilerStage::new(lex, parse, codegen),
        verbose,
    )? {
        println!("{}", filename.display());
    }

    Ok(())
}
