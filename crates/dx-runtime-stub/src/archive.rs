use crate::manifest::EXPORTED_SYMBOLS;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStubArtifactInfo {
    pub archive_path: PathBuf,
    pub exported_symbols: &'static [&'static str],
}

pub fn default_archive_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "dx_runtime_stub.lib"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "libdx_runtime_stub.a"
    }
}

pub fn default_archive_path(target_dir: &Path, profile: &str) -> PathBuf {
    target_dir.join(profile).join(default_archive_filename())
}

pub fn default_workspace_archive_path() -> PathBuf {
    workspace_archive_path(None, None)
}

pub fn workspace_archive_path(target_dir: Option<&Path>, profile: Option<&str>) -> PathBuf {
    let target_dir = target_dir
        .map(Path::to_path_buf)
        .or_else(configured_target_dir)
        .unwrap_or_else(|| PathBuf::from("target"));
    let profile = profile
        .map(ToOwned::to_owned)
        .or_else(configured_profile_dir)
        .unwrap_or_else(|| default_profile_dir().to_string());
    default_archive_path(&target_dir, &profile)
}

pub fn default_profile_dir() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

pub fn runtime_stub_artifact_info() -> RuntimeStubArtifactInfo {
    RuntimeStubArtifactInfo {
        archive_path: default_workspace_archive_path(),
        exported_symbols: EXPORTED_SYMBOLS,
    }
}

pub fn configured_target_dir() -> Option<PathBuf> {
    env::var_os("CARGO_TARGET_DIR").map(PathBuf::from)
}

pub fn configured_profile_dir() -> Option<String> {
    env::var("DX_RUNTIME_STUB_PROFILE").ok()
}

pub fn render_runtime_stub_artifact_info() -> String {
    let info = runtime_stub_artifact_info();
    let mut lines = vec![
        format!("archive {}", info.archive_path.display()),
        "symbols:".to_string(),
    ];
    for symbol in info.exported_symbols {
        lines.push(format!("  {symbol}"));
    }
    lines.join("\n")
}

pub fn render_runtime_stub_artifact_info_json() -> String {
    let info = runtime_stub_artifact_info();
    let symbols = info
        .exported_symbols
        .iter()
        .map(|symbol| format!("\"{}\"", json_escape(symbol)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"archive_path\":\"{}\",\"exported_symbols\":[{}]}}",
        json_escape(&info.archive_path.display().to_string()),
        symbols
    )
}

fn json_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_archive_path_uses_profile_subdir() {
        let path = default_archive_path(Path::new("target"), "debug");
        assert_eq!(
            path,
            PathBuf::from("target")
                .join("debug")
                .join(default_archive_filename())
        );
    }

    #[test]
    fn workspace_archive_path_points_into_target_profile() {
        let path = default_workspace_archive_path();
        assert!(path.starts_with("target"));
        assert!(path.to_string_lossy().contains(default_profile_dir()));
    }

    #[test]
    fn workspace_archive_path_can_use_explicit_target_dir_and_profile() {
        let path = workspace_archive_path(Some(Path::new("/tmp/dx-target")), Some("release"));
        assert_eq!(
            path,
            PathBuf::from("/tmp/dx-target")
                .join("release")
                .join(default_archive_filename())
        );
    }

    #[test]
    fn artifact_info_reuses_exported_symbol_manifest() {
        let info = runtime_stub_artifact_info();
        assert_eq!(info.exported_symbols, EXPORTED_SYMBOLS);
        assert!(info.archive_path.ends_with(default_archive_filename()));
    }

    #[test]
    fn rendered_artifact_info_is_deterministic() {
        assert_eq!(
            render_runtime_stub_artifact_info(),
            render_runtime_stub_artifact_info()
        );
    }

    #[test]
    fn rendered_artifact_info_json_is_deterministic() {
        assert_eq!(
            render_runtime_stub_artifact_info_json(),
            render_runtime_stub_artifact_info_json()
        );
    }
}
