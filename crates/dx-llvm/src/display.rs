use crate::llvm::{Block, ExternDecl, Function, Instruction, Module, Operand, Terminator, Type};
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
        assert!(out.contains("declare ptr @dx_rt_thunk_call("), "got:\n{out}");
    }

    #[test]
    fn snapshot_externs_sorted_alphabetically() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n",
        );
        let declare_lines: Vec<&str> = out.lines().filter(|l| l.starts_with("declare")).collect();
        let mut sorted = declare_lines.clone();
        sorted.sort();
        assert_eq!(declare_lines, sorted, "externs should be alphabetically sorted");
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
    fn snapshot_closure_create_instruction() {
        let out = render("fun f(x: Int) -> lazy Int:\n    lazy x\n.\n");
        assert!(out.contains("call ptr @dx_rt_closure_create("), "got:\n{out}");
        assert!(out.contains("closure-create"), "got:\n{out}");
    }

    #[test]
    fn snapshot_thunk_call_instruction() {
        let out = render("fun f(x: Int) -> Int:\n    val t = lazy x\n    t()\n.\n");
        assert!(out.contains("call i64 @dx_rt_thunk_call("), "got:\n{out}");
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

    // ── determinism ──────────────────────────────────────────────

    #[test]
    fn render_is_deterministic() {
        let src = "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py !throw:\n    val df = read_csv(path)\n    val t = lazy df\n    t()\n.\n";
        let out1 = render(src);
        let out2 = render(src);
        assert_eq!(out1, out2, "rendering should be deterministic");
    }
}
