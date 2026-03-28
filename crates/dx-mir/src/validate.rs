use crate::mir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    pub function: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub diagnostics: Vec<ValidationDiagnostic>,
}

pub fn validate_module(module: &mir::Module) -> ValidationReport {
    let mut diagnostics = Vec::new();

    for item in &module.items {
        if let mir::Item::Function(function) = item {
            validate_function(function, &mut diagnostics);
        }
    }

    ValidationReport { diagnostics }
}

fn validate_function(function: &mir::Function, diagnostics: &mut Vec<ValidationDiagnostic>) {
    if function.blocks.is_empty() {
        diag(function, diagnostics, "function has no basic blocks");
        return;
    }

    for &param in &function.params {
        if param >= function.locals.len() {
            diag(
                function,
                diagnostics,
                format!("parameter local {} is out of range", param),
            );
        }
    }

    for (block_id, block) in function.blocks.iter().enumerate() {
        for stmt in &block.statements {
            validate_statement(function, block_id, stmt, diagnostics);
        }
        validate_terminator(function, block_id, &block.terminator, diagnostics);
    }
}

fn validate_statement(
    function: &mir::Function,
    block_id: usize,
    stmt: &mir::Statement,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match stmt {
        mir::Statement::Assign { place, value } => {
            validate_local(function, block_id, *place, "assignment place", diagnostics);
            validate_rvalue(function, block_id, value, diagnostics);
        }
    }
}

fn validate_rvalue(
    function: &mir::Function,
    block_id: usize,
    value: &mir::Rvalue,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match value {
        mir::Rvalue::Use(operand) => validate_operand(function, block_id, operand, diagnostics),
        mir::Rvalue::BinaryOp { lhs, rhs, .. } => {
            validate_operand(function, block_id, lhs, diagnostics);
            validate_operand(function, block_id, rhs, diagnostics);
        }
        mir::Rvalue::Member { base, .. } => {
            validate_operand(function, block_id, base, diagnostics);
        }
        mir::Rvalue::Call { callee, args, .. } => {
            validate_operand(function, block_id, callee, diagnostics);
            for arg in args {
                match arg {
                    mir::CallArg::Positional(value) => {
                        validate_operand(function, block_id, value, diagnostics)
                    }
                    mir::CallArg::Named { value, .. } => {
                        validate_operand(function, block_id, value, diagnostics)
                    }
                }
            }
        }
        mir::Rvalue::Closure { captures, .. } => {
            for capture in captures {
                validate_local(function, block_id, capture.source, "closure capture source", diagnostics);
            }
        }
    }
}

fn validate_terminator(
    function: &mir::Function,
    block_id: usize,
    term: &mir::Terminator,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match term {
        mir::Terminator::Return(Some(value)) => {
            validate_operand(function, block_id, value, diagnostics);
        }
        mir::Terminator::Return(None) | mir::Terminator::Unreachable => {}
        mir::Terminator::Goto(target) => {
            validate_block_target(function, block_id, *target, "goto target", diagnostics);
        }
        mir::Terminator::SwitchBool {
            cond,
            then_bb,
            else_bb,
        } => {
            validate_operand(function, block_id, cond, diagnostics);
            validate_block_target(function, block_id, *then_bb, "then target", diagnostics);
            validate_block_target(function, block_id, *else_bb, "else target", diagnostics);
        }
        mir::Terminator::Match {
            scrutinee,
            arms,
            fallback,
        } => {
            validate_operand(function, block_id, scrutinee, diagnostics);
            for (_, target) in arms {
                validate_block_target(function, block_id, *target, "match arm target", diagnostics);
            }
            validate_block_target(function, block_id, *fallback, "match fallback", diagnostics);
        }
    }
}

fn validate_operand(
    function: &mir::Function,
    block_id: usize,
    operand: &mir::Operand,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match operand {
        mir::Operand::Copy(local) => {
            validate_local(function, block_id, *local, "operand local", diagnostics)
        }
        mir::Operand::Const(_) => {}
    }
}

fn validate_local(
    function: &mir::Function,
    block_id: usize,
    local: usize,
    label: &str,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if local >= function.locals.len() {
        diag(
            function,
            diagnostics,
            format!("{label} {} is out of range in block {}", local, block_id),
        );
    }
}

fn validate_block_target(
    function: &mir::Function,
    block_id: usize,
    target: usize,
    label: &str,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if target >= function.blocks.len() {
        diag(
            function,
            diagnostics,
            format!("{label} {} is out of range from block {}", target, block_id),
        );
    }
}

fn diag(
    function: &mir::Function,
    diagnostics: &mut Vec<ValidationDiagnostic>,
    message: impl Into<String>,
) {
    diagnostics.push(ValidationDiagnostic {
        function: function.name.clone(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
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
    fn lowered_modules_validate_cleanly() {
        let module = lower(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    if path == \"\":\n        read_csv(path)\n    else:\n        read_csv(path)'head()\n    .\n.\n",
        );
        let report = validate_module(&module);
        assert!(report.diagnostics.is_empty(), "{report:?}");
    }

    #[test]
    fn reports_invalid_goto_target() {
        let module = mir::Module {
            items: vec![mir::Item::Function(mir::Function {
                name: "bad".to_string(),
                params: vec![],
                locals: vec![],
                blocks: vec![mir::BasicBlock {
                    statements: vec![],
                    terminator: mir::Terminator::Goto(1),
                }],
                return_type: None,
                effects: vec![],
            })],
        };
        let report = validate_module(&module);
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("goto target"));
    }

    #[test]
    fn reports_invalid_local_use() {
        let module = mir::Module {
            items: vec![mir::Item::Function(mir::Function {
                name: "bad".to_string(),
                params: vec![],
                locals: vec![],
                blocks: vec![mir::BasicBlock {
                    statements: vec![mir::Statement::Assign {
                        place: 0,
                        value: mir::Rvalue::Use(mir::Operand::Const(mir::Constant::Unit)),
                    }],
                    terminator: mir::Terminator::Return(None),
                }],
                return_type: None,
                effects: vec![],
            })],
        };
        let report = validate_module(&module);
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("assignment place"));
    }
}
