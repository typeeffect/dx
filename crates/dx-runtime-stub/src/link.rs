use crate::archive::default_workspace_archive_path;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStubLinkInputs {
    pub archive_path: PathBuf,
    pub linker: &'static str,
    pub extra_args: &'static [&'static str],
}

pub fn runtime_stub_link_inputs() -> RuntimeStubLinkInputs {
    RuntimeStubLinkInputs {
        archive_path: default_workspace_archive_path(),
        linker: default_linker(),
        extra_args: default_extra_args(),
    }
}

pub fn build_link_command(object_path: &Path, output_path: &Path) -> Vec<String> {
    let inputs = runtime_stub_link_inputs();
    let mut command = vec![
        inputs.linker.to_string(),
        object_path.display().to_string(),
        inputs.archive_path.display().to_string(),
    ];
    command.extend(inputs.extra_args.iter().map(|arg| (*arg).to_string()));
    command.push("-o".to_string());
    command.push(output_path.display().to_string());
    command
}

pub fn render_link_command(object_path: &Path, output_path: &Path) -> String {
    build_link_command(object_path, output_path).join(" ")
}

fn default_linker() -> &'static str {
    "cc"
}

fn default_extra_args() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_inputs_point_at_workspace_archive() {
        let inputs = runtime_stub_link_inputs();
        assert_eq!(inputs.archive_path, default_workspace_archive_path());
        assert_eq!(inputs.linker, "cc");
        assert!(inputs.extra_args.is_empty());
    }

    #[test]
    fn link_command_includes_object_archive_and_output() {
        let command = build_link_command(Path::new("build/demo.o"), Path::new("build/demo"));
        assert_eq!(command[0], "cc");
        assert!(command.iter().any(|part| part == "build/demo.o"));
        assert!(command
            .iter()
            .any(|part| part.ends_with(crate::archive::default_archive_filename())));
        assert_eq!(command[command.len() - 2], "-o");
        assert_eq!(command[command.len() - 1], "build/demo");
    }

    #[test]
    fn rendered_link_command_is_deterministic() {
        let a = render_link_command(Path::new("build/demo.o"), Path::new("build/demo"));
        let b = render_link_command(Path::new("build/demo.o"), Path::new("build/demo"));
        assert_eq!(a, b);
    }
}
