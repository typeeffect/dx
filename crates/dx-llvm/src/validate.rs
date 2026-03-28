use crate::llvm::{Function, Instruction, Module, Operand, Terminator, Type};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationDiagnostic {
    pub function: Option<String>,
    pub block: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub diagnostics: Vec<ValidationDiagnostic>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

pub fn validate_module(module: &Module) -> ValidationReport {
    let mut diagnostics = Vec::new();

    let mut extern_symbols = BTreeSet::new();
    let mut extern_index = HashMap::new();
    for ext in &module.externs {
        if !extern_symbols.insert(ext.symbol) {
            diagnostics.push(ValidationDiagnostic {
                function: None,
                block: None,
                message: format!("duplicate extern symbol '{}'", ext.symbol),
            });
        }
        extern_index.insert(ext.symbol, ext);
    }

    let mut fn_names = BTreeSet::new();
    for function in &module.functions {
        if !fn_names.insert(function.name.clone()) {
            diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: None,
                message: format!("duplicate function '{}'", function.name),
            });
        }
        validate_function(function, &extern_index, &mut diagnostics);
    }

    ValidationReport { diagnostics }
}

fn validate_function(
    function: &Function,
    extern_index: &HashMap<&'static str, &crate::llvm::ExternDecl>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let mut block_labels = BTreeSet::new();
    for block in &function.blocks {
        if !block_labels.insert(block.label.clone()) {
            diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: Some(block.label.clone()),
                message: format!("duplicate block label '{}'", block.label),
            });
        }
    }

    for block in &function.blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::CallExtern {
                    result: _,
                    symbol,
                    ret,
                    args,
                    comment: _,
                } => {
                    let Some(ext) = extern_index.get(symbol) else {
                        diagnostics.push(ValidationDiagnostic {
                            function: Some(function.name.clone()),
                            block: Some(block.label.clone()),
                            message: format!("call references unknown extern '{}'", symbol),
                        });
                        continue;
                    };

                    if ext.ret != *ret {
                        diagnostics.push(ValidationDiagnostic {
                            function: Some(function.name.clone()),
                            block: Some(block.label.clone()),
                            message: format!(
                                "call return type mismatch for '{}': call has {:?}, extern has {:?}",
                                symbol, ret, ext.ret
                            ),
                        });
                    }

                    if ext.params.len() != args.len() {
                        diagnostics.push(ValidationDiagnostic {
                            function: Some(function.name.clone()),
                            block: Some(block.label.clone()),
                            message: format!(
                                "call arg count mismatch for '{}': call has {}, extern has {}",
                                symbol,
                                args.len(),
                                ext.params.len()
                            ),
                        });
                    } else {
                        for (arg, expected) in args.iter().zip(ext.params.iter()) {
                            if operand_type(arg) != *expected {
                                diagnostics.push(ValidationDiagnostic {
                                    function: Some(function.name.clone()),
                                    block: Some(block.label.clone()),
                                    message: format!(
                                        "call arg type mismatch for '{}': got {:?}, expected {:?}",
                                        symbol,
                                        operand_type(arg),
                                        expected
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        validate_terminator(function, block, &block_labels, diagnostics);
    }
}

fn validate_terminator(
    function: &Function,
    block: &crate::llvm::Block,
    block_labels: &BTreeSet<String>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match &block.terminator {
        Terminator::Ret(value) => match value {
            Some(value) if operand_type(value) != function.ret => diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: Some(block.label.clone()),
                message: format!(
                    "return type mismatch: got {:?}, function returns {:?}",
                    operand_type(value),
                    function.ret
                ),
            }),
            None if function.ret != Type::Void => diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: Some(block.label.clone()),
                message: format!("missing return value for non-void function returning {:?}", function.ret),
            }),
            Some(_) | None => {}
        },
        Terminator::Br(target) => {
            if !block_labels.contains(target) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("branch to unknown label '{}'", target),
                });
            }
        }
        Terminator::CondBr {
            cond,
            then_label,
            else_label,
        } => {
            if operand_type(cond) != Type::I1 {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("conditional branch expects i1 condition, got {:?}", operand_type(cond)),
                });
            }
            if !block_labels.contains(then_label) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("conditional branch to unknown label '{}'", then_label),
                });
            }
            if !block_labels.contains(else_label) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("conditional branch to unknown label '{}'", else_label),
                });
            }
        }
        Terminator::MatchBr {
            scrutinee: _,
            arms,
            fallback,
        } => {
            for (_, target) in arms {
                if !block_labels.contains(target) {
                    diagnostics.push(ValidationDiagnostic {
                        function: Some(function.name.clone()),
                        block: Some(block.label.clone()),
                        message: format!("match branch to unknown label '{}'", target),
                    });
                }
            }
            if !block_labels.contains(fallback) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("match fallback to unknown label '{}'", fallback),
                });
            }
        }
        Terminator::Unreachable => {}
    }
}

fn operand_type(op: &Operand) -> Type {
    match op {
        Operand::Register(_, ty) => ty.clone(),
        Operand::ConstInt(_) => Type::I64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llvm::{Block, ExternDecl, Function, Instruction, Module, Param, Terminator, Type};

    fn valid_module() -> Module {
        Module {
            externs: vec![ExternDecl {
                symbol: "dx_rt_py_call_function",
                params: vec![Type::Ptr, Type::I64],
                ret: Type::Ptr,
            }],
            functions: vec![Function {
                name: "f".into(),
                params: vec![Param {
                    name: "%0".into(),
                    ty: Type::Ptr,
                }],
                ret: Type::Ptr,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![Instruction::CallExtern {
                        result: Some("%1".into()),
                        symbol: "dx_rt_py_call_function",
                        ret: Type::Ptr,
                        args: vec![Operand::Register("%0".into(), Type::Ptr), Operand::ConstInt(1)],
                        comment: None,
                    }],
                    terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::Ptr))),
                }],
            }],
        }
    }

    #[test]
    fn accepts_valid_module() {
        let report = validate_module(&valid_module());
        assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
    }

    #[test]
    fn rejects_duplicate_externs() {
        let mut module = valid_module();
        module.externs.push(module.externs[0].clone());
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate extern symbol")));
    }

    #[test]
    fn rejects_unknown_extern_call() {
        let mut module = valid_module();
        let Function { blocks, .. } = &mut module.functions[0];
        blocks[0].instructions = vec![Instruction::CallExtern {
            result: None,
            symbol: "missing",
            ret: Type::Void,
            args: vec![],
            comment: None,
        }];
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown extern")));
    }

    #[test]
    fn rejects_bad_return_type() {
        let mut module = valid_module();
        module.functions[0].ret = Type::I64;
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("return type mismatch")));
    }

    #[test]
    fn rejects_bad_condbr_condition_type() {
        let mut module = valid_module();
        module.functions[0].blocks[0].terminator = Terminator::CondBr {
            cond: Operand::ConstInt(1),
            then_label: "bb0".into(),
            else_label: "bb0".into(),
        };
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("expects i1 condition")));
    }

    #[test]
    fn rejects_branch_to_unknown_label() {
        let mut module = valid_module();
        module.functions[0].blocks[0].terminator = Terminator::Br("missing".into());
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("unknown label")));
    }
}
