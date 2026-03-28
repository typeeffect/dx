use crate::{hir, typed, types::Type};
use dx_parser::BinOp;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCheckDiagnostic {
    pub function: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeCheckReport {
    pub module: typed::Module,
    pub diagnostics: Vec<TypeCheckDiagnostic>,
}

pub fn typecheck_module(module: &hir::Module) -> TypeCheckReport {
    let globals = collect_globals(module);
    let imported_py = collect_imported_py_names(module);
    let mut checker = Checker {
        globals,
        imported_py,
        diagnostics: Vec::new(),
    };
    let module = checker.typecheck_module(module);
    TypeCheckReport {
        module,
        diagnostics: checker.diagnostics,
    }
}

#[derive(Debug, Clone)]
struct CallableSig {
    param_names: Option<Vec<String>>,
    params: Vec<Type>,
    ret: Type,
}

#[derive(Debug, Clone)]
struct Binding {
    ty: Type,
    mutable: bool,
    callable: Option<CallableSig>,
}

#[derive(Debug, Clone)]
struct Scope {
    layers: Vec<HashMap<String, Binding>>,
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

    fn define(&mut self, name: String, binding: Binding) {
        self.layers
            .last_mut()
            .expect("scope has one layer")
            .insert(name, binding);
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        self.layers.iter().rev().find_map(|layer| layer.get(name))
    }
}

struct Checker {
    globals: HashMap<String, Binding>,
    imported_py: HashSet<String>,
    diagnostics: Vec<TypeCheckDiagnostic>,
}

impl Checker {
    fn typecheck_module(&mut self, module: &hir::Module) -> typed::Module {
        typed::Module {
            items: module
                .items
                .iter()
                .map(|item| self.typecheck_item(item))
                .collect(),
        }
    }

    fn typecheck_item(&mut self, item: &hir::Item) -> typed::Item {
        match item {
            hir::Item::ImportPy(import) => typed::Item::ImportPy(import.clone()),
            hir::Item::Function(function) => typed::Item::Function(self.typecheck_function(function)),
            hir::Item::Statement(stmt) => {
                let mut scope = Scope::new();
                for (name, binding) in &self.globals {
                    scope.define(name.clone(), binding.clone());
                }
                typed::Item::Statement(self.typecheck_stmt("<module>", stmt, &mut scope))
            }
        }
    }

    fn typecheck_function(&mut self, function: &hir::Function) -> typed::Function {
        let mut scope = Scope::new();
        for (name, binding) in &self.globals {
            scope.define(name.clone(), binding.clone());
        }

        let params: Vec<typed::Param> = function
            .params
            .iter()
            .map(|param| typed::Param {
                name: param.name.clone(),
                ty: Type::from_type_expr(&param.ty),
            })
            .collect();

        for param in &params {
            scope.define(
                param.name.clone(),
                Binding {
                    ty: param.ty.clone(),
                    mutable: false,
                    callable: callable_from_param(param),
                },
            );
        }

        let body = self.typecheck_block(&function.name, &function.body, &mut scope);
        let declared_return = function.return_type.as_ref().map(Type::from_type_expr);
        if let Some(expected) = &declared_return {
            if !compatible_types(expected, &body.ty) {
                self.diag(
                    &function.name,
                    format!(
                        "body type {:?} is not assignable to declared return type {:?}",
                        body.ty, expected
                    ),
                );
            }
        }

        typed::Function {
            name: function.name.clone(),
            params,
            return_type: declared_return,
            effects: function.effects.clone(),
            body,
        }
    }

    fn typecheck_block(
        &mut self,
        function_name: &str,
        block: &hir::Block,
        scope: &mut Scope,
    ) -> typed::Block {
        let stmts = block
            .stmts
            .iter()
            .map(|stmt| self.typecheck_stmt(function_name, stmt, scope))
            .collect();

        let result = block
            .result
            .as_ref()
            .map(|expr| Box::new(self.typecheck_expr(function_name, expr, scope)));
        let ty = result
            .as_ref()
            .map(|expr| expr.ty.clone())
            .unwrap_or(Type::Unit);

        typed::Block { stmts, result, ty }
    }

    fn typecheck_stmt(
        &mut self,
        function_name: &str,
        stmt: &hir::Stmt,
        scope: &mut Scope,
    ) -> typed::Stmt {
        match stmt {
            hir::Stmt::Let {
                name,
                mutable,
                value,
                synthetic,
            } => {
                let value = self.typecheck_expr(function_name, value, scope);
                scope.define(
                    name.clone(),
                    Binding {
                        ty: value.ty.clone(),
                        mutable: *mutable,
                        callable: callable_from_typed_expr(&value),
                    },
                );
                typed::Stmt::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    value,
                    synthetic: *synthetic,
                }
            }
            hir::Stmt::Rebind { name, value } => {
                let value = self.typecheck_expr(function_name, value, scope);
                if let Some(binding) = scope.lookup(name) {
                    if !binding.mutable {
                        self.diag(function_name, format!("cannot rebind immutable name `{name}`"));
                    } else if !compatible_types(&binding.ty, &value.ty) {
                        self.diag(
                            function_name,
                            format!(
                                "cannot assign value of type {:?} to `{}` of type {:?}",
                                value.ty, name, binding.ty
                            ),
                        );
                    }
                }
                typed::Stmt::Rebind {
                    name: name.clone(),
                    value,
                }
            }
            hir::Stmt::Expr(expr) => {
                typed::Stmt::Expr(self.typecheck_expr(function_name, expr, scope))
            }
        }
    }

    fn typecheck_expr(
        &mut self,
        function_name: &str,
        expr: &hir::Expr,
        scope: &mut Scope,
    ) -> typed::Expr {
        match expr {
            hir::Expr::Unit => typed::Expr {
                ty: Type::Unit,
                kind: typed::ExprKind::Unit,
            },
            hir::Expr::Name(name) => typed::Expr {
                ty: scope
                    .lookup(name)
                    .map(|binding| binding.ty.clone())
                    .unwrap_or(Type::Unknown),
                kind: typed::ExprKind::Name(name.clone()),
            },
            hir::Expr::Integer(value) => typed::Expr {
                ty: Type::Int,
                kind: typed::ExprKind::Integer(value.clone()),
            },
            hir::Expr::String(value) => typed::Expr {
                ty: Type::Str,
                kind: typed::ExprKind::String(value.clone()),
            },
            hir::Expr::Member { base, name } => {
                let base = Box::new(self.typecheck_expr(function_name, base, scope));
                let ty = match base.ty {
                    Type::PyObj => Type::PyObj,
                    _ => Type::Unknown,
                };
                typed::Expr {
                    ty,
                    kind: typed::ExprKind::Member {
                        base,
                        name: name.clone(),
                    },
                }
            }
            hir::Expr::Call { callee, args } => {
                let callee_typed = Box::new(self.typecheck_expr(function_name, callee, scope));
                let typed_args: Vec<typed::Arg> = args
                    .iter()
                    .map(|arg| match arg {
                        hir::Arg::Positional(expr) => {
                            typed::Arg::Positional(self.typecheck_expr(function_name, expr, scope))
                        }
                        hir::Arg::Named { name, value } => typed::Arg::Named {
                            name: name.clone(),
                            value: self.typecheck_expr(function_name, value, scope),
                        },
                    })
                    .collect();

                let target = self.classify_call_target(callee, &callee_typed, scope);
                let ret = self.infer_call_type(
                    function_name,
                    callee,
                    &callee_typed,
                    &typed_args,
                    scope,
                );
                typed::Expr {
                    ty: ret,
                    kind: typed::ExprKind::Call {
                        target,
                        callee: callee_typed,
                        args: typed_args,
                    },
                }
            }
            hir::Expr::Closure { params, body } => {
                let mut inner = scope.child();
                let typed_params: Vec<typed::ClosureParam> = params
                    .iter()
                    .map(|param| {
                        let ty = param
                            .ty
                            .as_ref()
                            .map(Type::from_type_expr)
                            .unwrap_or(Type::Unknown);
                        inner.define(
                            param.name.clone(),
                            Binding {
                                ty: ty.clone(),
                                mutable: false,
                                callable: callable_from_type(&ty),
                            },
                        );
                        typed::ClosureParam {
                            name: param.name.clone(),
                            ty,
                        }
                    })
                    .collect();

                let typed_body = Box::new(self.typecheck_closure_body(function_name, body, &mut inner));
                let ret = match typed_body.as_ref() {
                    typed::ClosureBody::Expr(expr) => expr.ty.clone(),
                    typed::ClosureBody::Block(block) => block.ty.clone(),
                };
                let effects = infer_typed_closure_body_effects(&typed_body);
                let ty = Type::Function {
                    params: typed_params.iter().map(|p| p.ty.clone()).collect(),
                    ret: Box::new(ret),
                    effects,
                };
                typed::Expr {
                    ty,
                    kind: typed::ExprKind::Closure {
                        params: typed_params,
                        body: typed_body,
                    },
                }
            }
            hir::Expr::If {
                branches,
                else_branch,
            } => {
                let typed_branches: Vec<(typed::Expr, typed::Block)> = branches
                    .iter()
                    .map(|(condition, block)| {
                        let condition = self.typecheck_expr(function_name, condition, scope);
                        if !compatible_types(&Type::Bool, &condition.ty) {
                            self.diag(
                                function_name,
                                format!("if condition should be Bool, found {:?}", condition.ty),
                            );
                        }
                        let mut branch_scope = scope.child();
                        let block = self.typecheck_block(function_name, block, &mut branch_scope);
                        (condition, block)
                    })
                    .collect();
                let typed_else = else_branch.as_ref().map(|block| {
                    let mut else_scope = scope.child();
                    self.typecheck_block(function_name, block, &mut else_scope)
                });
                let ty = unify_branch_types(
                    typed_branches.iter().map(|(_, block)| &block.ty),
                    typed_else.as_ref().map(|block| &block.ty),
                );
                typed::Expr {
                    ty,
                    kind: typed::ExprKind::If {
                        branches: typed_branches,
                        else_branch: typed_else,
                    },
                }
            }
            hir::Expr::Match { scrutinee, arms } => {
                let scrutinee = Box::new(self.typecheck_expr(function_name, scrutinee, scope));
                let typed_arms: Vec<typed::MatchArm> = arms
                    .iter()
                    .map(|arm| {
                        let mut arm_scope = scope.child();
                        bind_pattern(&arm.pattern, &mut arm_scope);
                        let body = self.typecheck_block(function_name, &arm.body, &mut arm_scope);
                        typed::MatchArm {
                            pattern: arm.pattern.clone(),
                            body,
                        }
                    })
                    .collect();
                let ty = unify_branch_types(typed_arms.iter().map(|arm| &arm.body.ty), None);
                typed::Expr {
                    ty,
                    kind: typed::ExprKind::Match {
                        scrutinee,
                        arms: typed_arms,
                    },
                }
            }
            hir::Expr::BinaryOp { op, lhs, rhs } => {
                let lhs = Box::new(self.typecheck_expr(function_name, lhs, scope));
                let rhs = Box::new(self.typecheck_expr(function_name, rhs, scope));
                let ty = self.infer_binary_type(function_name, op, &lhs.ty, &rhs.ty);
                typed::Expr {
                    ty,
                    kind: typed::ExprKind::BinaryOp {
                        op: op.clone(),
                        lhs,
                        rhs,
                    },
                }
            }
        }
    }

    fn typecheck_closure_body(
        &mut self,
        function_name: &str,
        body: &hir::ClosureBody,
        scope: &mut Scope,
    ) -> typed::ClosureBody {
        match body {
            hir::ClosureBody::Expr(expr) => {
                typed::ClosureBody::Expr(Box::new(self.typecheck_expr(function_name, expr, scope)))
            }
            hir::ClosureBody::Block(block) => {
                typed::ClosureBody::Block(Box::new(self.typecheck_block(function_name, block, scope)))
            }
        }
    }

    fn infer_call_type(
        &mut self,
        function_name: &str,
        callee_hir: &hir::Expr,
        callee_typed: &typed::Expr,
        args: &[typed::Arg],
        scope: &Scope,
    ) -> Type {
        if let hir::Expr::Name(name) = callee_hir {
            if self.imported_py.contains(name) {
                return Type::PyObj;
            }
            if let Some(binding) = scope.lookup(name).and_then(|binding| binding.callable.clone()) {
                self.check_call_args(function_name, Some(name), &binding, args);
                return binding.ret;
            }
        }

        if let Type::Function {
            params, ret, ..
        } = &callee_typed.ty
        {
            let sig = CallableSig {
                param_names: None,
                params: params.clone(),
                ret: (**ret).clone(),
            };
            self.check_call_args(function_name, None, &sig, args);
            return (**ret).clone();
        }

        if matches!(callee_typed.ty, Type::PyObj) {
            return Type::PyObj;
        }

        Type::Unknown
    }

    fn infer_binary_type(
        &mut self,
        function_name: &str,
        op: &BinOp,
        lhs: &Type,
        rhs: &Type,
    ) -> Type {
        match op {
            BinOp::Add => match (lhs, rhs) {
                (Type::Int, Type::Int) => Type::Int,
                (Type::Str, Type::Str) => Type::Str,
                (l, r) if l.is_unknown() || r.is_unknown() => Type::Unknown,
                _ => {
                    self.diag(
                        function_name,
                        format!("`+` is not defined for {:?} and {:?}", lhs, rhs),
                    );
                    Type::Unknown
                }
            },
            BinOp::Sub | BinOp::Mul => match (lhs, rhs) {
                (Type::Int, Type::Int) => Type::Int,
                (l, r) if l.is_unknown() || r.is_unknown() => Type::Unknown,
                _ => {
                    self.diag(
                        function_name,
                        format!("operator {:?} is not defined for {:?} and {:?}", op, lhs, rhs),
                    );
                    Type::Unknown
                }
            },
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq | BinOp::EqEq => {
                if !lhs.is_unknown() && !rhs.is_unknown() && lhs != rhs {
                    self.diag(
                        function_name,
                        format!(
                            "comparison {:?} uses incompatible operand types {:?} and {:?}",
                            op, lhs, rhs
                        ),
                    );
                }
                Type::Bool
            }
        }
    }

    fn diag(&mut self, function: &str, message: String) {
        self.diagnostics.push(TypeCheckDiagnostic {
            function: function.to_string(),
            message,
        });
    }

    fn check_call_args(
        &mut self,
        function_name: &str,
        callee_name: Option<&str>,
        sig: &CallableSig,
        args: &[typed::Arg],
    ) {
        let mut seen_named = false;
        let mut assigned = vec![false; sig.params.len()];
        let mut positional_index = 0usize;

        for arg in args {
            match arg {
                typed::Arg::Positional(expr) => {
                    if seen_named {
                        self.diag(
                            function_name,
                            format!(
                                "positional argument cannot appear after named arguments{}",
                                call_site_suffix(callee_name)
                            ),
                        );
                        continue;
                    }
                    if positional_index >= sig.params.len() {
                        self.diag(
                            function_name,
                            format!(
                                "call{} expects {} args, found {}",
                                call_site_suffix(callee_name),
                                sig.params.len(),
                                args.len()
                            ),
                        );
                        continue;
                    }
                    self.check_arg_type(
                        function_name,
                        callee_name,
                        positional_index,
                        &sig.params[positional_index],
                        &expr.ty,
                    );
                    assigned[positional_index] = true;
                    positional_index += 1;
                }
                typed::Arg::Named { name, value } => {
                    seen_named = true;
                    let Some(param_names) = &sig.param_names else {
                        self.diag(
                            function_name,
                            format!(
                                "call{} uses named arguments, but callee has no parameter names",
                                call_site_suffix(callee_name)
                            ),
                        );
                        continue;
                    };
                    let Some(index) = param_names.iter().position(|param| param == name) else {
                        self.diag(
                            function_name,
                            format!(
                                "unknown named argument `{}`{}",
                                name,
                                call_site_suffix(callee_name)
                            ),
                        );
                        continue;
                    };
                    if assigned[index] {
                        self.diag(
                            function_name,
                            format!(
                                "argument `{}` provided more than once{}",
                                name,
                                call_site_suffix(callee_name)
                            ),
                        );
                        continue;
                    }
                    self.check_arg_type(
                        function_name,
                        callee_name,
                        index,
                        &sig.params[index],
                        &value.ty,
                    );
                    assigned[index] = true;
                }
            }
        }

        let provided = assigned.iter().filter(|&&slot| slot).count();
        if provided != sig.params.len() {
            self.diag(
                function_name,
                format!(
                    "call{} expects {} args, found {}",
                    call_site_suffix(callee_name),
                    sig.params.len(),
                    provided
                ),
            );
        }
    }

    fn check_arg_type(
        &mut self,
        function_name: &str,
        callee_name: Option<&str>,
        index: usize,
        expected: &Type,
        actual: &Type,
    ) {
        if !compatible_types(expected, actual) {
            self.diag(
                function_name,
                format!(
                    "argument {}{} expects {:?}, found {:?}",
                    index + 1,
                    call_site_suffix(callee_name),
                    expected,
                    actual
                ),
            );
        }
    }

    fn classify_call_target(
        &self,
        callee_hir: &hir::Expr,
        callee_typed: &typed::Expr,
        scope: &Scope,
    ) -> typed::CallTarget {
        match callee_hir {
            hir::Expr::Name(name) if self.imported_py.contains(name) => {
                typed::CallTarget::PythonFunction { name: name.clone() }
            }
            hir::Expr::Member { name, .. } if matches!(callee_typed.ty, Type::PyObj) => {
                typed::CallTarget::PythonMember { name: name.clone() }
            }
            hir::Expr::Name(name) => {
                if let Some(binding) = scope.lookup(name) {
                    if binding.callable.is_some() {
                        if self.globals.contains_key(name) {
                            typed::CallTarget::NativeFunction { name: name.clone() }
                        } else {
                            typed::CallTarget::LocalClosure { name: name.clone() }
                        }
                    } else {
                        typed::CallTarget::Dynamic
                    }
                } else {
                    typed::CallTarget::Dynamic
                }
            }
            _ if matches!(callee_typed.ty, Type::PyObj) => typed::CallTarget::PythonDynamic,
            _ => typed::CallTarget::Dynamic,
        }
    }
}

fn collect_globals(module: &hir::Module) -> HashMap<String, Binding> {
    let mut globals = HashMap::new();
    for item in &module.items {
        match item {
            hir::Item::ImportPy(import) => {
                for name in &import.names {
                    globals.insert(
                        name.clone(),
                        Binding {
                            ty: Type::Unknown,
                            mutable: false,
                            callable: Some(CallableSig {
                                param_names: None,
                                params: vec![],
                                ret: Type::PyObj,
                            }),
                        },
                    );
                }
            }
            hir::Item::Function(function) => {
                let params: Vec<Type> =
                    function.params.iter().map(|param| Type::from_type_expr(&param.ty)).collect();
                let ret = function
                    .return_type
                    .as_ref()
                    .map(Type::from_type_expr)
                    .unwrap_or(Type::Unit);
                globals.insert(
                    function.name.clone(),
                    Binding {
                        ty: Type::Function {
                            params: params.clone(),
                            ret: Box::new(ret.clone()),
                            effects: function.effects.clone(),
                        },
                        mutable: false,
                        callable: Some(CallableSig {
                            param_names: Some(
                                function.params.iter().map(|param| param.name.clone()).collect(),
                            ),
                            params,
                            ret,
                        }),
                    },
                );
            }
            hir::Item::Statement(_) => {}
        }
    }
    globals
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

fn callable_from_type(ty: &Type) -> Option<CallableSig> {
    match ty {
        Type::Function { params, ret, .. } => Some(CallableSig {
            param_names: None,
            params: params.clone(),
            ret: (**ret).clone(),
        }),
        _ => None,
    }
}

fn callable_from_param(param: &typed::Param) -> Option<CallableSig> {
    callable_from_type(&param.ty)
}

fn callable_from_typed_expr(expr: &typed::Expr) -> Option<CallableSig> {
    match &expr.kind {
        typed::ExprKind::Closure { params, body } => {
            let ret = match body.as_ref() {
                typed::ClosureBody::Expr(expr) => expr.ty.clone(),
                typed::ClosureBody::Block(block) => block.ty.clone(),
            };
            Some(CallableSig {
                param_names: Some(params.iter().map(|param| param.name.clone()).collect()),
                params: params.iter().map(|param| param.ty.clone()).collect(),
                ret,
            })
        }
        _ => callable_from_type(&expr.ty),
    }
}

fn call_site_suffix(callee_name: Option<&str>) -> String {
    callee_name
        .map(|name| format!(" to `{name}`"))
        .unwrap_or_default()
}

fn infer_typed_closure_body_effects(body: &typed::ClosureBody) -> Vec<String> {
    let mut effects = BTreeSet::new();
    match body {
        typed::ClosureBody::Expr(expr) => infer_typed_expr_effects(expr, &mut effects),
        typed::ClosureBody::Block(block) => infer_typed_block_effects(block, &mut effects),
    }
    effects.into_iter().collect()
}

fn infer_typed_block_effects(block: &typed::Block, effects: &mut BTreeSet<String>) {
    for stmt in &block.stmts {
        match stmt {
            typed::Stmt::Let { value, .. } => infer_typed_expr_effects(value, effects),
            typed::Stmt::Rebind { value, .. } => infer_typed_expr_effects(value, effects),
            typed::Stmt::Expr(expr) => infer_typed_expr_effects(expr, effects),
        }
    }
    if let Some(result) = &block.result {
        infer_typed_expr_effects(result, effects);
    }
}

fn infer_typed_expr_effects(expr: &typed::Expr, effects: &mut BTreeSet<String>) {
    match &expr.kind {
        typed::ExprKind::Unit
        | typed::ExprKind::Name(_)
        | typed::ExprKind::Integer(_)
        | typed::ExprKind::String(_) => {}
        typed::ExprKind::Member { base, .. } => infer_typed_expr_effects(base, effects),
        typed::ExprKind::Call {
            target,
            callee,
            args,
        } => {
            infer_typed_expr_effects(callee, effects);
            for arg in args {
                match arg {
                    typed::Arg::Positional(expr) => infer_typed_expr_effects(expr, effects),
                    typed::Arg::Named { value, .. } => infer_typed_expr_effects(value, effects),
                }
            }

            match target {
                typed::CallTarget::PythonFunction { .. }
                | typed::CallTarget::PythonMember { .. }
                | typed::CallTarget::PythonDynamic => {
                    effects.insert("py".to_string());
                }
                typed::CallTarget::NativeFunction { .. }
                | typed::CallTarget::LocalClosure { .. }
                | typed::CallTarget::Dynamic => {}
            }

            if let Type::Function { effects: call_fx, .. } = &callee.ty {
                effects.extend(call_fx.iter().cloned());
            }
        }
        typed::ExprKind::Closure { .. } => {}
        typed::ExprKind::If {
            branches,
            else_branch,
        } => {
            for (condition, block) in branches {
                infer_typed_expr_effects(condition, effects);
                infer_typed_block_effects(block, effects);
            }
            if let Some(block) = else_branch {
                infer_typed_block_effects(block, effects);
            }
        }
        typed::ExprKind::Match { scrutinee, arms } => {
            infer_typed_expr_effects(scrutinee, effects);
            for arm in arms {
                infer_typed_block_effects(&arm.body, effects);
            }
        }
        typed::ExprKind::BinaryOp { lhs, rhs, .. } => {
            infer_typed_expr_effects(lhs, effects);
            infer_typed_expr_effects(rhs, effects);
        }
    }
}

fn bind_pattern(pattern: &hir::Pattern, scope: &mut Scope) {
    match pattern {
        hir::Pattern::Name(name) => scope.define(
            name.clone(),
            Binding {
                ty: Type::Unknown,
                mutable: false,
                callable: None,
            },
        ),
        hir::Pattern::Wildcard => {}
        hir::Pattern::Constructor { args, .. } => {
            for arg in args {
                bind_pattern(arg, scope);
            }
        }
    }
}

fn unify_branch_types<'a>(
    types: impl Iterator<Item = &'a Type>,
    extra: Option<&'a Type>,
) -> Type {
    let mut acc: Option<Type> = None;
    for ty in types.chain(extra.into_iter()) {
        match &acc {
            None => acc = Some(ty.clone()),
            Some(current) if compatible_types(current, ty) => {}
            Some(_) => return Type::Unknown,
        }
    }
    acc.unwrap_or(Type::Unit)
}

fn compatible_types(expected: &Type, actual: &Type) -> bool {
    expected == actual || expected.is_unknown() || actual.is_unknown()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
    use dx_parser::{Lexer, Parser};

    fn check(src: &str) -> TypeCheckReport {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_module(&ast);
        typecheck_module(&hir)
    }

    #[test]
    fn infers_int_addition() {
        let report = check(
            "fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => {
                assert_eq!(function.body.ty, Type::Int);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn infers_string_concat() {
        let report = check(
            "fun full() -> Str:\n    \"a\" + \"b\"\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => {
                assert_eq!(function.body.ty, Type::Str);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn comparison_is_bool() {
        let report = check(
            "fun test(x: Int) -> Bool:\n    x <= 1\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => {
                assert_eq!(function.body.ty, Type::Bool);
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn rebind_type_mismatch_reports_diagnostic() {
        let report = check(
            "fun test() -> Unit:\n    var x = 1\n    x = \"oops\"\n.\n",
        );
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("cannot assign value of type"));
    }

    #[test]
    fn function_call_uses_declared_return_type() {
        let report = check(
            "fun inner() -> Int:\n    1\n.\n\nfun outer() -> Int:\n    inner()\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[1] {
            typed::Item::Function(function) => assert_eq!(function.body.ty, Type::Int),
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn reports_argument_type_mismatch_for_named_call() {
        let report = check(
            "fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n\nfun use() -> Int:\n    add(a: 1, b: \"x\")\n.\n",
        );
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("argument 2"));
        assert!(report.diagnostics[0].message.contains("Int"));
        assert!(report.diagnostics[0].message.contains("Str"));
    }

    #[test]
    fn reports_unknown_named_argument() {
        let report = check(
            "fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n\nfun use() -> Int:\n    add(a: 1, c: 2)\n.\n",
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown named argument `c`")));
    }

    #[test]
    fn reports_duplicate_named_argument() {
        let report = check(
            "fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n\nfun use() -> Int:\n    add(a: 1, a: 2)\n.\n",
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("provided more than once")));
    }

    #[test]
    fn reports_positional_after_named_argument() {
        let report = check(
            "fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n\nfun use() -> Int:\n    add(a: 1, 2)\n.\n",
        );
        assert!(report
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("positional argument cannot appear after named")));
    }

    #[test]
    fn local_closure_named_args_use_parameter_names() {
        let report = check(
            "fun use() -> Int:\n    val f = (x: Int, y: Int) => x + y\n    f(x: 1, y: 2)\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => assert_eq!(function.body.ty, Type::Int),
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn closure_gets_function_type() {
        let report = check(
            "fun make() -> lazy Int:\n    val f = lazy 1\n    f\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => match &function.body.stmts[0] {
                typed::Stmt::Let { value, .. } => match &value.ty {
                    Type::Function { params, ret, .. } => {
                        assert!(params.is_empty());
                        assert_eq!(ret.as_ref(), &Type::Int);
                    }
                    other => panic!("expected function type, got {other:?}"),
                },
                other => panic!("expected let, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn closure_type_carries_py_effects() {
        let report = check(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py:\n    val f = lazy read_csv(path)\n    f\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[1] {
            typed::Item::Function(function) => match &function.body.stmts[0] {
                typed::Stmt::Let { value, .. } => match &value.ty {
                    Type::Function { effects, ret, .. } => {
                        assert_eq!(ret.as_ref(), &Type::PyObj);
                        assert_eq!(effects, &vec!["py".to_string()]);
                    }
                    other => panic!("expected function type, got {other:?}"),
                },
                other => panic!("expected let, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn classifies_python_function_calls() {
        let report = check(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[1] {
            typed::Item::Function(function) => match function.body.result.as_ref().map(|e| &e.kind) {
                Some(typed::ExprKind::Call { target, .. }) => {
                    assert_eq!(
                        target,
                        &typed::CallTarget::PythonFunction {
                            name: "read_csv".to_string()
                        }
                    );
                }
                other => panic!("expected call result, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn classifies_python_member_calls() {
        let report = check(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[1] {
            typed::Item::Function(function) => {
                assert_eq!(function.body.ty, Type::PyObj);
                match function.body.result.as_ref().map(|e| &e.kind) {
                    Some(typed::ExprKind::Call { target, .. }) => {
                        assert_eq!(
                            target,
                            &typed::CallTarget::PythonMember {
                                name: "head".to_string()
                            }
                        );
                    }
                    other => panic!("expected call result, got {other:?}"),
                }
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn classifies_native_function_calls() {
        let report = check(
            "fun inner() -> Int:\n    1\n.\n\nfun outer() -> Int:\n    inner()\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[1] {
            typed::Item::Function(function) => match function.body.result.as_ref().map(|e| &e.kind) {
                Some(typed::ExprKind::Call { target, .. }) => {
                    assert_eq!(
                        target,
                        &typed::CallTarget::NativeFunction {
                            name: "inner".to_string()
                        }
                    );
                }
                other => panic!("expected call result, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn classifies_local_closure_calls() {
        let report = check(
            "fun outer() -> Int:\n    val f = lazy 1\n    f()\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => match function.body.result.as_ref().map(|e| &e.kind) {
                Some(typed::ExprKind::Call { target, .. }) => {
                    assert_eq!(
                        target,
                        &typed::CallTarget::LocalClosure {
                            name: "f".to_string()
                        }
                    );
                }
                other => panic!("expected call result, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn if_unifies_branch_types() {
        let report = check(
            "fun test(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        );
        assert!(report.diagnostics.is_empty());
        match &report.module.items[0] {
            typed::Item::Function(function) => assert_eq!(function.body.ty, Type::Int),
            other => panic!("expected function, got {other:?}"),
        }
    }
}
