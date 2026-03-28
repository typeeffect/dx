use crate::llvm::{
    Block, ExternDecl, Function, Instruction, Module, Operand, Param, Type,
};
use dx_codegen::{low, LowModule, LowRuntimeCallKind, LowStep};

pub fn lower_module(low: &LowModule) -> Module {
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
        .map(|function| Function {
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
                        .map(lower_instruction)
                        .collect(),
                })
                .collect(),
        })
        .collect();

    Module { externs, functions }
}

fn lower_instruction(step: &LowStep) -> Instruction {
    match step {
        LowStep::RuntimeCall {
            statement,
            destination,
            symbol,
            ret,
            kind,
        } => Instruction::CallExtern {
            result: destination.map(|local| format!("%{}", local)),
            symbol,
            ret: ret.as_ref().map(lower_type).unwrap_or(Type::Void),
            args: runtime_call_args(kind),
            comment: Some(format!("stmt={statement}, {}", runtime_call_comment(kind))),
        },
        LowStep::ThrowCheck {
            statement,
            symbol,
            boundary,
        } => Instruction::CallExtern {
            result: None,
            symbol,
            ret: Type::Void,
            args: vec![],
            comment: Some(format!("stmt={statement}, throw-boundary={boundary:?}")),
        },
    }
}

fn runtime_call_args(kind: &LowRuntimeCallKind) -> Vec<Operand> {
    match kind {
        LowRuntimeCallKind::PyCall { arg_count } => vec![Operand::ConstInt(i64::from(*arg_count))],
        LowRuntimeCallKind::ClosureCreate {
            capture_count,
            arity,
        } => vec![
            Operand::ConstInt(*capture_count as i64),
            Operand::ConstInt(*arity as i64),
        ],
        LowRuntimeCallKind::ClosureInvoke { arg_count, thunk } => {
            let mut out = vec![Operand::Register("%closure".into(), Type::Ptr)];
            if !thunk {
                out.push(Operand::ConstInt(i64::from(*arg_count)));
            }
            out
        }
    }
}

fn runtime_call_comment(kind: &LowRuntimeCallKind) -> String {
    match kind {
        LowRuntimeCallKind::PyCall { arg_count } => format!("py-call args={arg_count}"),
        LowRuntimeCallKind::ClosureCreate {
            capture_count,
            arity,
        } => format!("closure-create captures={capture_count} arity={arity}"),
        LowRuntimeCallKind::ClosureInvoke { arg_count, thunk } => {
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
            Instruction::CallExtern { symbol, .. } if *symbol == "dx_rt_thunk_call"
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
}
