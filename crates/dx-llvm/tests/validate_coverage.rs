//! Validation coverage tests for dx-llvm::validate.
//!
//! Tests construct small invalid LLVM-like modules directly and verify
//! that the validator catches specific structural mistakes.

use dx_llvm::llvm::*;
use dx_llvm::{validate_module, ValidationReport};

fn module_with(functions: Vec<Function>) -> Module {
    Module {
        externs: vec![],
        functions,
    }
}

fn module_with_externs(externs: Vec<ExternDecl>, functions: Vec<Function>) -> Module {
    Module { externs, functions }
}

fn simple_fn(name: &str, blocks: Vec<Block>) -> Function {
    Function {
        name: name.into(),
        params: vec![],
        ret: Type::Void,
        blocks,
    }
}

fn simple_block(label: &str, term: Terminator) -> Block {
    Block {
        label: label.into(),
        instructions: vec![],
        terminator: term,
    }
}

fn has_diag(report: &ValidationReport, needle: &str) -> bool {
    report.diagnostics.iter().any(|d| d.message.contains(needle))
}

// ── duplicate function name ──────────────────────────────────────

#[test]
fn rejects_duplicate_function_name() {
    let module = module_with(vec![
        simple_fn("f", vec![simple_block("bb0", Terminator::Ret(None))]),
        simple_fn("f", vec![simple_block("bb0", Terminator::Ret(None))]),
    ]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "duplicate function"), "{:?}", report.diagnostics);
}

// ── duplicate block label ────────────────────────────────────────

#[test]
fn rejects_duplicate_block_label() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![
            simple_block("bb0", Terminator::Br("bb0".into())),
            simple_block("bb0", Terminator::Ret(None)),
        ],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "duplicate block label"), "{:?}", report.diagnostics);
}

// ── arg count mismatch ───────────────────────────────────────────

#[test]
fn rejects_arg_count_mismatch() {
    let module = module_with_externs(
        vec![ExternDecl {
            symbol: "ext",
            params: vec![Type::I64, Type::I64],
            ret: Type::Void,
        }],
        vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Void,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: None,
                    symbol: "ext",
                    ret: Type::Void,
                    args: vec![Operand::ConstInt(1)], // 1 arg, extern expects 2
                    comment: None,
                }],
                terminator: Terminator::Ret(None),
            }],
        }],
    );
    let report = validate_module(&module);
    assert!(has_diag(&report, "arg count mismatch"), "{:?}", report.diagnostics);
}

// ── arg type mismatch ────────────────────────────────────────────

#[test]
fn rejects_arg_type_mismatch() {
    let module = module_with_externs(
        vec![ExternDecl {
            symbol: "ext",
            params: vec![Type::Ptr],
            ret: Type::Void,
        }],
        vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Void,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: None,
                    symbol: "ext",
                    ret: Type::Void,
                    args: vec![Operand::ConstInt(42)], // i64, extern expects ptr
                    comment: None,
                }],
                terminator: Terminator::Ret(None),
            }],
        }],
    );
    let report = validate_module(&module);
    assert!(has_diag(&report, "arg type mismatch"), "{:?}", report.diagnostics);
}

// ── match arm to unknown label ───────────────────────────────────

#[test]
fn rejects_match_arm_to_unknown_label() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::MatchBr {
                scrutinee: Operand::ConstInt(0),
                arms: vec![("Ok".into(), "missing_bb".into())],
                fallback: "bb0".into(),
            },
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "match branch to unknown label"), "{:?}", report.diagnostics);
}

// ── match fallback to unknown label ──────────────────────────────

#[test]
fn rejects_match_fallback_to_unknown_label() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::MatchBr {
                scrutinee: Operand::ConstInt(0),
                arms: vec![],
                fallback: "missing_bb".into(),
            },
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "match fallback to unknown label"), "{:?}", report.diagnostics);
}

// ── condbr to unknown labels ─────────────────────────────────────

#[test]
fn rejects_condbr_then_to_unknown() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::CondBr {
                cond: Operand::Register("%c".into(), Type::I1),
                then_label: "missing".into(),
                else_label: "bb0".into(),
            },
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "unknown label 'missing'"), "{:?}", report.diagnostics);
}

#[test]
fn rejects_condbr_else_to_unknown() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::CondBr {
                cond: Operand::Register("%c".into(), Type::I1),
                then_label: "bb0".into(),
                else_label: "missing".into(),
            },
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "unknown label 'missing'"), "{:?}", report.diagnostics);
}

// ── missing return value ─────────────────────────────────────────

#[test]
fn rejects_missing_return_for_non_void() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::I64,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::Ret(None),
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "missing return value"), "{:?}", report.diagnostics);
}

// ── combinations: multiple errors in one module ──────────────────

#[test]
fn rejects_duplicate_extern_and_duplicate_function_together() {
    let ext = ExternDecl {
        symbol: "ext",
        params: vec![],
        ret: Type::Void,
    };
    let module = Module {
        externs: vec![ext.clone(), ext],
        functions: vec![
            simple_fn("f", vec![simple_block("bb0", Terminator::Ret(None))]),
            simple_fn("f", vec![simple_block("bb0", Terminator::Ret(None))]),
        ],
    };
    let report = validate_module(&module);
    assert!(has_diag(&report, "duplicate extern"), "{:?}", report.diagnostics);
    assert!(has_diag(&report, "duplicate function"), "{:?}", report.diagnostics);
}

#[test]
fn rejects_bad_match_arm_and_fallback_together() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::MatchBr {
                scrutinee: Operand::ConstInt(0),
                arms: vec![("A".into(), "no_such_arm".into())],
                fallback: "no_such_fallback".into(),
            },
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "match branch to unknown label 'no_such_arm'"), "{:?}", report.diagnostics);
    assert!(has_diag(&report, "match fallback to unknown label 'no_such_fallback'"), "{:?}", report.diagnostics);
}

#[test]
fn rejects_bad_condbr_type_and_unknown_labels_together() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Void,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::CondBr {
                cond: Operand::ConstInt(1), // i64 not i1
                then_label: "nope_then".into(),
                else_label: "nope_else".into(),
            },
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "expects i1 condition"), "{:?}", report.diagnostics);
    assert!(has_diag(&report, "unknown label 'nope_then'"), "{:?}", report.diagnostics);
    assert!(has_diag(&report, "unknown label 'nope_else'"), "{:?}", report.diagnostics);
}

// ── return type: ptr vs i64 mismatch ─────────────────────────────

#[test]
fn rejects_ptr_return_when_i64_expected() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::I64,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::Ret(Some(Operand::Register("%0".into(), Type::Ptr))),
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "return type mismatch"), "{:?}", report.diagnostics);
}

#[test]
fn rejects_i64_return_when_ptr_expected() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::Ptr,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::Ret(Some(Operand::ConstInt(42))),
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "return type mismatch"), "{:?}", report.diagnostics);
}

// ── multi-function: only one invalid ─────────────────────────────

#[test]
fn only_invalid_function_produces_diagnostics() {
    let module = module_with(vec![
        // valid
        Function {
            name: "good".into(),
            params: vec![],
            ret: Type::Void,
            blocks: vec![simple_block("bb0", Terminator::Ret(None))],
        },
        // invalid: return type mismatch
        Function {
            name: "bad".into(),
            params: vec![],
            ret: Type::I64,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![],
                terminator: Terminator::Ret(Some(Operand::Register("%0".into(), Type::Ptr))),
            }],
        },
    ]);
    let report = validate_module(&module);
    // Only bad function should have diagnostics
    assert!(report.diagnostics.iter().all(|d| d.function.as_deref() == Some("bad")),
        "expected only 'bad' diagnostics: {:?}", report.diagnostics);
    assert!(!report.diagnostics.is_empty());
}

// ── specialized closure hook: extern/call return type mismatch ───

#[test]
fn rejects_call_return_type_mismatch_i64_vs_ptr() {
    // Extern declares i64 return, call says ptr
    let module = module_with_externs(
        vec![ExternDecl {
            symbol: "dx_rt_thunk_call_i64",
            params: vec![Type::Ptr],
            ret: Type::I64,
        }],
        vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Ptr,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: Some("%1".into()),
                    symbol: "dx_rt_thunk_call_i64",
                    ret: Type::Ptr, // mismatch: extern says i64
                    args: vec![Operand::Register("%0".into(), Type::Ptr)],
                    comment: None,
                }],
                terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::Ptr))),
            }],
        }],
    );
    let report = validate_module(&module);
    assert!(has_diag(&report, "call return type mismatch"), "{:?}", report.diagnostics);
}

#[test]
fn accepts_matching_specialized_thunk_call() {
    // Extern and call both agree on i64 return
    let module = module_with_externs(
        vec![ExternDecl {
            symbol: "dx_rt_thunk_call_i64",
            params: vec![Type::Ptr],
            ret: Type::I64,
        }],
        vec![Function {
            name: "f".into(),
            params: vec![Param { name: "%0".into(), ty: Type::Ptr }],
            ret: Type::I64,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: Some("%1".into()),
                    symbol: "dx_rt_thunk_call_i64",
                    ret: Type::I64,
                    args: vec![Operand::Register("%0".into(), Type::Ptr)],
                    comment: None,
                }],
                terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I64))),
            }],
        }],
    );
    let report = validate_module(&module);
    assert!(report.is_ok(), "should be valid: {:?}", report.diagnostics);
}

#[test]
fn accepts_mixed_python_and_specialized_closure_externs() {
    let module = module_with_externs(
        vec![
            ExternDecl {
                symbol: "dx_rt_closure_create",
                params: vec![Type::Ptr, Type::I64],
                ret: Type::Ptr,
            },
            ExternDecl {
                symbol: "dx_rt_py_call_function",
                params: vec![Type::Ptr, Type::I64],
                ret: Type::Ptr,
            },
            ExternDecl {
                symbol: "dx_rt_thunk_call_ptr",
                params: vec![Type::Ptr],
                ret: Type::Ptr,
            },
        ],
        vec![simple_fn("f", vec![simple_block("bb0", Terminator::Ret(None))])],
    );
    let report = validate_module(&module);
    assert!(report.is_ok(), "mixed externs should validate: {:?}", report.diagnostics);
}

// ── pipeline helper ──────────────────────────────────────────────

fn pipeline(src: &str) -> (dx_llvm::Module, String, dx_llvm::ValidationReport) {
    let tokens = dx_parser::Lexer::new(src).tokenize();
    let mut parser = dx_parser::Parser::new(tokens);
    let ast = parser.parse_module().expect("parse");
    let hir = dx_hir::lower_module(&ast);
    let typed = dx_hir::typecheck_module(&hir);
    let mir = dx_mir::lower_module(&typed.module);
    let low = dx_codegen::lower_module(&mir);
    let llvm = dx_llvm::lower_module(&low);
    let rendered = dx_llvm::render_module(&llvm);
    let report = validate_module(&llvm);
    (llvm, rendered, report)
}

// ── integration: pipeline validates ──────────────────────────────

#[test]
fn pipeline_straight_line_validates() {
    let (_, _, report) = pipeline("fun f(x: Int) -> Int:\n    x + 1\n.\n");
    assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
}

#[test]
fn pipeline_thunk_returning_int_validates() {
    let (_, rendered, report) = pipeline("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
    assert!(report.is_ok(), "thunk Int: {:?}", report.diagnostics);
    assert!(rendered.contains("thunk_call_i64") || rendered.contains("thunk_call"),
        "specialized thunk symbol:\n{rendered}");
}

#[test]
fn pipeline_thunk_returning_pyobj_validates() {
    let (_, rendered, report) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );
    assert!(report.is_ok(), "thunk PyObj: {:?}", report.diagnostics);
    assert!(rendered.contains("thunk_call_ptr") || rendered.contains("thunk_call"),
        "ptr-return thunk symbol:\n{rendered}");
}

#[test]
fn pipeline_mixed_python_closure_validates() {
    let (_, rendered, report) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );
    assert!(report.is_ok(), "mixed: {:?}", report.diagnostics);
    assert!(rendered.contains("@dx_rt_py_call_function"), "py call:\n{rendered}");
    assert!(rendered.contains("@dx_rt_closure_create"), "create:\n{rendered}");
}

#[test]
fn pipeline_deterministic() {
    let src = "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n";
    let (_, r1, _) = pipeline(src);
    let (_, r2, _) = pipeline(src);
    assert_eq!(r1, r2, "must be deterministic");
}

// ── operand fidelity: real locals in thunk/closure calls ─────────

#[test]
fn thunk_call_uses_real_closure_operand() {
    // val t = lazy x; t() — the thunk call arg should be the real local for t (%N)
    let (_, rendered, report) = pipeline("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
    assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
    // Should contain a call with %N (real register), not a hardcoded %closure
    assert!(!rendered.contains("%closure"), "should not use placeholder %closure:\n{rendered}");
    // The thunk call instruction (not the declare) should use a real %N operand
    let thunk_line = rendered.lines()
        .find(|l| l.contains("thunk_call") && l.contains("call "))
        .expect("thunk call instruction line");
    assert!(thunk_line.contains("ptr %"), "thunk call should use real ptr operand:\n{thunk_line}");
}

#[test]
fn closure_call_uses_real_closure_operand() {
    // val f = (y: Int) => x + y; f(1) — closure call arg should be real local
    let (_, rendered, report) = pipeline(
        "fun g(x: Int) -> Int:\n    val f = (y: Int) => x + y\n    f(1)\n.\n",
    );
    assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
    assert!(!rendered.contains("%closure"), "should not use placeholder %closure:\n{rendered}");
    let closure_call_line = rendered.lines()
        .find(|l| l.contains("closure_call") && l.contains("call "));
    if let Some(line) = closure_call_line {
        assert!(line.contains("ptr %"), "closure call should use real ptr operand:\n{line}");
    }
}

// ── mixed module: python + create + specialized thunk + throw ────

#[test]
fn mixed_full_scenario_renders_completely() {
    let (_, rendered, report) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );
    assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);

    // All expected symbols present
    assert!(rendered.contains("@dx_rt_py_call_function"), "py call:\n{rendered}");
    assert!(rendered.contains("@dx_rt_closure_create"), "create:\n{rendered}");
    assert!(rendered.contains("@dx_rt_throw_check_pending"), "throw-check:\n{rendered}");
    // Thunk call with specialized or generic symbol
    assert!(
        rendered.contains("thunk_call_ptr") || rendered.contains("thunk_call"),
        "thunk call:\n{rendered}"
    );

    // Throw check follows py call
    let py_pos = rendered.find("@dx_rt_py_call_function").unwrap();
    let throw_pos = rendered.find("@dx_rt_throw_check_pending").unwrap();
    assert!(throw_pos > py_pos, "throw-check should follow py call");
}

// ── validator: closure operand type mismatch ─────────────────────

#[test]
fn rejects_closure_operand_type_mismatch_in_call() {
    // Extern expects (ptr) but call passes (i64) — type mismatch
    let module = module_with_externs(
        vec![ExternDecl {
            symbol: "dx_rt_thunk_call_i64",
            params: vec![Type::Ptr],
            ret: Type::I64,
        }],
        vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::I64,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: Some("%1".into()),
                    symbol: "dx_rt_thunk_call_i64",
                    ret: Type::I64,
                    args: vec![Operand::ConstInt(42)], // i64, but extern wants ptr
                    comment: None,
                }],
                terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I64))),
            }],
        }],
    );
    let report = validate_module(&module);
    assert!(has_diag(&report, "arg type mismatch"), "{:?}", report.diagnostics);
}

// ── PackEnv rendering and ordering ──────────────────────────────

#[test]
fn pack_env_renders_single_capture() {
    let (_, rendered, _report) = pipeline("fun f(x: Int) -> lazy Int:\n    lazy x\n.\n");
    assert!(rendered.contains("pack_env ["), "pack_env missing:\n{rendered}");
    // Within the function body (after "define"), pack_env appears before the call to closure_create
    let body_start = rendered.find("define ").unwrap_or(0);
    let body = &rendered[body_start..];
    if let (Some(pack_pos), Some(create_pos)) = (body.find("pack_env"), body.find("call ptr @dx_rt_closure_create")) {
        assert!(pack_pos < create_pos, "pack_env should precede closure_create call:\n{rendered}");
    }
}

#[test]
fn pack_env_renders_two_captures_stably() {
    let (_, r1, _) = pipeline("fun f(x: Int, y: Int) -> lazy Int:\n    lazy x + y\n.\n");
    assert!(r1.contains("pack_env ["), "pack_env missing:\n{r1}");
    let (_, r2, _) = pipeline("fun f(x: Int, y: Int) -> lazy Int:\n    lazy x + y\n.\n");
    assert_eq!(r1, r2, "pack_env ordering must be deterministic");
}

#[test]
fn pack_env_before_closure_create_in_mixed_module() {
    let (_, rendered, _report) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );
    let body_start = rendered.find("define ").unwrap_or(0);
    let body = &rendered[body_start..];
    if let (Some(pack_pos), Some(create_pos)) = (body.find("pack_env"), body.find("call ptr @dx_rt_closure_create")) {
        assert!(pack_pos < create_pos, "pack_env before closure_create call:\n{rendered}");
    }
    assert!(rendered.contains("@dx_rt_py_call_function"), "py call:\n{rendered}");
    assert!(rendered.contains("@dx_rt_throw_check_pending"), "throw-check:\n{rendered}");
}

#[test]
fn validator_accepts_module_with_pack_env() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![Param { name: "%0".into(), ty: Type::I64 }],
        ret: Type::Ptr,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![Instruction::PackEnv {
                result: "%env".into(),
                captures: vec![Operand::Register("%0".into(), Type::I64)],
            }],
            terminator: Terminator::Ret(Some(Operand::Register("%env".into(), Type::Ptr))),
        }],
    }]);
    let report = validate_module(&module);
    assert!(report.is_ok(), "PackEnv should not cause validation failure: {:?}", report.diagnostics);
}

#[test]
fn multiple_closure_creates_render_deterministically() {
    let (_, r1, _) = pipeline(
        "fun f(x: Int, y: Int) -> lazy Int:\n    val a = lazy x\n    val b = lazy y\n    a\n.\n",
    );
    let (_, r2, _) = pipeline(
        "fun f(x: Int, y: Int) -> lazy Int:\n    val a = lazy x\n    val b = lazy y\n    a\n.\n",
    );
    assert_eq!(r1, r2, "multiple closure creates must render identically");
    let create_count = r1.matches("@dx_rt_closure_create").count();
    assert!(create_count >= 2, "expected >=2 closure creates, got {create_count}:\n{r1}");
}
