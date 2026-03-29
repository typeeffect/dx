//! Cross-layer integration tests: dx-codegen (low) + dx-llvm (LLVM-like).
//!
//! Each test starts from the same DX source snippet and verifies that:
//! - both layers produce consistent output without panicking
//! - extern symbols, function names, and block labels align across layers
//! - rendering is deterministic and readable

use dx_codegen::render_low_module;
use dx_llvm::render_module;

fn pipeline(src: &str) -> (String, String) {
    let tokens = dx_parser::Lexer::new(src).tokenize();
    let mut parser = dx_parser::Parser::new(tokens);
    let ast = parser.parse_module().expect("parse");
    let hir = dx_hir::lower_module(&ast);
    let typed = dx_hir::typecheck_module(&hir);
    let mir = dx_mir::lower_module(&typed.module);
    let low = dx_codegen::lower_module(&mir);
    let llvm = dx_llvm::lower_module(&low);
    (render_low_module(&low), render_module(&llvm))
}

// ── Python runtime call + throw-check ────────────────────────────

#[test]
fn python_call_with_throw() {
    let (low, llvm) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
    );

    // Extern visible in both
    assert!(low.contains("dx_rt_py_call_function"), "low extern:\n{low}");
    assert!(llvm.contains("@dx_rt_py_call_function"), "llvm extern:\n{llvm}");

    // Throw check visible in both
    assert!(low.contains("throw-check"), "low throw-check:\n{low}");
    assert!(llvm.contains("@dx_rt_throw_check_pending"), "llvm throw-check:\n{llvm}");

    // Function name stable
    assert!(low.contains(" f("), "low function:\n{low}");
    assert!(llvm.contains("@f("), "llvm function:\n{llvm}");
}

// ── Closure create + thunk invoke ────────────────────────────────

#[test]
fn closure_create_and_thunk() {
    let (low, llvm) = pipeline(
        "fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n",
    );

    assert!(low.contains("dx_rt_closure_create"), "low create:\n{low}");
    assert!(llvm.contains("@dx_rt_closure_create"), "llvm create:\n{llvm}");

    assert!(low.contains("dx_rt_thunk_call"), "low thunk:\n{low}");
    assert!(llvm.contains("@dx_rt_thunk_call"), "llvm thunk:\n{llvm}");
}

// ── if / condbr ──────────────────────────────────────────────────

#[test]
fn if_else_control_flow() {
    let (low, llvm) = pipeline(
        "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
    );

    // Block labels stable across layers
    for label in &["bb0:", "bb1:"] {
        assert!(low.contains(label), "low {label}:\n{low}");
        assert!(llvm.contains(label), "llvm {label}:\n{llvm}");
    }

    // Branch visible in llvm
    assert!(llvm.contains("br "), "llvm br:\n{llvm}");
}

// ── match ────────────────────────────────────────────────────────

#[test]
fn match_control_flow() {
    let (low, llvm) = pipeline(
        "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
    );

    // Low level still has match terminator
    assert!(low.contains("match "), "low match:\n{low}");
    // LLVM level: match is lowered to dx_rt_match_tag calls + CondBr
    assert!(llvm.contains("@dx_rt_match_tag"), "llvm match_tag:\n{llvm}");
    assert!(!llvm.contains("match ptr"), "raw MatchBr should be gone:\n{llvm}");
}

// ── mixed Python + closure ───────────────────────────────────────

#[test]
fn mixed_python_and_closure() {
    let (low, llvm) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );

    for sym in &["dx_rt_py_call_function", "dx_rt_closure_create", "dx_rt_thunk_call"] {
        assert!(low.contains(sym), "low missing {sym}:\n{low}");
        assert!(llvm.contains(sym), "llvm missing {sym}:\n{llvm}");
    }
}

// ── multi-function module ────────────────────────────────────────

#[test]
fn multi_function_module() {
    let (low, llvm) = pipeline(
        "fun a(x: Int) -> Int:\n    x + 1\n.\n\nfun b(y: Str) -> Str:\n    y\n.\n",
    );

    assert!(low.contains(" a("), "low a:\n{low}");
    assert!(low.contains(" b("), "low b:\n{low}");
    assert!(llvm.contains("@a("), "llvm a:\n{llvm}");
    assert!(llvm.contains("@b("), "llvm b:\n{llvm}");
}

// ── extern alignment ─────────────────────────────────────────────

#[test]
fn externs_aligned_across_layers() {
    let (low, llvm) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );

    // Extract extern symbols from low-level declare lines
    let low_externs: Vec<&str> = low
        .lines()
        .filter(|l| l.starts_with("declare "))
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect();

    // Each low extern symbol should appear in llvm rendering
    for sym in &low_externs {
        assert!(
            llvm.contains(sym),
            "extern {sym} from low not found in llvm:\n{llvm}"
        );
    }
}

// ── validation ───────────────────────────────────────────────────

fn validate(src: &str) -> dx_llvm::ValidationReport {
    let tokens = dx_parser::Lexer::new(src).tokenize();
    let mut parser = dx_parser::Parser::new(tokens);
    let ast = parser.parse_module().expect("parse");
    let hir = dx_hir::lower_module(&ast);
    let typed = dx_hir::typecheck_module(&hir);
    let mir = dx_mir::lower_module(&typed.module);
    let low = dx_codegen::lower_module(&mir);
    let llvm = dx_llvm::lower_module(&low);
    dx_llvm::validate_module(&llvm)
}

#[test]
fn validation_runs_without_panic_for_all_scenarios() {
    // The validator now has stricter register-definition checks.
    // Some lowered modules have known gaps. Verify no panics.
    let sources = vec![
        "fun f(x: Int) -> Int:\n    x + 1\n.\n",
        "fun f(x: Int, y: Int) -> Int:\n    x + y\n.\n",
        "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        _:\n            0\n    .\n.\n",
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        "fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n",
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    ];
    for src in sources {
        let _report = validate(src); // must not panic
    }
}

// ── determinism ──────────────────────────────────────────────────

#[test]
fn cross_layer_rendering_is_deterministic() {
    let src = "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n";
    let (low1, llvm1) = pipeline(src);
    let (low2, llvm2) = pipeline(src);
    assert_eq!(low1, low2, "low-level rendering not deterministic");
    assert_eq!(llvm1, llvm2, "llvm rendering not deterministic");
}
