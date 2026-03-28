use crate::low::{
    LowBlock, LowExtern, LowFunction, LowModule, LowParam, LowRuntimeCallKind, LowStep,
    LowTerminator, LowType, LowValue,
};
use dx_hir::Type;
use dx_mir::mir;
use dx_runtime::{
    build_runtime_extern_plan_from_module, build_runtime_ops_plan, build_throw_runtime_plan_from_module,
    ClosureAbiType, RuntimeHookKind, RuntimeOp, RuntimeOpKind,
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
        .map(|(block_id, block)| {
            let mut steps = Vec::new();
            for ((name, bb, stmt), ops) in ops_by_pos.iter() {
                if name != &function.name || *bb != block_id {
                    continue;
                }
                for op in ops {
                    steps.push(lower_runtime_op(op, &function.locals));
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
                terminator: lower_terminator(&block.terminator, &function.locals, &ret),
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

fn lower_runtime_op(op: &RuntimeOp, locals: &[mir::Local]) -> LowStep {
    LowStep::RuntimeCall {
        statement: op.statement,
        destination: op.destination,
        symbol: op.runtime_symbol,
        ret: Some(low_ret_from_runtime_hook(op.hook)),
        kind: match &op.kind {
            RuntimeOpKind::PyCall { arg_count, .. } => LowRuntimeCallKind::PyCall {
                arg_count: *arg_count,
            },
            RuntimeOpKind::ClosureCreate {
                captures,
                param_types,
            } => LowRuntimeCallKind::ClosureCreate {
                captures: captures
                    .iter()
                    .map(|capture| {
                        LowValue::Local(
                            capture.source,
                            low_type_from_dx(&locals[capture.source].ty),
                        )
                    })
                    .collect(),
                arity: param_types.len(),
            },
            RuntimeOpKind::ClosureInvoke {
                closure_local,
                arg_count,
                thunk,
                ..
            } => LowRuntimeCallKind::ClosureInvoke {
                closure: Box::new(LowValue::Local(
                    *closure_local,
                    low_type_from_dx(&locals[*closure_local].ty),
                )),
                arg_count: *arg_count,
                thunk: *thunk,
            },
        },
    }
}

fn low_ret_from_runtime_hook(hook: RuntimeHookKind) -> LowType {
    match hook {
        RuntimeHookKind::Py(_) => LowType::Ptr,
        RuntimeHookKind::Closure(hook) => low_type_from_closure_abi(hook.signature().ret),
        RuntimeHookKind::Throw(_) => LowType::Void,
    }
}

fn low_type_from_closure_abi(ty: ClosureAbiType) -> LowType {
    match ty {
        ClosureAbiType::ClosureHandle | ClosureAbiType::EnvHandle | ClosureAbiType::Ptr => {
            LowType::Ptr
        }
        ClosureAbiType::I64 | ClosureAbiType::U32 => LowType::I64,
        ClosureAbiType::F64 => LowType::F64,
        ClosureAbiType::I1 => LowType::I1,
        ClosureAbiType::Void => LowType::Void,
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

fn lower_terminator(
    terminator: &mir::Terminator,
    locals: &[mir::Local],
    function_ret: &LowType,
) -> LowTerminator {
    match terminator {
        mir::Terminator::Return(value) => match function_ret {
            LowType::Void => LowTerminator::Return(None),
            _ => LowTerminator::Return(value.as_ref().map(|it| lower_operand(it, locals))),
        },
        mir::Terminator::Goto(target) => LowTerminator::Goto(format!("bb{target}")),
        mir::Terminator::SwitchBool {
            cond,
            then_bb,
            else_bb,
        } => LowTerminator::SwitchBool {
            cond: lower_operand(cond, locals),
            then_label: format!("bb{then_bb}"),
            else_label: format!("bb{else_bb}"),
        },
        mir::Terminator::Match {
            scrutinee,
            arms,
            fallback,
        } => LowTerminator::Match {
            scrutinee: lower_operand(scrutinee, locals),
            arms: arms
                .iter()
                .map(|(pattern, target)| (render_pattern(pattern), format!("bb{target}")))
                .collect(),
            fallback: format!("bb{fallback}"),
        },
        mir::Terminator::Unreachable => LowTerminator::Unreachable,
    }
}

fn lower_operand(operand: &mir::Operand, locals: &[mir::Local]) -> LowValue {
    match operand {
        mir::Operand::Copy(local) => LowValue::Local(*local, low_type_from_dx(&locals[*local].ty)),
        mir::Operand::Const(constant) => match constant {
            mir::Constant::Int(value) => LowValue::ConstInt(value.parse().unwrap_or(0)),
            mir::Constant::String(value) => LowValue::ConstString(value.clone()),
            mir::Constant::Unit => LowValue::Unit,
        },
    }
}

fn render_pattern(pattern: &dx_hir::Pattern) -> String {
    match pattern {
        dx_hir::Pattern::Name(name) => name.clone(),
        dx_hir::Pattern::Wildcard => "_".to_string(),
        dx_hir::Pattern::Constructor { name, args } => {
            if args.is_empty() {
                name.clone()
            } else {
                let args = args.iter().map(render_pattern).collect::<Vec<_>>().join(", ");
                format!("{name}({args})")
            }
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
        assert!(low
            .externs
            .iter()
            .any(|ext| ext.symbol == "dx_rt_throw_check_pending"));
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
            LowStep::RuntimeCall { symbol, kind: LowRuntimeCallKind::ClosureCreate { captures, arity: 0 }, .. }
            if *symbol == "dx_rt_closure_create" && captures.len() == 1
        )));
        assert!(steps.iter().any(|step| matches!(
            step,
            LowStep::RuntimeCall { symbol, kind: LowRuntimeCallKind::ClosureInvoke { thunk: true, closure, .. }, .. }
            if symbol.starts_with("dx_rt_thunk_call") && matches!(closure.as_ref(), LowValue::Local(_, LowType::Ptr))
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

    #[test]
    fn lowers_return_terminator() {
        let module = typed_mir("fun f(x: Int) -> Int:\n    x\n.\n");
        let low = lower_module(&module);
        let f = low.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(matches!(
            f.blocks[0].terminator,
            LowTerminator::Return(Some(LowValue::Local(_, LowType::I64)))
        ));
    }

    #[test]
    fn lowers_unit_function_return_to_void_return() {
        let module = typed_mir("fun f(x: Int) -> Unit:\n    x\n.\n");
        let low = lower_module(&module);
        let f = low.functions.iter().find(|f| f.name == "f").expect("f");

        assert_eq!(f.ret, LowType::Void);
        assert!(matches!(f.blocks[0].terminator, LowTerminator::Return(None)));
    }

    #[test]
    fn lowers_if_to_switch_terminator() {
        let module =
            typed_mir("fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n");
        let low = lower_module(&module);
        let f = low.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(f.blocks.iter().any(|bb| matches!(
            bb.terminator,
            LowTerminator::SwitchBool { .. }
        )));
    }

    #[test]
    fn lowers_match_to_match_terminator() {
        let module = typed_mir(
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
        );
        let low = lower_module(&module);
        let f = low.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(f.blocks.iter().any(|bb| matches!(
            bb.terminator,
            LowTerminator::Match { .. }
        )));
    }
}
