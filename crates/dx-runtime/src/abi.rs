use crate::py::{collect_python_call_sites, PyCallKind, PyRuntimeCallSite};
use dx_mir::mir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiType {
    PyObjHandle,
    Utf8Ptr,
    U32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeHook {
    PyCallFunction,
    PyCallMethod,
    PyCallDynamic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeHookSignature {
    pub symbol: &'static str,
    pub params: &'static [AbiType],
    pub ret: AbiType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PyImportBinding {
    pub module: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyRuntimePlan {
    pub imports: Vec<PyImportBinding>,
    pub required_hooks: Vec<RuntimeHook>,
    pub call_sites: Vec<PyRuntimeCallSite>,
}

const PY_CALL_FUNCTION_PARAMS: &[AbiType] = &[AbiType::Utf8Ptr, AbiType::U32];
const PY_CALL_METHOD_PARAMS: &[AbiType] =
    &[AbiType::PyObjHandle, AbiType::Utf8Ptr, AbiType::U32];
const PY_CALL_DYNAMIC_PARAMS: &[AbiType] = &[AbiType::PyObjHandle, AbiType::U32];

impl RuntimeHook {
    pub fn symbol(self) -> &'static str {
        self.signature().symbol
    }

    pub fn signature(self) -> RuntimeHookSignature {
        match self {
            RuntimeHook::PyCallFunction => RuntimeHookSignature {
                symbol: "dx_rt_py_call_function",
                params: PY_CALL_FUNCTION_PARAMS,
                ret: AbiType::PyObjHandle,
            },
            RuntimeHook::PyCallMethod => RuntimeHookSignature {
                symbol: "dx_rt_py_call_method",
                params: PY_CALL_METHOD_PARAMS,
                ret: AbiType::PyObjHandle,
            },
            RuntimeHook::PyCallDynamic => RuntimeHookSignature {
                symbol: "dx_rt_py_call_dynamic",
                params: PY_CALL_DYNAMIC_PARAMS,
                ret: AbiType::PyObjHandle,
            },
        }
    }
}

pub fn build_python_runtime_plan(module: &mir::Module) -> PyRuntimePlan {
    let call_sites = collect_python_call_sites(module);
    let mut required_hooks = Vec::new();

    for site in &call_sites {
        let hook = match site.kind {
            PyCallKind::Function { .. } => RuntimeHook::PyCallFunction,
            PyCallKind::Member { .. } => RuntimeHook::PyCallMethod,
            PyCallKind::Dynamic => RuntimeHook::PyCallDynamic,
        };

        if !required_hooks.contains(&hook) {
            required_hooks.push(hook);
        }
    }

    let imports = module
        .items
        .iter()
        .filter_map(|item| match item {
            mir::Item::ImportPy(import) => Some(import),
            mir::Item::Function(_) => None,
        })
        .flat_map(|import| {
            import.names.iter().map(move |name| PyImportBinding {
                module: import.module.clone(),
                name: name.clone(),
            })
        })
        .collect();

    PyRuntimePlan {
        imports,
        required_hooks,
        call_sites,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn builds_runtime_plan_with_imports_and_hooks() {
        let module = lower(
            "from py pandas import read_csv, DataFrame\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let plan = build_python_runtime_plan(&module);

        assert_eq!(
            plan.imports,
            vec![
                PyImportBinding {
                    module: "pandas".to_string(),
                    name: "read_csv".to_string(),
                },
                PyImportBinding {
                    module: "pandas".to_string(),
                    name: "DataFrame".to_string(),
                },
            ]
        );
        assert_eq!(
            plan.required_hooks,
            vec![RuntimeHook::PyCallFunction, RuntimeHook::PyCallMethod]
        );
        assert_eq!(plan.call_sites.len(), 2);
    }

    #[test]
    fn runtime_hook_signatures_are_stable() {
        assert_eq!(
            RuntimeHook::PyCallFunction.signature(),
            RuntimeHookSignature {
                symbol: "dx_rt_py_call_function",
                params: PY_CALL_FUNCTION_PARAMS,
                ret: AbiType::PyObjHandle,
            }
        );
        assert_eq!(
            RuntimeHook::PyCallMethod.signature(),
            RuntimeHookSignature {
                symbol: "dx_rt_py_call_method",
                params: PY_CALL_METHOD_PARAMS,
                ret: AbiType::PyObjHandle,
            }
        );
        assert_eq!(
            RuntimeHook::PyCallDynamic.signature(),
            RuntimeHookSignature {
                symbol: "dx_rt_py_call_dynamic",
                params: PY_CALL_DYNAMIC_PARAMS,
                ret: AbiType::PyObjHandle,
            }
        );
    }

    #[test]
    fn dynamic_python_calls_request_dynamic_hook() {
        let module = lower(
            "from py pandas import read_csv\n\nfun invoke(path: Str) -> PyObj !py:\n    val f = read_csv(path)\n    f()\n.\n",
        );
        let plan = build_python_runtime_plan(&module);

        assert_eq!(
            plan.required_hooks,
            vec![RuntimeHook::PyCallFunction, RuntimeHook::PyCallDynamic]
        );
        assert_eq!(plan.call_sites.len(), 2);
        assert_eq!(plan.call_sites[1].kind, PyCallKind::Dynamic);
    }
}
