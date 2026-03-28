use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("dx-llvm-ir-harness-{nonce}"));
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
fn cli_verify_uses_fake_llvm_tools_end_to_end() {
    let base = temp_dir();
    let input = base.join("input.dx");
    let output = base.join("output.ll");
    let log = base.join("tools.log");
    fs::write(&input, "fun f() -> Int:\n    1\n.\n").expect("write input");

    let llvm_as = write_script(
        &base,
        "llvm-as",
        &format!("echo llvm-as >> {}\nexit 0", log.display()),
    );
    let opt = write_script(
        &base,
        "opt",
        &format!("echo opt >> {}\nexit 0", log.display()),
    );

    let status = Command::new(env!("CARGO_BIN_EXE_dx-emit-llvm"))
        .arg("--verify")
        .arg(&input)
        .arg(&output)
        .env("DX_LLVM_AS", &llvm_as)
        .env("DX_LLVM_OPT", &opt)
        .status()
        .expect("run cli");

    assert!(status.success(), "cli failed with status {status}");
    let ir = fs::read_to_string(&output).expect("read output");
    assert!(ir.contains("define i64 @f()"), "got:\n{ir}");

    let tool_log = fs::read_to_string(&log).expect("read log");
    assert!(tool_log.contains("llvm-as"), "got:\n{tool_log}");
    assert!(tool_log.contains("opt"), "got:\n{tool_log}");

    let _ = fs::remove_dir_all(&base);
}

#[test]
#[cfg(unix)]
fn cli_verify_reports_missing_llvm_as() {
    let base = temp_dir();
    let input = base.join("input.dx");
    let output = base.join("output.ll");
    fs::write(&input, "fun f() -> Int:\n    1\n.\n").expect("write input");

    let out = Command::new(env!("CARGO_BIN_EXE_dx-emit-llvm"))
        .arg("--verify")
        .arg(&input)
        .arg(&output)
        .env_clear()
        .output()
        .expect("run cli");

    assert!(!out.status.success(), "cli unexpectedly succeeded");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("missing LLVM tool: llvm-as"), "got:\n{stderr}");

    let _ = fs::remove_dir_all(&base);
}
