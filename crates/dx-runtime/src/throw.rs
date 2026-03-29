use crate::lower::PyDispatchTarget;
use crate::ops::{build_runtime_ops_plan, RuntimeOpKind, RuntimeOpsPlan};
use dx_mir::mir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThrowRuntimeHook {
    CheckPending,
}

impl ThrowRuntimeHook {
    pub fn symbol(self) -> &'static str {
        match self {
            ThrowRuntimeHook::CheckPending => "dx_rt_throw_check_pending",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThrowBoundaryKind {
    PythonFunction,
    PythonMethod,
    PythonDynamic,
    ClosureCall,
    ThunkCall,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoweredThrowSite {
    pub function: String,
    pub block: mir::BlockId,
    pub statement: usize,
    pub source_runtime_symbol: &'static str,
    pub boundary: ThrowBoundaryKind,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThrowRuntimePlan {
    pub required_hooks: Vec<ThrowRuntimeHook>,
    pub sites: Vec<LoweredThrowSite>,
}

pub fn build_throw_runtime_plan_from_module(module: &mir::Module) -> ThrowRuntimePlan {
    let ops = build_runtime_ops_plan(module);
    build_throw_runtime_plan(&ops)
}

pub fn build_throw_runtime_plan(plan: &RuntimeOpsPlan) -> ThrowRuntimePlan {
    let mut sites = Vec::new();

    for op in &plan.ops {
        let Some(boundary) = classify_throw_boundary(op) else {
            continue;
        };

        sites.push(LoweredThrowSite {
            function: op.function.clone(),
            block: op.block,
            statement: op.statement,
            source_runtime_symbol: op.runtime_symbol,
            boundary,
            effects: op.effects.clone(),
        });
    }

    sites.sort_by(|a, b| {
        a.function
            .cmp(&b.function)
            .then(a.block.cmp(&b.block))
            .then(a.statement.cmp(&b.statement))
            .then(a.source_runtime_symbol.cmp(b.source_runtime_symbol))
    });

    let required_hooks = if sites.is_empty() {
        vec![]
    } else {
        vec![ThrowRuntimeHook::CheckPending]
    };

    ThrowRuntimePlan {
        required_hooks,
        sites,
    }
}

fn classify_throw_boundary(op: &crate::ops::RuntimeOp) -> Option<ThrowBoundaryKind> {
    match &op.kind {
        RuntimeOpKind::PyCall { dispatch, .. } => Some(match dispatch {
            PyDispatchTarget::Function { .. } => ThrowBoundaryKind::PythonFunction,
            PyDispatchTarget::Method { .. } => ThrowBoundaryKind::PythonMethod,
            PyDispatchTarget::Dynamic => ThrowBoundaryKind::PythonDynamic,
        }),
        RuntimeOpKind::ClosureInvoke { thunk, .. }
            if op.effects.iter().any(|effect| effect == "throw") =>
        {
            Some(if *thunk {
                ThrowBoundaryKind::ThunkCall
            } else {
                ThrowBoundaryKind::ClosureCall
            })
        }
        RuntimeOpKind::ClosureCreate { .. } => None,
        RuntimeOpKind::ClosureInvoke { .. } => None,
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
    fn python_throwing_call_creates_throw_site() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        let plan = build_throw_runtime_plan_from_module(&module);

        assert_eq!(plan.required_hooks, vec![ThrowRuntimeHook::CheckPending]);
        assert_eq!(plan.sites.len(), 1);
        assert_eq!(plan.sites[0].boundary, ThrowBoundaryKind::PythonFunction);
        assert_eq!(plan.sites[0].source_runtime_symbol, "dx_rt_py_call_function");
    }

    #[test]
    fn closure_creation_with_throw_effect_is_not_immediate_throw_site() {
        let module = lower(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py !throw:\n    lazy read_csv(path)\n.\n",
        );
        let plan = build_throw_runtime_plan_from_module(&module);

        assert_eq!(plan.required_hooks, vec![ThrowRuntimeHook::CheckPending]);
        assert_eq!(plan.sites.len(), 1);
        assert_eq!(plan.sites[0].function, "make$closure$0");
        assert_eq!(plan.sites[0].boundary, ThrowBoundaryKind::PythonFunction);
        assert_eq!(plan.sites[0].source_runtime_symbol, "dx_rt_py_call_function");
    }

    #[test]
    fn thunk_invocation_with_throw_effect_creates_throw_site() {
        let module = lower(
            "fun run(thunk: lazy PyObj !py !throw) -> PyObj !py !throw:\n    thunk()\n.\n",
        );
        let plan = build_throw_runtime_plan_from_module(&module);

        assert_eq!(plan.required_hooks, vec![ThrowRuntimeHook::CheckPending]);
        assert_eq!(plan.sites.len(), 1);
        assert_eq!(plan.sites[0].boundary, ThrowBoundaryKind::ThunkCall);
        assert_eq!(plan.sites[0].source_runtime_symbol, "dx_rt_thunk_call_ptr");
    }

    #[test]
    fn closure_call_with_throw_effect_creates_throw_site() {
        let module = lower(
            "fun run(f: (PyObj) -> PyObj !py !throw, df: PyObj) -> PyObj !py !throw:\n    f(df)\n.\n",
        );
        let plan = build_throw_runtime_plan_from_module(&module);

        assert_eq!(plan.required_hooks, vec![ThrowRuntimeHook::CheckPending]);
        assert_eq!(plan.sites.len(), 1);
        assert_eq!(plan.sites[0].boundary, ThrowBoundaryKind::ClosureCall);
        assert!(plan.sites[0]
            .source_runtime_symbol
            .starts_with("dx_rt_closure_call_ptr_"));
    }

    #[test]
    fn throw_sites_are_sorted_stably() {
        let module = lower(
            "from py pandas import read_csv\n\nfun a(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n\nfun b(path: Str) -> PyObj !py !throw:\n    val thunk = lazy read_csv(path)\n    thunk()\n.\n",
        );
        let plan = build_throw_runtime_plan_from_module(&module);
        let order: Vec<_> = plan
            .sites
            .iter()
            .map(|site| {
                (
                    site.function.as_str(),
                    site.block,
                    site.statement,
                    site.source_runtime_symbol,
                )
            })
            .collect();

        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
    }
}
