use crate::llvm::{
    Block, ExternDecl, Function, GlobalString, Instruction, Module, Operand, Param, Terminator,
    Type,
};
use dx_codegen::{
    low, LowAssignValue, LowCallArg, LowModule, LowRuntimeCallKind, LowStep, LowTerminator,
    LowValue,
};
use std::collections::BTreeMap;

pub fn lower_module(low: &LowModule) -> Module {
    let mut state = LoweringState::default();
    let mut externs: Vec<ExternDecl> = low
        .externs
        .iter()
        .map(|ext| ExternDecl {
            symbol: ext.symbol,
            params: ext.params.iter().map(lower_type).collect(),
            ret: lower_type(&ext.ret),
        })
        .collect();

    let functions: Vec<Function> = low
        .functions
        .iter()
        .map(|function| lower_function(function, &mut state))
        .collect();

    // If match lowering was used, inject the match_tag runtime extern
    if state.needs_match_tag {
        externs.push(ExternDecl {
            symbol: "dx_rt_match_tag",
            params: vec![Type::Ptr, Type::Ptr],
            ret: Type::I1,
        });
        externs.sort_by_key(|e| e.symbol);
    }

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
    needs_match_tag: bool,
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
    let mut extra_blocks: Vec<Block> = Vec::new();
    let mut block_counter = function.blocks.len();

    let blocks: Vec<Block> = function
        .blocks
        .iter()
        .map(|block| {
            let instructions: Vec<Instruction> = block
                .steps
                .iter()
                .flat_map(|step| lower_instructions(step, state))
                .collect();

            let terminator = lower_terminator_maybe_expand(
                &block.terminator,
                state,
                &mut extra_blocks,
                &mut block_counter,
            );

            Block {
                label: block.label.clone(),
                instructions,
                terminator,
            }
        })
        .collect();

    let mut all_blocks = blocks;
    all_blocks.append(&mut extra_blocks);

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
        blocks: all_blocks,
    }
}

fn lower_terminator_maybe_expand(
    term: &LowTerminator,
    state: &mut LoweringState,
    extra_blocks: &mut Vec<Block>,
    block_counter: &mut usize,
) -> Terminator {
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
        } => {
            state.needs_match_tag = true;

            if arms.is_empty() {
                return Terminator::Br(fallback.clone());
            }

            let scrutinee_op = lower_value(scrutinee, state);

            // Generate one check block per arm. Each block:
            //   1. calls dx_rt_match_tag(scrutinee, "Pattern") -> i1
            //   2. if true: branch to arm body
            //   3. if false: branch to next check (or fallback)
            let mut check_labels: Vec<String> = Vec::new();
            for _ in 0..arms.len() {
                let label = format!("match_check_{}", *block_counter);
                *block_counter += 1;
                check_labels.push(label);
            }

            for (i, (pattern, target)) in arms.iter().enumerate() {
                let next = if i + 1 < arms.len() {
                    check_labels[i + 1].clone()
                } else {
                    fallback.clone()
                };

                let cmp_name = format!("%match_cmp_{}", &check_labels[i]);
                let tag_global = state.intern_string(pattern);

                extra_blocks.push(Block {
                    label: check_labels[i].clone(),
                    instructions: vec![Instruction::CallExtern {
                        result: Some(cmp_name.clone()),
                        symbol: "dx_rt_match_tag",
                        ret: Type::I1,
                        args: vec![
                            scrutinee_op.clone(),
                            Operand::Global(tag_global, Type::Ptr),
                        ],
                        comment: Some(format!("match pattern={pattern}")),
                    }],
                    terminator: Terminator::CondBr {
                        cond: Operand::Register(cmp_name, Type::I1),
                        then_label: target.clone(),
                        else_label: next,
                    },
                });
            }

            // Current block branches to first check
            Terminator::Br(check_labels[0].clone())
        }
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
            LowRuntimeCallKind::ClosureCreate {
                captures,
                arity,
                entry_function,
            } => {
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
                        args: vec![
                            Operand::Global(entry_function.clone(), Type::Ptr),
                            Operand::Register(env, Type::Ptr),
                            Operand::ConstInt(*arity as i64),
                            Operand::ConstInt(captures.len() as i64),
                        ],
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
            args,
            thunk,
            ..
        } => {
            let mut out = vec![lower_value(closure, state)];
            if !thunk {
                // Pass actual call arguments after the closure handle
                for arg in args {
                    match arg {
                        LowCallArg::Positional(value) => out.push(lower_value(value, state)),
                        LowCallArg::Named { value, .. } => out.push(lower_value(value, state)),
                    }
                }
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
            entry_function,
        } => format!(
            "closure-create captures={} arity={arity} entry={entry_function}",
            captures.len()
        ),
        LowRuntimeCallKind::ClosureInvoke {
            arg_count,
            thunk,
            args,
            ..
        } => {
            if *thunk {
                "thunk-call".to_string()
            } else {
                format!(
                    "closure-call args={arg_count} call_args=[{}]",
                    args.iter()
                        .map(render_low_call_arg)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

fn render_low_call_arg(arg: &LowCallArg) -> String {
    match arg {
        LowCallArg::Positional(value) => render_low_value(value),
        LowCallArg::Named { name, value } => format!("{name}: {}", render_low_value(value)),
    }
}

fn render_low_value(value: &LowValue) -> String {
    match value {
        LowValue::Local(local, ty) => format!("_{}: {}", local, render_llvm_type(&lower_type(ty))),
        LowValue::ConstInt(v) => v.to_string(),
        LowValue::ConstString(s) => format!("{s:?}"),
        LowValue::Unit => "()".to_string(),
    }
}

fn render_llvm_type(ty: &Type) -> &'static str {
    match ty {
        Type::I64 => "i64",
        Type::Double => "double",
        Type::I1 => "i1",
        Type::Ptr => "ptr",
        Type::Void => "void",
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
    fn preserves_closure_call_args_in_llvm_comments() {
        let mir = typed_mir(
            "fun run(x: Int) -> Int:\n    val f = (y: Int) => x + y\n    f(1)\n.\n",
        );
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let run = llvm.functions.iter().find(|f| f.name == "run").expect("run");

        assert!(run.blocks[0].instructions.iter().any(|it| matches!(
            it,
            Instruction::CallExtern { symbol, comment: Some(comment), .. }
            if symbol.starts_with("dx_rt_closure_call")
                && comment.contains("closure-call args=1")
                && comment.contains("call_args=[1]")
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
    fn lowers_match_to_cond_br_chain() {
        let mir = typed_mir(
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
        );
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let f = llvm.functions.iter().find(|f| f.name == "f").expect("f");

        // Match should be lowered to check blocks with CondBr, not MatchBr
        assert!(
            !f.blocks.iter().any(|bb| matches!(bb.terminator, Terminator::MatchBr { .. })),
            "should not contain MatchBr after lowering"
        );
        // Should have check blocks with dx_rt_match_tag calls
        assert!(f.blocks.iter().any(|bb| bb.instructions.iter().any(|it| matches!(
            it,
            Instruction::CallExtern { symbol, .. } if *symbol == "dx_rt_match_tag"
        ))));
        // Should have dx_rt_match_tag extern
        assert!(llvm.externs.iter().any(|e| e.symbol == "dx_rt_match_tag"));
    }

    #[test]
    fn closure_call_passes_real_args() {
        let mir = typed_mir(
            "fun run(x: Int) -> Int:\n    val f = (y: Int) => x + y\n    f(1)\n.\n",
        );
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        let run = llvm.functions.iter().find(|f| f.name == "run").expect("run");

        // Find the closure call instruction
        let closure_call = run.blocks[0].instructions.iter().find(|it| matches!(
            it,
            Instruction::CallExtern { symbol, .. } if symbol.starts_with("dx_rt_closure_call")
        ));
        assert!(closure_call.is_some(), "should have closure call");
        if let Some(Instruction::CallExtern { args, .. }) = closure_call {
            // First arg: closure handle (ptr)
            assert!(matches!(args[0], Operand::Register(_, Type::Ptr)), "first arg should be closure handle");
            // Should have more than just closure handle — should have the actual call arg
            assert!(args.len() > 1, "closure call should pass real args, got {} args", args.len());
        }
    }
}
