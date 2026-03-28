use dx_hir::{typed, Type};
use dx_mir::mir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosureAbiType {
    ClosureHandle,
    EnvHandle,
    U32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosureRuntimeHook {
    Create,
    Call,
    ThunkCall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosureRuntimeHookSignature {
    pub symbol: &'static str,
    pub params: &'static [ClosureAbiType],
    pub ret: ClosureAbiType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoweredClosureCreation {
    pub function: String,
    pub block: mir::BlockId,
    pub statement: usize,
    pub destination: mir::LocalId,
    pub runtime_symbol: &'static str,
    pub captures: Vec<mir::ClosureCapture>,
    pub param_types: Vec<Type>,
    pub return_type: Type,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoweredClosureInvocation {
    pub function: String,
    pub block: mir::BlockId,
    pub statement: usize,
    pub destination: mir::LocalId,
    pub closure_local: mir::LocalId,
    pub target: typed::CallTarget,
    pub runtime_symbol: &'static str,
    pub arg_count: u32,
    pub result_type: Type,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureRuntimePlan {
    pub required_hooks: Vec<ClosureRuntimeHook>,
    pub creations: Vec<LoweredClosureCreation>,
    pub invocations: Vec<LoweredClosureInvocation>,
}

const CREATE_PARAMS: &[ClosureAbiType] = &[ClosureAbiType::EnvHandle, ClosureAbiType::U32];
const CALL_PARAMS: &[ClosureAbiType] = &[ClosureAbiType::ClosureHandle, ClosureAbiType::U32];
const THUNK_CALL_PARAMS: &[ClosureAbiType] = &[ClosureAbiType::ClosureHandle];

impl ClosureRuntimeHook {
    pub fn symbol(self) -> &'static str {
        self.signature().symbol
    }

    pub fn signature(self) -> ClosureRuntimeHookSignature {
        match self {
            ClosureRuntimeHook::Create => ClosureRuntimeHookSignature {
                symbol: "dx_rt_closure_create",
                params: CREATE_PARAMS,
                ret: ClosureAbiType::ClosureHandle,
            },
            ClosureRuntimeHook::Call => ClosureRuntimeHookSignature {
                symbol: "dx_rt_closure_call",
                params: CALL_PARAMS,
                ret: ClosureAbiType::ClosureHandle,
            },
            ClosureRuntimeHook::ThunkCall => ClosureRuntimeHookSignature {
                symbol: "dx_rt_thunk_call",
                params: THUNK_CALL_PARAMS,
                ret: ClosureAbiType::ClosureHandle,
            },
        }
    }
}

pub fn build_closure_runtime_plan(module: &mir::Module) -> ClosureRuntimePlan {
    let mut required_hooks = Vec::new();
    let mut creations = Vec::new();
    let mut invocations = Vec::new();

    for item in &module.items {
        let mir::Item::Function(function) = item else {
            continue;
        };

        for (block_id, block) in function.blocks.iter().enumerate() {
            for (statement_index, stmt) in block.statements.iter().enumerate() {
                let mir::Statement::Assign { place, value } = stmt;
                match value {
                    mir::Rvalue::Closure {
                        captures,
                        param_types,
                        return_type,
                        effects,
                    } => {
                        add_hook(&mut required_hooks, ClosureRuntimeHook::Create);
                        creations.push(LoweredClosureCreation {
                            function: function.name.clone(),
                            block: block_id,
                            statement: statement_index,
                            destination: *place,
                            runtime_symbol: ClosureRuntimeHook::Create.symbol(),
                            captures: captures.clone(),
                            param_types: param_types.clone(),
                            return_type: return_type.clone(),
                            effects: effects.clone(),
                        });
                    }
                    mir::Rvalue::Call {
                        target,
                        callee,
                        args,
                        ty,
                        effects,
                    } => {
                        let Some(closure_local) = local_closure_operand(callee) else {
                            continue;
                        };
                        let Some(hook) = hook_for_closure_call(target, args.len()) else {
                            continue;
                        };
                        add_hook(&mut required_hooks, hook);
                        invocations.push(LoweredClosureInvocation {
                            function: function.name.clone(),
                            block: block_id,
                            statement: statement_index,
                            destination: *place,
                            closure_local,
                            target: target.clone(),
                            runtime_symbol: hook.symbol(),
                            arg_count: args.len() as u32,
                            result_type: ty.clone(),
                            effects: effects.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }

    ClosureRuntimePlan {
        required_hooks,
        creations,
        invocations,
    }
}

fn hook_for_closure_call(
    target: &typed::CallTarget,
    arg_count: usize,
) -> Option<ClosureRuntimeHook> {
    match target {
        typed::CallTarget::LocalClosure { .. } => {
            if arg_count == 0 {
                Some(ClosureRuntimeHook::ThunkCall)
            } else {
                Some(ClosureRuntimeHook::Call)
            }
        }
        _ => None,
    }
}

fn local_closure_operand(operand: &mir::Operand) -> Option<mir::LocalId> {
    match operand {
        mir::Operand::Copy(local) => Some(*local),
        mir::Operand::Const(_) => None,
    }
}

fn add_hook(hooks: &mut Vec<ClosureRuntimeHook>, hook: ClosureRuntimeHook) {
    if !hooks.contains(&hook) {
        hooks.push(hook);
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
    fn records_closure_creation_with_captures() {
        let module = lower("fun make(x: Int) -> lazy Int:\n    lazy x\n.\n");
        let plan = build_closure_runtime_plan(&module);

        assert_eq!(plan.required_hooks, vec![ClosureRuntimeHook::Create]);
        assert_eq!(plan.creations.len(), 1);
        assert_eq!(plan.creations[0].runtime_symbol, "dx_rt_closure_create");
        assert_eq!(plan.creations[0].captures.len(), 1);
        assert_eq!(plan.creations[0].captures[0].name, "x");
        assert_eq!(plan.creations[0].captures[0].ty, Type::Int);
    }

    #[test]
    fn records_thunk_invocation_hook() {
        let module =
            lower("fun use(x: Int) -> Int:\n    val f = lazy x\n    f()\n.\n");
        let plan = build_closure_runtime_plan(&module);

        assert!(plan.required_hooks.contains(&ClosureRuntimeHook::Create));
        assert!(plan.required_hooks.contains(&ClosureRuntimeHook::ThunkCall));
        assert_eq!(plan.invocations.len(), 1);
        assert_eq!(plan.invocations[0].runtime_symbol, "dx_rt_thunk_call");
        assert_eq!(plan.invocations[0].arg_count, 0);
    }

    #[test]
    fn records_closure_call_hook() {
        let module =
            lower("fun use(x: Int) -> Int:\n    val f = (y: Int) => x + y\n    f(1)\n.\n");
        let plan = build_closure_runtime_plan(&module);

        assert!(plan.required_hooks.contains(&ClosureRuntimeHook::Create));
        assert!(plan.required_hooks.contains(&ClosureRuntimeHook::Call));
        assert_eq!(plan.invocations.len(), 1);
        assert_eq!(plan.invocations[0].runtime_symbol, "dx_rt_closure_call");
        assert_eq!(plan.invocations[0].arg_count, 1);
    }

    #[test]
    fn closure_runtime_hook_signatures_are_stable() {
        assert_eq!(
            ClosureRuntimeHook::Create.signature(),
            ClosureRuntimeHookSignature {
                symbol: "dx_rt_closure_create",
                params: CREATE_PARAMS,
                ret: ClosureAbiType::ClosureHandle,
            }
        );
        assert_eq!(
            ClosureRuntimeHook::Call.signature(),
            ClosureRuntimeHookSignature {
                symbol: "dx_rt_closure_call",
                params: CALL_PARAMS,
                ret: ClosureAbiType::ClosureHandle,
            }
        );
        assert_eq!(
            ClosureRuntimeHook::ThunkCall.signature(),
            ClosureRuntimeHookSignature {
                symbol: "dx_rt_thunk_call",
                params: THUNK_CALL_PARAMS,
                ret: ClosureAbiType::ClosureHandle,
            }
        );
    }
}
