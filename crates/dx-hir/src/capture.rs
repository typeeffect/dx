use crate::{typed, Type};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingOrigin {
    Global,
    Capturable,
    Local,
}

#[derive(Debug, Clone, PartialEq)]
struct CaptureBinding {
    ty: Type,
    mutable: bool,
    origin: BindingOrigin,
}

#[derive(Debug, Clone, PartialEq)]
struct CaptureScope {
    layers: Vec<HashMap<String, CaptureBinding>>,
}

impl CaptureScope {
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

    fn nested_closure_scope(&self) -> Self {
        let layers = self
            .layers
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|(name, binding)| {
                        let origin = match binding.origin {
                            BindingOrigin::Global => BindingOrigin::Global,
                            BindingOrigin::Capturable | BindingOrigin::Local => {
                                BindingOrigin::Capturable
                            }
                        };
                        (
                            name.clone(),
                            CaptureBinding {
                                ty: binding.ty.clone(),
                                mutable: binding.mutable,
                                origin,
                            },
                        )
                    })
                    .collect()
            })
            .collect();
        Self { layers }
    }

    fn define(&mut self, name: String, binding: CaptureBinding) {
        self.layers
            .last_mut()
            .expect("scope has one layer")
            .insert(name, binding);
    }

    fn lookup(&self, name: &str) -> Option<&CaptureBinding> {
        self.layers.iter().rev().find_map(|layer| layer.get(name))
    }
}

pub fn annotate_module_captures(module: typed::Module) -> typed::Module {
    let globals = collect_global_bindings(&module);

    typed::Module {
        items: module
            .items
            .into_iter()
            .map(|item| annotate_item(item, &globals))
            .collect(),
    }
}

fn collect_global_bindings(module: &typed::Module) -> HashMap<String, CaptureBinding> {
    let mut globals = HashMap::new();
    for item in &module.items {
        match item {
            typed::Item::ImportPy(import) => {
                for name in &import.names {
                    globals.insert(
                        name.clone(),
                        CaptureBinding {
                            ty: Type::PyObj,
                            mutable: false,
                            origin: BindingOrigin::Global,
                        },
                    );
                }
            }
            typed::Item::Function(function) => {
                if let Some(ret) = &function.return_type {
                    globals.insert(
                        function.name.clone(),
                        CaptureBinding {
                            ty: Type::Function {
                                params: function.params.iter().map(|param| param.ty.clone()).collect(),
                                ret: Box::new(ret.clone()),
                                effects: function.effects.clone(),
                            },
                            mutable: false,
                            origin: BindingOrigin::Global,
                        },
                    );
                }
            }
            typed::Item::Statement(_) => {}
        }
    }
    globals
}

fn annotate_item(
    item: typed::Item,
    globals: &HashMap<String, CaptureBinding>,
) -> typed::Item {
    match item {
        typed::Item::ImportPy(import) => typed::Item::ImportPy(import),
        typed::Item::Function(function) => {
            let mut scope = CaptureScope::new();
            for (name, binding) in globals {
                scope.define(name.clone(), binding.clone());
            }
            for param in &function.params {
                scope.define(
                    param.name.clone(),
                    CaptureBinding {
                        ty: param.ty.clone(),
                        mutable: false,
                        origin: BindingOrigin::Capturable,
                    },
                );
            }

            typed::Item::Function(typed::Function {
                body: annotate_block(function.body, &mut scope),
                ..function
            })
        }
        typed::Item::Statement(stmt) => {
            let mut scope = CaptureScope::new();
            for (name, binding) in globals {
                scope.define(name.clone(), binding.clone());
            }
            typed::Item::Statement(annotate_stmt(stmt, &mut scope))
        }
    }
}

fn annotate_block(block: typed::Block, scope: &mut CaptureScope) -> typed::Block {
    let mut stmts = Vec::with_capacity(block.stmts.len());
    for stmt in block.stmts {
        stmts.push(annotate_stmt(stmt, scope));
    }
    let result = block
        .result
        .map(|expr| Box::new(annotate_expr(*expr, scope)));

    typed::Block {
        stmts,
        result,
        ty: block.ty,
    }
}

fn annotate_stmt(stmt: typed::Stmt, scope: &mut CaptureScope) -> typed::Stmt {
    match stmt {
        typed::Stmt::Let {
            name,
            mutable,
            value,
            synthetic,
        } => {
            let value = annotate_expr(value, scope);
            scope.define(
                name.clone(),
                CaptureBinding {
                    ty: value.ty.clone(),
                    mutable,
                    origin: BindingOrigin::Capturable,
                },
            );
            typed::Stmt::Let {
                name,
                mutable,
                value,
                synthetic,
            }
        }
        typed::Stmt::Rebind { name, value } => typed::Stmt::Rebind {
            name,
            value: annotate_expr(value, scope),
        },
        typed::Stmt::Expr(expr) => typed::Stmt::Expr(annotate_expr(expr, scope)),
    }
}

fn annotate_expr(expr: typed::Expr, scope: &mut CaptureScope) -> typed::Expr {
    let ty = expr.ty.clone();
    let kind = match expr.kind {
        typed::ExprKind::Unit => typed::ExprKind::Unit,
        typed::ExprKind::Name(name) => typed::ExprKind::Name(name),
        typed::ExprKind::Integer(value) => typed::ExprKind::Integer(value),
        typed::ExprKind::String(value) => typed::ExprKind::String(value),
        typed::ExprKind::Member { base, name } => typed::ExprKind::Member {
            base: Box::new(annotate_expr(*base, scope)),
            name,
        },
        typed::ExprKind::Call {
            target,
            callee,
            args,
        } => typed::ExprKind::Call {
            target,
            callee: Box::new(annotate_expr(*callee, scope)),
            args: args
                .into_iter()
                .map(|arg| match arg {
                    typed::Arg::Positional(expr) => {
                        typed::Arg::Positional(annotate_expr(expr, scope))
                    }
                    typed::Arg::Named { name, value } => typed::Arg::Named {
                        name,
                        value: annotate_expr(value, scope),
                    },
                })
                .collect(),
        },
        typed::ExprKind::Closure {
            params,
            body,
            captures: _,
        } => {
            let mut nested_scope = scope.nested_closure_scope();
            for param in &params {
                nested_scope.define(
                    param.name.clone(),
                    CaptureBinding {
                        ty: param.ty.clone(),
                        mutable: false,
                        origin: BindingOrigin::Local,
                    },
                );
            }

            let body = Box::new(annotate_closure_body(*body, &mut nested_scope));
            let captures = collect_closure_captures(body.as_ref(), scope, &params);

            typed::ExprKind::Closure {
                params,
                body,
                captures,
            }
        }
        typed::ExprKind::If {
            branches,
            else_branch,
        } => typed::ExprKind::If {
            branches: branches
                .into_iter()
                .map(|(condition, block)| {
                    let mut branch_scope = scope.child();
                    (
                        annotate_expr(condition, scope),
                        annotate_block(block, &mut branch_scope),
                    )
                })
                .collect(),
            else_branch: else_branch.map(|block| {
                let mut branch_scope = scope.child();
                annotate_block(block, &mut branch_scope)
            }),
        },
        typed::ExprKind::Match { scrutinee, arms } => typed::ExprKind::Match {
            scrutinee: Box::new(annotate_expr(*scrutinee, scope)),
            arms: arms
                .into_iter()
                .map(|arm| {
                    let mut arm_scope = scope.child();
                    define_pattern_bindings(&arm.pattern, &mut arm_scope);
                    typed::MatchArm {
                        pattern: arm.pattern,
                        body: annotate_block(arm.body, &mut arm_scope),
                    }
                })
                .collect(),
        },
        typed::ExprKind::BinaryOp { op, lhs, rhs } => typed::ExprKind::BinaryOp {
            op,
            lhs: Box::new(annotate_expr(*lhs, scope)),
            rhs: Box::new(annotate_expr(*rhs, scope)),
        },
    };

    typed::Expr { ty, kind }
}

fn annotate_closure_body(body: typed::ClosureBody, scope: &mut CaptureScope) -> typed::ClosureBody {
    match body {
        typed::ClosureBody::Expr(expr) => {
            typed::ClosureBody::Expr(Box::new(annotate_expr(*expr, scope)))
        }
        typed::ClosureBody::Block(block) => {
            typed::ClosureBody::Block(Box::new(annotate_block(*block, scope)))
        }
    }
}

fn define_pattern_bindings(pattern: &crate::hir::Pattern, scope: &mut CaptureScope) {
    match pattern {
        crate::hir::Pattern::Name(name) => {
            scope.define(
                name.clone(),
                CaptureBinding {
                    ty: Type::Unknown,
                    mutable: false,
                    origin: BindingOrigin::Capturable,
                },
            );
        }
        crate::hir::Pattern::Wildcard => {}
        crate::hir::Pattern::Constructor { args, .. } => {
            for arg in args {
                define_pattern_bindings(arg, scope);
            }
        }
    }
}

fn collect_closure_captures(
    body: &typed::ClosureBody,
    ambient_scope: &CaptureScope,
    params: &[typed::ClosureParam],
) -> Vec<typed::ClosureCapture> {
    let mut scope = ambient_scope.nested_closure_scope();
    for param in params {
        scope.define(
            param.name.clone(),
            CaptureBinding {
                ty: param.ty.clone(),
                mutable: false,
                origin: BindingOrigin::Local,
            },
        );
    }

    let mut captures = Vec::new();
    let mut seen = HashSet::new();
    collect_body_captures(body, &mut scope, &mut seen, &mut captures);
    captures
}

fn collect_body_captures(
    body: &typed::ClosureBody,
    scope: &mut CaptureScope,
    seen: &mut HashSet<String>,
    captures: &mut Vec<typed::ClosureCapture>,
) {
    match body {
        typed::ClosureBody::Expr(expr) => collect_expr_captures(expr, scope, seen, captures),
        typed::ClosureBody::Block(block) => collect_block_captures(block, scope, seen, captures),
    }
}

fn collect_block_captures(
    block: &typed::Block,
    scope: &mut CaptureScope,
    seen: &mut HashSet<String>,
    captures: &mut Vec<typed::ClosureCapture>,
) {
    for stmt in &block.stmts {
        match stmt {
            typed::Stmt::Let {
                name,
                mutable,
                value,
                ..
            } => {
                collect_expr_captures(value, scope, seen, captures);
                scope.define(
                    name.clone(),
                    CaptureBinding {
                        ty: value.ty.clone(),
                        mutable: *mutable,
                        origin: BindingOrigin::Local,
                    },
                );
            }
            typed::Stmt::Rebind { value, .. } => {
                collect_expr_captures(value, scope, seen, captures);
            }
            typed::Stmt::Expr(expr) => collect_expr_captures(expr, scope, seen, captures),
        }
    }
    if let Some(result) = &block.result {
        collect_expr_captures(result, scope, seen, captures);
    }
}

fn collect_expr_captures(
    expr: &typed::Expr,
    scope: &mut CaptureScope,
    seen: &mut HashSet<String>,
    captures: &mut Vec<typed::ClosureCapture>,
) {
    match &expr.kind {
        typed::ExprKind::Unit
        | typed::ExprKind::Integer(_)
        | typed::ExprKind::String(_) => {}
        typed::ExprKind::Name(name) => {
            if let Some(binding) = scope.lookup(name) {
                if binding.origin == BindingOrigin::Capturable && seen.insert(name.clone()) {
                    captures.push(typed::ClosureCapture {
                        name: name.clone(),
                        ty: binding.ty.clone(),
                        mutable: binding.mutable,
                    });
                }
            }
        }
        typed::ExprKind::Member { base, .. } => {
            collect_expr_captures(base, scope, seen, captures);
        }
        typed::ExprKind::Call { callee, args, .. } => {
            collect_expr_captures(callee, scope, seen, captures);
            for arg in args {
                match arg {
                    typed::Arg::Positional(expr) => collect_expr_captures(expr, scope, seen, captures),
                    typed::Arg::Named { value, .. } => {
                        collect_expr_captures(value, scope, seen, captures)
                    }
                }
            }
        }
        typed::ExprKind::Closure { params, body, .. } => {
            let mut nested_scope = scope.nested_closure_scope();
            for param in params {
                nested_scope.define(
                    param.name.clone(),
                    CaptureBinding {
                        ty: param.ty.clone(),
                        mutable: false,
                        origin: BindingOrigin::Local,
                    },
                );
            }
            collect_body_captures_from_block_or_expr(body.as_ref(), &mut nested_scope, seen, captures);
        }
        typed::ExprKind::If {
            branches,
            else_branch,
        } => {
            for (condition, block) in branches {
                collect_expr_captures(condition, scope, seen, captures);
                let mut branch_scope = scope.child();
                collect_block_captures(block, &mut branch_scope, seen, captures);
            }
            if let Some(block) = else_branch {
                let mut branch_scope = scope.child();
                collect_block_captures(block, &mut branch_scope, seen, captures);
            }
        }
        typed::ExprKind::Match { scrutinee, arms } => {
            collect_expr_captures(scrutinee, scope, seen, captures);
            for arm in arms {
                let mut arm_scope = scope.child();
                define_pattern_bindings(&arm.pattern, &mut arm_scope);
                collect_block_captures(&arm.body, &mut arm_scope, seen, captures);
            }
        }
        typed::ExprKind::BinaryOp { lhs, rhs, .. } => {
            collect_expr_captures(lhs, scope, seen, captures);
            collect_expr_captures(rhs, scope, seen, captures);
        }
    }
}

fn collect_body_captures_from_block_or_expr(
    body: &typed::ClosureBody,
    scope: &mut CaptureScope,
    seen: &mut HashSet<String>,
    captures: &mut Vec<typed::ClosureCapture>,
) {
    match body {
        typed::ClosureBody::Expr(expr) => collect_expr_captures(expr, scope, seen, captures),
        typed::ClosureBody::Block(block) => collect_block_captures(block, scope, seen, captures),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lower::lower_module, typecheck::typecheck_module};
    use dx_parser::{Lexer, Parser};

    fn typed(src: &str) -> typed::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("parse");
        let hir = lower_module(&ast);
        let report = typecheck_module(&hir);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        report.module
    }

    #[test]
    fn annotates_simple_closure_capture() {
        let module = typed("fun make(x: Int) -> lazy Int:\n    lazy x\n.\n");
        match &module.items[0] {
            typed::Item::Function(function) => match function.body.result.as_ref().map(|expr| &expr.kind) {
                Some(typed::ExprKind::Closure { captures, .. }) => {
                    assert_eq!(
                        captures,
                        &vec![typed::ClosureCapture {
                            name: "x".to_string(),
                            ty: Type::Int,
                            mutable: false,
                        }]
                    );
                }
                other => panic!("expected closure, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn does_not_capture_own_param_or_local() {
        let module = typed("fun make(x: Int) -> (Int) -> Int:\n    (y: Int) =>:\n        val z = y\n        z\n    .\n.\n");
        match &module.items[0] {
            typed::Item::Function(function) => match function.body.result.as_ref().map(|expr| &expr.kind) {
                Some(typed::ExprKind::Closure { captures, .. }) => {
                    assert!(captures.is_empty());
                }
                other => panic!("expected closure, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn nested_closure_use_bubbles_outer_capture() {
        let module = typed("fun make(x: Int) -> lazy lazy Int:\n    lazy:\n        val inner = lazy x\n        inner\n    .\n.\n");
        match &module.items[0] {
            typed::Item::Function(function) => match function.body.result.as_ref().map(|expr| &expr.kind) {
                Some(typed::ExprKind::Closure { captures, body, .. }) => {
                    assert_eq!(captures.len(), 1);
                    assert_eq!(captures[0].name, "x");
                    match body.as_ref() {
                        typed::ClosureBody::Block(block) => match &block.stmts[0] {
                            typed::Stmt::Let { value, .. } => match &value.kind {
                                typed::ExprKind::Closure { captures, .. } => {
                                    assert_eq!(captures.len(), 1);
                                    assert_eq!(captures[0].name, "x");
                                }
                                other => panic!("expected inner closure, got {other:?}"),
                            },
                            other => panic!("expected let, got {other:?}"),
                        },
                        other => panic!("expected block body, got {other:?}"),
                    }
                }
                other => panic!("expected closure, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }
}
