use crate::abi::AbiType;
use crate::closure::ClosureAbiType;
use crate::ops::{build_runtime_ops_plan, RuntimeHookKind, RuntimeOpsPlan};
use dx_mir::mir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeExternAbiType {
    PyObjHandle,
    Utf8Ptr,
    ClosureHandle,
    EnvHandle,
    U32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExternSignature {
    pub symbol: &'static str,
    pub params: Vec<RuntimeExternAbiType>,
    pub ret: RuntimeExternAbiType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExtern {
    pub hook: RuntimeHookKind,
    pub signature: RuntimeExternSignature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExternPlan {
    pub externs: Vec<RuntimeExtern>,
}

pub fn build_runtime_extern_plan(plan: &RuntimeOpsPlan) -> RuntimeExternPlan {
    let mut externs: Vec<_> = plan
        .required_hooks
        .iter()
        .copied()
        .map(runtime_extern_for_hook)
        .collect();
    externs.sort_by(|a, b| a.signature.symbol.cmp(b.signature.symbol));
    RuntimeExternPlan { externs }
}

pub fn build_runtime_extern_plan_from_module(module: &mir::Module) -> RuntimeExternPlan {
    let ops = build_runtime_ops_plan(module);
    build_runtime_extern_plan(&ops)
}

fn runtime_extern_for_hook(hook: RuntimeHookKind) -> RuntimeExtern {
    let (params, ret, symbol) = match hook {
        RuntimeHookKind::Py(py_hook) => {
            let sig = py_hook.signature();
            (
                sig.params.iter().copied().map(from_py_abi).collect(),
                from_py_abi(sig.ret),
                sig.symbol,
            )
        }
        RuntimeHookKind::Closure(closure_hook) => {
            let sig = closure_hook.signature();
            (
                sig.params.iter().copied().map(from_closure_abi).collect(),
                from_closure_abi(sig.ret),
                sig.symbol,
            )
        }
    };

    RuntimeExtern {
        hook,
        signature: RuntimeExternSignature {
            symbol,
            params,
            ret,
        },
    }
}

fn from_py_abi(ty: AbiType) -> RuntimeExternAbiType {
    match ty {
        AbiType::PyObjHandle => RuntimeExternAbiType::PyObjHandle,
        AbiType::Utf8Ptr => RuntimeExternAbiType::Utf8Ptr,
        AbiType::U32 => RuntimeExternAbiType::U32,
    }
}

fn from_closure_abi(ty: ClosureAbiType) -> RuntimeExternAbiType {
    match ty {
        ClosureAbiType::ClosureHandle => RuntimeExternAbiType::ClosureHandle,
        ClosureAbiType::EnvHandle => RuntimeExternAbiType::EnvHandle,
        ClosureAbiType::U32 => RuntimeExternAbiType::U32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::RuntimeHook;
    use crate::closure::ClosureRuntimeHook;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_mir::lower_module as lower_mir;
    use dx_parser::{Lexer, Parser};

    fn lower(src: &str) -> mir::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        lower_mir(&typed.module)
    }

    #[test]
    fn builds_py_and_closure_externs_from_runtime_ops() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str, x: Int) -> PyObj !py:\n    val df = read_csv(path)\n    val thunk = lazy df\n    thunk()\n.\n",
        );
        let plan = build_runtime_extern_plan_from_module(&module);

        let hooks: Vec<_> = plan.externs.iter().map(|it| it.hook).collect();
        assert!(hooks.contains(&RuntimeHookKind::Py(RuntimeHook::PyCallFunction)));
        assert!(hooks.contains(&RuntimeHookKind::Closure(ClosureRuntimeHook::Create)));
        assert!(hooks.contains(&RuntimeHookKind::Closure(ClosureRuntimeHook::ThunkCall)));
    }

    #[test]
    fn py_function_hook_signature_is_lowered_to_unified_abi() {
        let plan = build_runtime_extern_plan(&RuntimeOpsPlan {
            required_hooks: vec![RuntimeHookKind::Py(RuntimeHook::PyCallFunction)],
            ops: vec![],
        });

        assert_eq!(
            plan.externs,
            vec![RuntimeExtern {
                hook: RuntimeHookKind::Py(RuntimeHook::PyCallFunction),
                signature: RuntimeExternSignature {
                    symbol: "dx_rt_py_call_function",
                    params: vec![RuntimeExternAbiType::Utf8Ptr, RuntimeExternAbiType::U32],
                    ret: RuntimeExternAbiType::PyObjHandle,
                },
            }]
        );
    }

    #[test]
    fn closure_hook_signature_is_lowered_to_unified_abi() {
        let plan = build_runtime_extern_plan(&RuntimeOpsPlan {
            required_hooks: vec![RuntimeHookKind::Closure(ClosureRuntimeHook::Call)],
            ops: vec![],
        });

        assert_eq!(
            plan.externs,
            vec![RuntimeExtern {
                hook: RuntimeHookKind::Closure(ClosureRuntimeHook::Call),
                signature: RuntimeExternSignature {
                    symbol: "dx_rt_closure_call",
                    params: vec![
                        RuntimeExternAbiType::ClosureHandle,
                        RuntimeExternAbiType::U32,
                    ],
                    ret: RuntimeExternAbiType::ClosureHandle,
                },
            }]
        );
    }

    #[test]
    fn externs_are_sorted_stably_by_symbol() {
        let plan = build_runtime_extern_plan(&RuntimeOpsPlan {
            required_hooks: vec![
                RuntimeHookKind::Closure(ClosureRuntimeHook::ThunkCall),
                RuntimeHookKind::Py(RuntimeHook::PyCallMethod),
                RuntimeHookKind::Closure(ClosureRuntimeHook::Create),
            ],
            ops: vec![],
        });

        let symbols: Vec<_> = plan.externs.iter().map(|it| it.signature.symbol).collect();
        assert_eq!(
            symbols,
            vec![
                "dx_rt_closure_create",
                "dx_rt_py_call_method",
                "dx_rt_thunk_call",
            ]
        );
    }
}
