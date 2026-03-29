use crate::llvm::{
    Block, ExternDecl, Function, GlobalString, Instruction, Module, Operand, Terminator, Type,
};
use std::fmt::Write;

pub fn render_module(module: &Module) -> String {
    let mut out = String::new();

    for global in &module.globals {
        writeln!(out, "{}", render_global(global)).unwrap();
    }

    if !module.globals.is_empty() && (!module.externs.is_empty() || !module.functions.is_empty()) {
        writeln!(out).unwrap();
    }

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

fn render_global(global: &GlobalString) -> String {
    format!(
        "@{} = private unnamed_addr constant [{} x i8] {}",
        global.symbol,
        global.value.len() + 1,
        render_c_string(&global.value)
    )
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
        render_block(out, block);
    }

    writeln!(out, "}}").unwrap();
}

fn render_block(out: &mut String, block: &Block) {
    writeln!(out, "{}:", block.label).unwrap();
    for instr in &block.instructions {
        writeln!(out, "  {}", render_instruction(instr)).unwrap();
    }
    writeln!(out, "  {}", render_terminator(&block.terminator)).unwrap();
}

fn render_instruction(instr: &Instruction) -> String {
    match instr {
        Instruction::Assign { result, ty: _, value } => format!("{result} = copy {}", render_operand(value)),
        Instruction::BinaryOp {
            result,
            op,
            ty,
            lhs,
            rhs,
        } => format!(
            "{result} = {} {} {}, {}",
            render_binop(op, ty),
            render_type(ty),
            render_operand(lhs),
            render_operand(rhs)
        ),
        Instruction::PackEnv { result, captures } => {
            let captures = captures
                .iter()
                .map(render_operand)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{result} = pack_env [{captures}]")
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

fn render_binop(op: &dx_parser::BinOp, ty: &Type) -> &'static str {
    match op {
        dx_parser::BinOp::Add => "add",
        dx_parser::BinOp::Sub => "sub",
        dx_parser::BinOp::Mul => "mul",
        dx_parser::BinOp::Lt => match ty {
            Type::I64 => "icmp slt",
            _ => "icmp lt",
        },
        dx_parser::BinOp::LtEq => match ty {
            Type::I64 => "icmp sle",
            _ => "icmp le",
        },
        dx_parser::BinOp::Gt => match ty {
            Type::I64 => "icmp sgt",
            _ => "icmp gt",
        },
        dx_parser::BinOp::GtEq => match ty {
            Type::I64 => "icmp sge",
            _ => "icmp ge",
        },
        dx_parser::BinOp::EqEq => "icmp eq",
    }
}

fn render_operand(op: &Operand) -> String {
    match op {
        Operand::Register(name, ty) => format!("{} {}", render_type(ty), name),
        Operand::Global(name, ty) => format!("{} @{}", render_type(ty), name),
        Operand::ConstInt(v) => format!("i64 {v}"),
    }
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

fn render_type(ty: &Type) -> &'static str {
    match ty {
        Type::I64 => "i64",
        Type::Double => "double",
        Type::I1 => "i1",
        Type::Ptr => "ptr",
        Type::Void => "void",
    }
}

fn render_terminator(term: &Terminator) -> String {
    match term {
        Terminator::Ret(Some(op)) => format!("ret {}", render_operand(op)),
        Terminator::Ret(None) => "ret void".to_string(),
        Terminator::Br(label) => format!("br label %{label}"),
        Terminator::CondBr {
            cond,
            then_label,
            else_label,
        } => format!(
            "br {}, label %{}, label %{}",
            render_operand(cond),
            then_label,
            else_label
        ),
        Terminator::MatchBr {
            scrutinee,
            arms,
            fallback,
        } => {
            let mut out = format!("match {}", render_operand(scrutinee));
            if !arms.is_empty() {
                out.push_str(" [");
                for (i, (pat, label)) in arms.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("{pat}: %{label}"));
                }
                out.push(']');
            }
            out.push_str(&format!(" else %{}", fallback));
            out
        }
        Terminator::Unreachable => "unreachable".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
    use dx_hir::{lower_module as lower_hir, typecheck_module};
    use dx_parser::{Lexer, Parser};

    fn render(src: &str) -> String {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("parse");
        let hir = lower_hir(&ast);
        let typed = typecheck_module(&hir);
        let mir = dx_mir::lower_module(&typed.module);
        let low = dx_codegen::lower_module(&mir);
        let llvm = lower_module(&low);
        render_module(&llvm)
    }

    // ── extern declarations ──────────────────────────────────────

    #[test]
    fn snapshot_extern_python_call() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("declare ptr @dx_rt_py_call_function("), "got:\n{out}");
    }

    #[test]
    fn snapshot_extern_closure_hooks() {
        let out = render("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
        assert!(out.contains("declare ptr @dx_rt_closure_create("), "got:\n{out}");
        assert!(out.contains("declare i64 @dx_rt_thunk_call_i64("), "got:\n{out}");
    }

    #[test]
    fn snapshot_externs_sorted_alphabetically() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
        );
        let declare_lines: Vec<&str> = out.lines().filter(|l| l.starts_with("declare")).collect();
        let symbols: Vec<&str> = declare_lines
            .iter()
            .filter_map(|line| line.split('@').nth(1))
            .filter_map(|tail| tail.split('(').next())
            .collect();
        let mut sorted = symbols.clone();
        sorted.sort();
        assert_eq!(symbols, sorted, "externs should be sorted by symbol");
    }

    #[test]
    fn snapshot_no_externs_when_none_needed() {
        let out = render("fun f(x: Int) -> Int:\n    x + 1\n.\n");
        assert!(!out.contains("declare"), "got:\n{out}");
    }

    // ── function signatures ──────────────────────────────────────

    #[test]
    fn snapshot_function_with_params() {
        let out = render("fun f(x: Int, y: Bool) -> Int:\n    x\n.\n");
        assert!(out.contains("define i64 @f(i64 %0, i1 %1)"), "got:\n{out}");
    }

    #[test]
    fn snapshot_void_function() {
        let out = render("fun f():\n    42\n.\n");
        assert!(out.contains("define void @f()"), "got:\n{out}");
    }

    #[test]
    fn snapshot_ptr_return_for_string() {
        let out = render("fun f(s: Str) -> Str:\n    s\n.\n");
        assert!(out.contains("define ptr @f(ptr %0)"), "got:\n{out}");
    }

    #[test]
    fn snapshot_string_literal_global() {
        let out = render("fun f() -> Str:\n    \"hello\"\n.\n");
        assert!(out.contains("@.str0 = private unnamed_addr constant [6 x i8] c\"hello\\00\""), "got:\n{out}");
        assert!(out.contains("ret ptr @.str0"), "got:\n{out}");
    }

    // ── runtime call instructions ────────────────────────────────

    #[test]
    fn snapshot_python_call_instruction() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("call ptr @dx_rt_py_call_function("), "got:\n{out}");
        assert!(out.contains("; stmt=0, py-call"), "got:\n{out}");
    }

    #[test]
    fn snapshot_plain_assignment_instruction() {
        let out = render("fun f() -> Int:\n    val y = 42\n    y\n.\n");
        assert!(out.contains("= copy i64 42"), "got:\n{out}");
    }

    #[test]
    fn snapshot_binary_op_instruction() {
        let out = render("fun f(x: Int) -> Int:\n    val y = x + 1\n    y\n.\n");
        assert!(out.contains("= add i64 i64 %0, i64 1"), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure_create_instruction() {
        let out = render("fun f(x: Int) -> lazy Int:\n    lazy x\n.\n");
        assert!(out.contains("pack_env [i64 %0]"), "got:\n{out}");
        assert!(out.contains("call ptr @dx_rt_closure_create("), "got:\n{out}");
        assert!(out.contains("closure-create"), "got:\n{out}");
    }

    #[test]
    fn snapshot_thunk_call_instruction() {
        let out = render("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
        assert!(out.contains("call i64 @dx_rt_thunk_call_i64("), "got:\n{out}");
        assert!(out.contains("thunk-call"), "got:\n{out}");
    }

    // ── throw-check instructions ─────────────────────────────────

    #[test]
    fn snapshot_throw_check_after_py_call() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("call void @dx_rt_throw_check_pending()"), "got:\n{out}");
        assert!(out.contains("throw-boundary="), "got:\n{out}");
        // throw-check must come after the py call
        let call_pos = out.find("@dx_rt_py_call_function").expect("py call");
        let check_pos = out.find("@dx_rt_throw_check_pending").expect("throw check");
        assert!(check_pos > call_pos, "throw check should follow py call");
    }

    #[test]
    fn snapshot_throw_check_python_method() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)'head()\n.\n",
        );
        assert!(out.contains("PythonMethod"), "got:\n{out}");
    }

    // ── mixed scenarios ──────────────────────────────────────────

    #[test]
    fn snapshot_mixed_python_and_closure() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
        );
        assert!(out.contains("@dx_rt_py_call_function"), "py call:\n{out}");
        assert!(out.contains("@dx_rt_closure_create"), "closure create:\n{out}");
        assert!(out.contains("@dx_rt_thunk_call"), "thunk call:\n{out}");
    }

    #[test]
    fn snapshot_multiple_functions() {
        let out = render(
            "fun a(x: Int) -> Int:\n    x\n.\n\nfun b(y: Str) -> Str:\n    y\n.\n",
        );
        assert!(out.contains("define i64 @a("), "got:\n{out}");
        assert!(out.contains("define ptr @b("), "got:\n{out}");
    }

    // ── terminators ──────────────────────────────────────────────

    #[test]
    fn snapshot_ret_with_value() {
        let out = render("fun f(x: Int) -> Int:\n    x\n.\n");
        // Should contain a ret instruction in some block
        assert!(out.contains("ret "), "got:\n{out}");
    }

    #[test]
    fn snapshot_ret_void_signature() {
        // fun f() with no declared return type has void signature
        // but the body still returns a value (the lowering uses the body result)
        let out = render("fun f():\n    42\n.\n");
        assert!(out.contains("define void @f()"), "got:\n{out}");
        assert!(out.contains("ret "), "got:\n{out}");
    }

    #[test]
    fn snapshot_br_and_condbr_in_if() {
        let out = render(
            "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        );
        // Should have conditional branch and unconditional branches (goto -> join block)
        assert!(out.contains("br "), "got:\n{out}");
        assert!(out.contains("label %"), "got:\n{out}");
    }

    #[test]
    fn snapshot_match_lowered_to_cond_checks() {
        let out = render(
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
        );
        // Match is now lowered to dx_rt_match_tag calls + CondBr chains
        assert!(out.contains("@dx_rt_match_tag"), "match_tag call:\n{out}");
        assert!(out.contains("br "), "branch instruction:\n{out}");
        // No raw MatchBr should remain
        assert!(!out.contains("match ptr"), "raw MatchBr should not appear:\n{out}");
    }

    #[test]
    fn snapshot_blocks_with_only_terminator() {
        // then/else branches of if typically have no runtime calls, just terminators
        let out = render(
            "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        );
        // Multiple blocks should be visible with labels
        let block_count = out.lines().filter(|l| l.ends_with(':')).count();
        assert!(block_count >= 3, "expected >=3 blocks for if/else, got {block_count}:\n{out}");
    }

    #[test]
    fn snapshot_block_ordering_stable() {
        let src = "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n";
        let out = render(src);
        let labels: Vec<&str> = out
            .lines()
            .filter(|l| l.ends_with(':') && !l.starts_with("declare") && !l.starts_with("define"))
            .collect();
        // Labels should be in ascending bb order
        for i in 1..labels.len() {
            assert!(
                labels[i] > labels[i - 1],
                "blocks out of order: {:?}",
                labels
            );
        }
    }

    // ── unreachable terminator ─────────────────────────────────

    #[test]
    fn snapshot_unreachable_via_unit_render() {
        // A standalone unit-type render with render_terminator
        use crate::llvm::Terminator;
        let out = render_terminator(&Terminator::Unreachable);
        assert_eq!(out, "unreachable");
    }

    // ── throw-boundary comment format ────────────────────────────

    #[test]
    fn snapshot_throw_boundary_comment_format() {
        // Document the current format of throw-boundary comments
        // (uses Debug formatting from lower.rs which we cannot change)
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    read_csv(path)\n.\n",
        );
        // The comment includes "throw-boundary=PythonFunction" (Debug format)
        assert!(
            out.contains("throw-boundary=PythonFunction"),
            "throw-boundary comment should be present:\n{out}"
        );
    }

    // ── determinism ──────────────────────────────────────────────

    #[test]
    fn render_is_deterministic() {
        let src = "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n";
        let out1 = render(src);
        let out2 = render(src);
        assert_eq!(out1, out2, "rendering should be deterministic");
    }
}
