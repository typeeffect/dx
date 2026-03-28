use crate::hir;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingDiagnostic {
    pub function: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameResolutionReport {
    pub diagnostics: Vec<BindingDiagnostic>,
}

pub fn resolve_module(module: &hir::Module) -> NameResolutionReport {
    let globals = collect_globals(module);
    let mut diagnostics = Vec::new();

    for item in &module.items {
        match item {
            hir::Item::Function(function) => {
                let mut scope = Scope::new();
                for name in &globals {
                    scope.define(name.clone(), false);
                }
                for param in &function.params {
                    scope.define(param.name.clone(), false);
                }
                resolve_block(&function.name, &function.body, &mut scope, &mut diagnostics);
            }
            hir::Item::Statement(stmt) => {
                let mut scope = Scope::new();
                for name in &globals {
                    scope.define(name.clone(), false);
                }
                resolve_stmt("<module>", stmt, &mut scope, &mut diagnostics);
            }
            hir::Item::ImportPy(_) => {}
        }
    }

    NameResolutionReport { diagnostics }
}

fn collect_globals(module: &hir::Module) -> HashSet<String> {
    let mut globals = HashSet::new();
    for item in &module.items {
        match item {
            hir::Item::ImportPy(import) => {
                for name in &import.names {
                    globals.insert(name.clone());
                }
            }
            hir::Item::Function(function) => {
                globals.insert(function.name.clone());
            }
            hir::Item::Statement(_) => {}
        }
    }
    globals
}

fn resolve_block(
    function_name: &str,
    block: &hir::Block,
    scope: &mut Scope,
    diagnostics: &mut Vec<BindingDiagnostic>,
) {
    for stmt in &block.stmts {
        resolve_stmt(function_name, stmt, scope, diagnostics);
    }
    if let Some(result) = &block.result {
        resolve_expr(function_name, result, scope, diagnostics);
    }
}

fn resolve_stmt(
    function_name: &str,
    stmt: &hir::Stmt,
    scope: &mut Scope,
    diagnostics: &mut Vec<BindingDiagnostic>,
) {
    match stmt {
        hir::Stmt::Let {
            name,
            mutable,
            value,
            synthetic: _,
        } => {
            resolve_expr(function_name, value, scope, diagnostics);
            if scope.current_contains(name) {
                diagnostics.push(BindingDiagnostic {
                    function: function_name.to_string(),
                    message: format!("duplicate binding `{name}` in the same scope"),
                });
            }
            scope.define(name.clone(), *mutable);
        }
        hir::Stmt::Rebind { name, value } => {
            resolve_expr(function_name, value, scope, diagnostics);
            match scope.lookup(name) {
                Some(binding) if binding.mutable => {}
                Some(_) => diagnostics.push(BindingDiagnostic {
                    function: function_name.to_string(),
                    message: format!("cannot rebind immutable name `{name}`"),
                }),
                None => diagnostics.push(BindingDiagnostic {
                    function: function_name.to_string(),
                    message: format!("cannot rebind undefined name `{name}`"),
                }),
            }
        }
        hir::Stmt::Expr(expr) => resolve_expr(function_name, expr, scope, diagnostics),
    }
}

fn resolve_expr(
    function_name: &str,
    expr: &hir::Expr,
    scope: &mut Scope,
    diagnostics: &mut Vec<BindingDiagnostic>,
) {
    match expr {
        hir::Expr::Name(name) => {
            if is_implicitly_allowed_name(name) {
                return;
            }
            if scope.lookup(name).is_none() {
                diagnostics.push(BindingDiagnostic {
                    function: function_name.to_string(),
                    message: format!("use of undefined name `{name}`"),
                });
            }
        }
        hir::Expr::Integer(_) | hir::Expr::String(_) => {}
        hir::Expr::Member { base, .. } => resolve_expr(function_name, base, scope, diagnostics),
        hir::Expr::Call { callee, args } => {
            resolve_expr(function_name, callee, scope, diagnostics);
            for arg in args {
                match arg {
                    hir::Arg::Positional(expr) => {
                        resolve_expr(function_name, expr, scope, diagnostics)
                    }
                    hir::Arg::Named { value, .. } => {
                        resolve_expr(function_name, value, scope, diagnostics)
                    }
                }
            }
        }
        hir::Expr::Closure { params, body } => {
            let mut inner = scope.child();
            for param in params {
                inner.define(param.name.clone(), false);
            }
            resolve_closure_body(function_name, body, &mut inner, diagnostics);
        }
        hir::Expr::If {
            branches,
            else_branch,
        } => {
            for (condition, block) in branches {
                resolve_expr(function_name, condition, scope, diagnostics);
                let mut branch_scope = scope.child();
                resolve_block(function_name, block, &mut branch_scope, diagnostics);
            }
            if let Some(block) = else_branch {
                let mut else_scope = scope.child();
                resolve_block(function_name, block, &mut else_scope, diagnostics);
            }
        }
        hir::Expr::Match { scrutinee, arms } => {
            resolve_expr(function_name, scrutinee, scope, diagnostics);
            for arm in arms {
                let mut arm_scope = scope.child();
                bind_pattern(&arm.pattern, &mut arm_scope);
                resolve_block(function_name, &arm.body, &mut arm_scope, diagnostics);
            }
        }
        hir::Expr::BinaryOp { lhs, rhs, .. } => {
            resolve_expr(function_name, lhs, scope, diagnostics);
            resolve_expr(function_name, rhs, scope, diagnostics);
        }
    }
}

fn resolve_closure_body(
    function_name: &str,
    body: &hir::ClosureBody,
    scope: &mut Scope,
    diagnostics: &mut Vec<BindingDiagnostic>,
) {
    match body {
        hir::ClosureBody::Expr(expr) => resolve_expr(function_name, expr, scope, diagnostics),
        hir::ClosureBody::Block(block) => resolve_block(function_name, block, scope, diagnostics),
    }
}

fn bind_pattern(pattern: &hir::Pattern, scope: &mut Scope) {
    match pattern {
        hir::Pattern::Name(name) => {
            scope.define(name.clone(), false);
        }
        hir::Pattern::Wildcard => {}
        hir::Pattern::Constructor { args, .. } => {
            for arg in args {
                bind_pattern(arg, scope);
            }
        }
    }
}

fn is_implicitly_allowed_name(name: &str) -> bool {
    name == "me" || name.starts_with("$it") || name.starts_with("$p")
}

#[derive(Debug, Clone)]
struct BindingInfo {
    mutable: bool,
}

#[derive(Debug, Clone)]
struct Scope {
    layers: Vec<HashMap<String, BindingInfo>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            layers: vec![HashMap::new()],
        }
    }

    fn child(&self) -> Self {
        let mut layers = self.layers.clone();
        layers.push(HashMap::new());
        Self { layers }
    }

    fn define(&mut self, name: String, mutable: bool) {
        self.layers
            .last_mut()
            .expect("scope always has at least one layer")
            .insert(name, BindingInfo { mutable });
    }

    fn lookup(&self, name: &str) -> Option<&BindingInfo> {
        self.layers.iter().rev().find_map(|layer| layer.get(name))
    }

    fn current_contains(&self, name: &str) -> bool {
        self.layers
            .last()
            .expect("scope always has at least one layer")
            .contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
    use dx_parser::{Lexer, Parser};

    fn resolve(src: &str) -> NameResolutionReport {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_module(&ast);
        resolve_module(&hir)
    }

    #[test]
    fn reports_undefined_local_name() {
        let report = resolve(
            r#"
fun demo() -> Unit:
    missing
.
"#,
        );
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("undefined name `missing`"));
    }

    #[test]
    fn allows_capture_from_outer_scope() {
        let report = resolve(
            r#"
fun demo() -> Unit:
    val x = "ok"
    val f = lazy x
    f()
.
"#,
        );
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn rejects_rebind_of_val() {
        let report = resolve(
            r#"
fun demo() -> Unit:
    val x = "ok"
    x = "no"
.
"#,
        );
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("immutable name `x`"));
    }

    #[test]
    fn match_pattern_binds_names_only_inside_arm() {
        let report = resolve(
            r#"
fun demo(x: Result) -> Unit:
    match x:
        Ok(v):
            v
    .
    v
.
"#,
        );
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("undefined name `v`"));
    }

    #[test]
    fn imported_python_names_resolve() {
        let report = resolve(
            r#"
from py pandas import read_csv

fun load(path: Str) -> Unit:
    read_csv(path)
.
"#,
        );
        assert!(report.diagnostics.is_empty());
    }
}
