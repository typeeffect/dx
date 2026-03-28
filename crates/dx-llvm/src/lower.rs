use crate::llvm::{
    Block, ExternDecl, Function, GlobalString, Instruction, Module, Operand, Param, Terminator,
    Type,
};
use dx_codegen::{low, LowAssignValue, LowModule, LowRuntimeCallKind, LowStep, LowTerminator, LowValue};
use std::collections::BTreeMap;

pub fn lower_module(low: &LowModule) -> Module {
    let mut state = LoweringState::default();
    let externs = low
        .externs
        .iter()
        .map(|ext| ExternDecl {
            symbol: ext.symbol,
            params: ext.params.iter().map(lower_type).collect(),
            ret: lower_type(&ext.ret),
        })
        .collect();

    let functions = low
        .functions
        .iter()
        .map(|function| lower_function(function, &mut state))
        .collect();

    Module {
        globals: state.globals,
        externs,
        functions,
    }
}

#[derive(Default)]
struct LoweringState {
    globals: Vec<GlobalString>,
    string_pool: BTreeMap<String, String>,
}

impl LoweringState {
    fn intern_string(&mut self, value: &str) -> String {
        if let Some(symbol) = self.string_pool.get(value) {
            return symbol.clone();
        }

        let symbol = format!(".str{}", self.globals.len());
        self.globals.push(GlobalString {
            symbol: symbol.clone(),
            value: value.to_string(),
        });
        self.string_pool.insert(value.to_string(), symbol.clone());
        symbol
    }
}

fn lower_function(function: &low::LowFunction, state: &mut LoweringState) -> Function {
    Function {
        name: function.name.clone(),
        params: function
            .params
            .iter()
            .map(|param| Param {
                name: format!("%{}", param.local),
                ty: lower_type(&param.ty),
            })
            .collect(),
        ret: lower_type(&function.ret),
        blocks: function
            .blocks
            .iter()
            .map(|block| Block {
                label: block.label.clone(),
                instructions: block
                    .steps
                    .iter()
                    .flat_map(|step| lower_instructions(step, state))
                    .collect(),
                terminator: lower_terminator(&block.terminator, state),
            })
            .collect(),
    }
}

fn lower_terminator(term: &LowTerminator, state: &mut LoweringState) -> Terminator {
    match term {
        LowTerminator::Return(value) => {
            Terminator::Ret(value.as_ref().map(|value| lower_value(value, state)))
        }
        LowTerminator::Goto(target) => Terminator::Br(target.clone()),
        LowTerminator::SwitchBool {
            cond,
            then_label,
            else_label,
        } => Terminator::CondBr {
            cond: lower_value(cond, state),
            then_label: then_label.clone(),
            else_label: else_label.clone(),
        },
        LowTerminator::Match {
            scrutinee,
            arms,
            fallback,
        } => Terminator::MatchBr {
            scrutinee: lower_value(scrutinee, state),
            arms: arms.clone(),
            fallback: fallback.clone(),
        },
        LowTerminator::Unreachable => Terminator::Unreachable,
    }
}

fn lower_instructions(step: &LowStep, state: &mut LoweringState) -> Vec<Instruction> {
    match step {
        LowStep::Assign {
            destination,
            ty,
            value,
        } => match value {
            LowAssignValue::Use(value) => vec![Instruction::Assign {
                result: format!("%{}", destination),
                ty: lower_type(ty),
                value: lower_value(value, state),
            }],
            LowAssignValue::BinaryOp { op, lhs, rhs } => vec![Instruction::BinaryOp {
                result: format!("%{}", destination),
                op: op.clone(),
                ty: lower_type(ty),
                lhs: lower_value(lhs, state),
                rhs: lower_value(rhs, state),
            }],
        },
        LowStep::RuntimeCall {
            statement,
            destination,
            symbol,
            ret,
            kind,
        } => match kind {
            LowRuntimeCallKind::ClosureCreate { captures, arity } => {
                let env = format!("%env_{statement}");
                vec![
                    Instruction::PackEnv {
                        result: env.clone(),
                        captures: captures
                            .iter()
                            .map(|capture| lower_value(capture, state))
                            .collect(),
                    },
                    Instruction::CallExtern {
                        result: destination.map(|local| format!("%{}", local)),
                        symbol,
                        ret: ret.as_ref().map(lower_type).unwrap_or(Type::Void),
                        args: vec![Operand::Register(env, Type::Ptr), Operand::ConstInt(*arity as i64)],
                        comment: Some(format!("stmt={statement}, {}", runtime_call_comment(kind))),
                    },
                ]
            }
            _ => vec![Instruction::CallExtern {
                result: destination.map(|local| format!("%{}", local)),
                symbol,
                ret: ret.as_ref().map(lower_type).unwrap_or(Type::Void),
                args: runtime_call_args(kind, state),
                comment: Some(format!("stmt={statement}, {}", runtime_call_comment(kind))),
            }],
        },
        LowStep::ThrowCheck {
            statement,
            symbol,
            boundary,
        } => vec![Instruction::CallExtern {
            result: None,
            symbol,
            ret: Type::Void,
            args: vec![],
            comment: Some(format!("stmt={statement}, throw-boundary={boundary:?}")),
        }],
    }
}

fn runtime_call_args(kind: &LowRuntimeCallKind, state: &mut LoweringState) -> Vec<Operand> {
    match kind {
        LowRuntimeCallKind::PyCall { args, .. } => {
            args.iter().map(|arg| lower_value(arg, state)).collect()
        }
        LowRuntimeCallKind::ClosureCreate { .. } => unreachable!("closure create lowered separately"),
        LowRuntimeCallKind::ClosureInvoke {
            closure,
            arg_count,
            thunk,
        } => {
            let mut out = vec![lower_value(closure, state)];
            if !thunk {
                out.push(Operand::ConstInt(i64::from(*arg_count)));
            }
            out
        }
    }
}

fn runtime_call_comment(kind: &LowRuntimeCallKind) -> String {
    match kind {
        LowRuntimeCallKind::PyCall { arg_count, args } => {
            format!("py-call args={arg_count} abi_args={}", args.len())
        }
        LowRuntimeCallKind::ClosureCreate {
            captures,
            arity,
        } => format!("closure-create captures={} arity={arity}", captures.len()),
        LowRuntimeCallKind::ClosureInvoke {
            arg_count,
            thunk,
            ..
        } => {
            if *thunk {
                "thunk-call".to_string()
            } else {
                format!("closure-call args={arg_count}")
            }
        }
    }
}

fn lower_type(ty: &low::LowType) -> Type {
    match ty {
        low::LowType::I64 => Type::I64,
        low::LowType::F64 => Type::Double,
        low::LowType::I1 => Type::I1,
        low::LowType::Ptr => Type::Ptr,
        low::LowType::Void => Type::Void,
    }
}

fn lower_value(value: &LowValue, state: &mut LoweringState) -> Operand {
    match value {
        LowValue::Local(local, ty) => Operand::Register(format!("%{}", local), lower_type(ty)),
        LowValue::ConstInt(v) => Operand::ConstInt(*v),
        LowValue::ConstString(value) => Operand::Global(state.intern_string(value), Type::Ptr),
        LowValue::Unit => Operand::Register("%unit".into(), Type::Ptr),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_parser::{Lexer, Parser};

    fn typed_mir(src: &str) -> dx_mir::mir::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        dx_mir::lower_module(&typed.module)
    }

    #[test]
    fn lowers_python_call_to_llvm_like_call() {
        let mir = typed_mir(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);

        assert!(llvm
            .externs
            .iter()
            .any(|ext| ext.symbol == "dx_rt_py_call_function"));
        let run = llvm.functions.iter().find(|f| f.name == "run").expect("run");
        assert!(run.blocks[0].instructions.iter().any(|it| matches!(
            it,
            Instruction::CallExtern { symbol, .. } if *symbol == "dx_rt_py_call_function"
        )));
        assert!(run.blocks[0].instructions.iter().any(|it| matches!(
            it,
            Instruction::CallExtern { symbol, .. } if *symbol == "dx_rt_throw_check_pending"
        )));
    }

    #[test]
    fn lowers_assignments_to_llvm_like_instructions() {
        let mir = typed_mir("fun f() -> Int:\n    val y = 42\n    y\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(matches!(
            f.blocks[0].instructions.first(),
            Some(Instruction::Assign { ty: Type::I64, .. })
        ));
    }

    #[test]
    fn lowers_binary_ops_to_llvm_like_instructions() {
        let mir = typed_mir("fun f(x: Int) -> Int:\n    val y = x + 1\n    y\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(f.blocks[0].instructions.iter().any(|it| matches!(
            it,
            Instruction::BinaryOp { op: dx_parser::BinOp::Add, ty: Type::I64, .. }
        )));
    }

    #[test]
    fn lowers_closure_runtime_ops_to_llvm_like_calls() {
        let mir = typed_mir("fun run(x: Int) -> Int:\n    val thunk = lazy x\n    thunk()\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let run = llvm.functions.iter().find(|f| f.name == "run").expect("run");

        assert!(run.blocks[0].instructions.iter().any(|it| matches!(
            it,
            Instruction::CallExtern { symbol, .. } if *symbol == "dx_rt_closure_create"
        )));
        assert!(run.blocks[0].instructions.iter().any(|it| matches!(
            it,
            Instruction::CallExtern { symbol, .. } if symbol.starts_with("dx_rt_thunk_call")
        )));
    }

    #[test]
    fn render_module_produces_llvm_like_text() {
        let mir = typed_mir(
            "from py pandas import read_csv\n\nfun run(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let out = crate::display::render_module(&llvm);

        assert!(out.contains("declare ptr @dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("define ptr @run"), "got:\n{out}");
        assert!(out.contains("call ptr @dx_rt_py_call_function"), "got:\n{out}");
        assert!(out.contains("call void @dx_rt_throw_check_pending()"), "got:\n{out}");
    }

    #[test]
    fn lowers_string_literals_to_globals() {
        let mir = typed_mir("fun f() -> Str:\n    \"hello\"\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);

        assert_eq!(llvm.globals.len(), 1);
        assert_eq!(llvm.globals[0].value, "hello");
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");
        assert!(matches!(
            f.blocks[0].terminator,
            Terminator::Ret(Some(Operand::Global(_, Type::Ptr)))
        ));
    }

    #[test]
    fn deduplicates_identical_string_literals() {
        let mir = typed_mir("fun a() -> Str:\n    \"hello\"\n.\n\nfun b() -> Str:\n    \"hello\"\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);

        assert_eq!(llvm.globals.len(), 1);
        assert_eq!(llvm.globals[0].value, "hello");
    }

    #[test]
    fn lowers_return_terminator_to_llvm_like_ret() {
        let mir = typed_mir("fun f(x: Int) -> Int:\n    x\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(matches!(
            f.blocks[0].terminator,
            Terminator::Ret(Some(Operand::Register(_, Type::I64)))
        ));
    }

    #[test]
    fn lowers_unit_function_to_ret_void() {
        let mir = typed_mir("fun f(x: Int) -> Unit:\n    x\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        assert_eq!(f.ret, Type::Void);
        assert!(matches!(f.blocks[0].terminator, Terminator::Ret(None)));
    }

    #[test]
    fn lowers_if_to_cond_br() {
        let mir =
            typed_mir("fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n");
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(f.blocks.iter().any(|bb| matches!(
            bb.terminator,
            Terminator::CondBr { .. }
        )));
    }

    #[test]
    fn lowers_match_to_match_br() {
        let mir = typed_mir(
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
        );
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        assert!(f.blocks.iter().any(|bb| matches!(
            bb.terminator,
            Terminator::MatchBr { .. }
        )));
    }
}
