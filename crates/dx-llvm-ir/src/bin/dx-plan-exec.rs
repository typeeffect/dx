use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
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
    build_dir: PathBuf,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = parse_args(std::env::args_os().skip(1))? else {
        println!("usage: dx-plan-exec <input.dx> [build-dir]");
        return Ok(());
    };
    let plan = build_source_executable_plan(&options.input, &options.build_dir);
    println!("{}", render_source_executable_plan(&plan));
    Ok(())
}

fn parse_args<I>(args: I) -> Result<Option<CliOptions>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut positional = Vec::new();
    for arg in args {
        let arg = arg.into();
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        }
        positional.push(PathBuf::from(arg));
    }

    let input = positional
        .first()
        .cloned()
        .ok_or_else(|| "usage: dx-plan-exec <input.dx> [build-dir]".to_string())?;
    let build_dir = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| PathBuf::from("build"));

    if positional.len() > 2 {
        return Err("usage: dx-plan-exec <input.dx> [build-dir]".into());
    }

    Ok(Some(CliOptions { input, build_dir }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_build_dir() {
        let opts = parse_args(["examples/demo.dx"]).expect("parse").expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("examples/demo.dx"),
                build_dir: PathBuf::from("build"),
            }
        );
    }

    #[test]
    fn parse_args_accepts_explicit_build_dir() {
        let opts = parse_args(["examples/demo.dx", "out"]).expect("parse").expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("examples/demo.dx"),
                build_dir: PathBuf::from("out"),
            }
        );
    }

    #[test]
    fn parse_args_handles_help() {
        let opts = parse_args(["--help"]).expect("parse");
        assert!(opts.is_none());
    }
}
