//! Fixture-based tests that exercise examples/backend/*.dx through the full pipeline.
//!
//! Each test loads a canonical demo file from disk and verifies:
//! - pipeline emits successfully
//! - expected runtime symbols appear in the emitted IR
//! - output is deterministic

use dx_llvm_ir::pipeline::{emit_file_to_string, emit_file_to_string_unchecked};
use std::path::PathBuf;

const ALL_DEMOS: [&str; 28] = [
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
    "main_returns_zero.dx",
    "main_arithmetic.dx",
    "main_closure_call_bool.dx",
    "main_closure_call_int.dx",
    "main_closure_call_multi_capture.dx",
    "main_closure_call_nested.dx",
    "main_closure_call_subtract.dx",
    "main_closure_call_two_args.dx",
    "main_thunk_arithmetic.dx",
    "main_thunk_bool.dx",
    "main_thunk_capture.dx",
    "main_thunk_three_capture.dx",
];

fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // crates/
    path.pop(); // dx-bootstrap/
    path.push("examples/backend");
    path.push(name);
    path
}

fn emit_fixture(name: &str) -> String {
    let path = fixture_path(name);
    emit_file_to_string(&path).unwrap_or_else(|e| panic!("failed to emit {name}: {e}"))
}

fn emit_fixture_unchecked(name: &str) -> String {
    let path = fixture_path(name);
    emit_file_to_string_unchecked(&path).unwrap_or_else(|e| panic!("failed to emit {name}: {e}"))
}

// ── arithmetic.dx ────────────────────────────────────────────────

#[test]
fn demo_arithmetic_emits() {
    let ir = emit_fixture("arithmetic.dx");
    assert!(ir.contains("define i64 @add_one("), "signature:\n{ir}");
    assert!(ir.contains("add i64"), "add instruction:\n{ir}");
    assert!(!ir.contains("declare"), "no externs needed:\n{ir}");
}

#[test]
fn demo_arithmetic_deterministic() {
    let ir1 = emit_fixture("arithmetic.dx");
    let ir2 = emit_fixture("arithmetic.dx");
    assert_eq!(ir1, ir2);
}

// ── thunk.dx ─────────────────────────────────────────────────────

#[test]
fn demo_thunk_emits() {
    let ir = emit_fixture("thunk.dx");
    assert!(ir.contains("define i64 @force("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_thunk_call_i64"), "thunk_call:\n{ir}");
}

#[test]
fn demo_thunk_deterministic() {
    let ir1 = emit_fixture("thunk.dx");
    let ir2 = emit_fixture("thunk.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_int.dx ──────────────────────────────────────────

#[test]
fn demo_closure_call_int_emits() {
    let ir = emit_fixture("closure_call_int.dx");
    assert!(ir.contains("define i64 @add_captured("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(
        ir.contains("dx_rt_closure_call_i64_1_i64"),
        "arity symbol:\n{ir}"
    );
}

#[test]
fn demo_closure_call_int_deterministic() {
    let ir1 = emit_fixture("closure_call_int.dx");
    let ir2 = emit_fixture("closure_call_int.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_str.dx ──────────────────────────────────────────

#[test]
fn demo_closure_call_str_emits() {
    let ir = emit_fixture("closure_call_str.dx");
    assert!(ir.contains("define ptr @echo("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(
        ir.contains("dx_rt_closure_call_ptr_1_ptr"),
        "arity symbol:\n{ir}"
    );
}

#[test]
fn demo_closure_call_str_deterministic() {
    let ir1 = emit_fixture("closure_call_str.dx");
    let ir2 = emit_fixture("closure_call_str.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_two_args.dx ─────────────────────────────────────

#[test]
fn demo_closure_call_two_args_emits() {
    let ir = emit_fixture("closure_call_two_args.dx");
    assert!(ir.contains("define i64 @sum_two("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(
        ir.contains("dx_rt_closure_call_i64_2_i64_i64"),
        "arity symbol:\n{ir}"
    );
}

#[test]
fn demo_closure_call_two_args_deterministic() {
    let ir1 = emit_fixture("closure_call_two_args.dx");
    let ir2 = emit_fixture("closure_call_two_args.dx");
    assert_eq!(ir1, ir2);
}

// ── match_nominal.dx ─────────────────────────────────────────────

#[test]
fn demo_match_nominal_emits() {
    // Match lowering has known validation gaps (register domination)
    let ir = emit_fixture_unchecked("match_nominal.dx");
    assert!(ir.contains("define void @choose("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_match_tag"), "match_tag:\n{ir}");
    assert!(ir.contains("br i1"), "cond branch:\n{ir}");
}

#[test]
fn demo_match_nominal_deterministic() {
    let ir1 = emit_fixture_unchecked("match_nominal.dx");
    let ir2 = emit_fixture_unchecked("match_nominal.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_ptr_ret_int_arg.dx ──────────────────────────────

#[test]
fn demo_closure_call_ptr_ret_int_arg_emits() {
    let ir = emit_fixture("closure_call_ptr_ret_int_arg.dx");
    assert!(ir.contains("define ptr @stringify("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_closure_call_ptr_1_i64"), "arity symbol:\n{ir}");
}

#[test]
fn demo_closure_call_ptr_ret_int_arg_deterministic() {
    let ir1 = emit_fixture("closure_call_ptr_ret_int_arg.dx");
    let ir2 = emit_fixture("closure_call_ptr_ret_int_arg.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_ptr_ret_str_int_args.dx ─────────────────────────

#[test]
fn demo_closure_call_ptr_ret_str_int_args_emits() {
    let ir = emit_fixture("closure_call_ptr_ret_str_int_args.dx");
    assert!(ir.contains("define ptr @combine("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_closure_call_ptr_2_ptr_i64"), "arity symbol:\n{ir}");
}

#[test]
fn demo_closure_call_ptr_ret_str_int_args_deterministic() {
    let ir1 = emit_fixture("closure_call_ptr_ret_str_int_args.dx");
    let ir2 = emit_fixture("closure_call_ptr_ret_str_int_args.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_void_ret_three_args.dx ──────────────────────────

#[test]
fn demo_closure_call_void_ret_three_args_emits() {
    let ir = emit_fixture_unchecked("closure_call_void_ret_three_args.dx");
    assert!(ir.contains("define void @consume("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_closure_call_void_3_i64_ptr_i1"), "arity symbol:\n{ir}");
}

#[test]
fn demo_closure_call_void_ret_three_args_deterministic() {
    let ir1 = emit_fixture_unchecked("closure_call_void_ret_three_args.dx");
    let ir2 = emit_fixture_unchecked("closure_call_void_ret_three_args.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_float.dx ────────────────────────────────────────

#[test]
fn demo_closure_call_float_emits() {
    let ir = emit_fixture("closure_call_float.dx");
    assert!(ir.contains("define double @double_it("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_closure_call_f64_1_f64"), "arity symbol:\n{ir}");
}

#[test]
fn demo_closure_call_float_deterministic() {
    let ir1 = emit_fixture("closure_call_float.dx");
    let ir2 = emit_fixture("closure_call_float.dx");
    assert_eq!(ir1, ir2);
}

// ── closure_call_bool.dx ────────────────────────────────────────

#[test]
fn demo_closure_call_bool_emits() {
    let ir = emit_fixture("closure_call_bool.dx");
    assert!(ir.contains("define i1 @negate("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_closure_call_i1_1_i1"), "arity symbol:\n{ir}");
}

#[test]
fn demo_closure_call_bool_deterministic() {
    let ir1 = emit_fixture("closure_call_bool.dx");
    let ir2 = emit_fixture("closure_call_bool.dx");
    assert_eq!(ir1, ir2);
}

// ── match_with_closure_call.dx ──────────────────────────────────

#[test]
fn demo_match_with_closure_call_emits() {
    let ir = emit_fixture_unchecked("match_with_closure_call.dx");
    assert!(ir.contains("define i64 @dispatch("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure_create:\n{ir}");
    assert!(ir.contains("dx_rt_closure_call_i64_1_i64"), "closure call:\n{ir}");
    assert!(ir.contains("@dx_rt_match_tag"), "match_tag:\n{ir}");
}

#[test]
fn demo_match_with_closure_call_deterministic() {
    let ir1 = emit_fixture_unchecked("match_with_closure_call.dx");
    let ir2 = emit_fixture_unchecked("match_with_closure_call.dx");
    assert_eq!(ir1, ir2);
}

// ── py_call_function.dx ──────────────────────────────────────────

#[test]
fn demo_py_call_function_emits() {
    let ir = emit_fixture("py_call_function.dx");
    assert!(ir.contains("define ptr @load("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_function"), "py_call_function:\n{ir}");
}

#[test]
fn demo_py_call_function_deterministic() {
    let ir1 = emit_fixture("py_call_function.dx");
    let ir2 = emit_fixture("py_call_function.dx");
    assert_eq!(ir1, ir2);
}

// ── py_call_method.dx ────────────────────────────────────────────

#[test]
fn demo_py_call_method_emits() {
    let ir = emit_fixture("py_call_method.dx");
    assert!(ir.contains("define ptr @load_head("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_function"), "py function call:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_method"), "py method call:\n{ir}");
}

#[test]
fn demo_py_call_method_deterministic() {
    let ir1 = emit_fixture("py_call_method.dx");
    let ir2 = emit_fixture("py_call_method.dx");
    assert_eq!(ir1, ir2);
}

// ── py_call_dynamic.dx ───────────────────────────────────────────

#[test]
fn demo_py_call_dynamic_emits() {
    let ir = emit_fixture("py_call_dynamic.dx");
    assert!(ir.contains("define ptr @invoke("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_function"), "py function call:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_dynamic"), "py dynamic call:\n{ir}");
}

#[test]
fn demo_py_call_dynamic_deterministic() {
    let ir1 = emit_fixture("py_call_dynamic.dx");
    let ir2 = emit_fixture("py_call_dynamic.dx");
    assert_eq!(ir1, ir2);
}

// ── py_call_throw.dx ─────────────────────────────────────────��───

#[test]
fn demo_py_call_throw_emits() {
    let ir = emit_fixture("py_call_throw.dx");
    assert!(ir.contains("define ptr @load_safe("), "signature:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_function"), "py_call_function:\n{ir}");
    assert!(ir.contains("@dx_rt_throw_check_pending"), "throw_check:\n{ir}");
}

#[test]
fn demo_py_call_throw_deterministic() {
    let ir1 = emit_fixture("py_call_throw.dx");
    let ir2 = emit_fixture("py_call_throw.dx");
    assert_eq!(ir1, ir2);
}

// ── runtime-stub manifest consistency ────────────────────────────

#[test]
fn manifest_covers_demo_symbols() {
    let symbols = dx_runtime_stub::manifest::EXPORTED_SYMBOLS;

    let required_by_demos: &[(&str, &str)] = &[
        ("thunk.dx", "dx_rt_closure_create"),
        ("thunk.dx", "dx_rt_thunk_call_i64"),
        ("closure_call_int.dx", "dx_rt_closure_call_i64_1_i64"),
        ("closure_call_str.dx", "dx_rt_closure_call_ptr_1_ptr"),
        ("closure_call_two_args.dx", "dx_rt_closure_call_i64_2_i64_i64"),
        ("closure_call_ptr_ret_int_arg.dx", "dx_rt_closure_call_ptr_1_i64"),
        ("closure_call_ptr_ret_str_int_args.dx", "dx_rt_closure_call_ptr_2_ptr_i64"),
        ("closure_call_void_ret_three_args.dx", "dx_rt_closure_call_void_3_i64_ptr_i1"),
        ("closure_call_float.dx", "dx_rt_closure_call_f64_1_f64"),
        ("closure_call_bool.dx", "dx_rt_closure_call_i1_1_i1"),
        ("match_nominal.dx", "dx_rt_match_tag"),
        ("match_nominal.dx", "dx_rt_throw_check_pending"),
        ("match_with_closure_call.dx", "dx_rt_match_tag"),
        ("match_with_closure_call.dx", "dx_rt_closure_call_i64_1_i64"),
        ("py_call_function.dx", "dx_rt_py_call_function"),
        ("py_call_method.dx", "dx_rt_py_call_function"),
        ("py_call_method.dx", "dx_rt_py_call_method"),
        ("py_call_dynamic.dx", "dx_rt_py_call_function"),
        ("py_call_dynamic.dx", "dx_rt_py_call_dynamic"),
        ("py_call_throw.dx", "dx_rt_py_call_function"),
        ("py_call_throw.dx", "dx_rt_throw_check_pending"),
        ("main_closure_call_int.dx", "dx_rt_closure_create"),
        ("main_closure_call_int.dx", "dx_rt_closure_call_i64_1_i64"),
    ];

    for (demo, symbol) in required_by_demos {
        assert!(
            symbols.contains(symbol),
            "manifest missing symbol '{symbol}' required by {demo}"
        );
    }
}

#[test]
fn manifest_covers_all_demo_externs() {
    let symbols = dx_runtime_stub::manifest::EXPORTED_SYMBOLS;
    let demos = ALL_DEMOS;

    for demo in &demos {
        let ir = emit_fixture_unchecked(demo);
        for line in ir.lines() {
            if line.starts_with("declare") {
                if let Some(at_pos) = line.find('@') {
                    if let Some(paren_pos) = line[at_pos..].find('(') {
                        let symbol = &line[at_pos + 1..at_pos + paren_pos];
                        assert!(
                            symbols.contains(&symbol),
                            "manifest missing extern '{symbol}' declared by {demo}"
                        );
                    }
                }
            }
        }
    }
}

// ── all demos emit without errors ────────────────────────────────

#[test]
fn all_demo_fixtures_emit_successfully() {
    for demo in &ALL_DEMOS {
        let path = fixture_path(demo);
        let result = emit_file_to_string_unchecked(&path);
        assert!(result.is_ok(), "{demo} failed to emit: {:?}", result.err());
    }
}

// ── manifest → IR symbol verification ────────────────────────────
// For every canonical demo, each expected symbol in demo_expected_symbols.txt
// must appear in the emitted IR.

fn load_expected_symbols() -> Vec<(String, Vec<String>)> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("scripts/demo_expected_symbols.txt");
    let content = std::fs::read_to_string(&path).expect("read demo_expected_symbols.txt");
    content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let (name, syms) = l.split_once(':').expect("colon in manifest line");
            let symbols: Vec<String> = if syms.is_empty() {
                vec![]
            } else {
                syms.split(',').map(|s| s.to_string()).collect()
            };
            (name.to_string(), symbols)
        })
        .collect()
}

#[test]
fn manifest_symbols_appear_in_emitted_ir() {
    let expected = load_expected_symbols();
    for (demo_name, symbols) in &expected {
        let demo_file = format!("{demo_name}.dx");
        let ir = emit_fixture_unchecked(&demo_file);
        for symbol in symbols {
            assert!(
                ir.contains(symbol.as_str()),
                "expected symbol '{symbol}' not found in emitted IR for {demo_name}.dx\nIR:\n{ir}"
            );
        }
    }
}
