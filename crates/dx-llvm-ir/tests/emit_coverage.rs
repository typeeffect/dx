//! Coverage tests for dx-llvm-ir textual LLVM IR emission.
//! Focuses on string global edge cases, non-ASCII, and mixed scenarios.

use dx_llvm::llvm::*;
use dx_llvm_ir::emit_module;

fn llvm_module(src: &str) -> Module {
    let tokens = dx_parser::Lexer::new(src).tokenize();
    let mut parser = dx_parser::Parser::new(tokens);
    let ast = parser.parse_module().expect("parse");
    let hir = dx_hir::lower_module(&ast);
    let typed = dx_hir::typecheck_module(&hir);
    let mir = dx_mir::lower_module(&typed.module);
    let low = dx_codegen::lower_module(&mir);
    dx_llvm::lower_module(&low)
}

fn emit(src: &str) -> String {
    emit_module(&llvm_module(src)).expect("emit")
}

fn module_with_string_global(symbol: &str, value: &str) -> Module {
    Module {
        globals: vec![GlobalString {
            symbol: symbol.into(),
            value: value.into(),
        }],
        externs: vec![],
        functions: vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Ptr,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![],
                terminator: Terminator::Ret(Some(Operand::Global(symbol.into(), Type::Ptr))),
            }],
        }],
    }
}

// ── non-ASCII / multibyte string emission ────────────────────────

#[test]
fn emit_accented_string_uses_hex_escapes() {
    let module = module_with_string_global(".str0", "café");
    let ir = emit_module(&module).expect("emit");
    // 'é' is U+00E9 = bytes C3 A9 in UTF-8
    assert!(ir.contains("\\C3"), "expected \\C3 for é first byte:\n{ir}");
    assert!(ir.contains("\\A9"), "expected \\A9 for é second byte:\n{ir}");
}

#[test]
fn emit_emoji_string_uses_hex_escapes() {
    let module = module_with_string_global(".str0", "hi 🎉");
    let ir = emit_module(&module).expect("emit");
    // 🎉 is U+1F389 = bytes F0 9F 8E 89 in UTF-8
    assert!(ir.contains("\\F0"), "expected \\F0 for emoji byte:\n{ir}");
    assert!(ir.contains("\\9F"), "expected \\9F for emoji byte:\n{ir}");
}

#[test]
fn emit_newline_in_string_uses_hex_escape() {
    let module = module_with_string_global(".str0", "a\nb");
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("\\0A"), "newline:\n{ir}");
}

#[test]
fn emit_quote_in_string_uses_hex_escape() {
    let module = module_with_string_global(".str0", "say \"hi\"");
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("\\22"), "quote:\n{ir}");
}

#[test]
fn emit_backslash_in_string_uses_hex_escape() {
    let module = module_with_string_global(".str0", "a\\b");
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("\\5C"), "backslash:\n{ir}");
}

#[test]
fn emit_null_terminated() {
    let module = module_with_string_global(".str0", "x");
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("\\00\""), "null terminated:\n{ir}");
}

#[test]
fn emit_string_length_correct_for_utf8() {
    // "café" = 4 code points but 5 bytes (c, a, f, 0xC3, 0xA9) + null = 6
    let module = module_with_string_global(".str0", "café");
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("[6 x i8]"), "byte length for café + null:\n{ir}");
}

// ── mixed real-IR scenarios ──────────────────────────────────────

#[test]
fn emit_string_global_plus_arithmetic() {
    let ir = emit("fun f(x: Int) -> Str:\n    val y = x + 1\n    \"result\"\n.\n");
    assert!(ir.contains("add i64"), "arithmetic:\n{ir}");
    assert!(ir.contains("@.str"), "string global ref:\n{ir}");
    assert!(ir.contains("private unnamed_addr constant"), "global decl:\n{ir}");
}

#[test]
fn emit_deterministic() {
    let src = "fun f(x: Int) -> Str:\n    val y = x + 1\n    \"hello\"\n.\n";
    let ir1 = emit(src);
    let ir2 = emit(src);
    assert_eq!(ir1, ir2, "emission must be deterministic");
}

#[test]
fn emit_two_string_globals_in_order() {
    let ir = emit("fun a() -> Str:\n    \"first\"\n.\n\nfun b() -> Str:\n    \"second\"\n.\n");
    assert!(ir.contains("first"), "first string:\n{ir}");
    assert!(ir.contains("second"), "second string:\n{ir}");
    let first_pos = ir.find("first").unwrap();
    let second_pos = ir.find("second").unwrap();
    assert!(first_pos < second_pos, "globals in stable order");
}

#[test]
fn emit_void_function() {
    let ir = emit("fun f() -> Unit:\n    42\n.\n");
    assert!(ir.contains("define void @f()"), "void signature:\n{ir}");
    assert!(ir.contains("ret void"), "ret void:\n{ir}");
}

// ── empty string emission ────────────────────────────────────────

#[test]
fn emit_empty_string_global() {
    let module = module_with_string_global(".str0", "");
    let ir = emit_module(&module).expect("emit");
    // "" = 0 chars + null = 1 byte
    assert!(ir.contains("[1 x i8]"), "empty string length:\n{ir}");
    assert!(ir.contains("c\"\\00\""), "empty string body:\n{ir}");
}

#[test]
fn emit_empty_and_nonempty_strings_together() {
    let module = Module {
        globals: vec![
            GlobalString { symbol: ".str0".into(), value: "".into() },
            GlobalString { symbol: ".str1".into(), value: "hello".into() },
        ],
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
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("[1 x i8]"), "empty string:\n{ir}");
    assert!(ir.contains("[6 x i8]"), "hello string:\n{ir}");
    let r1 = emit_module(&module).expect("emit");
    assert_eq!(ir, r1, "deterministic");
}

// ── mixed real-IR: arithmetic + string + ret void ────────────────

#[test]
fn emit_arithmetic_then_string_return() {
    let ir = emit("fun f(x: Int) -> Str:\n    val y = x + 1\n    \"result\"\n.\n");
    // Should have: alloca, load, add, store, getelementptr for string, ret
    assert!(ir.contains("alloca i64"), "alloca:\n{ir}");
    assert!(ir.contains("add i64"), "add:\n{ir}");
    assert!(ir.contains("getelementptr inbounds"), "gep for string:\n{ir}");
    assert!(ir.contains("ret ptr"), "ret ptr:\n{ir}");
}

#[test]
fn emit_unit_function_with_arithmetic() {
    let ir = emit("fun f(x: Int) -> Unit:\n    val y = x + 1\n    y\n.\n");
    assert!(ir.contains("define void @f("), "void sig:\n{ir}");
    assert!(ir.contains("add i64"), "arithmetic:\n{ir}");
    assert!(ir.contains("ret void"), "ret void:\n{ir}");
}

// ── thunk runtime path ──────────────────────────────────────────

#[test]
fn emit_thunk_shows_env_materialization() {
    let ir = emit("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
    // Should contain: alloca for env struct, getelementptr, store capture, closure_create call, thunk_call
    assert!(ir.contains("alloca { i64 }"), "env alloca:\n{ir}");
    assert!(ir.contains("getelementptr inbounds { i64 }"), "gep into env:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure create:\n{ir}");
    assert!(ir.contains("@dx_rt_thunk_call_i64"), "thunk call:\n{ir}");
}

#[test]
fn emit_thunk_deterministic() {
    let src = "fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n";
    let ir1 = emit(src);
    let ir2 = emit(src);
    assert_eq!(ir1, ir2, "thunk emission deterministic");
}

// ── intentionally unsupported cases ─────────────────────────────

#[test]
fn emit_rejects_match_with_unsupported_terminator() {
    let module = llvm_module(
        "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        _:\n            0\n    .\n.\n",
    );
    let err = emit_module(&module).expect_err("match should be unsupported");
    assert!(
        matches!(err, dx_llvm_ir::EmitError::UnsupportedTerminator("match")),
        "expected UnsupportedTerminator(match), got: {:?}", err
    );
}

#[test]
fn emit_rejects_python_placeholder_operands() {
    // Python calls use placeholder operands like %py_function that the emitter rejects
    use dx_llvm::llvm::*;
    let module = Module {
        globals: vec![],
        externs: vec![ExternDecl {
            symbol: "dx_rt_py_call_function",
            params: vec![Type::Ptr, Type::I64],
            ret: Type::Ptr,
        }],
        functions: vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Ptr,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: Some("%1".into()),
                    symbol: "dx_rt_py_call_function",
                    ret: Type::Ptr,
                    args: vec![
                        Operand::Register("%py_function".into(), Type::Ptr),
                        Operand::ConstInt(1),
                    ],
                    comment: None,
                }],
                terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::Ptr))),
            }],
        }],
    };
    let err = emit_module(&module).expect_err("py placeholder should be rejected");
    assert!(
        matches!(err, dx_llvm_ir::EmitError::UnsupportedOperand(ref name) if name.starts_with("%py_")),
        "expected UnsupportedOperand(%%py_...), got: {:?}", err
    );
}

// ── Python calls with string globals as args ─────────────────────

#[test]
fn emit_python_function_call_has_name_global() {
    let ir = emit(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
    );
    // Function name "read_csv" should be a global string
    assert!(ir.contains("c\"read_csv\\00\""), "function name global:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_function"), "py call:\n{ir}");
}

#[test]
fn emit_python_method_call_has_method_name_global() {
    let ir = emit(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
    );
    // Method name "head" should be a global string
    assert!(ir.contains("c\"head\\00\""), "method name global:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_method"), "method call:\n{ir}");
}

#[test]
fn emit_python_call_globals_in_stable_order() {
    let ir = emit(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
    );
    // Both "read_csv" and "head" globals should appear, in stable order
    let r1 = emit(
        "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
    );
    assert_eq!(ir, r1, "python call globals deterministic");
    assert!(ir.contains("c\"read_csv\\00\""), "read_csv global:\n{ir}");
    assert!(ir.contains("c\"head\\00\""), "head global:\n{ir}");
}

#[test]
fn emit_string_return_plus_python_call_globals_coexist() {
    let ir = emit(
        "from py builtins import print\n\nfun f() -> Str !py:\n    print(\"msg\")\n    \"result\"\n.\n",
    );
    // Should have globals for both "print" (function name) and "result" (return value)
    assert!(ir.contains("c\"print\\00\""), "print name global:\n{ir}");
    assert!(ir.contains("c\"result\\00\""), "result string global:\n{ir}");
    // Deterministic
    let ir2 = emit(
        "from py builtins import print\n\nfun f() -> Str !py:\n    print(\"msg\")\n    \"result\"\n.\n",
    );
    assert_eq!(ir, ir2, "mixed globals deterministic");
}

// ── empty string as call argument ────────────────────────────────

#[test]
fn emit_empty_string_global_as_call_arg() {
    // Construct a module where an empty string global is used as a call arg
    use dx_llvm::llvm::*;
    let module = Module {
        globals: vec![GlobalString { symbol: ".str0".into(), value: "".into() }],
        externs: vec![ExternDecl {
            symbol: "ext",
            params: vec![Type::Ptr],
            ret: Type::Void,
        }],
        functions: vec![Function {
            name: "f".into(),
            params: vec![],
            ret: Type::Void,
            blocks: vec![Block {
                label: "bb0".into(),
                instructions: vec![Instruction::CallExtern {
                    result: None,
                    symbol: "ext",
                    ret: Type::Void,
                    args: vec![Operand::Global(".str0".into(), Type::Ptr)],
                    comment: None,
                }],
                terminator: Terminator::Ret(None),
            }],
        }],
    };
    let ir = emit_module(&module).expect("emit");
    assert!(ir.contains("[1 x i8] c\"\\00\""), "empty string global:\n{ir}");
    assert!(ir.contains("getelementptr inbounds [1 x i8], ptr @.str0"), "gep for empty string:\n{ir}");
    assert!(ir.contains("call void @ext("), "call with string arg:\n{ir}");
}

// ── match mixed: supported code fails only because of match ──────

#[test]
fn emit_match_fails_even_with_supported_arithmetic() {
    // A function with both arithmetic (supported) and match (unsupported)
    // should fail specifically because of match, not arithmetic
    let module = llvm_module(
        "fun f(x: Result) -> Int:\n    val y = 1 + 2\n    match x:\n        Ok(v):\n            v\n        _:\n            y\n    .\n.\n",
    );
    let err = emit_module(&module).expect_err("should fail on match");
    assert!(
        matches!(err, dx_llvm_ir::EmitError::UnsupportedTerminator("match")),
        "error should be specifically about match: {:?}", err
    );
}

// ── mixed closure + string scenarios ─────────────────────────────

#[test]
fn emit_thunk_capturing_string_literal() {
    // The string is first materialized as a local, then captured by the thunk.
    let ir = emit("fun f() -> Str:\n    val s = \"hello\"\n    val t = lazy s\n    t()\n.\n");
    assert!(ir.contains("c\"hello\\00\""), "string global:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure create:\n{ir}");
    assert!(ir.contains("thunk_call"), "thunk call:\n{ir}");
}

#[test]
fn emit_string_return_plus_thunk() {
    // Function returns string and also creates/invokes a thunk
    let ir = emit("fun f(x: Int) -> Str:\n    val t = lazy x\n    t()\n    \"done\"\n.\n");
    assert!(ir.contains("c\"done\\00\""), "return string global:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure create:\n{ir}");
}

#[test]
fn emit_string_and_closure_deterministic() {
    let src = "fun f() -> Str:\n    val s = \"hello\"\n    val t = lazy s\n    t()\n.\n";
    let ir1 = emit(src);
    let ir2 = emit(src);
    assert_eq!(ir1, ir2, "string + closure emission deterministic");
}

#[test]
fn emit_mixed_string_globals_and_env_coexist() {
    // Both string globals (for return value and py call name) and closure env should coexist
    let ir = emit(
        "from py builtins import print\n\nfun f(x: Int) -> Str !py:\n    print(\"msg\")\n    val t = lazy x\n    t()\n    \"result\"\n.\n",
    );
    assert!(ir.contains("c\"print\\00\""), "print name:\n{ir}");
    assert!(ir.contains("c\"result\\00\""), "result string:\n{ir}");
    assert!(ir.contains("@dx_rt_closure_create"), "closure create:\n{ir}");
    assert!(ir.contains("@dx_rt_py_call_function"), "py call:\n{ir}");
}
