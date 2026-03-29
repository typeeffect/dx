use crate::archive::workspace_archive_path;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStubBuildPlan {
    pub env: Vec<(String, String)>,
    pub command: Vec<String>,
    pub archive_path: PathBuf,
}

pub fn build_runtime_stub_build_plan(
    profile: &str,
    target_dir: Option<&Path>,
) -> RuntimeStubBuildPlan {
    let mut env = Vec::new();
    if let Some(target_dir) = target_dir {
        env.push((
            "CARGO_TARGET_DIR".to_string(),
            target_dir.display().to_string(),
        ));
    }

    let mut command = vec![
        "cargo".to_string(),
        "build".to_string(),
        "-p".to_string(),
        "dx-runtime-stub".to_string(),
    ];
    match profile {
        "debug" => {}
        "release" => command.push("--release".to_string()),
        other => {
            command.push("--profile".to_string());
            command.push(other.to_string());
        }
    }

    RuntimeStubBuildPlan {
        env,
        command,
        archive_path: workspace_archive_path(target_dir, Some(profile)),
    }
}

pub fn render_runtime_stub_build_plan(profile: &str, target_dir: Option<&Path>) -> String {
    let plan = build_runtime_stub_build_plan(profile, target_dir);
    let mut lines = vec![];
    for (key, value) in &plan.env {
        lines.push(format!("{key}={value}"));
    }
    lines.push(plan.command.join(" "));
    lines.push(format!("archive {}", plan.archive_path.display()));
    lines.join("\n")
}

pub fn render_runtime_stub_build_plan_json(profile: &str, target_dir: Option<&Path>) -> String {
    let plan = build_runtime_stub_build_plan(profile, target_dir);
    let env = plan
        .env
        .iter()
        .map(|(key, value)| {
            format!(
                "{{\"key\":\"{}\",\"value\":\"{}\"}}",
                json_escape(key),
                json_escape(value)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{",
            "\"env\":[{}],",
            "\"command\":\"{}\",",
            "\"archive_path\":\"{}\"",
            "}}"
        ),
        env,
        json_escape(&plan.command.join(" ")),
        json_escape(&plan.archive_path.display().to_string()),
    )
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_build_plan_uses_plain_cargo_build() {
        let plan = build_runtime_stub_build_plan("debug", None);
        assert!(plan.env.is_empty());
        assert_eq!(
            plan.command,
            vec![
                "cargo".to_string(),
                "build".to_string(),
                "-p".to_string(),
                "dx-runtime-stub".to_string(),
            ]
        );
        assert!(plan.archive_path.ends_with("libdx_runtime_stub.a"));
    }

    #[test]
    fn release_build_plan_uses_release_flag() {
        let plan = build_runtime_stub_build_plan("release", None);
        assert!(plan.command.iter().any(|arg| arg == "--release"));
        assert!(plan.archive_path.to_string_lossy().contains("release"));
    }

    #[test]
    fn custom_profile_and_target_dir_are_preserved() {
        let plan = build_runtime_stub_build_plan("dist", Some(Path::new("/tmp/dx-target")));
        assert_eq!(
            plan.env,
            vec![("CARGO_TARGET_DIR".to_string(), "/tmp/dx-target".to_string())]
        );
        assert_eq!(
            plan.command,
            vec![
                "cargo".to_string(),
                "build".to_string(),
                "-p".to_string(),
                "dx-runtime-stub".to_string(),
                "--profile".to_string(),
                "dist".to_string(),
            ]
        );
        assert_eq!(
            plan.archive_path,
            PathBuf::from("/tmp/dx-target").join("dist").join("libdx_runtime_stub.a")
        );
    }

    #[test]
    fn rendered_build_plan_is_deterministic() {
        let a = render_runtime_stub_build_plan("release", Some(Path::new("/tmp/dx-target")));
        let b = render_runtime_stub_build_plan("release", Some(Path::new("/tmp/dx-target")));
        assert_eq!(a, b);
    }

    #[test]
    fn rendered_build_plan_json_is_deterministic() {
        let a = render_runtime_stub_build_plan_json("release", Some(Path::new("/tmp/dx-target")));
        let b = render_runtime_stub_build_plan_json("release", Some(Path::new("/tmp/dx-target")));
        assert_eq!(a, b);
    }
}
