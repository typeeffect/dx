use dx_llvm_ir::exec::{
    build_and_run_source_executable_plan,
    build_and_run_verified_executable_plan,
    build_source_executable_plan,
    build_verified_executable_plan,
    discover_executable_tools,
    ExecutableRunResult,
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
    runtime_archive: Option<PathBuf>,
    verify: bool,
    json: bool,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = parse_args(std::env::args_os().skip(1))? else {
        println!(
            "usage: dx-run-exec [--verify] [--json] [--runtime-archive <path>] <input.dx> [build-dir]"
        );
        return Ok(());
    };

    let tools = discover_executable_tools()?;
    let result = if options.verify {
        let mut plan = build_verified_executable_plan(&options.input, &options.build_dir);
        if let Some(runtime_archive) = &options.runtime_archive {
            plan.source.executable.runtime_archive = runtime_archive.clone();
            plan.source.executable.link_plan = dx_llvm_ir::build_link_command_plan(
                &plan.source.executable.ll_path,
                runtime_archive,
                &plan.source.executable.executable_path,
            );
        }
        build_and_run_verified_executable_plan(&plan, &tools)?
    } else {
        let mut plan = build_source_executable_plan(&options.input, &options.build_dir);
        if let Some(runtime_archive) = &options.runtime_archive {
            plan.executable.runtime_archive = runtime_archive.clone();
            plan.executable.link_plan = dx_llvm_ir::build_link_command_plan(
                &plan.executable.ll_path,
                runtime_archive,
                &plan.executable.executable_path,
            );
        }
        build_and_run_source_executable_plan(&plan, &tools)?
    };

    if options.json {
        println!("{}", render_result_json(&result));
    } else {
        println!(
            "{} {}",
            result.executable_path.display(),
            result
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_string())
        );
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
    let mut json = false;
    let mut runtime_archive = None;
    let mut args = args.into_iter().map(Into::into).peekable();
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return Ok(None);
        }
        if arg == "--verify" {
            verify = true;
            continue;
        }
        if arg == "--json" {
            json = true;
            continue;
        }
        if arg == "--runtime-archive" {
            runtime_archive = Some(
                args.next()
                    .map(PathBuf::from)
                    .ok_or_else(|| "--runtime-archive requires a path".to_string())?,
            );
            continue;
        }
        positional.push(PathBuf::from(arg));
    }

    let input = positional.first().cloned().ok_or_else(|| {
        "usage: dx-run-exec [--verify] [--json] [--runtime-archive <path>] <input.dx> [build-dir]"
            .to_string()
    })?;
    let build_dir = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| PathBuf::from("build"));

    if positional.len() > 2 {
        return Err(
            "usage: dx-run-exec [--verify] [--json] [--runtime-archive <path>] <input.dx> [build-dir]"
                .into(),
        );
    }

    Ok(Some(CliOptions {
        input,
        build_dir,
        runtime_archive,
        verify,
        json,
    }))
}

fn render_result_json(result: &ExecutableRunResult) -> String {
    match result.exit_code {
        Some(exit_code) => format!(
            "{{\"executable\":\"{}\",\"exit_code\":{exit_code}}}",
            json_escape(&result.executable_path.display().to_string())
        ),
        None => format!(
            "{{\"executable\":\"{}\",\"exit_code\":null}}",
            json_escape(&result.executable_path.display().to_string())
        ),
    }
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
                runtime_archive: None,
                verify: false,
                json: false,
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
                runtime_archive: None,
                verify: true,
                json: false,
            }
        );
    }

    #[test]
    fn parse_args_supports_json_flag() {
        let opts = parse_args(["--json", "examples/demo.dx"])
            .expect("parse")
            .expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("examples/demo.dx"),
                build_dir: PathBuf::from("build"),
                runtime_archive: None,
                verify: false,
                json: true,
            }
        );
    }

    #[test]
    fn parse_args_supports_runtime_archive_override() {
        let opts = parse_args([
            "--runtime-archive",
            "/tmp/libdx_runtime_stub.a",
            "examples/demo.dx",
        ])
        .expect("parse")
        .expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("examples/demo.dx"),
                build_dir: PathBuf::from("build"),
                runtime_archive: Some(PathBuf::from("/tmp/libdx_runtime_stub.a")),
                verify: false,
                json: false,
            }
        );
    }

    #[test]
    fn parse_args_handles_help() {
        assert!(parse_args(["--help"]).expect("parse").is_none());
    }

    #[test]
    fn render_result_json_reports_exit_code() {
        let result = ExecutableRunResult {
            executable_path: PathBuf::from("/tmp/demo"),
            exit_code: Some(0),
        };

        assert_eq!(
            render_result_json(&result),
            "{\"executable\":\"/tmp/demo\",\"exit_code\":0}"
        );
    }
}
