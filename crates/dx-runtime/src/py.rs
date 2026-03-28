use dx_hir::{typed, Type};
use dx_mir::mir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PyCallKind {
    Function { name: String },
    Member { name: String },
    Dynamic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PyRuntimeCallSite {
    pub function: String,
    pub block: mir::BlockId,
    pub statement: usize,
    pub kind: PyCallKind,
    pub receiver: Option<mir::Operand>,
    pub callable: Option<mir::Operand>,
    pub arg_count: usize,
    pub effects: Vec<String>,
    pub result_type: Type,
}

pub fn collect_python_call_sites(module: &mir::Module) -> Vec<PyRuntimeCallSite> {
    let mut sites = Vec::new();

    for item in &module.items {
        let mir::Item::Function(function) = item else {
            continue;
        };

        for (block_id, block) in function.blocks.iter().enumerate() {
            for (statement_index, stmt) in block.statements.iter().enumerate() {
                let mir::Statement::Assign { value, .. } = stmt;
                let mir::Rvalue::Call {
                    target,
                    callee,
                    args,
                    ty,
                    effects,
                    ..
                } = value
                else {
                    continue;
                };

                let kind = match target {
                    typed::CallTarget::PythonFunction { name } => {
                        Some(PyCallKind::Function { name: name.clone() })
                    }
                    typed::CallTarget::PythonMember { name } => {
                        Some(PyCallKind::Member { name: name.clone() })
                    }
                    typed::CallTarget::PythonDynamic => Some(PyCallKind::Dynamic),
                    typed::CallTarget::NativeFunction { .. }
                    | typed::CallTarget::LocalClosure { .. }
                    | typed::CallTarget::Dynamic => None,
                };

                let Some(kind) = kind else {
                    continue;
                };

                sites.push(PyRuntimeCallSite {
                    function: function.name.clone(),
                    block: block_id,
                    statement: statement_index,
                    kind,
                    receiver: resolve_python_receiver(function, block_id, statement_index, callee),
                    callable: Some(callee.clone()),
                    arg_count: args.len(),
                    effects: effects.clone(),
                    result_type: ty.clone(),
                });
            }
        }
    }

    sites
}

fn resolve_python_receiver(
    function: &mir::Function,
    block_id: mir::BlockId,
    statement_index: usize,
    callee: &mir::Operand,
) -> Option<mir::Operand> {
    let mir::Operand::Copy(local_id) = callee else {
        return None;
    };
    let block = function.blocks.get(block_id)?;
    for stmt in block.statements[..statement_index].iter().rev() {
        let mir::Statement::Assign { place, value } = stmt;
        if place != local_id {
            continue;
        }
        if let mir::Rvalue::Member { base, .. } = value {
            return Some(base.clone());
        }
        return None;
    }
    None
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
    fn collects_python_function_call_sites() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let sites = collect_python_call_sites(&module);
        assert_eq!(sites.len(), 1);
        assert_eq!(
            sites[0].kind,
            PyCallKind::Function {
                name: "read_csv".to_string()
            }
        );
        assert_eq!(sites[0].effects, vec!["py".to_string()]);
        assert_eq!(sites[0].result_type, Type::PyObj);
        assert_eq!(sites[0].receiver, None);
        assert!(matches!(sites[0].callable, Some(mir::Operand::Const(mir::Constant::Unit))));
    }

    #[test]
    fn collects_python_member_call_sites() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let sites = collect_python_call_sites(&module);
        assert_eq!(sites.len(), 2);
        assert_eq!(
            sites[0].kind,
            PyCallKind::Function {
                name: "read_csv".to_string()
            }
        );
        assert_eq!(
            sites[1].kind,
            PyCallKind::Member {
                name: "head".to_string()
            }
        );
        assert!(sites[1].receiver.is_some());
    }

    #[test]
    fn ignores_native_calls() {
        let module = lower(
            "fun inner() -> Int:\n    1\n.\n\nfun outer() -> Int:\n    inner()\n.\n",
        );
        let sites = collect_python_call_sites(&module);
        assert!(sites.is_empty());
    }

    #[test]
    fn preserves_dynamic_callable_operand() {
        let module = lower(
            "from py pandas import read_csv\n\nfun invoke(path: Str) -> PyObj !py:\n    val f = read_csv(path)\n    f()\n.\n",
        );
        let sites = collect_python_call_sites(&module);
        assert!(sites[1].callable.is_some());
    }
}
