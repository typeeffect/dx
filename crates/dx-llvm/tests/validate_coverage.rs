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

// ── integration: pipeline validates ──────────────────────────────

#[test]
fn pipeline_straight_line_validates() {
    let tokens = dx_parser::Lexer::new("fun f(x: Int) -> Int:\n    x + 1\n.\n").tokenize();
    let mut parser = dx_parser::Parser::new(tokens);
    let ast = parser.parse_module().expect("parse");
    let hir = dx_hir::lower_module(&ast);
    let typed = dx_hir::typecheck_module(&hir);
    let mir = dx_mir::lower_module(&typed.module);
    let low = dx_codegen::lower_module(&mir);
    let llvm = dx_llvm::lower_module(&low);
    let report = validate_module(&llvm);
    assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
}
