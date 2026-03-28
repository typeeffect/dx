use crate::llvm::{Function, Instruction, Module, Operand, Terminator, Type};
use dx_parser::BinOp;
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

    let mut global_symbols = BTreeSet::new();
    for global in &module.globals {
        if !global_symbols.insert(global.symbol.clone()) {
            diagnostics.push(ValidationDiagnostic {
                function: None,
                block: None,
                message: format!("duplicate global symbol '{}'", global.symbol),
            });
        }
    }

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
        validate_function(function, &global_symbols, &extern_index, &mut diagnostics);
    }

    ValidationReport { diagnostics }
}

fn validate_function(
    function: &Function,
    global_symbols: &BTreeSet<String>,
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

    let mut defined_registers = BTreeSet::new();
    for param in &function.params {
        defined_registers.insert(param.name.clone());
    }

    for block in &function.blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::Assign { result, .. } | Instruction::BinaryOp { result, .. } => {
                    if !defined_registers.insert(result.clone()) {
                        diagnostics.push(ValidationDiagnostic {
                            function: Some(function.name.clone()),
                            block: Some(block.label.clone()),
                            message: format!("duplicate register definition '{}'", result),
                        });
                    }
                }
                Instruction::PackEnv { result, .. } => {
                    if !defined_registers.insert(result.clone()) {
                        diagnostics.push(ValidationDiagnostic {
                            function: Some(function.name.clone()),
                            block: Some(block.label.clone()),
                            message: format!("duplicate register definition '{}'", result),
                        });
                    }
                }
                Instruction::CallExtern { result, .. } => {
                    if let Some(result) = result {
                        if !defined_registers.insert(result.clone()) {
                            diagnostics.push(ValidationDiagnostic {
                                function: Some(function.name.clone()),
                                block: Some(block.label.clone()),
                                message: format!("duplicate register definition '{}'", result),
                            });
                        }
                    }
                }
            }
        }
    }

    for block in &function.blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::Assign { ty, value, .. } => {
                    validate_operand_defined(value, &defined_registers, function, block, diagnostics);
                    validate_operand_global(value, global_symbols, function, block, diagnostics);
                    if operand_type(value) != *ty {
                        diagnostics.push(ValidationDiagnostic {
                            function: Some(function.name.clone()),
                            block: Some(block.label.clone()),
                            message: format!(
                                "assign type mismatch: got {:?}, destination is {:?}",
                                operand_type(value),
                                ty
                            ),
                        });
                    }
                }
                Instruction::BinaryOp { op, ty, lhs, rhs, .. } => {
                    validate_operand_defined(lhs, &defined_registers, function, block, diagnostics);
                    validate_operand_defined(rhs, &defined_registers, function, block, diagnostics);
                    validate_operand_global(lhs, global_symbols, function, block, diagnostics);
                    validate_operand_global(rhs, global_symbols, function, block, diagnostics);
                    validate_binary_op(op, ty, lhs, rhs, function, block, diagnostics);
                }
                Instruction::PackEnv { result: _, captures } => {
                    for capture in captures {
                        validate_operand_defined(
                            capture,
                            &defined_registers,
                            function,
                            block,
                            diagnostics,
                        );
                        validate_operand_global(
                            capture,
                            global_symbols,
                            function,
                            block,
                            diagnostics,
                        );
                    }
                }
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
                            validate_operand_defined(
                                arg,
                                &defined_registers,
                                function,
                                block,
                                diagnostics,
                            );
                            validate_operand_global(
                                arg,
                                global_symbols,
                                function,
                                block,
                                diagnostics,
                            );
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

        validate_terminator(
            function,
            block,
            &block_labels,
            &defined_registers,
            global_symbols,
            diagnostics,
        );
    }
}

fn validate_binary_op(
    op: &BinOp,
    result_ty: &Type,
    lhs: &Operand,
    rhs: &Operand,
    function: &Function,
    block: &crate::llvm::Block,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    let lhs_ty = operand_type(lhs);
    let rhs_ty = operand_type(rhs);

    if lhs_ty != rhs_ty {
        diagnostics.push(ValidationDiagnostic {
            function: Some(function.name.clone()),
            block: Some(block.label.clone()),
            message: format!(
                "binary op operand mismatch: lhs is {:?}, rhs is {:?}",
                lhs_ty, rhs_ty
            ),
        });
        return;
    }

    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul => {
            if *result_ty != lhs_ty {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!(
                        "arithmetic result type mismatch: op result is {:?}, operands are {:?}",
                        result_ty, lhs_ty
                    ),
                });
            }
            if !matches!(lhs_ty, Type::I64 | Type::Double) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("arithmetic op requires numeric operands, got {:?}", lhs_ty),
                });
            }
        }
        BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
            if *result_ty != Type::I1 {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!(
                        "comparison result type must be I1, got {:?}",
                        result_ty
                    ),
                });
            }
            if !matches!(lhs_ty, Type::I64 | Type::Double) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("ordered comparison requires numeric operands, got {:?}", lhs_ty),
                });
            }
        }
        BinOp::EqEq => {
            if *result_ty != Type::I1 {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!("equality result type must be I1, got {:?}", result_ty),
                });
            }
            if matches!(lhs_ty, Type::Void) {
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: "equality does not support void operands".to_string(),
                });
            }
        }
    }
}

fn validate_operand_defined(
    operand: &Operand,
    defined_registers: &BTreeSet<String>,
    function: &Function,
    block: &crate::llvm::Block,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if let Operand::Register(name, _) = operand {
        if !defined_registers.contains(name) && !is_implicitly_available_register(name) {
            diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: Some(block.label.clone()),
                message: format!("use of undefined register '{}'", name),
            });
        }
    }
}

fn validate_operand_global(
    operand: &Operand,
    global_symbols: &BTreeSet<String>,
    function: &Function,
    block: &crate::llvm::Block,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    if let Operand::Global(name, _) = operand {
        if !global_symbols.contains(name) {
            diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: Some(block.label.clone()),
                message: format!("use of unknown global '@{}'", name),
            });
        }
    }
}

fn is_implicitly_available_register(name: &str) -> bool {
    if name == "%unit" {
        return true;
    }
    if name.starts_with("%py_") {
        return true;
    }
    name.strip_prefix('%')
        .map(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

fn validate_terminator(
    function: &Function,
    block: &crate::llvm::Block,
    block_labels: &BTreeSet<String>,
    defined_registers: &BTreeSet<String>,
    global_symbols: &BTreeSet<String>,
    diagnostics: &mut Vec<ValidationDiagnostic>,
) {
    match &block.terminator {
        Terminator::Ret(value) => match value {
            Some(value) if operand_type(value) != function.ret => {
                validate_operand_defined(value, defined_registers, function, block, diagnostics);
                validate_operand_global(value, global_symbols, function, block, diagnostics);
                diagnostics.push(ValidationDiagnostic {
                    function: Some(function.name.clone()),
                    block: Some(block.label.clone()),
                    message: format!(
                        "return type mismatch: got {:?}, function returns {:?}",
                        operand_type(value),
                        function.ret
                    ),
                })
            }
            None if function.ret != Type::Void => diagnostics.push(ValidationDiagnostic {
                function: Some(function.name.clone()),
                block: Some(block.label.clone()),
                message: format!("missing return value for non-void function returning {:?}", function.ret),
            }),
            Some(value) => {
                validate_operand_defined(value, defined_registers, function, block, diagnostics);
                validate_operand_global(value, global_symbols, function, block, diagnostics);
            }
            None => {}
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
            validate_operand_defined(cond, defined_registers, function, block, diagnostics);
            validate_operand_global(cond, global_symbols, function, block, diagnostics);
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
            scrutinee,
            arms,
            fallback,
        } => {
            validate_operand_defined(scrutinee, defined_registers, function, block, diagnostics);
            validate_operand_global(scrutinee, global_symbols, function, block, diagnostics);
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
        Operand::Global(_, ty) => ty.clone(),
        Operand::ConstInt(_) => Type::I64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llvm::{Block, ExternDecl, Function, Instruction, Module, Param, Terminator, Type};

    fn valid_module() -> Module {
        Module {
            globals: vec![],
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
    fn rejects_use_of_undefined_register() {
        let mut module = valid_module();
        module.functions[0].blocks[0].instructions = vec![Instruction::CallExtern {
            result: Some("%1".into()),
            symbol: "dx_rt_py_call_function",
            ret: Type::Ptr,
            args: vec![
                Operand::Register("%missing".into(), Type::Ptr),
                Operand::ConstInt(1),
            ],
            comment: None,
        }];
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("use of undefined register '%missing'")));
    }

    #[test]
    fn rejects_duplicate_register_definition() {
        let module = Module {
            globals: vec![],
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
                    instructions: vec![
                        Instruction::PackEnv {
                            result: "%1".into(),
                            captures: vec![Operand::Register("%0".into(), Type::Ptr)],
                        },
                        Instruction::CallExtern {
                            result: Some("%1".into()),
                            symbol: "dx_rt_py_call_function",
                            ret: Type::Ptr,
                            args: vec![
                                Operand::Register("%0".into(), Type::Ptr),
                                Operand::ConstInt(1),
                            ],
                            comment: None,
                        },
                    ],
                    terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::Ptr))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("duplicate register definition '%1'")));
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
    fn accepts_declared_global_operand() {
        let module = Module {
            globals: vec![crate::llvm::GlobalString {
                symbol: ".str0".into(),
                value: "hello".into(),
            }],
            externs: vec![],
            functions: vec![Function {
                name: "f".into(),
                params: vec![],
                ret: Type::Ptr,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![],
                    terminator: Terminator::Ret(Some(Operand::Global(".str0".into(), Type::Ptr))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
    }

    #[test]
    fn rejects_unknown_global_operand() {
        let module = Module {
            globals: vec![],
            externs: vec![],
            functions: vec![Function {
                name: "f".into(),
                params: vec![],
                ret: Type::Ptr,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![],
                    terminator: Terminator::Ret(Some(Operand::Global(".missing".into(), Type::Ptr))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("use of unknown global '@.missing'")));
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

    #[test]
    fn rejects_binary_op_operand_type_mismatch() {
        let module = Module {
            globals: vec![],
            externs: vec![],
            functions: vec![Function {
                name: "f".into(),
                params: vec![],
                ret: Type::I64,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![Instruction::BinaryOp {
                        result: "%1".into(),
                        op: BinOp::Add,
                        ty: Type::I64,
                        lhs: Operand::ConstInt(1),
                        rhs: Operand::Register("%g".into(), Type::Ptr),
                    }],
                    terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I64))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("binary op operand mismatch")));
    }

    #[test]
    fn rejects_arithmetic_result_type_mismatch() {
        let module = Module {
            globals: vec![],
            externs: vec![],
            functions: vec![Function {
                name: "f".into(),
                params: vec![],
                ret: Type::I1,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![Instruction::BinaryOp {
                        result: "%1".into(),
                        op: BinOp::Add,
                        ty: Type::I1,
                        lhs: Operand::ConstInt(1),
                        rhs: Operand::ConstInt(2),
                    }],
                    terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I1))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("arithmetic result type mismatch")));
    }

    #[test]
    fn rejects_comparison_with_non_boolean_result() {
        let module = Module {
            globals: vec![],
            externs: vec![],
            functions: vec![Function {
                name: "f".into(),
                params: vec![],
                ret: Type::I64,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![Instruction::BinaryOp {
                        result: "%1".into(),
                        op: BinOp::Lt,
                        ty: Type::I64,
                        lhs: Operand::ConstInt(1),
                        rhs: Operand::ConstInt(2),
                    }],
                    terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I64))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("comparison result type must be I1")));
    }

    #[test]
    fn accepts_well_typed_comparison() {
        let module = Module {
            globals: vec![],
            externs: vec![],
            functions: vec![Function {
                name: "f".into(),
                params: vec![],
                ret: Type::I1,
                blocks: vec![Block {
                    label: "bb0".into(),
                    instructions: vec![Instruction::BinaryOp {
                        result: "%1".into(),
                        op: BinOp::LtEq,
                        ty: Type::I1,
                        lhs: Operand::ConstInt(1),
                        rhs: Operand::ConstInt(2),
                    }],
                    terminator: Terminator::Ret(Some(Operand::Register("%1".into(), Type::I1))),
                }],
            }],
        };
        let report = validate_module(&module);
        assert!(report.is_ok(), "diagnostics: {:?}", report.diagnostics);
    }
}
