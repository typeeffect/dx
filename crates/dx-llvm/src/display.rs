use crate::llvm::{ExternDecl, Function, Instruction, Module, Operand, Type};
use std::fmt::Write;

pub fn render_module(module: &Module) -> String {
    let mut out = String::new();

    for ext in &module.externs {
        writeln!(out, "{}", render_extern(ext)).unwrap();
    }

    if !module.externs.is_empty() && !module.functions.is_empty() {
        writeln!(out).unwrap();
    }

    for (i, function) in module.functions.iter().enumerate() {
        if i > 0 {
            writeln!(out).unwrap();
        }
        render_function(&mut out, function);
    }

    out
}

fn render_extern(ext: &ExternDecl) -> String {
    let params = ext.params.iter().map(render_type).collect::<Vec<_>>().join(", ");
    format!("declare {} @{}({})", render_type(&ext.ret), ext.symbol, params)
}

fn render_function(out: &mut String, function: &Function) {
    let params = function
        .params
        .iter()
        .map(|param| format!("{} {}", render_type(&param.ty), param.name))
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(
        out,
        "define {} @{}({}) {{",
        render_type(&function.ret),
        function.name,
        params
    )
    .unwrap();

    for block in &function.blocks {
        writeln!(out, "{}:", block.label).unwrap();
        for instr in &block.instructions {
            writeln!(out, "  {}", render_instruction(instr)).unwrap();
        }
    }

    writeln!(out, "}}").unwrap();
}

fn render_instruction(instr: &Instruction) -> String {
    match instr {
        Instruction::CallExtern {
            result,
            symbol,
            ret,
            args,
            comment,
        } => {
            let args = args
                .iter()
                .map(render_operand)
                .collect::<Vec<_>>()
                .join(", ");
            let mut out = String::new();
            if let Some(result) = result {
                write!(out, "{result} = ").unwrap();
            }
            write!(out, "call {} @{}({})", render_type(ret), symbol, args).unwrap();
            if let Some(comment) = comment {
                write!(out, " ; {comment}").unwrap();
            }
            out
        }
    }
}

fn render_operand(op: &Operand) -> String {
    match op {
        Operand::Register(name, ty) => format!("{} {}", render_type(ty), name),
        Operand::ConstInt(v) => format!("i64 {v}"),
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
