use crate::abi::{AbiType, PyRuntimePlan};
use crate::closure::{ClosureAbiType, ClosureRuntimePlan};
use crate::externs::{RuntimeExternAbiType, RuntimeExternPlan};
use crate::lower::{LoweredPyCall, PyDispatchTarget};
use crate::ops::{RuntimeOp, RuntimeOpKind, RuntimeOpsPlan};
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

pub fn render_combined_plan(
    py_plan: &PyRuntimePlan,
    closure_plan: &ClosureRuntimePlan,
) -> String {
    let mut out = render_runtime_plan(py_plan);
    out.push_str(&render_closure_plan(closure_plan));
    out
}

pub fn render_runtime_ops_plan(plan: &RuntimeOpsPlan) -> String {
    let mut out = String::new();

    writeln!(out, "=== Runtime Ops Plan ===").unwrap();
    writeln!(out).unwrap();

    if !plan.required_hooks.is_empty() {
        writeln!(out, "required hooks:").unwrap();
        for hook in &plan.required_hooks {
            writeln!(out, "  {}", hook.symbol()).unwrap();
        }
        writeln!(out).unwrap();
    }

    if !plan.ops.is_empty() {
        writeln!(out, "ops:").unwrap();
        for op in &plan.ops {
            writeln!(out, "  {}", render_runtime_op(op)).unwrap();
        }
    }

    out
}

pub fn render_runtime_extern_plan(plan: &RuntimeExternPlan) -> String {
    let mut out = String::new();

    writeln!(out, "=== Runtime Externs ===").unwrap();
    writeln!(out).unwrap();

    if plan.externs.is_empty() {
        writeln!(out, "(none)").unwrap();
    } else {
        for ext in &plan.externs {
            let params: Vec<&str> = ext
                .signature
                .params
                .iter()
                .map(render_extern_abi_type)
                .collect();
            writeln!(
                out,
                "  extern {} ({}) -> {}",
                ext.signature.symbol,
                params.join(", "),
                render_extern_abi_type(&ext.signature.ret)
            )
            .unwrap();
        }
    }

    out
}

fn render_extern_abi_type(ty: &RuntimeExternAbiType) -> &'static str {
    match ty {
        RuntimeExternAbiType::PyObjHandle => "PyObjHandle",
        RuntimeExternAbiType::Utf8Ptr => "Utf8Ptr",
        RuntimeExternAbiType::ClosureHandle => "ClosureHandle",
        RuntimeExternAbiType::EnvHandle => "EnvHandle",
        RuntimeExternAbiType::U32 => "U32",
    }
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

fn render_runtime_op(op: &RuntimeOp) -> String {
    let mut out = String::new();
    write!(out, "{}/bb{}[{}]", op.function, op.block, op.statement).unwrap();
    if let Some(dest) = op.destination {
        write!(out, " -> _{dest}").unwrap();
    }
    write!(out, "  {}", op.runtime_symbol).unwrap();

    match &op.kind {
        RuntimeOpKind::PyCall { dispatch, arg_count } => {
            write!(out, " {}", render_dispatch(dispatch)).unwrap();
            write!(out, "(args={arg_count})").unwrap();
        }
        RuntimeOpKind::ClosureCreate {
            captures,
            param_types,
        } => {
            let params: Vec<String> = param_types.iter().map(render_type).collect();
            write!(out, " closure({})", params.join(", ")).unwrap();
            if !captures.is_empty() {
                write!(out, " captures [").unwrap();
                for (i, cap) in captures.iter().enumerate() {
                    if i > 0 {
                        write!(out, ", ").unwrap();
                    }
                    write!(out, "{}: {} <= _{}", cap.name, render_type(&cap.ty), cap.source)
                        .unwrap();
                }
                write!(out, "]").unwrap();
            }
        }
        RuntimeOpKind::ClosureInvoke {
            closure_local,
            arg_count,
            thunk,
        } => {
            if *thunk {
                write!(out, " thunk(_{closure_local})").unwrap();
            } else {
                write!(out, " closure(_{closure_local}, args={arg_count})").unwrap();
            }
        }
    }

    if let Some(result_type) = &op.result_type {
        write!(out, " -> {}", render_type(result_type)).unwrap();
    }
    if !op.effects.is_empty() {
        for effect in &op.effects {
            write!(out, " !{effect}").unwrap();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::{build_python_runtime_plan, RuntimeHook};
    use crate::lower::lower_python_runtime_calls;
    use crate::ops::{build_runtime_ops_plan, RuntimeHookKind};
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

    // ── combined closure + python scenarios ──────────────────────

    #[test]
    fn combined_closure_capturing_python_result() {
        // Closure captures a value obtained from a Python call
        let module = lower(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py:\n    val df = read_csv(path)\n    lazy df\n.\n",
        );
        let py_plan = build_python_runtime_plan(&module);
        let cl_plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_combined_plan(&py_plan, &cl_plan);

        // Python side
        assert!(out.contains("py.call(read_csv)"), "got:\n{out}");
        assert!(out.contains("dx_rt_py_call_function"), "got:\n{out}");
        // Closure side
        assert!(out.contains("dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("captures [df:"), "got:\n{out}");
    }

    #[test]
    fn combined_thunk_wrapping_python_call() {
        // lazy read_csv(path) — thunk that captures path, calling Python when invoked
        let module = lower(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py:\n    lazy read_csv(path)\n.\n",
        );
        let py_plan = build_python_runtime_plan(&module);
        let cl_plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_combined_plan(&py_plan, &cl_plan);

        assert!(out.contains("Python Runtime Plan"), "got:\n{out}");
        assert!(out.contains("Closure Runtime Plan"), "got:\n{out}");
        // The closure should have py effects
        assert!(out.contains("closure() -> PyObj !py"), "got:\n{out}");
    }

    #[test]
    fn combined_multiple_closures_in_one_function() {
        let module = lower(
            "fun make(x: Int, y: Int) -> lazy Int:\n    val a = lazy x\n    val b = lazy y\n    a\n.\n",
        );
        let cl_plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&cl_plan);

        // Should have 2 closure creations
        assert!(out.contains("creations:"), "got:\n{out}");
        // Count occurrences of "closure()" — expect at least 2
        let closure_count = out.matches("closure() -> Int").count();
        assert!(closure_count >= 2, "expected >=2 closure() entries, got {closure_count}:\n{out}");
    }

    #[test]
    fn combined_multiple_hooks_in_module() {
        // Module with Python calls and closures together
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n\nfun make(x: Int) -> lazy Int:\n    lazy x\n.\n",
        );
        let py_plan = build_python_runtime_plan(&module);
        let cl_plan = crate::closure::build_closure_runtime_plan(&module);

        // Python side has hooks
        assert!(!py_plan.required_hooks.is_empty());
        assert!(!py_plan.call_sites.is_empty());
        // Closure side has hooks
        assert!(!cl_plan.required_hooks.is_empty());
        assert!(!cl_plan.creations.is_empty());

        let out = render_combined_plan(&py_plan, &cl_plan);
        assert!(out.contains("Python Runtime Plan"), "got:\n{out}");
        assert!(out.contains("Closure Runtime Plan"), "got:\n{out}");
    }

    #[test]
    fn combined_thunk_creation_and_invocation_with_python() {
        // lazy read_csv(path) — the py call is inside the closure body,
        // which MIR doesn't preserve, so py_plan sees no call sites here.
        // But the closure plan shows creation + invocation with !py effects.
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val thunk = lazy read_csv(path)\n    thunk()\n.\n",
        );
        let cl_plan = crate::closure::build_closure_runtime_plan(&module);
        let out = render_closure_plan(&cl_plan);

        assert!(out.contains("dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("dx_rt_thunk_call"), "got:\n{out}");
        assert!(out.contains("!py"), "got:\n{out}");
        assert!(out.contains("captures [path: Str <="), "got:\n{out}");
    }

    #[test]
    fn snapshot_runtime_ops_plan_combines_python_and_closure_ops() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val thunk = lazy read_csv(path)\n    thunk()\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        assert!(out.contains("=== Runtime Ops Plan ==="), "got:\n{out}");
        assert!(out.contains("dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("dx_rt_thunk_call"), "got:\n{out}");
    }

    #[test]
    fn snapshot_runtime_ops_plan_includes_python_dispatch() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        assert!(out.contains("dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("pandas.read_csv(args=1)"), "got:\n{out}");
        assert!(out.contains("-> PyObj !py"), "got:\n{out}");
    }

    // ── targeted RuntimeOpsPlan scenarios ─────────────────────────

    #[test]
    fn ops_plan_python_only() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        // Only python hooks
        assert!(plan.required_hooks.iter().all(|h| matches!(h, RuntimeHookKind::Py(_))));
        // Rendered shows destination, symbol, dispatch, result type, effects
        assert!(out.contains("-> _"), "destination local missing:\n{out}");
        assert!(out.contains("dx_rt_py_call_function"), "symbol missing:\n{out}");
        assert!(out.contains("pandas.read_csv"), "dispatch missing:\n{out}");
        assert!(out.contains("-> PyObj"), "result type missing:\n{out}");
        assert!(out.contains("!py"), "effects missing:\n{out}");
    }

    #[test]
    fn ops_plan_closure_only() {
        let module = lower(
            "fun make(x: Int) -> lazy Int:\n    val f = lazy x\n    f()\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        // Only closure hooks
        assert!(plan.required_hooks.iter().all(|h| matches!(h, RuntimeHookKind::Closure(_))));
        assert!(out.contains("dx_rt_closure_create"), "create missing:\n{out}");
        assert!(out.contains("dx_rt_thunk_call"), "thunk call missing:\n{out}");
        assert!(out.contains("closure()"), "closure shape missing:\n{out}");
        assert!(out.contains("thunk("), "thunk shape missing:\n{out}");
    }

    #[test]
    fn ops_plan_python_and_closure_combined() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val thunk = lazy df\n    thunk()\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        // Has both python and closure hooks
        assert!(plan.required_hooks.iter().any(|h| matches!(h, RuntimeHookKind::Py(_))));
        assert!(plan.required_hooks.iter().any(|h| matches!(h, RuntimeHookKind::Closure(_))));
        assert!(out.contains("dx_rt_py_call_function"), "py hook missing:\n{out}");
        assert!(out.contains("dx_rt_closure_create"), "create missing:\n{out}");
        assert!(out.contains("dx_rt_thunk_call"), "thunk missing:\n{out}");
    }

    #[test]
    fn ops_plan_multi_function_stable_ordering() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n\nfun make(x: Int) -> lazy Int:\n    lazy x\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);

        // Ops sorted by function name, then block, then statement
        let names: Vec<&str> = plan.ops.iter().map(|op| op.function.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "ops should be sorted by function name");

        // Hooks are deduplicated
        let hook_count = plan.required_hooks.len();
        let deduped: std::collections::BTreeSet<_> = plan.required_hooks.iter().collect();
        assert_eq!(hook_count, deduped.len(), "hooks should be deduplicated");
    }

    #[test]
    fn ops_plan_thunk_with_py_throw_effects() {
        let module = lower(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py !throw:\n    lazy read_csv(path)\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        // Closure carries !py (from the closure plan effects)
        assert!(out.contains("!py"), "py effect missing:\n{out}");
        assert!(out.contains("closure()"), "closure shape missing:\n{out}");
    }

    #[test]
    fn ops_plan_closure_capturing_pyobj() {
        let module = lower(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py:\n    val df = read_csv(path)\n    lazy df\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);
        let out = render_runtime_ops_plan(&plan);

        // Py call + closure create with capture
        assert!(out.contains("dx_rt_py_call_function"), "py call missing:\n{out}");
        assert!(out.contains("dx_rt_closure_create"), "create missing:\n{out}");
        assert!(out.contains("captures ["), "captures missing:\n{out}");
    }

    // ── RuntimeExternPlan rendering ──────────────────────────────

    #[test]
    fn extern_plan_python_only() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);
        let out = render_runtime_extern_plan(&plan);

        assert!(out.contains("=== Runtime Externs ==="), "got:\n{out}");
        assert!(out.contains("extern dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("(Utf8Ptr, U32) -> PyObjHandle"), "got:\n{out}");
        // No closure externs
        assert!(!out.contains("dx_rt_closure"), "got:\n{out}");
    }

    #[test]
    fn extern_plan_closure_only() {
        let module = lower(
            "fun make(x: Int) -> lazy Int:\n    val f = lazy x\n    f()\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);
        let out = render_runtime_extern_plan(&plan);

        assert!(out.contains("extern dx_rt_closure_create"), "got:\n{out}");
        assert!(out.contains("extern dx_rt_thunk_call"), "got:\n{out}");
        // No python externs
        assert!(!out.contains("dx_rt_py_call"), "got:\n{out}");
    }

    #[test]
    fn extern_plan_mixed_python_and_closure() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val thunk = lazy df\n    thunk()\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);
        let out = render_runtime_extern_plan(&plan);

        assert!(out.contains("dx_rt_py_call_function"), "py extern missing:\n{out}");
        assert!(out.contains("dx_rt_closure_create"), "closure create missing:\n{out}");
        assert!(out.contains("dx_rt_thunk_call"), "thunk call missing:\n{out}");
    }

    #[test]
    fn extern_plan_sorted_by_symbol() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val thunk = lazy df\n    thunk()\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);

        let symbols: Vec<&str> = plan.externs.iter().map(|e| e.signature.symbol).collect();
        let mut sorted = symbols.clone();
        sorted.sort();
        assert_eq!(symbols, sorted, "externs should be sorted by symbol");
    }

    #[test]
    fn extern_plan_abi_types_are_readable() {
        let module = lower(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);
        let out = render_runtime_extern_plan(&plan);

        // PyCallFunction: (Utf8Ptr, U32) -> PyObjHandle
        assert!(out.contains("(Utf8Ptr, U32) -> PyObjHandle"), "py func abi:\n{out}");
        // PyCallMethod: (PyObjHandle, Utf8Ptr, U32) -> PyObjHandle
        assert!(
            out.contains("(PyObjHandle, Utf8Ptr, U32) -> PyObjHandle"),
            "py method abi:\n{out}"
        );
    }

    #[test]
    fn extern_plan_consistent_with_ops_hooks() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val thunk = lazy df\n    thunk()\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);

        // Every hook in ops plan should have a corresponding extern
        let extern_hooks: std::collections::BTreeSet<_> =
            plan.externs.iter().map(|e| e.hook).collect();
        for hook in &ops.required_hooks {
            assert!(
                extern_hooks.contains(hook),
                "ops hook {:?} missing from externs",
                hook
            );
        }
        // And vice versa
        assert_eq!(ops.required_hooks.len(), plan.externs.len());
    }

    #[test]
    fn extern_plan_empty_module() {
        let module = lower("fun f() -> Int:\n    1\n.\n");
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);
        let out = render_runtime_extern_plan(&plan);

        assert!(out.contains("(none)"), "got:\n{out}");
        assert!(plan.externs.is_empty());
    }

    #[test]
    fn extern_plan_with_py_throw_effects() {
        // Module with !py !throw effects — externs should still be present
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        let ops = build_runtime_ops_plan(&module);
        let plan = crate::externs::build_runtime_extern_plan(&ops);
        let out = render_runtime_extern_plan(&plan);

        assert!(out.contains("extern dx_rt_py_call_function"), "got:\n{out}");
        // Effects don't change the extern signature
        assert!(out.contains("(Utf8Ptr, U32) -> PyObjHandle"), "got:\n{out}");
    }
}
