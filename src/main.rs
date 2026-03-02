use crustydrageon::{cli, driver, error};

fn main() -> Result<(), error::CompilerError> {
    let src_fn = cli::positional_arg(0).expect("source filename should be first argument");

    let lex = cli::flag("lex");
    let parse = cli::flag("parse");
    let codegen = cli::flag("codegen");

    let filename = driver::compile(
        &src_fn,
        driver::FinalCompilerStage::new(lex, parse, codegen),
    )?;

    println!("{}", filename.display());

    Ok(())
}
