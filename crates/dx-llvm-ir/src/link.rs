use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkCommandPlan {
    pub bitcode_path: PathBuf,
    pub object_path: PathBuf,
    pub assemble: Vec<String>,
    pub compile: Vec<String>,
    pub link: Vec<String>,
}

pub fn build_link_command_plan(
    ll_path: &Path,
    runtime_archive: &Path,
    output_executable: &Path,
) -> LinkCommandPlan {
    let bitcode_path = output_executable.with_extension("bc");
    let object_path = output_executable.with_extension("o");

    let assemble = vec![
        "llvm-as".to_string(),
        ll_path.display().to_string(),
        "-o".to_string(),
        bitcode_path.display().to_string(),
    ];

    let compile = vec![
        "llc".to_string(),
        "-filetype=obj".to_string(),
        bitcode_path.display().to_string(),
        "-o".to_string(),
        object_path.display().to_string(),
    ];

    let link = vec![
        "cc".to_string(),
        object_path.display().to_string(),
        runtime_archive.display().to_string(),
        "-o".to_string(),
        output_executable.display().to_string(),
    ];

    LinkCommandPlan {
        bitcode_path,
        object_path,
        assemble,
        compile,
        link,
    }
}

pub fn render_link_plan(plan: &LinkCommandPlan) -> String {
    [
        plan.assemble.join(" "),
        plan.compile.join(" "),
        plan.link.join(" "),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_intermediate_paths_from_output_executable() {
        let plan = build_link_command_plan(
            Path::new("build/program.ll"),
            Path::new("target/debug/libdx_runtime_stub.a"),
            Path::new("build/program"),
        );

        assert_eq!(plan.bitcode_path, PathBuf::from("build/program.bc"));
        assert_eq!(plan.object_path, PathBuf::from("build/program.o"));
    }

    #[test]
    fn includes_runtime_archive_in_final_link_step() {
        let runtime = Path::new("target/debug/libdx_runtime_stub.a");
        let plan = build_link_command_plan(
            Path::new("build/program.ll"),
            runtime,
            Path::new("build/program"),
        );

        assert!(plan
            .link
            .iter()
            .any(|arg| arg == &runtime.display().to_string()));
    }

    #[test]
    fn rendered_plan_is_deterministic() {
        let a = build_link_command_plan(
            Path::new("build/program.ll"),
            Path::new("target/debug/libdx_runtime_stub.a"),
            Path::new("build/program"),
        );
        let b = build_link_command_plan(
            Path::new("build/program.ll"),
            Path::new("target/debug/libdx_runtime_stub.a"),
            Path::new("build/program"),
        );

        assert_eq!(render_link_plan(&a), render_link_plan(&b));
    }
}
