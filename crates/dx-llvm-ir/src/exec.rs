use crate::link::{build_link_command_plan, LinkCommandPlan};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutablePlan {
    pub ll_path: PathBuf,
    pub runtime_archive: PathBuf,
    pub executable_path: PathBuf,
    pub link_plan: LinkCommandPlan,
}

pub fn build_executable_plan_from_ll(
    ll_path: &Path,
    runtime_archive: &Path,
    executable_path: &Path,
) -> ExecutablePlan {
    let link_plan = build_link_command_plan(ll_path, runtime_archive, executable_path);
    ExecutablePlan {
        ll_path: ll_path.to_path_buf(),
        runtime_archive: runtime_archive.to_path_buf(),
        executable_path: executable_path.to_path_buf(),
        link_plan,
    }
}

pub fn build_executable_plan_from_source(input_dx: &Path, build_dir: &Path) -> ExecutablePlan {
    let stem = input_dx
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    let ll_path = build_dir.join(format!("{stem}.ll"));
    let executable_path = build_dir.join(stem);
    let runtime_archive = default_runtime_archive_path();
    build_executable_plan_from_ll(&ll_path, &runtime_archive, &executable_path)
}

pub fn default_runtime_archive_path() -> PathBuf {
    PathBuf::from("target")
        .join(default_target_profile_dir())
        .join(default_runtime_archive_filename())
}

fn default_target_profile_dir() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn default_runtime_archive_filename() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "dx_runtime_stub.lib"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "libdx_runtime_stub.a"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_source_based_plan_with_expected_paths() {
        let plan = build_executable_plan_from_source(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );

        assert_eq!(plan.ll_path, PathBuf::from("build/demo.ll"));
        assert_eq!(plan.executable_path, PathBuf::from("build/demo"));
        assert!(plan
            .runtime_archive
            .ends_with(default_runtime_archive_filename()));
    }

    #[test]
    fn ll_based_plan_reuses_link_command_plan() {
        let runtime = Path::new("target/debug/libdx_runtime_stub.a");
        let plan = build_executable_plan_from_ll(
            Path::new("build/demo.ll"),
            runtime,
            Path::new("build/demo"),
        );

        assert_eq!(plan.link_plan.bitcode_path, PathBuf::from("build/demo.bc"));
        assert!(plan
            .link_plan
            .link
            .iter()
            .any(|arg| arg == &runtime.display().to_string()));
    }

    #[test]
    fn default_runtime_archive_path_points_into_target_profile_dir() {
        let path = default_runtime_archive_path();
        assert!(path.starts_with("target"));
        assert!(path
            .to_string_lossy()
            .contains(default_target_profile_dir()));
    }
}
