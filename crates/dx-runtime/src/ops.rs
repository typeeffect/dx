use crate::abi::RuntimeHook;
use crate::closure::{
    build_closure_runtime_plan, ClosureRuntimeHook, LoweredClosureCreation, LoweredClosureInvocation,
};
use crate::lower::{lower_python_runtime_calls, PyDispatchTarget};
use dx_hir::Type;
use dx_mir::mir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeHookKind {
    Py(RuntimeHook),
    Closure(ClosureRuntimeHook),
}

impl RuntimeHookKind {
    pub fn symbol(self) -> &'static str {
        match self {
            RuntimeHookKind::Py(hook) => hook.symbol(),
            RuntimeHookKind::Closure(hook) => hook.symbol(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeOpKind {
    PyCall {
        dispatch: PyDispatchTarget,
        arg_count: u32,
    },
    ClosureCreate {
        captures: Vec<mir::ClosureCapture>,
        param_types: Vec<Type>,
    },
    ClosureInvoke {
        closure_local: mir::LocalId,
        arg_count: u32,
        thunk: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeOp {
    pub function: String,
    pub block: mir::BlockId,
    pub statement: usize,
    pub destination: Option<mir::LocalId>,
    pub hook: RuntimeHookKind,
    pub runtime_symbol: &'static str,
    pub effects: Vec<String>,
    pub result_type: Option<Type>,
    pub kind: RuntimeOpKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeOpsPlan {
    pub required_hooks: Vec<RuntimeHookKind>,
    pub ops: Vec<RuntimeOp>,
}

pub fn build_runtime_ops_plan(module: &mir::Module) -> RuntimeOpsPlan {
    let py_calls = lower_python_runtime_calls(module);
    let closure_plan = build_closure_runtime_plan(module);
    let local_types = build_local_type_index(module);

    let mut required_hooks = Vec::new();
    let mut ops = Vec::new();

    for call in &py_calls {
        add_hook(&mut required_hooks, RuntimeHookKind::Py(call.hook));
        ops.push(RuntimeOp {
            function: call.function.clone(),
            block: call.block,
            statement: call.statement,
            destination: find_destination_local(module, call.function.as_str(), call.block, call.statement),
            hook: RuntimeHookKind::Py(call.hook),
            runtime_symbol: call.runtime_symbol,
            effects: call.effects.clone(),
            result_type: Some(call.result_type.clone()),
            kind: RuntimeOpKind::PyCall {
                dispatch: call.dispatch.clone(),
                arg_count: call.arg_count,
            },
        });
    }

    for creation in &closure_plan.creations {
        add_hook(
            &mut required_hooks,
            RuntimeHookKind::Closure(ClosureRuntimeHook::Create),
        );
        ops.push(lower_creation(creation, &local_types));
    }

    for invocation in &closure_plan.invocations {
        let hook = match invocation.target {
            dx_hir::typed::CallTarget::LocalClosure { .. } if invocation.arg_count == 0 => {
                ClosureRuntimeHook::ThunkCall
            }
            dx_hir::typed::CallTarget::LocalClosure { .. } => ClosureRuntimeHook::Call,
            _ => continue,
        };
        add_hook(&mut required_hooks, RuntimeHookKind::Closure(hook));
        ops.push(lower_invocation(invocation));
    }

    required_hooks.sort();
    ops.sort_by(|a, b| {
        a.function
            .cmp(&b.function)
            .then(a.block.cmp(&b.block))
            .then(a.statement.cmp(&b.statement))
            .then(a.runtime_symbol.cmp(b.runtime_symbol))
    });

    RuntimeOpsPlan { required_hooks, ops }
}

fn lower_creation(
    creation: &LoweredClosureCreation,
    local_types: &std::collections::HashMap<(String, mir::LocalId), Type>,
) -> RuntimeOp {
    RuntimeOp {
        function: creation.function.clone(),
        block: creation.block,
        statement: creation.statement,
        destination: Some(creation.destination),
        hook: RuntimeHookKind::Closure(ClosureRuntimeHook::Create),
        runtime_symbol: creation.runtime_symbol,
        effects: creation.effects.clone(),
        result_type: local_types
            .get(&(creation.function.clone(), creation.destination))
            .cloned(),
        kind: RuntimeOpKind::ClosureCreate {
            captures: creation.captures.clone(),
            param_types: creation.param_types.clone(),
        },
    }
}

fn lower_invocation(invocation: &LoweredClosureInvocation) -> RuntimeOp {
    RuntimeOp {
        function: invocation.function.clone(),
        block: invocation.block,
        statement: invocation.statement,
        destination: Some(invocation.destination),
        hook: RuntimeHookKind::Closure(match invocation.target {
            dx_hir::typed::CallTarget::LocalClosure { .. } if invocation.arg_count == 0 => {
                ClosureRuntimeHook::ThunkCall
            }
            dx_hir::typed::CallTarget::LocalClosure { .. } => ClosureRuntimeHook::Call,
            _ => unreachable!("non-closure invocation passed to lower_invocation"),
        }),
        runtime_symbol: invocation.runtime_symbol,
        effects: invocation.effects.clone(),
        result_type: Some(invocation.result_type.clone()),
        kind: RuntimeOpKind::ClosureInvoke {
            closure_local: invocation.closure_local,
            arg_count: invocation.arg_count,
            thunk: invocation.arg_count == 0,
        },
    }
}

fn build_local_type_index(
    module: &mir::Module,
) -> std::collections::HashMap<(String, mir::LocalId), Type> {
    let mut out = std::collections::HashMap::new();
    for item in &module.items {
        let mir::Item::Function(function) = item else {
            continue;
        };
        for (local_id, local) in function.locals.iter().enumerate() {
            out.insert((function.name.clone(), local_id), local.ty.clone());
        }
    }
    out
}

fn find_destination_local(
    module: &mir::Module,
    function_name: &str,
    block: mir::BlockId,
    statement: usize,
) -> Option<mir::LocalId> {
    module.items.iter().find_map(|item| {
        let mir::Item::Function(function) = item else {
            return None;
        };
        if function.name != function_name {
            return None;
        }
        let stmt = function.blocks.get(block)?.statements.get(statement)?;
        let mir::Statement::Assign { place, .. } = stmt;
        Some(*place)
    })
}

fn add_hook(hooks: &mut Vec<RuntimeHookKind>, hook: RuntimeHookKind) {
    if !hooks.contains(&hook) {
        hooks.push(hook);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_hir::{lower_module as lower_hir, typecheck_module, Type};
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
    fn merges_python_and_closure_runtime_ops() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str, x: Int) -> PyObj !py:\n    val df = read_csv(path)\n    val thunk = lazy df\n    thunk()\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);

        assert!(plan
            .required_hooks
            .contains(&RuntimeHookKind::Py(RuntimeHook::PyCallFunction)));
        assert!(plan
            .required_hooks
            .contains(&RuntimeHookKind::Closure(ClosureRuntimeHook::Create)));
        assert!(plan
            .required_hooks
            .contains(&RuntimeHookKind::Closure(ClosureRuntimeHook::ThunkCall)));
        assert_eq!(plan.ops.len(), 3);
    }

    #[test]
    fn closure_creation_uses_destination_local_type() {
        let module = lower("fun make(x: Int) -> lazy Int:\n    lazy x\n.\n");
        let plan = build_runtime_ops_plan(&module);
        let op = plan
            .ops
            .iter()
            .find(|op| matches!(op.kind, RuntimeOpKind::ClosureCreate { .. }))
            .expect("closure create op");

        assert_eq!(
            op.result_type,
            Some(Type::Function {
                params: vec![],
                ret: Box::new(Type::Int),
                effects: vec![],
            })
        );
    }

    #[test]
    fn ops_are_stably_sorted_by_position() {
        let module = lower(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    val thunk = lazy read_csv(path)\n    thunk()\n.\n",
        );
        let plan = build_runtime_ops_plan(&module);

        let positions: Vec<_> = plan
            .ops
            .iter()
            .map(|op| (op.function.as_str(), op.block, op.statement, op.runtime_symbol))
            .collect();
        let mut sorted = positions.clone();
        sorted.sort();
        assert_eq!(positions, sorted);
    }
}
