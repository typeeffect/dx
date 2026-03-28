use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlvmToolchain {
    pub llvm_as: PathBuf,
    pub opt: Option<PathBuf>,
    pub llc: Option<PathBuf>,
}

#[derive(Debug)]
pub enum ToolchainError {
    MissingTool(&'static str),
    Io(std::io::Error),
    CommandFailed {
        tool: &'static str,
        status: Option<i32>,
        stderr: String,
    },
}

impl std::fmt::Display for ToolchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolchainError::MissingTool(tool) => write!(f, "missing LLVM tool: {tool}"),
            ToolchainError::Io(err) => write!(f, "i/o error: {err}"),
            ToolchainError::CommandFailed { tool, status, stderr } => {
                write!(f, "{tool} failed")?;
                if let Some(status) = status {
                    write!(f, " with exit code {status}")?;
                }
                if !stderr.trim().is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ToolchainError {}

impl From<std::io::Error> for ToolchainError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl LlvmToolchain {
    pub fn discover() -> Option<Self> {
        Self::discover_with_env(|key| env::var_os(key), env::split_paths)
    }

    fn discover_with_env<F, G>(get_var: F, split_paths_fn: G) -> Option<Self>
    where
        F: Fn(&str) -> Option<OsString>,
        G: Fn(&OsString) -> env::SplitPaths<'_>,
    {
        let llvm_as = explicit_or_path("DX_LLVM_AS", "llvm-as", &get_var, &split_paths_fn)?;
        let opt = explicit_or_path("DX_LLVM_OPT", "opt", &get_var, &split_paths_fn);
        let llc = explicit_or_path("DX_LLVM_LLC", "llc", &get_var, &split_paths_fn);
        Some(Self { llvm_as, opt, llc })
    }

    pub fn verify_ll_file(&self, ll_path: &Path) -> Result<(), ToolchainError> {
        run_tool(
            "llvm-as",
            &self.llvm_as,
            &[ll_path.as_os_str(), OsString::from("-o").as_os_str(), OsString::from("/dev/null").as_os_str()],
        )?;

        if let Some(opt) = &self.opt {
            run_tool(
                "opt",
                opt,
                &[
                    OsString::from("-disable-output").as_os_str(),
                    OsString::from("-verify").as_os_str(),
                    ll_path.as_os_str(),
                ],
            )?;
        }

        Ok(())
    }
}

fn explicit_or_path<F, G>(
    env_key: &str,
    binary: &str,
    get_var: &F,
    split_paths_fn: &G,
) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
    G: Fn(&OsString) -> env::SplitPaths<'_>,
{
    if let Some(path) = get_var(env_key) {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let path_var = get_var("PATH")?;
    for dir in split_paths_fn(&path_var) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn run_tool(tool_name: &'static str, tool_path: &Path, args: &[&std::ffi::OsStr]) -> Result<(), ToolchainError> {
    let output = Command::new(tool_path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(ToolchainError::CommandFailed {
        tool: tool_name,
        status: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dx-llvm-tools-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    #[cfg(unix)]
    fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write script");
        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod");
        path
    }

    #[test]
    #[cfg(unix)]
    fn discovers_tools_from_environment_paths() {
        let dir = temp_dir();
        let llvm_as = write_script(&dir, "llvm-as", "exit 0");
        let opt = write_script(&dir, "opt", "exit 0");

        let path = dir.as_os_str().to_os_string();
        let tools = LlvmToolchain::discover_with_env(
            |key| match key {
                "PATH" => Some(path.clone()),
                _ => None,
            },
            env::split_paths,
        )
        .expect("discover");

        assert_eq!(tools.llvm_as, llvm_as);
        assert_eq!(tools.opt.as_deref(), Some(opt.as_path()));

        let _ = fs::remove_file(llvm_as);
        let _ = fs::remove_file(opt);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn explicit_env_overrides_path_lookup() {
        let dir = temp_dir();
        let llvm_as = write_script(&dir, "my-llvm-as", "exit 0");
        let path_tool = write_script(&dir, "llvm-as", "exit 1");

        let path = dir.as_os_str().to_os_string();
        let explicit = llvm_as.as_os_str().to_os_string();
        let tools = LlvmToolchain::discover_with_env(
            |key| match key {
                "DX_LLVM_AS" => Some(explicit.clone()),
                "PATH" => Some(path.clone()),
                _ => None,
            },
            env::split_paths,
        )
        .expect("discover");

        assert_eq!(tools.llvm_as, llvm_as);

        let _ = fs::remove_file(path_tool);
        let _ = fs::remove_file(tools.llvm_as);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn verify_runs_llvm_as_and_optional_opt() {
        let dir = temp_dir();
        let log = dir.join("log.txt");
        let llvm_as = write_script(
            &dir,
            "llvm-as",
            &format!("echo llvm-as >> {}\nexit 0", log.display()),
        );
        let opt = write_script(
            &dir,
            "opt",
            &format!("echo opt >> {}\nexit 0", log.display()),
        );
        let ll = dir.join("test.ll");
        fs::write(&ll, "define i64 @f() { ret i64 0 }\n").expect("write ll");

        let tools = LlvmToolchain {
            llvm_as,
            opt: Some(opt),
            llc: None,
        };
        tools.verify_ll_file(&ll).expect("verify");

        let contents = fs::read_to_string(&log).expect("read log");
        assert!(contents.contains("llvm-as"), "got:\n{contents}");
        assert!(contents.contains("opt"), "got:\n{contents}");

        let _ = fs::remove_file(&ll);
        let _ = fs::remove_file(&log);
        let _ = fs::remove_dir_all(&dir);
    }
}
