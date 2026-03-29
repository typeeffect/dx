use dx_llvm_ir::exec::{
    build_source_executable_plan,
    build_verified_executable_plan,
    discover_executable_tools,
    materialize_source_executable_plan,
    materialize_verified_executable_plan,
};
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
    verify: bool,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = parse_args(std::env::args_os().skip(1))? else {
        println!("usage: dx-build-exec [--verify] <input.dx> [build-dir]");
        return Ok(());
    };

    let tools = discover_executable_tools()?;
    if options.verify {
        let plan = build_verified_executable_plan(&options.input, &options.build_dir);
        materialize_verified_executable_plan(&plan, &tools)?;
        println!("{}", plan.source.executable.executable_path.display());
    } else {
        let plan = build_source_executable_plan(&options.input, &options.build_dir);
        materialize_source_executable_plan(&plan, &tools)?;
        println!("{}", plan.executable.executable_path.display());
    }

    Ok(())
}

fn parse_args<I>(args: I) -> Result<Option<CliOptions>, Box<dyn std::error::Error>>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut positional = Vec::new();
    let mut verify = false;
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
        .ok_or_else(|| "usage: dx-build-exec [--verify] <input.dx> [build-dir]".to_string())?;
    let build_dir = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| PathBuf::from("build"));

    if positional.len() > 2 {
        return Err("usage: dx-build-exec [--verify] <input.dx> [build-dir]".into());
    }

    Ok(Some(CliOptions {
        input,
        build_dir,
        verify,
    }))
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
                verify: false,
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
                verify: false,
            }
        );
    }

    #[test]
    fn parse_args_supports_verify_flag() {
        let opts = parse_args(["--verify", "examples/demo.dx"])
            .expect("parse")
            .expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("examples/demo.dx"),
                build_dir: PathBuf::from("build"),
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
