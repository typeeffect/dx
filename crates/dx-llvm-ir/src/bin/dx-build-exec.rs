use dx_llvm_ir::exec::{
    build_source_executable_plan,
    build_verified_executable_plan,
    discover_executable_tools,
    materialize_source_executable_plan,
    materialize_verified_executable_plan,
    render_source_executable_plan,
    render_verified_executable_plan,
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
    dry_run: bool,
    json: bool,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Some(options) = parse_args(std::env::args_os().skip(1))? else {
        println!("usage: dx-build-exec [--verify] [--dry-run] [--json] [--runtime-archive <path>] <input.dx> [build-dir]");
        return Ok(());
    };

    if options.verify {
        let mut plan = build_verified_executable_plan(&options.input, &options.build_dir);
        if let Some(runtime_archive) = &options.runtime_archive {
            plan.source.executable.runtime_archive = runtime_archive.clone();
            plan.source.executable.link_plan = dx_llvm_ir::build_link_command_plan(
                &plan.source.executable.ll_path,
                runtime_archive,
                &plan.source.executable.executable_path,
            );
        }
        if options.dry_run {
            if options.json {
                println!("{}", render_verified_plan_json(&plan));
            } else {
                println!("{}", render_verified_executable_plan(&plan));
            }
        } else {
            let tools = discover_executable_tools()?;
            materialize_verified_executable_plan(&plan, &tools)?;
            if options.json {
                println!("{}", render_verified_result_json(&plan));
            } else {
                println!("{}", plan.source.executable.executable_path.display());
            }
        }
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
        if options.dry_run {
            if options.json {
                println!("{}", render_source_plan_json(&plan));
            } else {
                println!("{}", render_source_executable_plan(&plan));
            }
        } else {
            let tools = discover_executable_tools()?;
            materialize_source_executable_plan(&plan, &tools)?;
            if options.json {
                println!("{}", render_source_result_json(&plan));
            } else {
                println!("{}", plan.executable.executable_path.display());
            }
        }
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
    let mut dry_run = false;
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
        if arg == "--dry-run" {
            dry_run = true;
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

    let input = positional
        .first()
        .cloned()
        .ok_or_else(|| "usage: dx-build-exec [--verify] [--dry-run] [--json] [--runtime-archive <path>] <input.dx> [build-dir]".to_string())?;
    let build_dir = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| PathBuf::from("build"));

    if positional.len() > 2 {
        return Err("usage: dx-build-exec [--verify] [--dry-run] [--json] [--runtime-archive <path>] <input.dx> [build-dir]".into());
    }

    Ok(Some(CliOptions {
        input,
        build_dir,
        runtime_archive,
        verify,
        dry_run,
        json,
    }))
}

fn render_source_plan_json(plan: &dx_llvm_ir::SourceExecutablePlan) -> String {
    format!(
        concat!(
            "{{",
            "\"mode\":\"source-plan\",",
            "\"input\":\"{}\",",
            "\"ll\":\"{}\",",
            "\"runtime_archive\":\"{}\",",
            "\"executable\":\"{}\",",
            "\"emit_command\":\"{}\",",
            "\"assemble_command\":\"{}\",",
            "\"compile_command\":\"{}\",",
            "\"link_command\":\"{}\"",
            "}}"
        ),
        json_escape(&plan.input_dx.display().to_string()),
        json_escape(&plan.executable.ll_path.display().to_string()),
        json_escape(&plan.executable.runtime_archive.display().to_string()),
        json_escape(&plan.executable.executable_path.display().to_string()),
        json_escape(&plan.emit_command.join(" ")),
        json_escape(&plan.executable.link_plan.assemble.join(" ")),
        json_escape(&plan.executable.link_plan.compile.join(" ")),
        json_escape(&plan.executable.link_plan.link.join(" ")),
    )
}

fn render_verified_plan_json(plan: &dx_llvm_ir::VerifiedExecutablePlan) -> String {
    format!(
        concat!(
            "{{",
            "\"mode\":\"verified-plan\",",
            "\"input\":\"{}\",",
            "\"ll\":\"{}\",",
            "\"runtime_archive\":\"{}\",",
            "\"executable\":\"{}\",",
            "\"emit_command\":\"{}\",",
            "\"assemble_command\":\"{}\",",
            "\"compile_command\":\"{}\",",
            "\"link_command\":\"{}\"",
            "}}"
        ),
        json_escape(&plan.source.input_dx.display().to_string()),
        json_escape(&plan.source.executable.ll_path.display().to_string()),
        json_escape(&plan.source.executable.runtime_archive.display().to_string()),
        json_escape(&plan.source.executable.executable_path.display().to_string()),
        json_escape(&plan.verify_emit_command.join(" ")),
        json_escape(&plan.source.executable.link_plan.assemble.join(" ")),
        json_escape(&plan.source.executable.link_plan.compile.join(" ")),
        json_escape(&plan.source.executable.link_plan.link.join(" ")),
    )
}

fn render_source_result_json(plan: &dx_llvm_ir::SourceExecutablePlan) -> String {
    format!(
        "{{\"mode\":\"source-build\",\"executable\":\"{}\"}}",
        json_escape(&plan.executable.executable_path.display().to_string())
    )
}

fn render_verified_result_json(plan: &dx_llvm_ir::VerifiedExecutablePlan) -> String {
    format!(
        "{{\"mode\":\"verified-build\",\"executable\":\"{}\"}}",
        json_escape(&plan.source.executable.executable_path.display().to_string())
    )
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
                dry_run: false,
                json: false,
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
                runtime_archive: None,
                verify: false,
                dry_run: false,
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
                dry_run: false,
                json: false,
            }
        );
    }

    #[test]
    fn parse_args_supports_dry_run_flag() {
        let opts = parse_args(["--dry-run", "examples/demo.dx"])
            .expect("parse")
            .expect("options");
        assert_eq!(
            opts,
            CliOptions {
                input: PathBuf::from("examples/demo.dx"),
                build_dir: PathBuf::from("build"),
                runtime_archive: None,
                verify: false,
                dry_run: true,
                json: false,
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
                dry_run: false,
                json: false,
            }
        );
    }

    #[test]
    fn parse_args_rejects_missing_runtime_archive_path() {
        let err = parse_args(["--runtime-archive"]).expect_err("missing path should fail");
        assert!(err.to_string().contains("--runtime-archive requires a path"));
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
                dry_run: false,
                json: true,
            }
        );
    }

    #[test]
    fn parse_args_handles_help() {
        let opts = parse_args(["--help"]).expect("parse");
        assert!(opts.is_none());
    }
}
