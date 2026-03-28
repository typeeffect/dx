use crate::abi::{AbiType, PyRuntimePlan};
use crate::closure::{ClosureAbiType, ClosureRuntimePlan};
use crate::lower::{LoweredPyCall, PyDispatchTarget};
use crate::py::PyCallKind;
use dx_mir::display::render_type;
use std::fmt::Write;

pub fn render_runtime_plan(plan: &PyRuntimePlan) -> String {
    let mut out = String::new();

    writeln!(out, "=== Python Runtime Plan ===").unwrap();
    writeln!(out).unwrap();

    if !plan.imports.is_empty() {
        writeln!(out, "imports:").unwrap();
        for import in &plan.imports {
            writeln!(out, "  {} from {}", import.name, import.module).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !plan.required_hooks.is_empty() {
        writeln!(out, "required hooks:").unwrap();
        for hook in &plan.required_hooks {
            let sig = hook.signature();
            writeln!(out, "  {} -> {}", sig.symbol, render_abi_type(sig.ret)).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !plan.call_sites.is_empty() {
        writeln!(out, "call sites:").unwrap();
        for site in &plan.call_sites {
            write!(out, "  {}  bb{}[{}]  ", render_call_kind(&site.kind), site.block, site.statement).unwrap();
            write!(out, "args={}", site.arg_count).unwrap();
            if !site.effects.is_empty() {
                for effect in &site.effects {
                    write!(out, " !{effect}").unwrap();
                }
            }
            writeln!(out).unwrap();
        }
    }

    out
}

pub fn render_lowered_calls(calls: &[LoweredPyCall]) -> String {
    let mut out = String::new();

    for call in calls {
        write!(out, "{}/bb{}[{}]: ", call.function, call.block, call.statement).unwrap();
        write!(out, "{} via {} ", render_dispatch(&call.dispatch), call.runtime_symbol).unwrap();
        write!(out, "(args={})", call.arg_count).unwrap();
        if !call.effects.is_empty() {
            for effect in &call.effects {
                write!(out, " !{effect}").unwrap();
            }
        }
        writeln!(out).unwrap();
    }

    out
}

fn render_call_kind(kind: &PyCallKind) -> String {
    match kind {
        PyCallKind::Function { name } => format!("py.call({name})"),
        PyCallKind::Member { name } => format!("py.method({name})"),
        PyCallKind::Dynamic => "py.dynamic".to_string(),
    }
}

fn render_dispatch(dispatch: &PyDispatchTarget) -> String {
    match dispatch {
        PyDispatchTarget::Function { module: Some(m), name } => format!("{m}.{name}"),
        PyDispatchTarget::Function { module: None, name } => format!("?.{name}"),
        PyDispatchTarget::Method { name } => format!(".{name}"),
        PyDispatchTarget::Dynamic => "dynamic".to_string(),
    }
}

pub fn render_closure_plan(plan: &ClosureRuntimePlan) -> String {
    let mut out = String::new();

    writeln!(out, "=== Closure Runtime Plan ===").unwrap();
    writeln!(out).unwrap();

    if !plan.required_hooks.is_empty() {
        writeln!(out, "required hooks:").unwrap();
        for hook in &plan.required_hooks {
            let sig = hook.signature();
            writeln!(
                out,
                "  {} -> {}",
                sig.symbol,
                render_closure_abi_type(sig.ret)
            )
            .unwrap();
        }
        writeln!(out).unwrap();
    }

    if !plan.creations.is_empty() {
        writeln!(out, "creations:").unwrap();
        for creation in &plan.creations {
            write!(
                out,
                "  {}/bb{}[{}] -> _{}  ",
                creation.function, creation.block, creation.statement, creation.destination
            )
            .unwrap();
            let params: Vec<String> = creation.param_types.iter().map(render_type).collect();
            write!(out, "closure({}) -> {}", params.join(", "), render_type(&creation.return_type))
                .unwrap();
            for effect in &creation.effects {
                write!(out, " !{effect}").unwrap();
            }
            if !creation.captures.is_empty() {
                write!(out, "  captures [").unwrap();
                for (i, cap) in creation.captures.iter().enumerate() {
                    if i > 0 {
                        write!(out, ", ").unwrap();
                    }
                    write!(out, "{}: {} <= _{}", cap.name, render_type(&cap.ty), cap.source)
                        .unwrap();
                }
                write!(out, "]").unwrap();
            }
            writeln!(out).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !plan.invocations.is_empty() {
        writeln!(out, "invocations:").unwrap();
        for inv in &plan.invocations {
            write!(
                out,
                "  {}/bb{}[{}] -> _{}  ",
                inv.function, inv.block, inv.statement, inv.destination
            )
            .unwrap();
            write!(out, "{} _{}(args={})", inv.runtime_symbol, inv.closure_local, inv.arg_count)
                .unwrap();
            if !inv.effects.is_empty() {
                for effect in &inv.effects {
                    write!(out, " !{effect}").unwrap();
                }
            }
            writeln!(out).unwrap();
        }
    }

    out
}

fn render_closure_abi_type(ty: ClosureAbiType) -> &'static str {
    match ty {
        ClosureAbiType::ClosureHandle => "ClosureHandle",
        ClosureAbiType::EnvHandle => "EnvHandle",
        ClosureAbiType::U32 => "U32",
    }
}

fn render_abi_type(ty: AbiType) -> &'static str {
    match ty {
        AbiType::PyObjHandle => "PyObjHandle",
        AbiType::Utf8Ptr => "Utf8Ptr",
        AbiType::U32 => "U32",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{build_python_runtime_plan, RuntimeHook};
    use crate::lower::lower_python_runtime_calls;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_mir::{lower_module as lower_mir, mir};
    use dx_parser::{Lexer, Parser};

    fn lower(src: &str) -> mir::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        lower_mir(&typed.module)
    }

    #[test]
    fn snapshot_plan_function_call() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let plan = build_python_runtime_plan(&module);
        let out = render_runtime_plan(&plan);
        assert!(out.contains("read_csv from pandas"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("py.call(read_csv)"), "got:\n{out}");
    }

    #[test]
    fn snapshot_plan_member_call() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let plan = build_python_runtime_plan(&module);
        let out = render_runtime_plan(&plan);
        assert!(out.contains("py.method(head)"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_method"), "got:\n{out}");
    }

    #[test]
    fn snapshot_plan_dynamic_call() {
        let module = lower(
            "from py pandas import read_csv\n\nfun invoke(path: Str) -> PyObj !py:\n    val f = read_csv(path)\n    f()\n.\n",
        );
        let plan = build_python_runtime_plan(&module);
        let out = render_runtime_plan(&plan);
        assert!(out.contains("py.dynamic"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_dynamic"), "got:\n{out}");
    }

    #[test]
    fn snapshot_plan_multiple_imports() {
        let module = lower(
            "from py pandas import read_csv, DataFrame\nfrom py numpy import array\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let plan = build_python_runtime_plan(&module);
        let out = render_runtime_plan(&plan);
        assert!(out.contains("read_csv from pandas"), "got:\n{out}");
        assert!(out.contains("DataFrame from pandas"), "got:\n{out}");
        assert!(out.contains("array from numpy"), "got:\n{out}");
    }

    #[test]
    fn snapshot_lowered_calls_function() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let calls = lower_python_runtime_calls(&module);
        let out = render_lowered_calls(&calls);
        assert!(out.contains("pandas.read_csv"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("!py"), "got:\n{out}");
    }

    #[test]
    fn snapshot_lowered_calls_member() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let calls = lower_python_runtime_calls(&module);
        let out = render_lowered_calls(&calls);
        assert!(out.contains(".head"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_method"), "got:\n{out}");
    }

    #[test]
    fn snapshot_lowered_calls_dynamic() {
        let module = lower(
            "from py pandas import read_csv\n\nfun invoke(path: Str) -> PyObj !py:\n    val f = read_csv(path)\n    f()\n.\n",
        );
        let calls = lower_python_runtime_calls(&module);
        let out = render_lowered_calls(&calls);
        assert!(out.contains("dynamic"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_dynamic"), "got:\n{out}");
    }

    #[test]
    fn hook_dedup_in_plan() {
        let module = lower(
            "from py pandas import read_csv\nfrom py numpy import array\n\nfun f(a: Str, b: Str) -> PyObj !py:\n    val x = read_csv(a)\n    array(b)\n.\n",
        );
        let plan = build_python_runtime_plan(&module);
        // Two py function calls should produce only one PyCallFunction hook
        assert_eq!(
            plan.required_hooks.iter().filter(|h| **h == RuntimeHook::PyCallFunction).count(),
            1,
            "hook should be deduplicated"
        );
        assert_eq!(plan.call_sites.len(), 2);
    }

    #[test]
    fn empty_module_produces_empty_plan() {
        let module = lower("fun f() -> Int:\n    1\n.\n");
        let plan = build_python_runtime_plan(&module);
        assert!(plan.imports.is_empty());
        assert!(plan.required_hooks.is_empty());
        assert!(plan.call_sites.is_empty());
        let out = render_runtime_plan(&plan);
        assert!(out.contains("Python Runtime Plan"), "got:\n{out}");
    }

    // ── closure plan rendering ───────────────────────────────────

    #[test]
    fn snapshot_closure_plan_thunk() {
        let module = lower("fun make(x: Int) -> lazy Int:\n    lazy x\n.\n");
        let plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&plan);
        assert!(out.contains("Closure Runtime Plan"), "got:\n{out}");
        assert!(out.contains("dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("closure() -> Int"), "got:\n{out}");
        assert!(out.contains("captures [x: Int <="), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure_plan_lambda_with_capture() {
        let module = lower(
            "fun make(x: Int) -> (Int) -> Int:\n    (y: Int) => x + y\n.\n",
        );
        let plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&plan);
        assert!(out.contains("closure(Int) -> Int"), "got:\n{out}");
        assert!(out.contains("captures [x: Int <="), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure_plan_thunk_invocation() {
        let module = lower(
            "fun use_it(x: Int) -> Int:\n    val f = lazy x\n    f()\n.\n",
        );
        let plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&plan);
        assert!(out.contains("creations:"), "got:\n{out}");
        assert!(out.contains("invocations:"), "got:\n{out}");
        assert!(out.contains("dx_rt_thunk_call"), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure_plan_pyobj_capture() {
        let module = lower(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py:\n    lazy read_csv(path)\n.\n",
        );
        let plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&plan);
        assert!(out.contains("closure() -> PyObj !py"), "got:\n{out}");
        assert!(out.contains("captures [path: Str <="), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure_plan_empty_module() {
        let module = lower("fun f() -> Int:\n    1\n.\n");
        let plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&plan);
        assert!(out.contains("Closure Runtime Plan"), "got:\n{out}");
        assert!(!out.contains("creations:"), "got:\n{out}");
        assert!(!out.contains("invocations:"), "got:\n{out}");
    }
}
