use crate::mir;
use dx_hir::{typed, Type};
use std::collections::HashMap;

pub fn lower_module(module: &typed::Module) -> mir::Module {
    let mut lowerer = Lowerer::default();
    lowerer.lower_module(module)
}

#[derive(Default)]
struct Lowerer;

impl Lowerer {
    fn lower_module(&mut self, module: &typed::Module) -> mir::Module {
        mir::Module {
            items: module
                .items
                .iter()
                .filter_map(|item| match item {
                    typed::Item::ImportPy(import) => Some(mir::Item::ImportPy(import.clone())),
                    typed::Item::Function(function) => {
                        Some(mir::Item::Function(self.lower_function(function)))
                    }
                    typed::Item::Statement(_) => None,
                })
                .collect(),
        }
    }

    fn lower_function(&mut self, function: &typed::Function) -> mir::Function {
        let mut builder = FunctionBuilder::new(function.name.clone(), function.return_type.clone(), function.effects.clone());
        let mut env = HashMap::new();

        let params = function
            .params
            .iter()
            .map(|param| {
                let local = builder.alloc_local(param.name.clone(), param.ty.clone(), false, false);
                env.insert(param.name.clone(), local);
                local
            })
            .collect();

        let entry = builder.entry;
        self.lower_block_tail(&function.body, &mut builder, entry, &env, Tail::Return);

        mir::Function {
            name: builder.name,
            params,
            locals: builder.locals,
            blocks: builder.blocks,
            return_type: builder.return_type,
            effects: builder.effects,
        }
    }

    fn lower_block_tail(
        &mut self,
        block: &typed::Block,
        builder: &mut FunctionBuilder,
        start_bb: mir::BlockId,
        env: &HashMap<String, mir::LocalId>,
        tail: Tail,
    ) {
        let mut env = env.clone();
        let mut current_bb = start_bb;

        for stmt in &block.stmts {
            match stmt {
                typed::Stmt::Let {
                    name,
                    mutable,
                    value,
                    synthetic,
                } => {
                    let local =
                        builder.alloc_local(name.clone(), value.ty.clone(), *mutable, *synthetic);
                    env.insert(name.clone(), local);
                    current_bb = self.lower_expr_assign(value, local, builder, current_bb, &env);
                }
                typed::Stmt::Rebind { name, value } => {
                    if let Some(local) = env.get(name).copied() {
                        current_bb =
                            self.lower_expr_assign(value, local, builder, current_bb, &env);
                    }
                }
                typed::Stmt::Expr(expr) => {
                    let temp = builder.alloc_temp(expr.ty.clone());
                    current_bb = self.lower_expr_assign(expr, temp, builder, current_bb, &env);
                }
            }
        }

        match (&block.result, tail) {
            (Some(result), Tail::Return) => {
                let (current_bb, operand) =
                    self.lower_expr_operand_in_block(result, builder, current_bb, &env);
                builder.set_terminator(current_bb, mir::Terminator::Return(Some(operand)));
            }
            (None, Tail::Return) => {
                builder.set_terminator(
                    current_bb,
                    mir::Terminator::Return(Some(mir::Operand::Const(mir::Constant::Unit))),
                );
            }
            (Some(result), Tail::AssignAndGoto { dest, join_bb }) => {
                current_bb = self.lower_expr_assign(result, dest, builder, current_bb, &env);
                builder.set_terminator(current_bb, mir::Terminator::Goto(join_bb));
            }
            (None, Tail::AssignAndGoto { dest, join_bb }) => {
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::Unit)),
                    },
                );
                builder.set_terminator(current_bb, mir::Terminator::Goto(join_bb));
            }
        }
    }

    fn lower_expr_assign(
        &mut self,
        expr: &typed::Expr,
        dest: mir::LocalId,
        builder: &mut FunctionBuilder,
        current_bb: mir::BlockId,
        env: &HashMap<String, mir::LocalId>,
    ) -> mir::BlockId {
        match &expr.kind {
            typed::ExprKind::Unit => {
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::Unit)),
                    },
                );
                current_bb
            }
            typed::ExprKind::Name(name) => {
                if let Some(local) = env.get(name) {
                    builder.push_statement(
                        current_bb,
                        mir::Statement::Assign {
                            place: dest,
                            value: mir::Rvalue::Use(mir::Operand::Copy(*local)),
                        },
                    );
                }
                current_bb
            }
            typed::ExprKind::Integer(value) => {
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::Int(
                            value.clone(),
                        ))),
                    },
                );
                current_bb
            }
            typed::ExprKind::String(value) => {
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::String(
                            value.clone(),
                        ))),
                    },
                );
                current_bb
            }
            typed::ExprKind::Member { base, name } => {
                let (current_bb, base_operand) =
                    self.lower_expr_operand_in_block(base, builder, current_bb, env);
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Member {
                            base: base_operand,
                            name: name.clone(),
                        },
                    },
                );
                current_bb
            }
            typed::ExprKind::BinaryOp { op, lhs, rhs } => {
                let (current_bb, lhs) =
                    self.lower_expr_operand_in_block(lhs, builder, current_bb, env);
                let (current_bb, rhs) =
                    self.lower_expr_operand_in_block(rhs, builder, current_bb, env);
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::BinaryOp {
                            op: op.clone(),
                            lhs,
                            rhs,
                        },
                    },
                );
                current_bb
            }
            typed::ExprKind::Call {
                target,
                callee,
                args,
            } => {
                let (mut current_bb, callee) =
                    self.lower_expr_operand_in_block(callee, builder, current_bb, env);
                let mut lowered_args = Vec::with_capacity(args.len());
                for arg in args {
                    match arg {
                        typed::Arg::Positional(expr) => {
                            let (next_bb, value) =
                                self.lower_expr_operand_in_block(expr, builder, current_bb, env);
                            current_bb = next_bb;
                            lowered_args.push(mir::CallArg::Positional(value));
                        }
                        typed::Arg::Named { name, value } => {
                            let (next_bb, value) =
                                self.lower_expr_operand_in_block(value, builder, current_bb, env);
                            current_bb = next_bb;
                            lowered_args.push(mir::CallArg::Named {
                                name: name.clone(),
                                value,
                            });
                        }
                    }
                }
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Call {
                            target: target.clone(),
                            callee,
                            args: lowered_args,
                            ty: expr.ty.clone(),
                        },
                    },
                );
                current_bb
            }
            typed::ExprKind::Closure { params, body } => {
                let return_type = match body.as_ref() {
                    typed::ClosureBody::Expr(expr) => expr.ty.clone(),
                    typed::ClosureBody::Block(block) => block.ty.clone(),
                };
                builder.push_statement(
                    current_bb,
                    mir::Statement::Assign {
                        place: dest,
                        value: mir::Rvalue::Closure {
                            param_types: params.iter().map(|p| p.ty.clone()).collect(),
                            return_type,
                        },
                    },
                );
                current_bb
            }
            typed::ExprKind::If {
                branches,
                else_branch,
            } => self.lower_if_expr(branches, else_branch.as_ref(), dest, builder, current_bb, env),
            typed::ExprKind::Match { scrutinee, arms } => {
                self.lower_match_expr(scrutinee, arms, dest, builder, current_bb, env)
            }
        }
    }

    fn lower_expr_operand_in_block(
        &mut self,
        expr: &typed::Expr,
        builder: &mut FunctionBuilder,
        current_bb: mir::BlockId,
        env: &HashMap<String, mir::LocalId>,
    ) -> (mir::BlockId, mir::Operand) {
        match &expr.kind {
            typed::ExprKind::Unit => (current_bb, mir::Operand::Const(mir::Constant::Unit)),
            typed::ExprKind::Name(name) => env
                .get(name)
                .copied()
                .map(mir::Operand::Copy)
                .map(|operand| (current_bb, operand))
                .unwrap_or((current_bb, mir::Operand::Const(mir::Constant::Unit))),
            typed::ExprKind::Integer(value) => {
                (current_bb, mir::Operand::Const(mir::Constant::Int(value.clone())))
            }
            typed::ExprKind::String(value) => {
                (current_bb, mir::Operand::Const(mir::Constant::String(value.clone())))
            }
            _ => {
                let temp = builder.alloc_temp(expr.ty.clone());
                let current_bb = self.lower_expr_assign(expr, temp, builder, current_bb, env);
                (current_bb, mir::Operand::Copy(temp))
            }
        }
    }

    fn lower_if_expr(
        &mut self,
        branches: &[(typed::Expr, typed::Block)],
        else_branch: Option<&typed::Block>,
        dest: mir::LocalId,
        builder: &mut FunctionBuilder,
        start_bb: mir::BlockId,
        env: &HashMap<String, mir::LocalId>,
    ) -> mir::BlockId {
        let join_bb = builder.new_block();
        let mut current_cond_bb = start_bb;

        for (index, (condition, block)) in branches.iter().enumerate() {
            let then_bb = builder.new_block();
            let else_bb = if index + 1 == branches.len() {
                if else_branch.is_some() {
                    builder.new_block()
                } else {
                    join_bb
                }
            } else {
                builder.new_block()
            };

            let (next_cond_bb, cond) =
                self.lower_expr_operand_in_block(condition, builder, current_cond_bb, env);
            builder.set_terminator(
                next_cond_bb,
                mir::Terminator::SwitchBool {
                    cond,
                    then_bb,
                    else_bb,
                },
            );

            self.lower_block_tail(
                block,
                builder,
                then_bb,
                env,
                Tail::AssignAndGoto {
                    dest,
                    join_bb,
                },
            );

            current_cond_bb = else_bb;
        }

        if let Some(block) = else_branch {
            self.lower_block_tail(
                block,
                builder,
                current_cond_bb,
                env,
                Tail::AssignAndGoto { dest, join_bb },
            );
        } else if current_cond_bb != join_bb {
            builder.push_statement(
                current_cond_bb,
                mir::Statement::Assign {
                    place: dest,
                    value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::Unit)),
                },
            );
            builder.set_terminator(current_cond_bb, mir::Terminator::Goto(join_bb));
        }

        join_bb
    }

    fn lower_match_expr(
        &mut self,
        scrutinee: &typed::Expr,
        arms: &[typed::MatchArm],
        dest: mir::LocalId,
        builder: &mut FunctionBuilder,
        start_bb: mir::BlockId,
        env: &HashMap<String, mir::LocalId>,
    ) -> mir::BlockId {
        let join_bb = builder.new_block();
        let fallback_bb = builder.new_block();
        let (match_bb, scrutinee) =
            self.lower_expr_operand_in_block(scrutinee, builder, start_bb, env);

        let mut arm_targets = Vec::new();
        for arm in arms {
            let arm_bb = builder.new_block();
            arm_targets.push((arm.pattern.clone(), arm_bb));
            self.lower_block_tail(
                &arm.body,
                builder,
                arm_bb,
                env,
                Tail::AssignAndGoto { dest, join_bb },
            );
        }

        builder.set_terminator(
            match_bb,
            mir::Terminator::Match {
                scrutinee,
                arms: arm_targets,
                fallback: fallback_bb,
            },
        );

        builder.push_statement(
            fallback_bb,
            mir::Statement::Assign {
                place: dest,
                value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::Unit)),
            },
        );
        builder.set_terminator(fallback_bb, mir::Terminator::Goto(join_bb));

        join_bb
    }
}

#[derive(Clone, Copy)]
enum Tail {
    Return,
    AssignAndGoto { dest: mir::LocalId, join_bb: mir::BlockId },
}

struct FunctionBuilder {
    name: String,
    return_type: Option<Type>,
    effects: Vec<String>,
    entry: mir::BlockId,
    locals: Vec<mir::Local>,
    blocks: Vec<mir::BasicBlock>,
}

impl FunctionBuilder {
    fn new(name: String, return_type: Option<Type>, effects: Vec<String>) -> Self {
        Self {
            name,
            return_type,
            effects,
            entry: 0,
            locals: Vec::new(),
            blocks: vec![mir::BasicBlock {
                statements: Vec::new(),
                terminator: mir::Terminator::Unreachable,
            }],
        }
    }

    fn alloc_local(
        &mut self,
        name: String,
        ty: Type,
        mutable: bool,
        synthetic: bool,
    ) -> mir::LocalId {
        let id = self.locals.len();
        self.locals.push(mir::Local {
            name,
            ty,
            mutable,
            synthetic,
        });
        id
    }

    fn alloc_temp(&mut self, ty: Type) -> mir::LocalId {
        let id = self.locals.len();
        self.locals.push(mir::Local {
            name: format!("$tmp{}", id),
            ty,
            mutable: false,
            synthetic: true,
        });
        id
    }

    fn new_block(&mut self) -> mir::BlockId {
        let id = self.blocks.len();
        self.blocks.push(mir::BasicBlock {
            statements: Vec::new(),
            terminator: mir::Terminator::Unreachable,
        });
        id
    }

    fn push_statement(&mut self, block: mir::BlockId, stmt: mir::Statement) {
        self.blocks[block].statements.push(stmt);
    }

    fn set_terminator(&mut self, block: mir::BlockId, term: mir::Terminator) {
        self.blocks[block].terminator = term;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_parser::{Lexer, Parser};

    fn lower(src: &str) -> mir::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        lower_module(&typed.module)
    }

    #[test]
    fn lowers_straight_line_return() {
        let module = lower("fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n");
        match &module.items[0] {
            mir::Item::Function(function) => {
                assert_eq!(function.blocks.len(), 1);
                assert!(matches!(
                    function.blocks[0].terminator,
                    mir::Terminator::Return(Some(_))
                ));
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn lowers_if_to_switch_and_join() {
        let module = lower(
            "fun test(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        );
        match &module.items[0] {
            mir::Item::Function(function) => {
                assert!(function.blocks.len() >= 4);
                assert!(matches!(
                    function.blocks[0].terminator,
                    mir::Terminator::SwitchBool { .. }
                ));
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn lowers_match_to_match_terminator() {
        let module = lower(
            "fun test(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
        );
        match &module.items[0] {
            mir::Item::Function(function) => {
                assert!(matches!(
                    function.blocks[0].terminator,
                    mir::Terminator::Match { .. }
                ));
            }
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn preserves_python_call_target() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        match &module.items[1] {
            mir::Item::Function(function) => match &function.blocks[0].statements[0] {
                mir::Statement::Assign { value, .. } => match value {
                    mir::Rvalue::Call { target, .. } => {
                        assert_eq!(
                            target,
                            &typed::CallTarget::PythonFunction {
                                name: "read_csv".to_string()
                            }
                        );
                    }
                    other => panic!("expected call rvalue, got {other:?}"),
                },
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn preserves_python_member_call_target() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        match &module.items[1] {
            mir::Item::Function(function) => {
                let found = function.blocks[0].statements.iter().any(|stmt| {
                    matches!(
                        stmt,
                        mir::Statement::Assign {
                            value: mir::Rvalue::Call {
                                target: typed::CallTarget::PythonMember { name },
                                ..
                            },
                            ..
                        } if name == "head"
                    )
                });
                assert!(found, "expected Python member call target in block 0");
            }
            other => panic!("expected function, got {other:?}"),
        }
    }
}
