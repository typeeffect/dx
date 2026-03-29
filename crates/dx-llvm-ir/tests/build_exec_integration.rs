//! Real CLI-level build tests for `dx-build-exec`.
//!
//! These tests invoke the actual binary and verify build success and JSON output.
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
    let dir = std::env::temp_dir().join(format!("dx-build-exec-test-{nonce}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn dx_build_exec_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("target/debug/dx-build-exec");
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

// ── build success ───────────────────────────────────────────────

#[test]
#[cfg(unix)]
fn build_exec_succeeds_on_main_returns_zero() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_build_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"executable\""), "json executable field: {stdout}");
    assert!(stdout.contains("main_returns_zero"), "executable name: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[cfg(unix)]
fn build_exec_succeeds_on_main_arithmetic() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_build_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_arithmetic.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let _ = std::fs::remove_dir_all(&dir);
}

// ── produced artifact ───────────────────────────────────────────

#[test]
#[cfg(unix)]
fn build_exec_produces_executable_file() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_build_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let executable = dir.join("main_returns_zero");
    assert!(executable.exists(), "executable should exist at {}", executable.display());

    let _ = std::fs::remove_dir_all(&dir);
}

// ── structured surface: --verify --json ─────────────────────────

#[test]
#[cfg(unix)]
fn build_exec_verify_json_succeeds() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_build_exec_bin())
        .args(["--verify", "--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "verify should succeed.\nstderr: {stderr}\nstdout: {stdout}");
    assert!(stdout.contains("\"mode\":\"verified-build\""), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&dir);
}

// ── structured surface: --runtime-archive ───────────────────────

#[test]
#[cfg(unix)]
fn build_exec_with_explicit_runtime_archive() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_build_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let _ = std::fs::remove_dir_all(&dir);
}

// ── stderr cleanliness ──────────────────────────────────────────

#[test]
#[cfg(unix)]
fn build_exec_success_has_clean_stderr() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let output = Command::new(dx_build_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(fixture_path("main_returns_zero.dx"))
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "stderr should be clean on success, got: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ── negative: invalid entrypoint ────────────────────────────────

#[test]
#[cfg(unix)]
fn build_exec_rejects_non_int_main() {
    if !can_run() {
        eprintln!("skipping: LLVM tools or runtime archive not available");
        return;
    }
    let dir = build_dir();
    let bad_source = dir.join("bad_main.dx");
    std::fs::write(&bad_source, "fun main() -> Unit:\n    42\n.\n").unwrap();

    let output = Command::new(dx_build_exec_bin())
        .args(["--json", "--runtime-archive"])
        .arg(runtime_archive())
        .arg(&bad_source)
        .arg(&dir)
        .output()
        .expect("run dx-build-exec");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // CLI must exit non-zero with a precise entrypoint-contract error
    assert!(!output.status.success(), "void main should fail.\nstderr: {stderr}");
    assert!(
        stderr.contains("invalid executable entrypoint"),
        "expected entrypoint contract error in stderr.\nstderr: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
