use crate::archive::runtime_stub_artifact_info;
use crate::link::{build_link_command, runtime_stub_link_inputs};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStubPlan {
    pub archive_path: PathBuf,
    pub exported_symbols: &'static [&'static str],
    pub linker: &'static str,
    pub example_link_command: Vec<String>,
}

pub fn build_runtime_stub_plan(example_object: &Path, example_output: &Path) -> RuntimeStubPlan {
    let artifact = runtime_stub_artifact_info();
    let link_inputs = runtime_stub_link_inputs();

    RuntimeStubPlan {
        archive_path: artifact.archive_path,
        exported_symbols: artifact.exported_symbols,
        linker: link_inputs.linker,
        example_link_command: build_link_command(example_object, example_output),
    }
}

pub fn render_runtime_stub_plan(example_object: &Path, example_output: &Path) -> String {
    let plan = build_runtime_stub_plan(example_object, example_output);
    let mut lines = vec![
        "=== Runtime Stub Plan ===".to_string(),
        format!("archive {}", plan.archive_path.display()),
        format!("linker {}", plan.linker),
        "symbols:".to_string(),
    ];
    for symbol in plan.exported_symbols {
        lines.push(format!("  {symbol}"));
    }
    lines.push("example link:".to_string());
    lines.push(format!("  {}", plan.example_link_command.join(" ")));
    lines.join("\n")
}

pub fn render_runtime_stub_plan_json(example_object: &Path, example_output: &Path) -> String {
    let plan = build_runtime_stub_plan(example_object, example_output);
    let symbols = plan
        .exported_symbols
        .iter()
        .map(|symbol| format!("\"{}\"", json_escape(symbol)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{",
            "\"archive_path\":\"{}\",",
            "\"linker\":\"{}\",",
            "\"exported_symbols\":[{}],",
            "\"example_link_command\":\"{}\"",
            "}}"
        ),
        json_escape(&plan.archive_path.display().to_string()),
        json_escape(plan.linker),
        symbols,
        json_escape(&plan.example_link_command.join(" ")),
    )
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::default_archive_filename;

    #[test]
    fn plan_reuses_archive_and_symbol_manifest() {
        let plan = build_runtime_stub_plan(Path::new("build/demo.o"), Path::new("build/demo"));
        assert!(plan.archive_path.ends_with(default_archive_filename()));
        assert!(plan
            .exported_symbols
            .contains(&"dx_rt_throw_check_pending"));
        assert_eq!(plan.linker, "cc");
    }

    #[test]
    fn plan_renders_example_link_command() {
        let rendered = render_runtime_stub_plan(
            Path::new("build/demo.o"),
            Path::new("build/demo"),
        );
        assert!(rendered.contains("=== Runtime Stub Plan ==="));
        assert!(rendered.contains("archive target/"));
        assert!(rendered.contains("dx_rt_match_tag"));
        assert!(rendered.contains("cc build/demo.o"));
    }

    #[test]
    fn rendered_plan_is_deterministic() {
        let a = render_runtime_stub_plan(
            Path::new("build/demo.o"),
            Path::new("build/demo"),
        );
        let b = render_runtime_stub_plan(
            Path::new("build/demo.o"),
            Path::new("build/demo"),
        );
        assert_eq!(a, b);
    }

    #[test]
    fn rendered_plan_json_is_deterministic() {
        let a = render_runtime_stub_plan_json(
            Path::new("build/demo.o"),
            Path::new("build/demo"),
        );
        let b = render_runtime_stub_plan_json(
            Path::new("build/demo.o"),
            Path::new("build/demo"),
        );
        assert_eq!(a, b);
    }
}
