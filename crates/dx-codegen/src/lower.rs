use crate::low::{
    LowBlock, LowExtern, LowFunction, LowModule, LowParam, LowRuntimeCallKind, LowStep, LowType,
};
use dx_hir::Type;
use dx_mir::mir;
use dx_runtime::{
    build_runtime_extern_plan_from_module, build_runtime_ops_plan, build_throw_runtime_plan_from_module,
    RuntimeOp, RuntimeOpKind,
};
use std::collections::BTreeMap;

pub fn lower_module(module: &mir::Module) -> LowModule {
    let extern_plan = build_runtime_extern_plan_from_module(module);
    let ops_plan = build_runtime_ops_plan(module);
    let throw_plan = build_throw_runtime_plan_from_module(module);

    let externs = extern_plan
        .externs
        .into_iter()
        .map(|ext| LowExtern {
            symbol: ext.signature.symbol,
            params: ext
                .signature
                .params
                .into_iter()
                .map(LowType::from_runtime_abi)
                .collect(),
            ret: LowType::from_runtime_abi(ext.signature.ret),
        })
        .collect();

    let mut ops_by_pos: BTreeMap<(String, mir::BlockId, usize), Vec<&RuntimeOp>> = BTreeMap::new();
    for op in &ops_plan.ops {
        ops_by_pos
            .entry((op.function.clone(), op.block, op.statement))
            .or_default()
            .push(op);
    }

    let mut throw_by_pos: BTreeMap<(String, mir::BlockId, usize), Vec<&dx_runtime::LoweredThrowSite>> =
        BTreeMap::new();
    for site in &throw_plan.sites {
        throw_by_pos
            .entry((site.function.clone(), site.block, site.statement))
            .or_default()
            .push(site);
    }

    let functions = module
        .items
        .iter()
        .filter_map(|item| match item {
            mir::Item::Function(function) => Some(lower_function(function, &ops_by_pos, &throw_by_pos)),
            mir::Item::ImportPy(_) => None,
        })
        .collect();

    LowModule { externs, functions }
}

fn lower_function(
    function: &mir::Function,
    ops_by_pos: &BTreeMap<(String, mir::BlockId, usize), Vec<&RuntimeOp>>,
    throw_by_pos: &BTreeMap<(String, mir::BlockId, usize), Vec<&dx_runtime::LoweredThrowSite>>,
) -> LowFunction {
    let params = function
        .params
        .iter()
        .map(|local| LowParam {
            local: *local,
            ty: low_type_from_dx(&function.locals[*local].ty),
        })
        .collect();

    let ret = function
        .return_type
        .as_ref()
        .map(low_type_from_dx)
        .unwrap_or(LowType::Void);

    let blocks = function
        .blocks
        .iter()
        .enumerate()
        .map(|(block_id, _)| {
            let mut steps = Vec::new();
            for ((name, bb, stmt), ops) in ops_by_pos.iter() {
                if name != &function.name || *bb != block_id {
                    continue;
                }
                for op in ops {
                    steps.push(lower_runtime_op(op));
                }
                if let Some(sites) = throw_by_pos.get(&(name.clone(), *bb, *stmt)) {
                    for site in sites {
                        steps.push(LowStep::ThrowCheck {
                            statement: *stmt,
                            symbol: dx_runtime::ThrowRuntimeHook::CheckPending.symbol(),
                            boundary: site.boundary.clone(),
                        });
                    }
                }
            }
            LowBlock {
                label: format!("bb{block_id}"),
                steps,
            }
        })
        .collect();

    LowFunction {
        name: function.name.clone(),
        params,
        ret,
        blocks,
    }
}

fn lower_runtime_op(op: &RuntimeOp) -> LowStep {
    LowStep::RuntimeCall {
        statement: op.statement,
        destination: op.destination,
        symbol: op.runtime_symbol,
        ret: op.result_type.as_ref().map(low_type_from_dx),
        kind: match &op.kind {
            RuntimeOpKind::PyCall { arg_count, .. } => LowRuntimeCallKind::PyCall {
                arg_count: *arg_count,
            },
            RuntimeOpKind::ClosureCreate {
                captures,
                param_types,
            } => LowRuntimeCallKind::ClosureCreate {
                capture_count: captures.len(),
                arity: param_types.len(),
            },
            RuntimeOpKind::ClosureInvoke {
                arg_count,
                thunk,
                ..
            } => LowRuntimeCallKind::ClosureInvoke {
                arg_count: *arg_count,
                thunk: *thunk,
            },
        },
    }
}

fn low_type_from_dx(ty: &Type) -> LowType {
    match ty {
        Type::Int => LowType::I64,
        Type::Float => LowType::F64,
        Type::Bool => LowType::I1,
        Type::Unit => LowType::Void,
        Type::Str | Type::PyObj | Type::Named(_) | Type::Function { .. } | Type::Unknown => {
            LowType::Ptr
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_parser::{Lexer, Parser};

    fn typed_mir(src: &str) -> mir::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        dx_mir::lower_module(&typed.module)
    }

    #[test]
    fn lowers_runtime_externs_into_low_module() {
        let module = typed_mir(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let low = lower_module(&module);

        assert!(low.externs.iter().any(|ext| ext.symbol == "dx_rt_py_call_function"));
        assert!(low.functions.iter().any(|f| f.name == "run"));
    }

    #[test]
    fn lowers_runtime_call_and_throw_check_steps() {
        let module = typed_mir(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        let low = lower_module(&module);
        let run = low.functions.iter().find(|f| f.name == "run").expect("run");
        let steps = &run.blocks[0].steps;

        assert!(steps.iter().any(|step| matches!(
            step,
            LowStep::RuntimeCall { symbol, kind: LowRuntimeCallKind::PyCall { .. }, .. }
            if *symbol == "dx_rt_py_call_function"
        )));
        assert!(steps.iter().any(|step| matches!(
            step,
            LowStep::ThrowCheck { symbol, .. } if *symbol == "dx_rt_throw_check_pending"
        )));
    }

    #[test]
    fn closure_create_and_invoke_lower_to_runtime_steps() {
        let module = typed_mir(
            "fun run(x: Int) -> Int:\n    val thunk = lazy x\n    thunk()\n.\n",
        );
        let low = lower_module(&module);
        let run = low.functions.iter().find(|f| f.name == "run").expect("run");
        let steps = &run.blocks[0].steps;

        assert!(steps.iter().any(|step| matches!(
            step,
            LowStep::RuntimeCall { symbol, kind: LowRuntimeCallKind::ClosureCreate { capture_count: 1, arity: 0 }, .. }
            if *symbol == "dx_rt_closure_create"
        )));
        assert!(steps.iter().any(|step| matches!(
            step,
            LowStep::RuntimeCall { symbol, kind: LowRuntimeCallKind::ClosureInvoke { thunk: true, .. }, .. }
            if *symbol == "dx_rt_thunk_call"
        )));
    }

    #[test]
    fn low_function_signature_uses_lowered_param_and_return_types() {
        let module = typed_mir("fun f(x: Int, y: Bool) -> Int:\n    x\n.\n");
        let low = lower_module(&module);
        let f = low.functions.iter().find(|f| f.name == "f").expect("f");

        assert_eq!(f.params[0].ty, LowType::I64);
        assert_eq!(f.params[1].ty, LowType::I1);
        assert_eq!(f.ret, LowType::I64);
    }
}
