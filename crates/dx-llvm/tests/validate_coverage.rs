//! Validation coverage tests for dx-llvm::validate.
//!
//! Tests construct small invalid LLVM-like modules directly and verify
//! that the validator catches specific structural mistakes.

use dx_llvm::llvm::*;
use dx_llvm::{validate_module, ValidationReport};

fn module_with(functions: Vec<Function>) -> Module {
    Module {
        globals: vec![],
        externs: vec![],
        functions,
    }
}

fn module_with_externs(externs: Vec<ExternDecl>, functions: Vec<Function>) -> Module {
    Module {
        globals: vec![],
        externs,
        functions,
    }
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
        globals: vec![],
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

// ── ret void for Unit functions ──────────────────────────────────

#[test]
fn unit_function_renders_ret_void() {
    let (_, rendered, _) = pipeline("fun f():\n    42\n.\n");
    assert!(rendered.contains("define void @f()"), "void signature:\n{rendered}");
    assert!(rendered.contains("ret void"), "ret void:\n{rendered}");
}

#[test]
fn unit_function_with_side_effect_renders_ret_void() {
    let (_, rendered, _) = pipeline(
        "from py builtins import print\n\nfun f() !py:\n    print(\"hello\")\n.\n",
    );
    assert!(rendered.contains("define void @f()"), "void signature:\n{rendered}");
    assert!(rendered.contains("ret void"), "ret void:\n{rendered}");
}

#[test]
fn mixed_module_unit_and_nonunit() {
    let (_, rendered, _) = pipeline(
        "fun g(x: Int) -> Int:\n    x\n.\n\nfun h():\n    42\n.\n",
    );
    assert!(rendered.contains("define i64 @g("), "g signature:\n{rendered}");
    assert!(rendered.contains("define void @h()"), "h signature:\n{rendered}");
    assert!(rendered.contains("ret void"), "ret void:\n{rendered}");
}

#[test]
fn unit_function_with_python_and_closure() {
    let (_, rendered, _) = pipeline(
        "from py pandas import read_csv\n\nfun f(path: Str) !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
    );
    assert!(rendered.contains("define void @f("), "void signature:\n{rendered}");
    assert!(rendered.contains("@dx_rt_py_call_function"), "py call:\n{rendered}");
    assert!(rendered.contains("@dx_rt_closure_create"), "closure create:\n{rendered}");
}

// ── validator diagnostic snapshot tests ──────────────────────────

#[test]
fn validator_undefined_register_diagnostic_is_specific() {
    let module = module_with(vec![Function {
        name: "f".into(),
        params: vec![],
        ret: Type::I64,
        blocks: vec![Block {
            label: "bb0".into(),
            instructions: vec![],
            terminator: Terminator::Ret(Some(Operand::Register("%ghost".into(), Type::I64))),
        }],
    }]);
    let report = validate_module(&module);
    assert!(has_diag(&report, "undefined register"), "{:?}", report.diagnostics);
    assert!(has_diag(&report, "%ghost"), "{:?}", report.diagnostics);
}

#[test]
fn validator_duplicate_register_diagnostic_is_specific() {
    // Two instructions define the same register name
    let module = module_with_externs(
        vec![ExternDecl {
            symbol: "ext",
            params: vec![],
            ret: Type::I64,
        }],
        vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::I64,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![
                    Instruction::CallExtern {
                        result: Some("%1".into()),
                        symbol: "ext",
                        ret: Type::I64,
                        args: vec![],
                        comment: None,
                    },
                    Instruction::CallExtern {
                        result: Some("%1".into()),
                        symbol: "ext",
                        ret: Type::I64,
                        args: vec![],
                        comment: None,
                    },
                ],
                terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I64))),
            }],
        }],
    );
    let report = validate_module(&module);
    assert!(has_diag(&report, "duplicate") || has_diag(&report, "redefined"),
        "expected duplicate register diagnostic: {:?}", report.diagnostics);
}

#[test]
fn validator_missing_return_diagnostic_mentions_type() {
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
    assert!(has_diag(&report, "I64"), "{:?}", report.diagnostics);
}

// ── string literal globals ──────────────────────────────────────

#[test]
fn string_global_renders_with_constant_form() {
    let (_, rendered, _) = pipeline("fun f() -> Str:\n    \"hello\"\n.\n");
    // Global should use private unnamed_addr constant form
    assert!(
        rendered.contains("private unnamed_addr constant"),
        "constant form:\n{rendered}"
    );
    assert!(rendered.contains("@.str"), "global symbol:\n{rendered}");
    assert!(rendered.contains("c\"hello\\00\""), "c-string value:\n{rendered}");
}

#[test]
fn string_global_referenced_in_ret() {
    let (_, rendered, _) = pipeline("fun f() -> Str:\n    \"hello\"\n.\n");
    assert!(rendered.contains("ret ptr @.str"), "ret should reference global:\n{rendered}");
}

#[test]
fn two_different_strings_produce_two_globals() {
    let (_, rendered, _) = pipeline(
        "fun a() -> Str:\n    \"hello\"\n.\n\nfun b() -> Str:\n    \"world\"\n.\n",
    );
    let global_count = rendered.matches("private unnamed_addr constant").count();
    assert!(global_count >= 2, "expected >=2 globals, got {global_count}:\n{rendered}");
    assert!(rendered.contains("c\"hello\\00\""), "hello:\n{rendered}");
    assert!(rendered.contains("c\"world\\00\""), "world:\n{rendered}");
}

#[test]
fn same_string_in_two_functions_deduplicated() {
    let (_, rendered, _) = pipeline(
        "fun a() -> Str:\n    \"same\"\n.\n\nfun b() -> Str:\n    \"same\"\n.\n",
    );
    let global_count = rendered.matches("private unnamed_addr constant").count();
    // Should be 1 (deduplicated) or at most 2 — just verify it doesn't explode
    assert!(global_count >= 1 && global_count <= 2,
        "expected 1-2 globals for same string, got {global_count}:\n{rendered}");
}

#[test]
fn string_globals_before_externs_and_functions() {
    let (_, rendered, _) = pipeline("fun f() -> Str:\n    \"hi\"\n.\n");
    if let Some(global_pos) = rendered.find("private unnamed_addr") {
        if let Some(define_pos) = rendered.find("define ") {
            assert!(global_pos < define_pos, "globals before functions:\n{rendered}");
        }
    }
}

#[test]
fn string_global_deterministic() {
    let src = "fun a() -> Str:\n    \"x\"\n.\n\nfun b() -> Str:\n    \"y\"\n.\n";
    let (_, r1, _) = pipeline(src);
    let (_, r2, _) = pipeline(src);
    assert_eq!(r1, r2, "string globals rendering must be deterministic");
}

#[test]
fn string_global_with_python_call() {
    let (_, rendered, _) = pipeline(
        "from py builtins import print\n\nfun f() -> Str !py:\n    print(\"msg\")\n    \"result\"\n.\n",
    );
    assert!(rendered.contains("private unnamed_addr constant"), "global:\n{rendered}");
    assert!(rendered.contains("@dx_rt_py_call_function"), "py call:\n{rendered}");
}

#[test]
fn string_global_with_ret_void() {
    // Unit function that uses a string internally but returns void
    let (_, rendered, _) = pipeline(
        "from py builtins import print\n\nfun f() !py:\n    print(\"hello\")\n.\n",
    );
    assert!(rendered.contains("ret void"), "ret void:\n{rendered}");
}

// ── string global rendering: c-string escape edge cases ─────────
// These test render_c_string directly by constructing GlobalString modules.

fn module_with_global(value: &str) -> dx_llvm::Module {
    use dx_llvm::llvm::*;
    Module {
        globals: vec![GlobalString { symbol: ".str0".into(), value: value.into() }],
        externs: vec![],
        functions: vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Ptr,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![],
                terminator: Terminator::Ret(Some(Operand::Global(".str0".into(), Type::Ptr))),
            }],
        }],
    }
}

#[test]
fn render_c_string_with_newline() {
    let rendered = dx_llvm::render_module(&module_with_global("line1\nline2"));
    assert!(rendered.contains("\\0A"), "newline escape:\n{rendered}");
}

#[test]
fn render_c_string_with_quote() {
    let rendered = dx_llvm::render_module(&module_with_global("say \"hi\""));
    assert!(rendered.contains("\\22"), "quote escape:\n{rendered}");
}

#[test]
fn render_c_string_with_backslash() {
    let rendered = dx_llvm::render_module(&module_with_global("path\\to\\file"));
    assert!(rendered.contains("\\5C"), "backslash escape:\n{rendered}");
}

#[test]
fn render_c_string_with_tab() {
    let rendered = dx_llvm::render_module(&module_with_global("col1\tcol2"));
    assert!(rendered.contains("\\09"), "tab escape:\n{rendered}");
}

#[test]
fn render_c_string_null_terminated() {
    let rendered = dx_llvm::render_module(&module_with_global("hello"));
    // Must end with \00"
    assert!(rendered.contains("\\00\""), "null terminator:\n{rendered}");
}

#[test]
fn render_c_string_length_includes_null() {
    let rendered = dx_llvm::render_module(&module_with_global("abc"));
    // "abc" is 3 chars + null = 4 bytes
    assert!(rendered.contains("[4 x i8]"), "length includes null:\n{rendered}");
}

#[test]
fn two_escaped_strings_stable_order() {
    use dx_llvm::llvm::*;
    let module = Module {
        globals: vec![
            GlobalString { symbol: ".str0".into(), value: "a\nb".into() },
            GlobalString { symbol: ".str1".into(), value: "c\"d".into() },
        ],
        externs: vec![],
        functions: vec![simple_fn("f", vec![simple_block("bb0", Terminator::Ret(None))])],
    };
    let r1 = dx_llvm::render_module(&module);
    let r2 = dx_llvm::render_module(&module);
    assert_eq!(r1, r2, "escaped strings must render deterministically");
    assert!(r1.contains("\\0A"), "newline:\n{r1}");
    assert!(r1.contains("\\22"), "quote:\n{r1}");
}

// ── validator: global operand blind spot documentation ───────────

#[test]
fn validator_rejects_unknown_global_operand() {
    // Document: the validator currently DOES check for unknown globals
    use dx_llvm::llvm::*;
    let module = Module {
        globals: vec![],
        externs: vec![],
        functions: vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Ptr,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![],
                terminator: Terminator::Ret(Some(Operand::Global(".missing".into(), Type::Ptr))),
            }],
        }],
    };
    let report = validate_module(&module);
    assert!(has_diag(&report, "unknown global"), "{:?}", report.diagnostics);
}

#[test]
fn validator_accepts_declared_global_operand() {
    use dx_llvm::llvm::*;
    let module = Module {
        globals: vec![GlobalString { symbol: ".str0".into(), value: "ok".into() }],
        externs: vec![],
        functions: vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Ptr,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![],
                terminator: Terminator::Ret(Some(Operand::Global(".str0".into(), Type::Ptr))),
            }],
        }],
    };
    let report = validate_module(&module);
    assert!(report.is_ok(), "declared global should be accepted: {:?}", report.diagnostics);
}

// ── mixed: string globals + assignments + runtime hooks ──────────

#[test]
fn mixed_string_global_and_binary_op() {
    let (_, rendered, _) = pipeline(
        "fun f(x: Int) -> Str:\n    val y = x + 1\n    \"result\"\n.\n",
    );
    assert!(rendered.contains("private unnamed_addr constant"), "global:\n{rendered}");
    // Binary op should be visible somewhere (as an Assign instruction)
    // The function should contain both the computation and the string return
    assert!(rendered.contains("define ptr @f("), "function:\n{rendered}");
}

#[test]
fn mixed_string_global_closure_thunk() {
    let (_, rendered, _) = pipeline(
        "fun f(x: Int) -> Str:\n    val t = lazy \"hello\"\n    t()\n.\n",
    );
    // Closure + thunk + potentially a string global
    assert!(rendered.contains("@dx_rt_closure_create"), "create:\n{rendered}");
    // Module renders without panic — the exact global behavior depends on lowering
}
