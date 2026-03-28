use crate::abi::{build_python_runtime_plan, PyImportBinding, RuntimeHook};
use crate::py::{PyCallKind, PyRuntimeCallSite};
use dx_hir::Type;
use dx_mir::mir;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyDispatchTarget {
    Function { module: Option<String>, name: String },
    Method { name: String },
    Dynamic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoweredPyCall {
    pub function: String,
    pub block: mir::BlockId,
    pub statement: usize,
    pub hook: RuntimeHook,
    pub runtime_symbol: &'static str,
    pub dispatch: PyDispatchTarget,
    pub arg_count: u32,
    pub effects: Vec<String>,
    pub result_type: Type,
}

pub fn lower_python_runtime_calls(module: &mir::Module) -> Vec<LoweredPyCall> {
    let plan = build_python_runtime_plan(module);
    let import_index = build_import_index(&plan.imports);

    plan.call_sites
        .iter()
        .map(|site| lower_call_site(site, &import_index))
        .collect()
}

fn build_import_index(imports: &[PyImportBinding]) -> HashMap<&str, &str> {
    let mut index = HashMap::new();
    for import in imports {
        index.insert(import.name.as_str(), import.module.as_str());
    }
    index
}

fn lower_call_site(
    site: &PyRuntimeCallSite,
    import_index: &HashMap<&str, &str>,
) -> LoweredPyCall {
    let (hook, dispatch) = match &site.kind {
        PyCallKind::Function { name } => (
            RuntimeHook::PyCallFunction,
            PyDispatchTarget::Function {
                module: import_index.get(name.as_str()).map(|module| (*module).to_string()),
                name: name.clone(),
            },
        ),
        PyCallKind::Member { name } => (
            RuntimeHook::PyCallMethod,
            PyDispatchTarget::Method { name: name.clone() },
        ),
        PyCallKind::Dynamic => (RuntimeHook::PyCallDynamic, PyDispatchTarget::Dynamic),
    };

    LoweredPyCall {
        function: site.function.clone(),
        block: site.block,
        statement: site.statement,
        hook,
        runtime_symbol: hook.symbol(),
        dispatch,
        arg_count: site.arg_count as u32,
        effects: site.effects.clone(),
        result_type: site.result_type.clone(),
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
    fn lowers_python_function_calls_to_runtime_hook() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let ops = lower_python_runtime_calls(&module);

        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].hook, RuntimeHook::PyCallFunction);
        assert_eq!(ops[0].runtime_symbol, "dx_rt_py_call_function");
        assert_eq!(
            ops[0].dispatch,
            PyDispatchTarget::Function {
                module: Some("pandas".to_string()),
                name: "read_csv".to_string(),
            }
        );
        assert_eq!(ops[0].arg_count, 1);
        assert_eq!(ops[0].result_type, Type::PyObj);
    }

    #[test]
    fn lowers_python_member_calls_to_runtime_hook() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let ops = lower_python_runtime_calls(&module);

        assert_eq!(ops.len(), 2);
        assert_eq!(ops[1].hook, RuntimeHook::PyCallMethod);
        assert_eq!(ops[1].runtime_symbol, "dx_rt_py_call_method");
        assert_eq!(
            ops[1].dispatch,
            PyDispatchTarget::Method {
                name: "head".to_string(),
            }
        );
    }

    #[test]
    fn lowers_dynamic_python_calls_to_runtime_hook() {
        let module = lower(
            "from py pandas import read_csv\n\nfun invoke(path: Str) -> PyObj !py:\n    val f = read_csv(path)\n    f()\n.\n",
        );
        let ops = lower_python_runtime_calls(&module);

        assert_eq!(ops.len(), 2);
        assert_eq!(ops[1].hook, RuntimeHook::PyCallDynamic);
        assert_eq!(ops[1].runtime_symbol, "dx_rt_py_call_dynamic");
        assert_eq!(ops[1].dispatch, PyDispatchTarget::Dynamic);
    }
}
