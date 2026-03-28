use dx_llvm_ir::pipeline::emit_file_to_path;
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let Some(input) = args.next() else {
        return Err("usage: dx-emit-llvm <input.dx> [output.ll]".into());
    };
    if input == "--help" || input == "-h" {
        println!("usage: dx-emit-llvm <input.dx> [output.ll]");
        return Ok(());
    }
    let input = PathBuf::from(input);
    let output = match args.next() {
        Some(path) => PathBuf::from(path),
        None => input.with_extension("ll"),
    };
    if args.next().is_some() {
        return Err("usage: dx-emit-llvm <input.dx> [output.ll]".into());
    }
    emit_file_to_path(&input, &output)?;
    Ok(())
}
