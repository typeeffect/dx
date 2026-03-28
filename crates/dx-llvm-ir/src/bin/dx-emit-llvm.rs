use dx_llvm_ir::pipeline::{emit_file_to_path, emit_file_to_path_and_verify};
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    input: PathBuf,
    output: PathBuf,
    verify: bool,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = parse_args(std::env::args_os().skip(1))? else {
        println!("usage: dx-emit-llvm [--verify] <input.dx> [output.ll]");
        return Ok(());
    };
    if options.verify {
        emit_file_to_path_and_verify(&options.input, &options.output)?;
    } else {
        emit_file_to_path(&options.input, &options.output)?;
    }
    Ok(())
}

fn parse_args<I>(args: I) -> Result<Option<CliOptions>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut verify = false;
    let mut positional = Vec::new();

    for arg in args {
        let arg = arg.into();
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        }
        if arg == "--verify" {
            verify = true;
            continue;
        }
        positional.push(PathBuf::from(arg));
    }

    let input = positional
        .first()
        .cloned()
        .ok_or_else(|| "usage: dx-emit-llvm [--verify] <input.dx> [output.ll]".to_string())?;
    let output = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| input.with_extension("ll"));

    if positional.len() > 2 {
        return Err("usage: dx-emit-llvm [--verify] <input.dx> [output.ll]".into());
    }

    Ok(Some(CliOptions {
        input,
        output,
        verify,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_output_path() {
        let opts = parse_args(["input.dx"]).expect("parse").expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("input.dx"),
                output: PathBuf::from("input.ll"),
                verify: false,
            }
        );
    }

    #[test]
    fn parse_args_supports_verify_flag() {
        let opts = parse_args(["--verify", "input.dx", "out.ll"])
            .expect("parse")
            .expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("input.dx"),
                output: PathBuf::from("out.ll"),
                verify: true,
            }
        );
    }

    #[test]
    fn parse_args_handles_help() {
        let opts = parse_args(["--help"]).expect("parse");
        assert!(opts.is_none());
    }
}
