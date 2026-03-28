use dx_llvm::llvm::{Block, Function, GlobalString, Instruction, Module, Operand, Terminator, Type};
use dx_parser::BinOp;
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmitError {
    UnsupportedInstruction(&'static str),
    UnsupportedTerminator(&'static str),
    UnsupportedOperand(String),
    MissingStringGlobal(String),
}

pub fn emit_module(module: &Module) -> Result<String, EmitError> {
    let mut out = String::new();

    for global in &module.globals {
        writeln!(out, "{}", render_global(global)).unwrap();
    }
    if !module.globals.is_empty() && (!module.externs.is_empty() || !module.functions.is_empty()) {
        writeln!(out).unwrap();
    }

    for ext in &module.externs {
        let params = ext.params.iter().map(render_type).collect::<Vec<_>>().join(", ");
        writeln!(out, "declare {} @{}({})", render_type(&ext.ret), ext.symbol, params).unwrap();
    }
    if !module.externs.is_empty() && !module.functions.is_empty() {
        writeln!(out).unwrap();
    }

    for (i, function) in module.functions.iter().enumerate() {
        if i > 0 {
            writeln!(out).unwrap();
        }
        emit_function(module, function, &mut out)?;
    }

    Ok(out)
}

fn emit_function(module: &Module, function: &Function, out: &mut String) -> Result<(), EmitError> {
    let params = function
        .params
        .iter()
        .map(|p| format!("{} {}", render_type(&p.ty), p.name))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "define {} @{}({}) {{", render_type(&function.ret), function.name, params).unwrap();

    let slots = collect_slots(function);
    let mut state = FunctionEmitState::new(module, slots);

    writeln!(out, "entry:").unwrap();
    for (reg, ty) in &state.slots {
        writeln!(out, "  {} = alloca {}", slot_name(reg), render_type(ty)).unwrap();
    }
    for param in &function.params {
        writeln!(
            out,
            "  store {} {}, ptr {}",
            render_type(&param.ty),
            param.name,
            slot_name(&param.name)
        )
        .unwrap();
    }
    if let Some(first) = function.blocks.first() {
        writeln!(out, "  br label %{}", first.label).unwrap();
    } else {
        writeln!(out, "  ret {}", default_value(&function.ret)).unwrap();
    }

    for block in &function.blocks {
        emit_block(&mut state, block, out)?;
    }

    writeln!(out, "}}").unwrap();
    Ok(())
}

fn emit_block(state: &mut FunctionEmitState<'_>, block: &Block, out: &mut String) -> Result<(), EmitError> {
    writeln!(out, "{}:", block.label).unwrap();
    for instr in &block.instructions {
        emit_instruction(state, instr, out)?;
    }
    emit_terminator(state, &block.terminator, out)?;
    Ok(())
}

fn emit_instruction(
    state: &mut FunctionEmitState<'_>,
    instr: &Instruction,
    out: &mut String,
) -> Result<(), EmitError> {
    match instr {
        Instruction::Assign { result, ty, value } => {
            let value = lower_operand(state, value, out)?;
            writeln!(out, "  store {} {}, ptr {}", render_type(ty), value, slot_name(result)).unwrap();
            Ok(())
        }
        Instruction::BinaryOp {
            result,
            op,
            ty,
            lhs,
            rhs,
        } => {
            let lhs_ty = operand_type(lhs);
            let rhs_ty = operand_type(rhs);
            let lhs = lower_operand(state, lhs, out)?;
            let rhs = lower_operand(state, rhs, out)?;
            let tmp = state.fresh();
            let op_text = match op {
                BinOp::Add => "add",
                BinOp::Sub => "sub",
                BinOp::Mul => "mul",
                BinOp::Lt => "icmp slt",
                BinOp::LtEq => "icmp sle",
                BinOp::Gt => "icmp sgt",
                BinOp::GtEq => "icmp sge",
                BinOp::EqEq => "icmp eq",
            };
            let value_ty = lhs_ty.clone();
            if lhs_ty != rhs_ty {
                return Err(EmitError::UnsupportedInstruction("mixed-type binary op"));
            }
            writeln!(
                out,
                "  {tmp} = {op_text} {} {}, {}",
                render_type(&value_ty),
                lhs,
                rhs
            )
            .unwrap();
            writeln!(out, "  store {} {tmp}, ptr {}", render_type(ty), slot_name(result)).unwrap();
            Ok(())
        }
        Instruction::PackEnv { result, captures } => {
            if captures.is_empty() {
                writeln!(out, "  store ptr null, ptr {}", slot_name(result)).unwrap();
                return Ok(());
            }

            let struct_ty = format!(
                "{{ {} }}",
                captures
                    .iter()
                    .map(|op| render_type(&operand_type(op)))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let env_ptr = state.fresh();
            writeln!(out, "  {env_ptr} = alloca {struct_ty}").unwrap();
            for (idx, capture) in captures.iter().enumerate() {
                let capture_val = lower_operand(state, capture, out)?;
                let field_ptr = state.fresh();
                let field_ty = operand_type(capture);
                writeln!(
                    out,
                    "  {field_ptr} = getelementptr inbounds {struct_ty}, ptr {env_ptr}, i32 0, i32 {idx}"
                )
                .unwrap();
                writeln!(
                    out,
                    "  store {} {}, ptr {}",
                    render_type(&field_ty),
                    capture_val,
                    field_ptr
                )
                .unwrap();
            }
            writeln!(out, "  store ptr {env_ptr}, ptr {}", slot_name(result)).unwrap();
            Ok(())
        }
        Instruction::CallExtern {
            result,
            symbol,
            ret,
            args,
            comment,
        } => {
            let args = args
                .iter()
                .map(|arg| {
                    let ty = operand_type(arg);
                    lower_operand(state, arg, out).map(|value| format!("{} {}", render_type(&ty), value))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            if let Some(result) = result {
                let tmp = state.fresh();
                write!(out, "  {tmp} = call {} @{}({})", render_type(ret), symbol, args).unwrap();
                if let Some(comment) = comment {
                    write!(out, " ; {comment}").unwrap();
                }
                writeln!(out).unwrap();
                writeln!(out, "  store {} {tmp}, ptr {}", render_type(ret), slot_name(result)).unwrap();
            } else {
                write!(out, "  call {} @{}({})", render_type(ret), symbol, args).unwrap();
                if let Some(comment) = comment {
                    write!(out, " ; {comment}").unwrap();
                }
                writeln!(out).unwrap();
            }
            Ok(())
        }
    }
}

fn emit_terminator(
    state: &mut FunctionEmitState<'_>,
    term: &Terminator,
    out: &mut String,
) -> Result<(), EmitError> {
    match term {
        Terminator::Ret(Some(value)) => {
            let ty = operand_type(value);
            let value = lower_operand(state, value, out)?;
            writeln!(out, "  ret {} {}", render_type(&ty), value).unwrap();
            Ok(())
        }
        Terminator::Ret(None) => {
            writeln!(out, "  ret void").unwrap();
            Ok(())
        }
        Terminator::Br(label) => {
            writeln!(out, "  br label %{}", label).unwrap();
            Ok(())
        }
        Terminator::CondBr {
            cond,
            then_label,
            else_label,
        } => {
            let cond = lower_operand(state, cond, out)?;
            writeln!(out, "  br i1 {}, label %{}, label %{}", cond, then_label, else_label).unwrap();
            Ok(())
        }
        Terminator::MatchBr { .. } => Err(EmitError::UnsupportedTerminator("match")),
        Terminator::Unreachable => {
            writeln!(out, "  unreachable").unwrap();
            Ok(())
        }
    }
}

struct FunctionEmitState<'a> {
    module: &'a Module,
    slots: BTreeMap<String, Type>,
    next_tmp: usize,
}

impl<'a> FunctionEmitState<'a> {
    fn new(module: &'a Module, slots: BTreeMap<String, Type>) -> Self {
        Self {
            module,
            slots,
            next_tmp: 0,
        }
    }

    fn fresh(&mut self) -> String {
        let tmp = format!("%t{}", self.next_tmp);
        self.next_tmp += 1;
        tmp
    }
}

fn collect_slots(function: &Function) -> BTreeMap<String, Type> {
    let mut out = BTreeMap::new();
    for param in &function.params {
        out.insert(param.name.clone(), param.ty.clone());
    }
    for block in &function.blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::Assign { result, ty, .. } | Instruction::BinaryOp { result, ty, .. } => {
                    out.insert(result.clone(), ty.clone());
                }
                Instruction::PackEnv { result, .. } => {
                    out.insert(result.clone(), Type::Ptr);
                }
                Instruction::CallExtern { result, ret, .. } => {
                    if let Some(result) = result {
                        out.insert(result.clone(), ret.clone());
                    }
                }
            }
        }
    }
    out
}

fn lower_operand(state: &mut FunctionEmitState<'_>, operand: &Operand, out: &mut String) -> Result<String, EmitError> {
    match operand {
        Operand::ConstInt(value) => Ok(value.to_string()),
        Operand::Register(name, ty) => {
            if name.starts_with("%py_") {
                return Err(EmitError::UnsupportedOperand(name.clone()));
            }
            if name == "%unit" {
                return Ok("null".to_string());
            }
            let tmp = state.fresh();
            writeln!(out, "  {tmp} = load {}, ptr {}", render_type(ty), slot_name(name)).unwrap();
            Ok(tmp)
        }
        Operand::Global(name, _) => {
            let global = state
                .module
                .globals
                .iter()
                .find(|g| g.symbol == *name)
                .ok_or_else(|| EmitError::MissingStringGlobal(name.clone()))?;
            let tmp = state.fresh();
            let len = global.value.len() + 1;
            writeln!(
                out,
                "  {tmp} = getelementptr inbounds [{} x i8], ptr @{}, i64 0, i64 0",
                len, name
            )
            .unwrap();
            Ok(tmp)
        }
    }
}

fn operand_type(op: &Operand) -> Type {
    match op {
        Operand::Register(_, ty) | Operand::Global(_, ty) => ty.clone(),
        Operand::ConstInt(_) => Type::I64,
    }
}

fn render_type(ty: &Type) -> &'static str {
    match ty {
        Type::I64 => "i64",
        Type::Double => "double",
        Type::I1 => "i1",
        Type::Ptr => "ptr",
        Type::Void => "void",
    }
}

fn render_global(global: &GlobalString) -> String {
    format!(
        "@{} = private unnamed_addr constant [{} x i8] {}",
        global.symbol,
        global.value.len() + 1,
        render_c_string(&global.value)
    )
}

fn render_c_string(value: &str) -> String {
    let mut out = String::from("c\"");
    for byte in value.bytes() {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\r' => out.push_str("\\0D"),
            b'\t' => out.push_str("\\09"),
            0x20..=0x7E => out.push(byte as char),
            _ => out.push_str(&format!("\\{:02X}", byte)),
        }
    }
    out.push_str("\\00\"");
    out
}

fn slot_name(reg: &str) -> String {
    format!("%slot{}", &reg[1..])
}

fn default_value(ty: &Type) -> String {
    match ty {
        Type::Void => "void".to_string(),
        Type::I64 => "i64 0".to_string(),
        Type::Double => "double 0.0".to_string(),
        Type::I1 => "i1 false".to_string(),
        Type::Ptr => "ptr null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_codegen::lower_module as lower_low;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_llvm::lower_module as lower_llvm_like;
    use dx_mir::lower_module as lower_mir;
    use dx_parser::{Lexer, Parser};

    fn llvm_module(src: &str) -> Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        let mir = lower_mir(&typed.module);
        let low = lower_low(&mir);
        lower_llvm_like(&low)
    }

    #[test]
    fn emits_real_ir_for_plain_arithmetic() {
        let module = llvm_module("fun f(x: Int) -> Int:\n    val y = x + 1\n    y\n.\n");
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("entry:"), "got:\n{ir}");
        assert!(ir.contains("alloca i64"), "got:\n{ir}");
        assert!(ir.contains("add i64"), "got:\n{ir}");
        assert!(ir.contains("ret i64"), "got:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_string_global_return() {
        let module = llvm_module("fun f() -> Str:\n    \"hello\"\n.\n");
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("@.str0 = private unnamed_addr constant [6 x i8] c\"hello\\00\""), "got:\n{ir}");
        assert!(ir.contains("getelementptr inbounds [6 x i8], ptr @.str0, i64 0, i64 0"), "got:\n{ir}");
        assert!(ir.contains("ret ptr %t"), "got:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_if_without_runtime_hooks() {
        let module = llvm_module("fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n");
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("br i1"), "got:\n{ir}");
        assert!(ir.contains("bb0:"), "got:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_unit_return() {
        let module = llvm_module("fun f() -> Unit:\n    42\n.\n");
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("define void @f()"), "got:\n{ir}");
        assert!(ir.contains("ret void"), "got:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_thunk_runtime_call() {
        let module = llvm_module("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("@dx_rt_closure_create"), "got:\n{ir}");
        assert!(ir.contains("@dx_rt_thunk_call_i64"), "got:\n{ir}");
        assert!(ir.contains("alloca { i64 }"), "got:\n{ir}");
        assert!(ir.contains("getelementptr inbounds { i64 }"), "got:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_python_function_runtime_call() {
        let module = llvm_module(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("declare ptr @dx_rt_py_call_function(ptr, i64)"), "got:\n{ir}");
        assert!(ir.contains("@.str0 = private unnamed_addr constant [9 x i8] c\"read_csv\\00\""), "got:\n{ir}");
        assert!(ir.contains("call ptr @dx_rt_py_call_function("), "got:\n{ir}");
        assert!(ir.contains("ptr %t"), "expected lowered string global gep arg:\n{ir}");
        assert!(ir.contains("i64 1"), "expected arg count:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_python_method_runtime_call() {
        let module = llvm_module(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("declare ptr @dx_rt_py_call_method(ptr, ptr, i64)"), "got:\n{ir}");
        assert!(ir.contains("c\"head\\00\""), "expected method name global:\n{ir}");
        assert!(ir.contains("call ptr @dx_rt_py_call_method("), "got:\n{ir}");
        assert!(ir.contains("i64 0"), "expected method arg count:\n{ir}");
    }

    #[test]
    fn emits_real_ir_for_python_dynamic_runtime_call() {
        let module = llvm_module(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val g = read_csv(path)\n    g()\n.\n",
        );
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("declare ptr @dx_rt_py_call_dynamic(ptr, i64)"), "got:\n{ir}");
        assert!(ir.contains("call ptr @dx_rt_py_call_dynamic("), "got:\n{ir}");
        assert!(ir.contains("i64 0"), "expected dynamic arg count:\n{ir}");
    }

    #[test]
    fn emits_runtime_call_comments_for_closure_call_path() {
        let module = llvm_module(
            "fun run(x: Int) -> Int:\n    val f = (y: Int) => x + y\n    f(1)\n.\n",
        );
        let ir = emit_module(&module).expect("emit");
        assert!(ir.contains("@dx_rt_closure_call_i64"), "got:\n{ir}");
        assert!(ir.contains("; stmt="), "expected runtime comment:\n{ir}");
        assert!(ir.contains("closure-call args=1"), "expected closure call comment:\n{ir}");
        assert!(ir.contains("call_args=[1]"), "expected preserved call args in comment:\n{ir}");
    }

    #[test]
    fn rejects_match_for_now() {
        let module = llvm_module(
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        _:\n            0\n    .\n.\n",
        );
        let err = emit_module(&module).expect_err("match should be unsupported");
        assert!(matches!(err, EmitError::UnsupportedTerminator("match")));
    }
}
