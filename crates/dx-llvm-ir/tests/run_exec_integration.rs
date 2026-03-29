//! Real CLI-level execution tests for `dx-run-exec`.
//!
//! These tests invoke the actual binary and verify exit codes and JSON output.
//! They require LLVM tools (llvm-as, llc, cc) to be installed.
//! Tests are gated with `#[cfg(unix)]` since they depend on process execution.

use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("examples/backend");
    path.push(name);
    path
}

fn build_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("dx-run-exec-test-{nonce}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn dx_run_exec_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/dx-run-exec");
    path
}

fn runtime_archive() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/libdx_runtime_stub.a");
    path
}

fn has_llvm_tools() -> bool {
    Command::new("llvm-as").arg("--version").output().is_ok()
        && Command::new("llc").arg("--version").output().is_ok()
        && Command::new("cc").arg("--version").output().is_ok()
}

fn can_run() -> bool {
    has_llvm_tools() && runtime_archive().exists()
}

// ── real execution: exit codes ───────────────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_main_returns_zero_exits_with_zero() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":0"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_arithmetic_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_arithmetic.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_closure_call_int_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_closure_call_int.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_closure_call_two_args_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_closure_call_two_args.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_closure_call_subtract_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_closure_call_subtract.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_thunk_arithmetic_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_thunk_arithmetic.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_thunk_capture_builds_and_runs() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_thunk_capture.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_thunk_three_capture_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_thunk_three_capture.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_closure_call_bool_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_closure_call_bool.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_thunk_bool_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_thunk_bool.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_closure_call_multi_capture_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_closure_call_multi_capture.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_main_closure_call_nested_exits_with_42() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_closure_call_nested.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":42"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── structured surface: --json ───────────────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_json_output_contains_executable_and_exit_code() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"executable\""), "json executable field: {stdout}");
    assert!(stdout.contains("\"exit_code\""), "json exit_code field: {stdout}");
    assert!(stdout.contains("main_returns_zero"), "executable name: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── structured surface: --verify --json ──────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_verify_json_succeeds() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--verify", "--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "verify should succeed.\nstderr: {stderr}\nstdout: {stdout}");
    assert!(stdout.contains("\"exit_code\":0"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── structured surface: --runtime-archive ────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_with_explicit_runtime_archive() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"exit_code\":0"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── negative: invalid entrypoint ─────────────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_rejects_non_int_main() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let bad_source = dir.join("bad_main.dx");
    std::fs::write(&bad_source, "fun main() -> Str:\n    \"nope\"\n.\n").unwrap();

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(&bad_source)
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // CLI must exit non-zero with a precise entrypoint-contract error
    assert!(!output.status.success(), "void main should fail.\nstderr: {stderr}");
    assert!(
        stderr.contains("invalid executable entrypoint"),
        "expected entrypoint contract error in stderr.\nstderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_rejects_main_with_args() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let bad_source = dir.join("main_with_args.dx");
    std::fs::write(&bad_source, "fun main(x: Int) -> Int:\n    x\n.\n").unwrap();

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(&bad_source)
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "main with args should fail.\nstderr: {stderr}");
    assert!(
        stderr.contains("invalid executable entrypoint"),
        "expected entrypoint contract error.\nstderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_rejects_missing_main() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let bad_source = dir.join("no_main.dx");
    std::fs::write(&bad_source, "fun compute() -> Int:\n    42\n.\n").unwrap();

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(&bad_source)
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "missing main should fail.\nstderr: {stderr}");
    assert!(
        stderr.contains("invalid executable entrypoint"),
        "expected entrypoint contract error.\nstderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_rejects_empty_source() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let bad_source = dir.join("empty.dx");
    std::fs::write(&bad_source, "").unwrap();

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(&bad_source)
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "empty source should fail.\nstderr: {stderr}");
    assert!(
        stderr.contains("invalid executable entrypoint"),
        "expected entrypoint contract error.\nstderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── runtime archive failure ─────────────────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_fails_with_invalid_runtime_archive() {
    if !has_llvm_tools() {
        eprintln!("skipping: LLVM tools not available");
        return;
    }
    let dir = build_dir();
    let fake_archive = dir.join("fake.a");
    std::fs::write(&fake_archive, "not a real archive").unwrap();

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(&fake_archive)
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    assert!(
        !output.status.success(),
        "invalid archive should fail.\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn run_exec_fails_with_missing_runtime_archive() {
    if !has_llvm_tools() {
        eprintln!("skipping: LLVM tools not available");
        return;
    }
    let dir = build_dir();
    let missing = dir.join("nonexistent.a");

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(&missing)
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    assert!(
        !output.status.success(),
        "missing archive should fail.\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── observational: effectful main ───────────────────────────────
// Effectful main is outside the executable contract but currently succeeds
// because effects are annotations that don't change codegen. This test
// documents the current behavior, not a guarantee.

#[test]
#[cfg(unix)]
fn run_exec_rejects_effectful_main() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let source = dir.join("effectful_main.dx");
    std::fs::write(&source, "fun main() -> Int !io:\n    42\n.\n").unwrap();

    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(&source)
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "effectful main should be rejected.\nstderr: {stderr}");
    assert!(
        stderr.contains("effectful") || stderr.contains("invalid executable entrypoint"),
        "expected effectful-main rejection.\nstderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── stderr cleanliness ──────────────────────────────────────────

#[test]
#[cfg(unix)]
fn run_exec_success_has_clean_stderr() {
    if !can_run() {
        eprintln!("skipping");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_run_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-run-exec");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "stderr should be clean on success, got: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── manifest-driven execution proof ─────────────────────────────
// Reads scripts/runnable_entry_expected_exit_codes.txt and verifies
// that every listed demo actually executes to the declared exit code.

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

#[test]
#[cfg(unix)]
fn manifest_exit_codes_match_actual_execution() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let root = repo_root();
    let manifest = root.join("scripts/runnable_entry_expected_exit_codes.txt");
    let content = std::fs::read_to_string(&manifest)
        .expect("read runnable_entry_expected_exit_codes.txt");

    let entries: Vec<(&str, i32)> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(parts.len(), 2, "bad manifest line: {line}");
            let code: i32 = parts[1].parse().expect("bad exit code");
            (parts[0], code)
        })
        .collect();

    assert!(!entries.is_empty(), "manifest is empty");

    for (demo_name, expected_code) in &entries {
        let demo_file = root.join(format!("examples/backend/{demo_name}.dx"));
        assert!(demo_file.exists(), "manifest demo missing on disk: {demo_name}");

        let dir = build_dir();
        let output = Command::new(dx_run_exec_bin())
            .args(["--json", "--runtime-archive"])
            .arg(runtime_archive())
            .arg(&demo_file)
            .arg(&dir)
            .output()
            .unwrap_or_else(|e| panic!("run dx-run-exec for {demo_name}: {e}"));

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{demo_name}: dx-run-exec failed.\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let expected_fragment = format!("\"exit_code\":{expected_code}");
        assert!(
            stdout.contains(&expected_fragment),
            "{demo_name}: expected exit_code {expected_code}, got stdout: {stdout}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
