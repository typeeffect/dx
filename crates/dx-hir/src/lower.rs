use crate::hir;
use dx_parser as ast;

pub fn lower_module(module: &ast::Module) -> hir::Module {
    let mut lowerer = Lowerer::default();
    lowerer.lower_module(module)
}

#[derive(Default)]
struct Lowerer {
    next_tmp: usize,
    next_placeholder: usize,
}

impl Lowerer {
    fn lower_module(&mut self, module: &ast::Module) -> hir::Module {
        hir::Module {
            items: module
                .items
                .iter()
                .map(|item| self.lower_item(item))
                .collect(),
        }
    }

    fn lower_item(&mut self, item: &ast::Item) -> hir::Item {
        match item {
            ast::Item::Schema(schema) => hir::Item::Schema(schema.clone()),
            ast::Item::ImportPy(import) => hir::Item::ImportPy(import.clone()),
            ast::Item::Function(function) => hir::Item::Function(self.lower_function(function)),
            ast::Item::Statement(stmt) => hir::Item::Statement(self.lower_stmt(stmt, &None)),
        }
    }

    fn lower_function(&mut self, function: &ast::FunctionDecl) -> hir::Function {
        hir::Function {
            name: function.name.clone(),
            params: function
                .params
                .iter()
                .map(|param| hir::Param {
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                })
                .collect(),
            return_type: function.return_type.clone(),
            effects: function.effects.clone(),
            body: self.lower_block(&function.body, None),
        }
    }

    fn lower_block(&mut self, stmts: &[ast::Stmt], inherited_it: Option<String>) -> hir::Block {
        let mut lowered = Vec::new();
        let mut current_it = inherited_it;

        if stmts.is_empty() {
            return hir::Block {
                stmts: lowered,
                result: None,
            };
        }

        for stmt in &stmts[..stmts.len() - 1] {
            match stmt {
                ast::Stmt::Expr(expr) => {
                    let value = self.lower_expr(expr, &current_it);
                    let temp_name = self.fresh_it_name();
                    lowered.push(hir::Stmt::Let {
                        name: temp_name.clone(),
                        mutable: false,
                        value,
                        synthetic: true,
                    });
                    current_it = Some(temp_name);
                }
                _ => lowered.push(self.lower_stmt(stmt, &current_it)),
            }
        }

        let last = &stmts[stmts.len() - 1];
        let result = match last {
            ast::Stmt::Expr(expr) => Some(self.lower_expr(expr, &current_it)),
            _ => {
                lowered.push(self.lower_stmt(last, &current_it));
                None
            }
        };

        hir::Block {
            stmts: lowered,
            result: result.map(Box::new),
        }
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt, current_it: &Option<String>) -> hir::Stmt {
        match stmt {
            ast::Stmt::ValBind { name, value } => hir::Stmt::Let {
                name: name.clone(),
                mutable: false,
                value: self.lower_expr(value, current_it),
                synthetic: false,
            },
            ast::Stmt::VarBind { name, value } => hir::Stmt::Let {
                name: name.clone(),
                mutable: true,
                value: self.lower_expr(value, current_it),
                synthetic: false,
            },
            ast::Stmt::Rebind { name, value } => hir::Stmt::Rebind {
                name: name.clone(),
                value: self.lower_expr(value, current_it),
            },
            ast::Stmt::Expr(expr) => hir::Stmt::Expr(self.lower_expr(expr, current_it)),
        }
    }

    fn lower_expr(&mut self, expr: &ast::Expr, current_it: &Option<String>) -> hir::Expr {
        if contains_placeholder(expr) {
            let param_name = self.fresh_placeholder_name();
            let rewritten = replace_placeholder(expr, &param_name);
            let body = self.lower_expr(&rewritten, current_it);
            return hir::Expr::Closure {
                params: vec![hir::ClosureParam {
                    name: param_name,
                    ty: None,
                }],
                body: Box::new(hir::ClosureBody::Expr(Box::new(body))),
            };
        }

        self.lower_expr_without_placeholder(expr, current_it)
    }

    fn lower_expr_without_placeholder(
        &mut self,
        expr: &ast::Expr,
        current_it: &Option<String>,
    ) -> hir::Expr {
        match expr {
            ast::Expr::Unit => hir::Expr::Unit,
            ast::Expr::Name(name) if name == "it" => hir::Expr::Name(
                current_it
                    .clone()
                    .unwrap_or_else(|| "it".to_string()),
            ),
            ast::Expr::Name(name) => hir::Expr::Name(name.clone()),
            ast::Expr::Integer(value) => hir::Expr::Integer(value.clone()),
            ast::Expr::String(value) => hir::Expr::String(value.clone()),
            ast::Expr::Member { base, name } => hir::Expr::Member {
                base: Box::new(self.lower_expr(base, current_it)),
                name: name.clone(),
            },
            ast::Expr::Call { callee, args } => hir::Expr::Call {
                callee: Box::new(self.lower_expr(callee, current_it)),
                args: args
                    .iter()
                    .map(|arg| match arg {
                        ast::Arg::Positional(expr) => {
                            hir::Arg::Positional(self.lower_expr(expr, current_it))
                        }
                        ast::Arg::Named { name, value } => hir::Arg::Named {
                            name: name.clone(),
                            value: self.lower_expr(value, current_it),
                        },
                    })
                    .collect(),
            },
            ast::Expr::Lambda { params, body } => hir::Expr::Closure {
                params: params
                    .iter()
                    .map(|param| hir::ClosureParam {
                        name: param.name.clone(),
                        ty: param.ty.clone(),
                    })
                    .collect(),
                body: Box::new(self.lower_lambda_body(body)),
            },
            ast::Expr::Lazy { body } => hir::Expr::Closure {
                params: vec![],
                body: Box::new(self.lower_lambda_body(body)),
            },
            ast::Expr::If {
                branches,
                else_branch,
            } => hir::Expr::If {
                branches: branches
                    .iter()
                    .map(|(condition, block)| {
                        (
                            self.lower_expr(condition, current_it),
                            self.lower_block(block, None),
                        )
                    })
                    .collect(),
                else_branch: else_branch
                    .as_ref()
                    .map(|block| self.lower_block(block, None)),
            },
            ast::Expr::Match { scrutinee, arms } => hir::Expr::Match {
                scrutinee: Box::new(self.lower_expr(scrutinee, current_it)),
                arms: arms
                    .iter()
                    .map(|arm| hir::MatchArm {
                        pattern: self.lower_pattern(&arm.pattern),
                        body: self.lower_block(&arm.body, None),
                    })
                    .collect(),
            },
            ast::Expr::Placeholder => unreachable!("placeholder should be lifted before HIR"),
            ast::Expr::BinaryOp { op, lhs, rhs } => hir::Expr::BinaryOp {
                op: op.clone(),
                lhs: Box::new(self.lower_expr(lhs, current_it)),
                rhs: Box::new(self.lower_expr(rhs, current_it)),
            },
        }
    }

    fn lower_lambda_body(&mut self, body: &ast::LambdaBody) -> hir::ClosureBody {
        match body {
            ast::LambdaBody::Expr(expr) => {
                hir::ClosureBody::Expr(Box::new(self.lower_expr(expr, &None)))
            }
            ast::LambdaBody::Block(stmts) => {
                hir::ClosureBody::Block(Box::new(self.lower_block(stmts, None)))
            }
        }
    }

    fn lower_pattern(&self, pattern: &ast::Pattern) -> hir::Pattern {
        match pattern {
            ast::Pattern::Name(name) => hir::Pattern::Name(name.clone()),
            ast::Pattern::Wildcard => hir::Pattern::Wildcard,
            ast::Pattern::Constructor { name, args } => hir::Pattern::Constructor {
                name: name.clone(),
                args: args.iter().map(|arg| self.lower_pattern(arg)).collect(),
            },
        }
    }

    fn fresh_it_name(&mut self) -> String {
        let name = format!("$it{}", self.next_tmp);
        self.next_tmp += 1;
        name
    }

    fn fresh_placeholder_name(&mut self) -> String {
        let name = format!("$p{}", self.next_placeholder);
        self.next_placeholder += 1;
        name
    }
}

fn contains_placeholder(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Placeholder => true,
        ast::Expr::Member { base, .. } => contains_placeholder(base),
        ast::Expr::Call { callee, args } => {
            contains_placeholder(callee)
                || args.iter().any(|arg| match arg {
                    ast::Arg::Positional(expr) => contains_placeholder(expr),
                    ast::Arg::Named { value, .. } => contains_placeholder(value),
                })
        }
        ast::Expr::Lambda { .. } | ast::Expr::Lazy { .. } => false,
        ast::Expr::If {
            branches,
            else_branch,
        } => {
            branches
                .iter()
                .any(|(expr, body)| contains_placeholder(expr) || stmts_contain_placeholder(body))
                || else_branch
                    .as_ref()
                    .is_some_and(|body| stmts_contain_placeholder(body))
        }
        ast::Expr::Match { scrutinee, arms } => {
            contains_placeholder(scrutinee)
                || arms
                    .iter()
                    .any(|arm| stmts_contain_placeholder(&arm.body))
        }
        ast::Expr::BinaryOp { lhs, rhs, .. } => {
            contains_placeholder(lhs) || contains_placeholder(rhs)
        }
        ast::Expr::Unit | ast::Expr::Name(_) | ast::Expr::Integer(_) | ast::Expr::String(_) => {
            false
        }
    }
}

fn stmts_contain_placeholder(stmts: &[ast::Stmt]) -> bool {
    stmts.iter().any(|stmt| match stmt {
        ast::Stmt::ValBind { value, .. }
        | ast::Stmt::VarBind { value, .. }
        | ast::Stmt::Rebind { value, .. }
        | ast::Stmt::Expr(value) => contains_placeholder(value),
    })
}

fn replace_placeholder(expr: &ast::Expr, param_name: &str) -> ast::Expr {
    match expr {
        ast::Expr::Unit => ast::Expr::Unit,
        ast::Expr::Placeholder => ast::Expr::Name(param_name.to_string()),
        ast::Expr::Name(name) => ast::Expr::Name(name.clone()),
        ast::Expr::Integer(value) => ast::Expr::Integer(value.clone()),
        ast::Expr::String(value) => ast::Expr::String(value.clone()),
        ast::Expr::Member { base, name } => ast::Expr::Member {
            base: Box::new(replace_placeholder(base, param_name)),
            name: name.clone(),
        },
        ast::Expr::Call { callee, args } => ast::Expr::Call {
            callee: Box::new(replace_placeholder(callee, param_name)),
            args: args
                .iter()
                .map(|arg| match arg {
                    ast::Arg::Positional(expr) => {
                        ast::Arg::Positional(replace_placeholder(expr, param_name))
                    }
                    ast::Arg::Named { name, value } => ast::Arg::Named {
                        name: name.clone(),
                        value: replace_placeholder(value, param_name),
                    },
                })
                .collect(),
        },
        ast::Expr::Lambda { params, body } => ast::Expr::Lambda {
            params: params.clone(),
            body: replace_placeholder_in_lambda_body(body, param_name),
        },
        ast::Expr::Lazy { body } => ast::Expr::Lazy {
            body: replace_placeholder_in_lambda_body(body, param_name),
        },
        ast::Expr::If {
            branches,
            else_branch,
        } => ast::Expr::If {
            branches: branches
                .iter()
                .map(|(cond, body)| {
                    (
                        replace_placeholder(cond, param_name),
                        replace_placeholder_in_stmts(body, param_name),
                    )
                })
                .collect(),
            else_branch: else_branch
                .as_ref()
                .map(|body| replace_placeholder_in_stmts(body, param_name)),
        },
        ast::Expr::Match { scrutinee, arms } => ast::Expr::Match {
            scrutinee: Box::new(replace_placeholder(scrutinee, param_name)),
            arms: arms
                .iter()
                .map(|arm| ast::MatchArm {
                    pattern: arm.pattern.clone(),
                    body: replace_placeholder_in_stmts(&arm.body, param_name),
                })
                .collect(),
        },
        ast::Expr::BinaryOp { op, lhs, rhs } => ast::Expr::BinaryOp {
            op: op.clone(),
            lhs: Box::new(replace_placeholder(lhs, param_name)),
            rhs: Box::new(replace_placeholder(rhs, param_name)),
        },
    }
}

fn replace_placeholder_in_lambda_body(body: &ast::LambdaBody, param_name: &str) -> ast::LambdaBody {
    match body {
        ast::LambdaBody::Expr(expr) => {
            ast::LambdaBody::Expr(Box::new(replace_placeholder(expr, param_name)))
        }
        ast::LambdaBody::Block(stmts) => {
            ast::LambdaBody::Block(replace_placeholder_in_stmts(stmts, param_name))
        }
    }
}

fn replace_placeholder_in_stmts(stmts: &[ast::Stmt], param_name: &str) -> Vec<ast::Stmt> {
    stmts.iter()
        .map(|stmt| match stmt {
            ast::Stmt::ValBind { name, value } => ast::Stmt::ValBind {
                name: name.clone(),
                value: replace_placeholder(value, param_name),
            },
            ast::Stmt::VarBind { name, value } => ast::Stmt::VarBind {
                name: name.clone(),
                value: replace_placeholder(value, param_name),
            },
            ast::Stmt::Rebind { name, value } => ast::Stmt::Rebind {
                name: name.clone(),
                value: replace_placeholder(value, param_name),
            },
            ast::Stmt::Expr(expr) => ast::Stmt::Expr(replace_placeholder(expr, param_name)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_parser::{Expr as AstExpr, Item as AstItem, Lexer, Parser, Stmt as AstStmt};

    fn parse_module(src: &str) -> ast::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_module().expect("module should parse")
    }

    #[test]
    fn lowers_lazy_to_zero_param_closure() {
        let module = parse_module(
            r#"
fun demo() -> Unit:
    val thunk = lazy me'name
.
"#,
        );

        let lowered = lower_module(&module);
        match &lowered.items[0] {
            hir::Item::Function(function) => match &function.body.stmts[0] {
                hir::Stmt::Let { value, .. } => match value {
                    hir::Expr::Closure { params, .. } => assert!(params.is_empty()),
                    other => panic!("expected closure, got {other:?}"),
                },
                other => panic!("expected let, got {other:?}"),
            },
            other => panic!("expected function, got {other:?}"),
        }
    }

    #[test]
    fn lowers_placeholder_expression_to_unary_closure() {
        let expr = AstExpr::Member {
            base: Box::new(AstExpr::Placeholder),
            name: "email".to_string(),
        };
        let lowered = Lowerer::default().lower_expr(&expr, &None);
        match lowered {
            hir::Expr::Closure { params, body } => {
                assert_eq!(params.len(), 1);
                match *body {
                    hir::ClosureBody::Expr(expr) => match *expr {
                        hir::Expr::Member { .. } => {}
                        other => panic!("expected member body, got {other:?}"),
                    },
                    other => panic!("expected expr body, got {other:?}"),
                }
            }
            other => panic!("expected closure, got {other:?}"),
        }
    }

    #[test]
    fn lowers_it_pipeline_to_synthetic_temps() {
        let module = ast::Module {
            items: vec![AstItem::Statement(AstStmt::Expr(AstExpr::Name(
                "read_csv".to_string(),
            )))],
        };
        let lowered = lower_module(&module);
        match &lowered.items[0] {
            hir::Item::Statement(hir::Stmt::Expr(hir::Expr::Name(name))) => {
                assert_eq!(name, "read_csv");
            }
            other => panic!("unexpected lowered form: {other:?}"),
        }

        let module = parse_module(
            r#"
fun active_emails(path: Str) -> PyObj !py:
    read_csv(path)
    it'filter(_'active)
    it'map(_'email)
.
"#,
        );
        let lowered = lower_module(&module);
        match &lowered.items[0] {
            hir::Item::Function(function) => {
                assert_eq!(function.body.stmts.len(), 2);
                match &function.body.stmts[0] {
                    hir::Stmt::Let { synthetic, .. } => assert!(*synthetic),
                    other => panic!("expected synthetic let, got {other:?}"),
                }
                match &function.body.stmts[1] {
                    hir::Stmt::Let { synthetic, .. } => assert!(*synthetic),
                    other => panic!("expected synthetic let, got {other:?}"),
                }
                assert!(function.body.result.is_some());
            }
            other => panic!("expected function, got {other:?}"),
        }
    }
}
