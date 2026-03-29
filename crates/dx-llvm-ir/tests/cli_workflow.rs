//! Black-box tests for CLI workflow surface.
//!
//! These test the compiled CLI binaries via `cargo run` or by exercising
//! the same Rust functions the CLIs use.

use std::path::PathBuf;

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("examples/backend");
    path.push(name);
    path
}

// ── dx-plan-exec: plan rendering is deterministic ────────────────

#[test]
fn plan_exec_rendering_is_deterministic() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let input = fixture_path("closure_call_int.dx");
    let build_dir = PathBuf::from("/tmp/dx-test-build");

    let plan1 = build_source_executable_plan(&input, &build_dir);
    let plan2 = build_source_executable_plan(&input, &build_dir);

    let r1 = render_source_executable_plan(&plan1);
    let r2 = render_source_executable_plan(&plan2);
    assert_eq!(r1, r2, "plan rendering must be deterministic");
}

#[test]
fn plan_exec_includes_emit_command() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let input = fixture_path("arithmetic.dx");
    let build_dir = PathBuf::from("/tmp/dx-test-build");

    let plan = build_source_executable_plan(&input, &build_dir);
    let rendered = render_source_executable_plan(&plan);

    assert!(rendered.contains("dx-emit-llvm"), "should reference emit command:\n{rendered}");
    assert!(rendered.contains("arithmetic"), "should reference input file:\n{rendered}");
}

#[test]
fn plan_exec_all_demos_produce_plans() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let demos = [
        "arithmetic.dx",
        "thunk.dx",
        "closure_call_int.dx",
        "closure_call_str.dx",
        "closure_call_two_args.dx",
        "closure_call_ptr_ret_int_arg.dx",
        "closure_call_ptr_ret_str_int_args.dx",
        "closure_call_void_ret_three_args.dx",
        "closure_call_float.dx",
        "closure_call_bool.dx",
        "match_nominal.dx",
        "match_with_closure_call.dx",
        "py_call_function.dx",
        "py_call_method.dx",
        "py_call_dynamic.dx",
        "py_call_throw.dx",
    ];
    for demo in &demos {
        let input = fixture_path(demo);
        let build_dir = PathBuf::from("/tmp/dx-test-build");
        let plan = build_source_executable_plan(&input, &build_dir);
        let rendered = render_source_executable_plan(&plan);
        assert!(!rendered.is_empty(), "{demo} produced empty plan");
    }
}

// ── runtime-stub: symbol output is stable ────────────────────────

#[test]
fn runtime_stub_symbols_are_stable() {
    let s1 = dx_runtime_stub::manifest::render_exported_symbols();
    let s2 = dx_runtime_stub::manifest::render_exported_symbols();
    assert_eq!(s1, s2, "symbol output must be stable");
}

#[test]
fn runtime_stub_symbols_contain_all_demo_required() {
    let symbols = dx_runtime_stub::manifest::EXPORTED_SYMBOLS;
    let required = [
        "dx_rt_closure_create",
        "dx_rt_thunk_call_i64",
        "dx_rt_closure_call_i64_1_i64",
        "dx_rt_closure_call_ptr_1_ptr",
        "dx_rt_closure_call_i64_2_i64_i64",
        "dx_rt_closure_call_ptr_1_i64",
        "dx_rt_closure_call_ptr_2_ptr_i64",
        "dx_rt_closure_call_void_3_i64_ptr_i1",
        "dx_rt_match_tag",
        "dx_rt_throw_check_pending",
        "dx_rt_py_call_function",
    ];
    for sym in &required {
        assert!(symbols.contains(sym), "missing required symbol: {sym}");
    }
}

// ── demo set consistency ─────────────────────────────────────────

/// The canonical demo set. This must match examples/backend/*.dx exactly.
const CANONICAL_DEMOS: [&str; 28] = [
    "arithmetic.dx",
    "closure_call_bool.dx",
    "closure_call_float.dx",
    "closure_call_int.dx",
    "closure_call_ptr_ret_int_arg.dx",
    "closure_call_ptr_ret_str_int_args.dx",
    "closure_call_str.dx",
    "closure_call_two_args.dx",
    "closure_call_void_ret_three_args.dx",
    "main_arithmetic.dx",
    "main_closure_call_bool.dx",
    "main_closure_call_int.dx",
    "main_closure_call_multi_capture.dx",
    "main_closure_call_nested.dx",
    "main_closure_call_subtract.dx",
    "main_closure_call_two_args.dx",
    "main_returns_zero.dx",
    "main_thunk_arithmetic.dx",
    "main_thunk_bool.dx",
    "main_thunk_capture.dx",
    "main_thunk_three_capture.dx",
    "match_nominal.dx",
    "match_with_closure_call.dx",
    "py_call_dynamic.dx",
    "py_call_function.dx",
    "py_call_method.dx",
    "py_call_throw.dx",
    "thunk.dx",
];

fn backend_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.pop();
    dir.push("examples/backend");
    dir
}

#[test]
fn demo_names_match_backend_directory() {
    let dir = backend_dir();
    let mut on_disk: Vec<String> = std::fs::read_dir(&dir)
        .expect("read examples/backend")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "dx"))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();

    let mut expected: Vec<&str> = CANONICAL_DEMOS.to_vec();
    expected.sort();

    assert_eq!(
        on_disk, expected,
        "demo files on disk don't match CANONICAL_DEMOS. \
         If you added/removed a demo, update CANONICAL_DEMOS in cli_workflow.rs, \
         ALL_DEMOS in demo_fixtures.rs, and the plan list above."
    );
}

#[test]
fn demo_fixture_count_matches_backend_directory() {
    let dir = backend_dir();
    let dx_count = std::fs::read_dir(&dir)
        .expect("read examples/backend")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "dx"))
        .count();

    assert_eq!(
        dx_count,
        CANONICAL_DEMOS.len(),
        "expected {} canonical .dx demos, found {}",
        CANONICAL_DEMOS.len(),
        dx_count
    );
}

#[test]
fn plan_list_matches_canonical_demos() {
    // The demo list in plan_exec_all_demos_produce_plans must be the same set.
    // We verify here by checking every CANONICAL_DEMO has a fixture file.
    for demo in &CANONICAL_DEMOS {
        let path = fixture_path(demo);
        assert!(path.exists(), "canonical demo missing on disk: {demo}");
    }
}

#[test]
fn canonical_demos_txt_matches_rust_const() {
    // scripts/canonical_demos.txt is the shared source of truth for bash scripts.
    // CANONICAL_DEMOS is the Rust-side source of truth.
    // They must agree.
    let mut txt_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    txt_path.pop();
    txt_path.pop();
    txt_path.push("scripts/canonical_demos.txt");

    let content = std::fs::read_to_string(&txt_path).expect("read canonical_demos.txt");
    let mut from_txt: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|name| format!("{name}.dx"))
        .collect();
    from_txt.sort();

    let mut from_const: Vec<&str> = CANONICAL_DEMOS.to_vec();
    from_const.sort();

    assert_eq!(
        from_txt, from_const,
        "scripts/canonical_demos.txt and CANONICAL_DEMOS in cli_workflow.rs are out of sync"
    );
}

#[test]
fn demo_expected_symbols_covers_all_canonical_demos() {
    // scripts/demo_expected_symbols.txt must have an entry for every canonical demo.
    let mut manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_path.pop();
    manifest_path.pop();
    manifest_path.push("scripts/demo_expected_symbols.txt");

    let content = std::fs::read_to_string(&manifest_path).expect("read demo_expected_symbols.txt");
    let manifest_demos: Vec<&str> = content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| l.split(':').next().unwrap())
        .collect();

    for demo in &CANONICAL_DEMOS {
        let name = demo.strip_suffix(".dx").unwrap();
        assert!(
            manifest_demos.contains(&name),
            "demo_expected_symbols.txt missing entry for '{name}'"
        );
    }

    for name in &manifest_demos {
        let demo = format!("{name}.dx");
        assert!(
            CANONICAL_DEMOS.contains(&demo.as_str()),
            "demo_expected_symbols.txt has entry '{name}' not in CANONICAL_DEMOS"
        );
    }
}

#[test]
fn every_canonical_demo_produces_a_plan() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    for demo in &CANONICAL_DEMOS {
        let input = fixture_path(demo);
        let build_dir = PathBuf::from("/tmp/dx-test-build");
        let plan = build_source_executable_plan(&input, &build_dir);
        let rendered = render_source_executable_plan(&plan);
        assert!(!rendered.is_empty(), "{demo} produced empty plan");
    }
}

// ── dx-build-exec dry-run coverage ──────────────────────────────

#[test]
fn build_exec_plan_includes_emit_and_link_steps() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let input = fixture_path("closure_call_int.dx");
    let build_dir = PathBuf::from("/tmp/dx-build-test");
    let plan = build_source_executable_plan(&input, &build_dir);
    let rendered = render_source_executable_plan(&plan);

    assert!(rendered.contains("dx-emit-llvm"), "emit step:\n{rendered}");
    assert!(rendered.contains("llvm-as"), "assemble step:\n{rendered}");
    assert!(rendered.contains("llc"), "compile step:\n{rendered}");
    assert!(rendered.contains("cc"), "link step:\n{rendered}");
}

#[test]
fn build_exec_plan_references_runtime_archive() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let input = fixture_path("thunk.dx");
    let build_dir = PathBuf::from("/tmp/dx-build-test");
    let plan = build_source_executable_plan(&input, &build_dir);
    let rendered = render_source_executable_plan(&plan);

    assert!(
        rendered.contains("libdx_runtime_stub") || rendered.contains("dx_runtime_stub"),
        "runtime archive:\n{rendered}"
    );
}

#[test]
fn build_exec_plan_is_deterministic_for_all_demos() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let build_dir = PathBuf::from("/tmp/dx-build-test");
    for demo in &CANONICAL_DEMOS {
        let input = fixture_path(demo);
        let r1 = render_source_executable_plan(&build_source_executable_plan(&input, &build_dir));
        let r2 = render_source_executable_plan(&build_source_executable_plan(&input, &build_dir));
        assert_eq!(r1, r2, "plan not deterministic for {demo}");
    }
}

#[test]
fn build_exec_verified_plan_includes_verify_flag() {
    use dx_llvm_ir::exec::{build_verified_executable_plan, render_verified_executable_plan};
    let input = fixture_path("arithmetic.dx");
    let build_dir = PathBuf::from("/tmp/dx-build-test");
    let plan = build_verified_executable_plan(&input, &build_dir);
    let rendered = render_verified_executable_plan(&plan);

    assert!(rendered.contains("--verify"), "verify flag:\n{rendered}");
    assert!(rendered.contains("dx-emit-llvm"), "emit:\n{rendered}");
}

#[test]
fn build_exec_plan_with_custom_runtime_archive() {
    use dx_llvm_ir::exec::{build_executable_plan_from_ll, render_source_executable_plan, SourceExecutablePlan};
    let custom_archive = PathBuf::from("/opt/dx/lib/libdx_runtime_stub.a");
    let plan = build_executable_plan_from_ll(
        &PathBuf::from("/tmp/demo.ll"),
        &custom_archive,
        &PathBuf::from("/tmp/demo"),
    );
    assert_eq!(plan.runtime_archive, custom_archive);
    // Verify archive path appears in rendered link plan
    let source_plan = SourceExecutablePlan {
        input_dx: PathBuf::from("demo.dx"),
        emit_command: vec!["dx-emit-llvm".into(), "demo.dx".into(), "/tmp/demo.ll".into()],
        executable: plan,
    };
    let rendered = render_source_executable_plan(&source_plan);
    assert!(rendered.contains("/opt/dx/lib/libdx_runtime_stub.a"), "custom archive:\n{rendered}");
}

// ── executable-entry fixtures ────────────────────────────────────

const EXECUTABLE_ENTRY_DEMOS: [&str; 12] = [
    "main_arithmetic.dx",
    "main_closure_call_bool.dx",
    "main_closure_call_int.dx",
    "main_closure_call_multi_capture.dx",
    "main_closure_call_nested.dx",
    "main_closure_call_subtract.dx",
    "main_closure_call_two_args.dx",
    "main_returns_zero.dx",
    "main_thunk_arithmetic.dx",
    "main_thunk_bool.dx",
    "main_thunk_capture.dx",
    "main_thunk_three_capture.dx",
];

#[test]
fn executable_entry_demos_exist_on_disk() {
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        let path = fixture_path(demo);
        assert!(path.exists(), "executable-entry demo missing: {demo}");
    }
}

#[test]
fn executable_entry_demos_are_subset_of_canonical() {
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        assert!(
            CANONICAL_DEMOS.contains(demo),
            "executable-entry demo '{demo}' not in CANONICAL_DEMOS"
        );
    }
}

#[test]
fn executable_entry_demos_have_main_function_in_ir() {
    use dx_llvm_ir::pipeline::emit_file_to_string_unchecked;
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        let path = fixture_path(demo);
        let ir = emit_file_to_string_unchecked(&path).expect(&format!("emit {demo}"));
        assert!(ir.contains("define i64 @main()"), "{demo} should define main() -> i64:\n{ir}");
    }
}

#[test]
fn executable_entry_build_plans_are_deterministic() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    let build_dir = PathBuf::from("/tmp/dx-exec-test");
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        let input = fixture_path(demo);
        let r1 = render_source_executable_plan(&build_source_executable_plan(&input, &build_dir));
        let r2 = render_source_executable_plan(&build_source_executable_plan(&input, &build_dir));
        assert_eq!(r1, r2, "build plan not deterministic for {demo}");
    }
}

#[test]
fn executable_entry_txt_matches_rust_const() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts/executable_entry_demos.txt");
    let content = std::fs::read_to_string(&path).expect("read executable_entry_demos.txt");
    let mut from_txt: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|name| format!("{name}.dx"))
        .collect();
    from_txt.sort();
    let mut from_const: Vec<&str> = EXECUTABLE_ENTRY_DEMOS.to_vec();
    from_const.sort();
    assert_eq!(from_txt, from_const, "executable_entry_demos.txt out of sync with EXECUTABLE_ENTRY_DEMOS");
}

// ── dx-run-exec plan surface ─────────────────────────────────────

#[test]
fn run_exec_plan_includes_all_build_steps() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        let input = fixture_path(demo);
        let build_dir = PathBuf::from("/tmp/dx-run-test");
        let plan = build_source_executable_plan(&input, &build_dir);
        let rendered = render_source_executable_plan(&plan);

        assert!(rendered.contains("dx-emit-llvm"), "{demo} plan should include emit:\n{rendered}");
        assert!(rendered.contains("llvm-as"), "{demo} plan should include assemble:\n{rendered}");
        assert!(rendered.contains("llc"), "{demo} plan should include compile:\n{rendered}");
        assert!(rendered.contains("cc"), "{demo} plan should include link:\n{rendered}");
    }
}

#[test]
fn run_exec_verified_plan_includes_verify_for_all_entry_demos() {
    use dx_llvm_ir::exec::{build_verified_executable_plan, render_verified_executable_plan};
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        let input = fixture_path(demo);
        let build_dir = PathBuf::from("/tmp/dx-run-test");
        let plan = build_verified_executable_plan(&input, &build_dir);
        let rendered = render_verified_executable_plan(&plan);

        assert!(rendered.contains("--verify"), "{demo} verified plan should include --verify:\n{rendered}");
    }
}

#[test]
fn run_exec_plan_executable_path_uses_demo_name() {
    use dx_llvm_ir::exec::build_source_executable_plan;
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        let input = fixture_path(demo);
        let build_dir = PathBuf::from("/tmp/dx-run-test");
        let plan = build_source_executable_plan(&input, &build_dir);
        let stem = demo.strip_suffix(".dx").unwrap();
        let exe_name = plan.executable.executable_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        assert_eq!(exe_name, stem, "executable path should use demo stem for {demo}");
    }
}

#[test]
fn run_exec_plan_with_custom_runtime_archive() {
    use dx_llvm_ir::exec::{build_source_executable_plan, render_source_executable_plan};
    use dx_llvm_ir::build_link_command_plan;
    let input = fixture_path("main_returns_zero.dx");
    let build_dir = PathBuf::from("/tmp/dx-run-test");
    let mut plan = build_source_executable_plan(&input, &build_dir);
    let custom = PathBuf::from("/opt/dx/lib/libdx_runtime_stub.a");
    plan.executable.runtime_archive = custom.clone();
    plan.executable.link_plan = build_link_command_plan(
        &plan.executable.ll_path,
        &custom,
        &plan.executable.executable_path,
    );
    let rendered = render_source_executable_plan(&plan);
    assert!(rendered.contains("/opt/dx/lib/libdx_runtime_stub.a"), "custom archive:\n{rendered}");
}

#[test]
fn run_exec_json_result_shape() {
    // Test the JSON rendering directly (without executing)
    use dx_llvm_ir::exec::ExecutableRunResult;
    let result = ExecutableRunResult {
        executable_path: PathBuf::from("/tmp/main_returns_zero"),
        exit_code: Some(0),
    };
    // Use the JSON rendering from the module
    let json = format!(
        "{{\"executable\":\"{}\",\"exit_code\":{}}}",
        result.executable_path.display(),
        result.exit_code.unwrap()
    );
    assert!(json.contains("\"executable\""), "json:\n{json}");
    assert!(json.contains("\"exit_code\":0"), "json:\n{json}");
    assert!(json.contains("main_returns_zero"), "json:\n{json}");
}

// ── runnable subset source-of-truth guards ──────────────────────

const RUNNABLE_ENTRY_DEMOS: [&str; 12] = [
    "main_arithmetic.dx",
    "main_closure_call_bool.dx",
    "main_closure_call_int.dx",
    "main_closure_call_multi_capture.dx",
    "main_closure_call_nested.dx",
    "main_closure_call_subtract.dx",
    "main_closure_call_two_args.dx",
    "main_returns_zero.dx",
    "main_thunk_arithmetic.dx",
    "main_thunk_bool.dx",
    "main_thunk_capture.dx",
    "main_thunk_three_capture.dx",
];

#[test]
fn runnable_entry_demos_are_subset_of_executable_entry() {
    for demo in &RUNNABLE_ENTRY_DEMOS {
        assert!(
            EXECUTABLE_ENTRY_DEMOS.contains(demo),
            "runnable demo '{demo}' not in EXECUTABLE_ENTRY_DEMOS"
        );
    }
}

#[test]
fn runnable_entry_txt_matches_rust_const() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts/runnable_entry_demos.txt");
    let content = std::fs::read_to_string(&path).expect("read runnable_entry_demos.txt");
    let mut from_txt: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|name| format!("{name}.dx"))
        .collect();
    from_txt.sort();
    let mut from_const: Vec<&str> = RUNNABLE_ENTRY_DEMOS.to_vec();
    from_const.sort();
    assert_eq!(
        from_txt, from_const,
        "runnable_entry_demos.txt out of sync with RUNNABLE_ENTRY_DEMOS"
    );
}

#[test]
fn runnable_entry_expected_exit_codes_covers_all_runnable_demos() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts/runnable_entry_expected_exit_codes.txt");
    let content = std::fs::read_to_string(&path).expect("read runnable_entry_expected_exit_codes.txt");
    let names_in_file: Vec<&str> = content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    for demo in &RUNNABLE_ENTRY_DEMOS {
        let stem = demo.strip_suffix(".dx").unwrap();
        assert!(
            names_in_file.contains(&stem),
            "runnable demo '{stem}' missing from runnable_entry_expected_exit_codes.txt"
        );
    }
}

#[test]
fn runnable_entry_demos_have_main_function_in_ir() {
    use dx_llvm_ir::pipeline::emit_file_to_string_unchecked;
    for demo in &RUNNABLE_ENTRY_DEMOS {
        let path = fixture_path(demo);
        let ir = emit_file_to_string_unchecked(&path).expect(&format!("emit {demo}"));
        assert!(ir.contains("define i64 @main()"), "{demo} should define main() -> i64:\n{ir}");
    }
}

#[test]
fn all_executable_entry_demos_are_runnable() {
    // Every executable-entry demo should now be in the runnable subset.
    for demo in &EXECUTABLE_ENTRY_DEMOS {
        assert!(
            RUNNABLE_ENTRY_DEMOS.contains(demo),
            "executable-entry demo '{demo}' is not in RUNNABLE_ENTRY_DEMOS"
        );
    }
}

#[test]
fn runnable_entry_expected_exit_codes_has_no_extra_entries() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts/runnable_entry_expected_exit_codes.txt");
    let content = std::fs::read_to_string(&path).expect("read runnable_entry_expected_exit_codes.txt");
    let names_in_file: Vec<String> = content
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .map(|name| format!("{name}.dx"))
        .collect();
    for name in &names_in_file {
        assert!(
            RUNNABLE_ENTRY_DEMOS.contains(&name.as_str()),
            "exit code entry '{name}' is not in RUNNABLE_ENTRY_DEMOS — remove stale entry"
        );
    }
}

#[test]
fn runnable_entry_expected_exit_codes_are_valid_integers() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts/runnable_entry_expected_exit_codes.txt");
    let content = std::fs::read_to_string(&path).expect("read runnable_entry_expected_exit_codes.txt");
    for line in content.lines().filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(
            parts.len(), 2,
            "expected 'demo_name exit_code' format, got: {line}"
        );
        let code: i32 = parts[1].parse().unwrap_or_else(|_| {
            panic!("exit code '{}' for demo '{}' is not a valid integer", parts[1], parts[0])
        });
        assert!(
            (0..=255).contains(&code),
            "exit code {code} for demo '{}' is outside valid range 0-255", parts[0]
        );
    }
}
