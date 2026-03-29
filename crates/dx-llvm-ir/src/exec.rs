use crate::link::{build_link_command_plan, LinkCommandPlan};
use crate::pipeline::{emit_file_to_path, emit_file_to_path_and_verify, PipelineError};
use crate::toolchain::{LlvmToolchain, ToolchainError};
use dx_parser::{Item, Lexer, Parser};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutablePlan {
    pub ll_path: PathBuf,
    pub runtime_archive: PathBuf,
    pub executable_path: PathBuf,
    pub link_plan: LinkCommandPlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceExecutablePlan {
    pub input_dx: PathBuf,
    pub emit_command: Vec<String>,
    pub executable: ExecutablePlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedExecutablePlan {
    pub source: SourceExecutablePlan,
    pub verify_emit_command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableTools {
    pub llvm_as: PathBuf,
    pub llc: PathBuf,
    pub cc: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableRunResult {
    pub executable_path: PathBuf,
    pub exit_code: Option<i32>,
}

#[derive(Debug)]
pub enum ExecutableBuildError {
    Pipeline(PipelineError),
    Toolchain(ToolchainError),
    Io(std::io::Error),
    MissingTool(&'static str),
    StaleRuntimeArchive(PathBuf),
    InvalidEntrypoint(&'static str),
    CommandFailed {
        tool: String,
        status: Option<i32>,
        stderr: String,
    },
}

impl std::fmt::Display for ExecutableBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutableBuildError::Pipeline(err) => write!(f, "{err}"),
            ExecutableBuildError::Toolchain(err) => write!(f, "{err}"),
            ExecutableBuildError::Io(err) => write!(f, "i/o error: {err}"),
            ExecutableBuildError::MissingTool(tool) => write!(f, "missing build tool: {tool}"),
            ExecutableBuildError::StaleRuntimeArchive(path) => write!(
                f,
                "runtime archive is stale: {} (rebuild with `cargo build -p dx-runtime-stub`)",
                path.display()
            ),
            ExecutableBuildError::InvalidEntrypoint(message) => {
                write!(f, "invalid executable entrypoint: {message}")
            }
            ExecutableBuildError::CommandFailed { tool, status, stderr } => {
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

impl std::error::Error for ExecutableBuildError {}

impl From<PipelineError> for ExecutableBuildError {
    fn from(value: PipelineError) -> Self {
        Self::Pipeline(value)
    }
}

impl From<ToolchainError> for ExecutableBuildError {
    fn from(value: ToolchainError) -> Self {
        Self::Toolchain(value)
    }
}

impl From<std::io::Error> for ExecutableBuildError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
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

pub fn build_source_executable_plan(input_dx: &Path, build_dir: &Path) -> SourceExecutablePlan {
    let executable = build_executable_plan_from_source(input_dx, build_dir);
    let emit_command = vec![
        "dx-emit-llvm".to_string(),
        input_dx.display().to_string(),
        executable.ll_path.display().to_string(),
    ];

    SourceExecutablePlan {
        input_dx: input_dx.to_path_buf(),
        emit_command,
        executable,
    }
}

pub fn build_verified_executable_plan(input_dx: &Path, build_dir: &Path) -> VerifiedExecutablePlan {
    let source = build_source_executable_plan(input_dx, build_dir);
    let verify_emit_command = vec![
        "dx-emit-llvm".to_string(),
        "--verify".to_string(),
        input_dx.display().to_string(),
        source.executable.ll_path.display().to_string(),
    ];

    VerifiedExecutablePlan {
        source,
        verify_emit_command,
    }
}

pub fn render_source_executable_plan(plan: &SourceExecutablePlan) -> String {
    let emit = plan.emit_command.join(" ");
    let link = crate::link::render_link_plan(&plan.executable.link_plan);
    [emit, link].join("\n")
}

pub fn render_verified_executable_plan(plan: &VerifiedExecutablePlan) -> String {
    let verify_emit = plan.verify_emit_command.join(" ");
    let link = crate::link::render_link_plan(&plan.source.executable.link_plan);
    [verify_emit, link].join("\n")
}

pub fn discover_executable_tools() -> Result<ExecutableTools, ExecutableBuildError> {
    let llvm = LlvmToolchain::discover().ok_or(ExecutableBuildError::MissingTool("llvm-as"))?;
    let llc = llvm
        .llc
        .clone()
        .ok_or(ExecutableBuildError::MissingTool("llc"))?;
    let cc = find_tool_with_env("DX_CC", "cc", |key| env::var_os(key), env::split_paths)
        .ok_or(ExecutableBuildError::MissingTool("cc"))?;
    Ok(ExecutableTools {
        llvm_as: llvm.llvm_as,
        llc,
        cc,
    })
}

pub fn execute_link_plan(
    plan: &LinkCommandPlan,
    tools: &ExecutableTools,
) -> Result<(), ExecutableBuildError> {
    ensure_parent_dirs(plan)?;

    run_command(
        &tools.llvm_as,
        &[
            plan.assemble[1].as_str(),
            plan.assemble[2].as_str(),
            plan.assemble[3].as_str(),
        ],
    )?;
    run_command(
        &tools.llc,
        &[
            plan.compile[1].as_str(),
            plan.compile[2].as_str(),
            plan.compile[3].as_str(),
            plan.compile[4].as_str(),
        ],
    )?;
    run_command(
        &tools.cc,
        &[
            plan.link[1].as_str(),
            plan.link[2].as_str(),
            plan.link[3].as_str(),
            plan.link[4].as_str(),
        ],
    )?;

    Ok(())
}

pub fn materialize_source_executable_plan(
    plan: &SourceExecutablePlan,
    tools: &ExecutableTools,
) -> Result<(), ExecutableBuildError> {
    if let Some(parent) = plan.executable.ll_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    ensure_source_executable_entrypoint_contract(&plan.input_dx)?;
    emit_file_to_path(&plan.input_dx, &plan.executable.ll_path)?;
    ensure_minimal_executable_entrypoint(&plan.executable.ll_path)?;
    ensure_runtime_archive_is_fresh(&plan.executable.runtime_archive)?;
    execute_link_plan(&plan.executable.link_plan, tools)
}

pub fn materialize_verified_executable_plan(
    plan: &VerifiedExecutablePlan,
    tools: &ExecutableTools,
) -> Result<(), ExecutableBuildError> {
    if let Some(parent) = plan.source.executable.ll_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    ensure_source_executable_entrypoint_contract(&plan.source.input_dx)?;
    emit_file_to_path_and_verify(&plan.source.input_dx, &plan.source.executable.ll_path)?;
    ensure_minimal_executable_entrypoint(&plan.source.executable.ll_path)?;
    ensure_runtime_archive_is_fresh(&plan.source.executable.runtime_archive)?;
    execute_link_plan(&plan.source.executable.link_plan, tools)
}

pub fn run_executable(executable_path: &Path) -> Result<ExecutableRunResult, ExecutableBuildError> {
    let output = Command::new(executable_path).output()?;
    Ok(ExecutableRunResult {
        executable_path: executable_path.to_path_buf(),
        exit_code: output.status.code(),
    })
}

pub fn build_and_run_source_executable_plan(
    plan: &SourceExecutablePlan,
    tools: &ExecutableTools,
) -> Result<ExecutableRunResult, ExecutableBuildError> {
    materialize_source_executable_plan(plan, tools)?;
    run_executable(&plan.executable.executable_path)
}

pub fn build_and_run_verified_executable_plan(
    plan: &VerifiedExecutablePlan,
    tools: &ExecutableTools,
) -> Result<ExecutableRunResult, ExecutableBuildError> {
    materialize_verified_executable_plan(plan, tools)?;
    run_executable(&plan.source.executable.executable_path)
}

pub fn default_runtime_archive_path() -> PathBuf {
    if let Some(path) = configured_runtime_archive_with_env(|key| env::var_os(key)) {
        return path;
    }
    configured_target_dir_with_env(|key| env::var_os(key))
        .unwrap_or_else(|| PathBuf::from("target"))
        .join(
            configured_profile_dir_with_env(|key| env::var(key).ok())
                .unwrap_or_else(|| default_target_profile_dir().to_string()),
        )
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

fn configured_target_dir_with_env<F>(get_var: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    get_var("CARGO_TARGET_DIR").map(PathBuf::from)
}

fn configured_profile_dir_with_env<F>(get_var: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    get_var("DX_RUNTIME_STUB_PROFILE")
}

fn configured_runtime_archive_with_env<F>(get_var: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
{
    get_var("DX_RUNTIME_STUB_ARCHIVE").map(PathBuf::from)
}

fn ensure_parent_dirs(plan: &LinkCommandPlan) -> Result<(), std::io::Error> {
    let executable_path = Path::new(
        plan.link
            .last()
            .map(String::as_str)
            .unwrap_or_default(),
    );
    for path in [&plan.bitcode_path, &plan.object_path, executable_path] {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn run_command(binary: &Path, args: &[&str]) -> Result<(), ExecutableBuildError> {
    let output = Command::new(binary).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(ExecutableBuildError::CommandFailed {
        tool: binary.display().to_string(),
        status: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn ensure_minimal_executable_entrypoint(ll_path: &Path) -> Result<(), ExecutableBuildError> {
    let ir = std::fs::read_to_string(ll_path)?;
    if ir.contains("define i64 @main()") {
        return Ok(());
    }
    Err(ExecutableBuildError::InvalidEntrypoint(
        "expected top-level zero-arg `main` returning `Int`",
    ))
}

fn ensure_source_executable_entrypoint_contract(
    input_dx: &Path,
) -> Result<(), ExecutableBuildError> {
    let src = std::fs::read_to_string(input_dx)?;
    let tokens = Lexer::new(&src).tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse_module()
        .map_err(|err| ExecutableBuildError::Pipeline(PipelineError::Parse(err.message)))?;

    for item in ast.items {
        if let Item::Function(function) = item {
            if function.name == "main" && !function.effects.is_empty() {
                return Err(ExecutableBuildError::InvalidEntrypoint(
                    "effectful `main` is outside the current executable contract",
                ));
            }
        }
    }

    Ok(())
}

fn ensure_runtime_archive_is_fresh(runtime_archive: &Path) -> Result<(), ExecutableBuildError> {
    let crate_root = runtime_stub_crate_root();
    if !crate_root.is_dir() || !looks_like_runtime_stub_archive(runtime_archive) {
        return Ok(());
    }
    ensure_runtime_archive_is_fresh_with_root(runtime_archive, &crate_root)
}

fn ensure_runtime_archive_is_fresh_with_root(
    runtime_archive: &Path,
    crate_root: &Path,
) -> Result<(), ExecutableBuildError> {
    let archive_mtime = std::fs::metadata(runtime_archive)?.modified()?;
    let source_mtime = newest_runtime_stub_source_mtime(crate_root)?;
    if archive_mtime < source_mtime {
        return Err(ExecutableBuildError::StaleRuntimeArchive(
            runtime_archive.to_path_buf(),
        ));
    }
    Ok(())
}

fn newest_runtime_stub_source_mtime(crate_root: &Path) -> Result<SystemTime, ExecutableBuildError> {
    let mut newest = std::fs::metadata(crate_root.join("Cargo.toml"))?.modified()?;
    collect_newest_mtime(&crate_root.join("src"), &mut newest)?;
    Ok(newest)
}

fn collect_newest_mtime(dir: &Path, newest: &mut SystemTime) -> Result<(), ExecutableBuildError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_newest_mtime(&path, newest)?;
            continue;
        }
        let modified = metadata.modified()?;
        if modified > *newest {
            *newest = modified;
        }
    }
    Ok(())
}

fn looks_like_runtime_stub_archive(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == default_runtime_archive_filename())
        .unwrap_or(false)
}

fn runtime_stub_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("crates")
        .join("dx-runtime-stub")
}

fn find_tool_with_env<F, G>(env_key: &str, name: &str, get_var: F, split_paths_fn: G) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
    G: Fn(&std::ffi::OsString) -> env::SplitPaths<'_>,
{
    if let Some(path) = get_var(env_key) {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let path_var = get_var("PATH")?;
    for dir in split_paths_fn(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
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
        let dir = std::env::temp_dir().join(format!("dx-exec-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    fn temp_source(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).expect("write source");
        path
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
        let target_root = configured_target_dir_with_env(|key| env::var_os(key))
            .unwrap_or_else(|| PathBuf::from("target"));
        let profile = configured_profile_dir_with_env(|key| env::var(key).ok())
            .unwrap_or_else(|| default_target_profile_dir().to_string());
        assert!(path.starts_with(target_root));
        assert!(path.to_string_lossy().contains(&profile));
    }

    #[test]
    fn source_executable_plan_includes_emit_step_and_link_plan() {
        let plan = build_source_executable_plan(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );

        assert_eq!(plan.input_dx, PathBuf::from("examples/demo.dx"));
        assert_eq!(
            plan.emit_command,
            vec![
                "dx-emit-llvm".to_string(),
                "examples/demo.dx".to_string(),
                "build/demo.ll".to_string(),
            ]
        );
        assert_eq!(plan.executable.ll_path, PathBuf::from("build/demo.ll"));
        assert_eq!(plan.executable.link_plan.object_path, PathBuf::from("build/demo.o"));
    }

    #[test]
    fn rendered_source_executable_plan_is_deterministic() {
        let a = build_source_executable_plan(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );
        let b = build_source_executable_plan(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );

        assert_eq!(render_source_executable_plan(&a), render_source_executable_plan(&b));
    }

    #[test]
    fn verified_plan_includes_verify_emit_step() {
        let plan = build_verified_executable_plan(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );

        assert_eq!(
            plan.verify_emit_command,
            vec![
                "dx-emit-llvm".to_string(),
                "--verify".to_string(),
                "examples/demo.dx".to_string(),
                "build/demo.ll".to_string(),
            ]
        );
    }

    #[test]
    fn rendered_verified_plan_is_deterministic() {
        let a = build_verified_executable_plan(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );
        let b = build_verified_executable_plan(
            Path::new("examples/demo.dx"),
            Path::new("build"),
        );

        assert_eq!(render_verified_executable_plan(&a), render_verified_executable_plan(&b));
    }

    #[test]
    #[cfg(unix)]
    fn execute_link_plan_runs_all_steps_and_creates_outputs() {
        let base = temp_dir();
        let log = base.join("log.txt");
        let llvm_as = write_script(
            &base,
            "llvm-as",
            &format!(
                "echo llvm-as \"$@\" >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\n done\n: > \"$output\"\n",
                log.display()
            ),
        );
        let llc = write_script(
            &base,
            "llc",
            &format!(
                "echo llc \"$@\" >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\n done\n: > \"$output\"\n",
                log.display()
            ),
        );
        let cc = write_script(
            &base,
            "cc",
            &format!(
                "echo cc \"$@\" >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\n done\n: > \"$output\"\n",
                log.display()
            ),
        );
        let ll_path = base.join("build").join("demo.ll");
        fs::create_dir_all(ll_path.parent().expect("parent")).expect("mkdir");
        fs::write(&ll_path, "define i64 @f() { ret i64 0 }\n").expect("write ll");
        let runtime = base.join("libdx_runtime_stub.a");
        fs::write(&runtime, "").expect("write runtime");
        let plan = build_executable_plan_from_ll(&ll_path, &runtime, &base.join("build").join("demo"));
        let tools = ExecutableTools { llvm_as, llc, cc };

        execute_link_plan(&plan.link_plan, &tools).expect("execute");

        assert!(plan.link_plan.bitcode_path.is_file());
        assert!(plan.link_plan.object_path.is_file());
        assert!(plan.executable_path.is_file());

        let contents = fs::read_to_string(&log).expect("read log");
        assert!(contents.contains("llvm-as"), "got:\n{contents}");
        assert!(contents.contains("llc"), "got:\n{contents}");
        assert!(contents.contains("cc"), "got:\n{contents}");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    #[cfg(unix)]
    fn materialize_source_plan_emits_then_links_with_fake_tools() {
        let base = temp_dir();
        let log = base.join("log.txt");
        let llvm_as = write_script(
            &base,
            "llvm-as",
            &format!(
                "echo llvm-as >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\n done\n: > \"$output\"\n",
                log.display()
            ),
        );
        let llc = write_script(
            &base,
            "llc",
            &format!(
                "echo llc >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\n done\n: > \"$output\"\n",
                log.display()
            ),
        );
        let cc = write_script(
            &base,
            "cc",
            &format!(
                "echo cc >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\n done\n: > \"$output\"\n",
                log.display()
            ),
        );
        let runtime = base.join("target").join(default_target_profile_dir()).join(default_runtime_archive_filename());
        fs::create_dir_all(runtime.parent().expect("parent")).expect("mkdir");
        fs::write(&runtime, "").expect("write runtime");
        let input = temp_source(&base, "main.dx", "fun main() -> Int:\n    0\n.\n");
        let build_dir = base.join("build");
        let mut plan = build_source_executable_plan(&input, &build_dir);
        plan.executable.runtime_archive = runtime.clone();
        plan.executable.link_plan = build_link_command_plan(&plan.executable.ll_path, &runtime, &plan.executable.executable_path);
        let tools = ExecutableTools { llvm_as, llc, cc };

        materialize_source_executable_plan(&plan, &tools).expect("materialize");

        assert!(plan.executable.ll_path.is_file());
        assert!(plan.executable.link_plan.bitcode_path.is_file());
        assert!(plan.executable.link_plan.object_path.is_file());
        assert!(plan.executable.executable_path.is_file());

        let contents = fs::read_to_string(&log).expect("read log");
        assert!(contents.contains("llvm-as"), "got:\n{contents}");
        assert!(contents.contains("llc"), "got:\n{contents}");
        assert!(contents.contains("cc"), "got:\n{contents}");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    #[cfg(unix)]
    fn discovers_cc_from_explicit_env_override() {
        let base = temp_dir();
        let cc = write_script(&base, "my-cc", "exit 0");
        let path = base.as_os_str().to_os_string();
        let explicit = cc.as_os_str().to_os_string();
        let found = find_tool_with_env(
            "DX_CC",
            "cc",
            |key| match key {
                "DX_CC" => Some(explicit.clone()),
                "PATH" => Some(path.clone()),
                _ => None,
            },
            env::split_paths,
        );

        assert_eq!(found.as_deref(), Some(cc.as_path()));

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn runtime_archive_path_honors_env_configuration() {
        let path = configured_target_dir_with_env(|key| match key {
            "CARGO_TARGET_DIR" => Some("/tmp/dx-target".into()),
            _ => None,
        })
        .unwrap()
        .join(
            configured_profile_dir_with_env(|key| match key {
                "DX_RUNTIME_STUB_PROFILE" => Some("dist".to_string()),
                _ => None,
            })
            .unwrap(),
        )
        .join(default_runtime_archive_filename());

        assert_eq!(
            path,
            PathBuf::from("/tmp/dx-target")
                .join("dist")
                .join(default_runtime_archive_filename())
        );
    }

    #[test]
    fn runtime_archive_path_honors_explicit_archive_override() {
        let path = configured_runtime_archive_with_env(|key| match key {
            "DX_RUNTIME_STUB_ARCHIVE" => Some("/tmp/custom/libdx_runtime_stub.a".into()),
            _ => None,
        })
        .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/custom/libdx_runtime_stub.a"));
    }

    #[test]
    fn accepts_minimal_int_main_entrypoint() {
        let base = temp_dir();
        let ll = base.join("main.ll");
        fs::write(&ll, "define i64 @main() {\n  ret i64 0\n}\n").expect("write ll");

        ensure_minimal_executable_entrypoint(&ll).expect("valid entrypoint");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rejects_void_main_entrypoint() {
        let base = temp_dir();
        let ll = base.join("main.ll");
        fs::write(&ll, "define void @main() {\n  ret void\n}\n").expect("write ll");

        let err = ensure_minimal_executable_entrypoint(&ll).expect_err("should reject");

        assert_eq!(
            err.to_string(),
            "invalid executable entrypoint: expected top-level zero-arg `main` returning `Int`"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn accepts_source_main_without_effects() {
        let base = temp_dir();
        let source = temp_source(&base, "main.dx", "fun main() -> Int:\n    0\n.\n");

        ensure_source_executable_entrypoint_contract(&source).expect("source contract");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rejects_effectful_source_main_entrypoint() {
        let base = temp_dir();
        let source = temp_source(&base, "main.dx", "fun main() -> Int !io:\n    0\n.\n");

        let err =
            ensure_source_executable_entrypoint_contract(&source).expect_err("should reject");

        assert_eq!(
            err.to_string(),
            "invalid executable entrypoint: effectful `main` is outside the current executable contract"
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn accepts_fresh_runtime_archive() {
        let base = temp_dir();
        let crate_root = base.join("dx-runtime-stub");
        fs::create_dir_all(crate_root.join("src")).expect("mkdir");
        fs::write(crate_root.join("Cargo.toml"), "[package]\nname = \"dx-runtime-stub\"\n")
            .expect("write cargo");
        fs::write(crate_root.join("src").join("lib.rs"), "pub fn stub() {}\n")
            .expect("write lib");
        let archive = base.join(default_runtime_archive_filename());
        fs::write(&archive, "").expect("write archive");

        ensure_runtime_archive_is_fresh_with_root(&archive, &crate_root).expect("fresh archive");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn rejects_stale_runtime_archive() {
        let base = temp_dir();
        let crate_root = base.join("dx-runtime-stub");
        fs::create_dir_all(crate_root.join("src")).expect("mkdir");
        fs::write(crate_root.join("Cargo.toml"), "[package]\nname = \"dx-runtime-stub\"\n")
            .expect("write cargo");
        let archive = base.join(default_runtime_archive_filename());
        fs::write(&archive, "").expect("write archive");
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(crate_root.join("src").join("lib.rs"), "pub fn changed() {}\n")
            .expect("write lib");

        let err =
            ensure_runtime_archive_is_fresh_with_root(&archive, &crate_root).expect_err("stale");

        assert_eq!(
            err.to_string(),
            format!(
                "runtime archive is stale: {} (rebuild with `cargo build -p dx-runtime-stub`)",
                archive.display()
            )
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    #[cfg(unix)]
    fn run_executable_reports_exit_code() {
        let base = temp_dir();
        let executable = write_script(&base, "demo-exec", "exit 7");

        let result = run_executable(&executable).expect("run executable");

        assert_eq!(
            result,
            ExecutableRunResult {
                executable_path: executable,
                exit_code: Some(7),
            }
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    #[cfg(unix)]
    fn build_and_run_source_plan_runs_materialized_program() {
        let base = temp_dir();
        let log = base.join("log.txt");
        let llvm_as = write_script(
            &base,
            "llvm-as",
            "output=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\ndone\n: > \"$output\"\n",
        );
        let llc = write_script(
            &base,
            "llc",
            "output=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\ndone\n: > \"$output\"\n",
        );
        let cc = write_script(
            &base,
            "cc",
            &format!(
                "echo cc >> {}\noutput=\"\"\nwhile [ \"$#\" -gt 0 ]; do\n  if [ \"$1\" = \"-o\" ]; then\n    shift\n    output=\"$1\"\n    break\n  fi\n  shift\ndone\ncat <<'EOF' > \"$output\"\n#!/bin/sh\nexit 5\nEOF\nchmod +x \"$output\"\n",
                log.display()
            ),
        );
        let source = temp_source(&base, "main.dx", "fun main() -> Int:\n    0\n.\n");
        let runtime = base.join("libdx_runtime_stub.a");
        fs::write(&runtime, "").expect("write runtime");
        let build_dir = base.join("build");
        let mut plan = build_source_executable_plan(&source, &build_dir);
        plan.executable.runtime_archive = runtime.clone();
        plan.executable.link_plan = build_link_command_plan(
            &plan.executable.ll_path,
            &runtime,
            &plan.executable.executable_path,
        );
        let tools = ExecutableTools { llvm_as, llc, cc };

        let result = build_and_run_source_executable_plan(&plan, &tools).expect("build and run");

        assert_eq!(result.executable_path, plan.executable.executable_path);
        assert_eq!(result.exit_code, Some(5));
        let contents = fs::read_to_string(&log).expect("read log");
        assert!(contents.contains("cc"), "got:\n{contents}");

        let _ = fs::remove_dir_all(&base);
    }
}
