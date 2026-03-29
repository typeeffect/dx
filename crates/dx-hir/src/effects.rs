use crate::hir;
use std::collections::{BTreeSet, HashMap, HashSet};

pub type EffectSet = BTreeSet<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub function: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionEffectReport {
    pub name: String,
    pub declared: EffectSet,
    pub inferred: EffectSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleEffectReport {
    pub functions: Vec<FunctionEffectReport>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn check_module_effects(module: &hir::Module) -> ModuleEffectReport {
    let function_effects = collect_function_effects(module);
    let imported_py_names = collect_imported_py_names(module);

    let mut reports = Vec::new();
    let mut diagnostics = Vec::new();

    for item in &module.items {
        if let hir::Item::Function(function) = item {
            let inferred = infer_block_effects(
                &function.body,
                &function_effects,
                &imported_py_names,
                &mut HashMap::new(),
            );
            let declared: EffectSet = function.effects.iter().cloned().collect();

            let missing: Vec<String> = inferred.difference(&declared).cloned().collect();
            if !missing.is_empty() {
                diagnostics.push(Diagnostic {
                    function: function.name.clone(),
                    message: format!(
                        "function `{}` is missing declared effects: {}",
                        function.name,
                        missing.join(", ")
                    ),
                });
            }

            reports.push(FunctionEffectReport {
                name: function.name.clone(),
                declared,
                inferred,
            });
        }
    }

    ModuleEffectReport {
        functions: reports,
        diagnostics,
    }
}

fn collect_function_effects(module: &hir::Module) -> HashMap<String, EffectSet> {
    module
        .items
        .iter()
        .filter_map(|item| match item {
            hir::Item::Function(function) => Some((
                function.name.clone(),
                function.effects.iter().cloned().collect::<EffectSet>(),
            )),
            hir::Item::Schema(_) => None,
            _ => None,
        })
        .collect()
}

fn collect_imported_py_names(module: &hir::Module) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in &module.items {
        if let hir::Item::ImportPy(import) = item {
            for name in &import.names {
                names.insert(name.clone());
            }
        }
    }
    names
}

fn infer_block_effects(
    block: &hir::Block,
    function_effects: &HashMap<String, EffectSet>,
    imported_py_names: &HashSet<String>,
    local_closures: &mut HashMap<String, EffectSet>,
) -> EffectSet {
    let mut effects = EffectSet::new();

    for stmt in &block.stmts {
        effects.extend(infer_stmt_effects(
            stmt,
            function_effects,
            imported_py_names,
            local_closures,
        ));
    }

    if let Some(result) = &block.result {
        effects.extend(infer_expr_effects(
            result,
            function_effects,
            imported_py_names,
            local_closures,
        ));
    }

    effects
}

fn infer_stmt_effects(
    stmt: &hir::Stmt,
    function_effects: &HashMap<String, EffectSet>,
    imported_py_names: &HashSet<String>,
    local_closures: &mut HashMap<String, EffectSet>,
) -> EffectSet {
    match stmt {
        hir::Stmt::Let {
            name,
            value,
            mutable: _,
            synthetic: _,
        } => {
            if let hir::Expr::Closure { body, .. } = value {
                local_closures.insert(
                    name.clone(),
                    infer_closure_body_effects(
                        body,
                        function_effects,
                        imported_py_names,
                        &mut local_closures.clone(),
                    ),
                );
            } else {
                local_closures.remove(name);
            }
            infer_expr_effects(value, function_effects, imported_py_names, local_closures)
        }
        hir::Stmt::Rebind { name, value } => {
            local_closures.remove(name);
            infer_expr_effects(value, function_effects, imported_py_names, local_closures)
        }
        hir::Stmt::Expr(expr) => {
            infer_expr_effects(expr, function_effects, imported_py_names, local_closures)
        }
    }
}

fn infer_expr_effects(
    expr: &hir::Expr,
    function_effects: &HashMap<String, EffectSet>,
    imported_py_names: &HashSet<String>,
    local_closures: &mut HashMap<String, EffectSet>,
) -> EffectSet {
    match expr {
        hir::Expr::Unit | hir::Expr::Name(_) | hir::Expr::Integer(_) | hir::Expr::String(_) => {
            EffectSet::new()
        }
        hir::Expr::Member { base, .. } => {
            infer_expr_effects(base, function_effects, imported_py_names, local_closures)
        }
        hir::Expr::Call { callee, args } => {
            let mut effects =
                infer_expr_effects(callee, function_effects, imported_py_names, local_closures);

            for arg in args {
                match arg {
                    hir::Arg::Positional(expr) => {
                        effects.extend(infer_expr_effects(
                            expr,
                            function_effects,
                            imported_py_names,
                            local_closures,
                        ));
                    }
                    hir::Arg::Named { value, .. } => {
                        effects.extend(infer_expr_effects(
                            value,
                            function_effects,
                            imported_py_names,
                            local_closures,
                        ));
                    }
                }
            }

            effects.extend(call_target_effects(
                callee,
                function_effects,
                imported_py_names,
                local_closures,
            ));
            effects
        }
        hir::Expr::Closure { .. } => EffectSet::new(),
        hir::Expr::If {
            branches,
            else_branch,
        } => {
            let mut effects = EffectSet::new();
            for (condition, block) in branches {
                effects.extend(infer_expr_effects(
                    condition,
                    function_effects,
                    imported_py_names,
                    local_closures,
                ));
                effects.extend(infer_block_effects(
                    block,
                    function_effects,
                    imported_py_names,
                    &mut local_closures.clone(),
                ));
            }
            if let Some(block) = else_branch {
                effects.extend(infer_block_effects(
                    block,
                    function_effects,
                    imported_py_names,
                    &mut local_closures.clone(),
                ));
            }
            effects
        }
        hir::Expr::Match { scrutinee, arms } => {
            let mut effects =
                infer_expr_effects(scrutinee, function_effects, imported_py_names, local_closures);
            for arm in arms {
                effects.extend(infer_block_effects(
                    &arm.body,
                    function_effects,
                    imported_py_names,
                    &mut local_closures.clone(),
                ));
            }
            effects
        }
        hir::Expr::BinaryOp { lhs, rhs, .. } => {
            let mut effects =
                infer_expr_effects(lhs, function_effects, imported_py_names, local_closures);
            effects.extend(infer_expr_effects(
                rhs,
                function_effects,
                imported_py_names,
                local_closures,
            ));
            effects
        }
    }
}

fn infer_closure_body_effects(
    body: &hir::ClosureBody,
    function_effects: &HashMap<String, EffectSet>,
    imported_py_names: &HashSet<String>,
    local_closures: &mut HashMap<String, EffectSet>,
) -> EffectSet {
    match body {
        hir::ClosureBody::Expr(expr) => {
            infer_expr_effects(expr, function_effects, imported_py_names, local_closures)
        }
        hir::ClosureBody::Block(block) => {
            infer_block_effects(block, function_effects, imported_py_names, local_closures)
        }
    }
}

fn call_target_effects(
    callee: &hir::Expr,
    function_effects: &HashMap<String, EffectSet>,
    imported_py_names: &HashSet<String>,
    local_closures: &HashMap<String, EffectSet>,
) -> EffectSet {
    match callee {
        hir::Expr::Name(name) => {
            if let Some(effects) = local_closures.get(name) {
                return effects.clone();
            }
            if let Some(effects) = function_effects.get(name) {
                return effects.clone();
            }
            if imported_py_names.contains(name) {
                return ["py".to_string()].into_iter().collect();
            }
            EffectSet::new()
        }
        hir::Expr::Closure { body, .. } => infer_closure_body_effects(
            body,
            function_effects,
            imported_py_names,
            &mut local_closures.clone(),
        ),
        _ if is_python_value_expr(
            callee,
            function_effects,
            imported_py_names,
            local_closures,
        ) => ["py".to_string()].into_iter().collect(),
        _ => EffectSet::new(),
    }
}

fn is_python_value_expr(
    expr: &hir::Expr,
    function_effects: &HashMap<String, EffectSet>,
    imported_py_names: &HashSet<String>,
    local_closures: &HashMap<String, EffectSet>,
) -> bool {
    match expr {
        hir::Expr::Name(name) => imported_py_names.contains(name),
        hir::Expr::Member { base, .. } => {
            is_python_value_expr(base, function_effects, imported_py_names, local_closures)
        }
        hir::Expr::Call { callee, .. } => {
            let target_effects =
                call_target_effects(callee, function_effects, imported_py_names, local_closures);
            target_effects.contains("py")
                || is_python_value_expr(callee, function_effects, imported_py_names, local_closures)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
    use dx_parser::{Lexer, Parser};

    fn analyze(src: &str) -> ModuleEffectReport {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_module(&ast);
        check_module_effects(&hir)
    }

    #[test]
    fn reports_missing_py_effect_for_python_call() {
        let report = analyze(
            r#"
from py pandas import read_csv

fun load(path: Str) -> PyObj:
    read_csv(path)
.
"#,
        );

        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("py"));
    }

    #[test]
    fn propagates_declared_effects_through_local_function_calls() {
        let report = analyze(
            r#"
fun inner() -> Unit !io:
.

fun outer() -> Unit:
    inner()
.
"#,
        );

        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("io"));
    }

    #[test]
    fn constructing_lazy_closure_does_not_execute_its_effects() {
        let report = analyze(
            r#"
from py pandas import read_csv

fun build(path: Str) -> lazy PyObj !py:
    val thunk = lazy read_csv(path)
    thunk
.
"#,
        );

        assert!(report.diagnostics.is_empty());
        assert_eq!(report.functions[0].inferred, EffectSet::new());
    }

    #[test]
    fn calling_local_lazy_closure_executes_its_effects() {
        let report = analyze(
            r#"
from py pandas import read_csv

fun run(path: Str) -> PyObj !py:
    val thunk = lazy read_csv(path)
    thunk()
.
"#,
        );

        assert!(report.diagnostics.is_empty());
        assert_eq!(
            report.functions[0].inferred,
            ["py".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn chained_python_member_calls_still_require_py_effect() {
        let report = analyze(
            r#"
from py pandas import read_csv

fun load(path: Str) -> PyObj !py:
    read_csv(path)'head()
.
"#,
        );

        assert!(report.diagnostics.is_empty());
        assert_eq!(
            report.functions[0].inferred,
            ["py".to_string()].into_iter().collect()
        );
    }
}
