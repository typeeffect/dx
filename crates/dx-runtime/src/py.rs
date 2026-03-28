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
                    arg_count: args.len(),
                    effects: effects.clone(),
                    result_type: ty.clone(),
                });
            }
        }
    }

    sites
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
    }

    #[test]
    fn ignores_native_calls() {
        let module = lower(
            "fun inner() -> Int:\n    1\n.\n\nfun outer() -> Int:\n    inner()\n.\n",
        );
        let sites = collect_python_call_sites(&module);
        assert!(sites.is_empty());
    }
}
