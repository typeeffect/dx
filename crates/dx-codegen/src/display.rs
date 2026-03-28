use crate::low::{LowFunction, LowModule, LowRuntimeCallKind, LowStep, LowType};
use std::fmt::Write;

pub fn render_low_module(module: &LowModule) -> String {
    let mut out = String::new();

    if !module.externs.is_empty() {
        for ext in &module.externs {
            let params: Vec<&str> = ext.params.iter().map(low_type_str).collect();
            writeln!(
                out,
                "declare {} ({}) -> {}",
                ext.symbol,
                params.join(", "),
                low_type_str(&ext.ret)
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    for function in &module.functions {
        render_low_function(function, &mut out);
    }

    out
}

fn render_low_function(function: &LowFunction, out: &mut String) {
    let params: Vec<String> = function
        .params
        .iter()
        .map(|p| format!("_{}: {}", p.local, low_type_str(&p.ty)))
        .collect();

    writeln!(
        out,
        "define {} {}({}) {{",
        low_type_str(&function.ret),
        function.name,
        params.join(", ")
    )
    .unwrap();

    for block in &function.blocks {
        writeln!(out, "  {}:", block.label).unwrap();
        for step in &block.steps {
            write!(out, "    ").unwrap();
            render_low_step(step, out);
            writeln!(out).unwrap();
        }
        if block.steps.is_empty() {
            writeln!(out, "    (empty)").unwrap();
        }
    }

    writeln!(out, "}}").unwrap();
}

fn render_low_step(step: &LowStep, out: &mut String) {
    match step {
        LowStep::RuntimeCall {
            statement,
            destination,
            symbol,
            ret,
            kind,
        } => {
            if let Some(dest) = destination {
                write!(out, "_{dest} = ").unwrap();
            }
            write!(out, "call {symbol}").unwrap();
            match kind {
                LowRuntimeCallKind::PyCall { arg_count } => {
                    write!(out, " (args={arg_count})").unwrap();
                }
                LowRuntimeCallKind::ClosureCreate {
                    capture_count,
                    arity,
                } => {
                    write!(out, " (captures={capture_count}, arity={arity})").unwrap();
                }
                LowRuntimeCallKind::ClosureInvoke { arg_count, thunk } => {
                    if *thunk {
                        write!(out, " thunk()").unwrap();
                    } else {
                        write!(out, " (args={arg_count})").unwrap();
                    }
                }
            }
            if let Some(ret) = ret {
                write!(out, " -> {}", low_type_str(ret)).unwrap();
            }
            write!(out, "  ; stmt {statement}").unwrap();
        }
        LowStep::ThrowCheck {
            statement,
            symbol,
            boundary,
        } => {
            write!(
                out,
                "throw-check {symbol} [{}]  ; stmt {statement}",
                throw_boundary_str(boundary),
            )
            .unwrap();
        }
    }
}

fn low_type_str(ty: &LowType) -> &'static str {
    match ty {
        LowType::I64 => "i64",
        LowType::F64 => "f64",
        LowType::I1 => "i1",
        LowType::Ptr => "ptr",
        LowType::Void => "void",
    }
}

fn throw_boundary_str(kind: &dx_runtime::ThrowBoundaryKind) -> &'static str {
    match kind {
        dx_runtime::ThrowBoundaryKind::PythonFunction => "py-function",
        dx_runtime::ThrowBoundaryKind::PythonMethod => "py-method",
        dx_runtime::ThrowBoundaryKind::PythonDynamic => "py-dynamic",
        dx_runtime::ThrowBoundaryKind::ClosureCall => "closure-call",
        dx_runtime::ThrowBoundaryKind::ThunkCall => "thunk-call",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_parser::{Lexer, Parser};

    fn lower(src: &str) -> LowModule {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        let mir = dx_mir::lower_module(&typed.module);
        lower_module(&mir)
    }

    fn render(src: &str) -> String {
        render_low_module(&lower(src))
    }

    // ── externs ──────────────────────────────────────────────────

    #[test]
    fn snapshot_externs_python() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("declare dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("(ptr, i64) -> ptr"), "got:\n{out}");
    }

    #[test]
    fn snapshot_externs_closure() {
        let out = render("fun f(x: Int) -> lazy Int:\n    val t = lazy x\n    t()\n.\n");
        assert!(out.contains("declare dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("declare dx_rt_thunk_call"), "got:\n{out}");
    }

    #[test]
    fn snapshot_externs_sorted() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
        );
        let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("declare ")).collect();
        let mut sorted = lines.clone();
        sorted.sort();
        assert_eq!(lines, sorted, "externs should be sorted by symbol");
    }

    // ── runtime call steps ───────────────────────────────────────

    #[test]
    fn snapshot_python_call_step() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("call dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("(args=1)"), "got:\n{out}");
        assert!(out.contains("-> ptr"), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure_create_step() {
        let out = render("fun f(x: Int) -> lazy Int:\n    lazy x\n.\n");
        assert!(out.contains("call dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("(captures=1, arity=0)"), "got:\n{out}");
    }

    #[test]
    fn snapshot_thunk_invoke_step() {
        let out = render("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
        assert!(out.contains("call dx_rt_thunk_call"), "got:\n{out}");
        assert!(out.contains("thunk()"), "got:\n{out}");
    }

    // ── throw-check steps ────────────────────────────────────────

    #[test]
    fn snapshot_throw_check_after_python_call() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("throw-check dx_rt_throw_check_pending [py-function]"), "got:\n{out}");
        // throw-check should come after the runtime call in the output
        let call_pos = out.find("call dx_rt_py_call_function").expect("call present");
        let check_pos = out.find("throw-check").expect("throw check present");
        assert!(check_pos > call_pos, "throw-check should follow runtime call");
    }

    #[test]
    fn snapshot_throw_check_python_method() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)'head()\n.\n",
        );
        assert!(out.contains("[py-method]"), "got:\n{out}");
    }

    // ── function signatures ──────────────────────────────────────

    #[test]
    fn snapshot_function_signature_types() {
        let out = render("fun f(x: Int, y: Bool) -> Int:\n    x\n.\n");
        assert!(out.contains("define i64 f(_0: i64, _1: i1)"), "got:\n{out}");
    }

    #[test]
    fn snapshot_void_return() {
        let out = render("fun f():\n    42\n.\n");
        assert!(out.contains("define void f()"), "got:\n{out}");
    }

    // ── mixed scenarios ──────────────────────────────────────────

    #[test]
    fn snapshot_mixed_python_and_closure() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
        );
        assert!(out.contains("call dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("call dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("call dx_rt_thunk_call"), "got:\n{out}");
    }

    #[test]
    fn snapshot_multiple_functions() {
        let out = render(
            "fun a(x: Int) -> Int:\n    x\n.\n\nfun b(y: Str) -> Str:\n    y\n.\n",
        );
        assert!(out.contains("define i64 a("), "got:\n{out}");
        assert!(out.contains("define ptr b("), "got:\n{out}");
    }

    #[test]
    fn snapshot_empty_function_no_externs() {
        let out = render("fun f() -> Int:\n    1\n.\n");
        // No externs needed
        assert!(!out.contains("declare"), "got:\n{out}");
        assert!(out.contains("define i64 f()"), "got:\n{out}");
    }

    // ── block structure ──────────────────────────────────────────

    #[test]
    fn snapshot_block_labels() {
        let out = render(
            "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        );
        assert!(out.contains("bb0:"), "got:\n{out}");
        assert!(out.contains("bb1:"), "got:\n{out}");
    }
}
