use crate::mir;
use dx_hir::{typed::CallTarget, Type};
use std::fmt::Write;

pub fn render_module(module: &mir::Module) -> String {
    let mut out = String::new();
    for item in &module.items {
        match item {
            mir::Item::ImportPy(import) => {
                writeln!(out, "from py {} import {}", import.module, import.names.join(", ")).unwrap();
            }
            mir::Item::Function(function) => {
                render_function(function, &mut out);
            }
        }
    }
    out
}

pub fn render_function(function: &mir::Function, out: &mut String) {
    // signature
    write!(out, "fn {}(", function.name).unwrap();
    for (i, &param) in function.params.iter().enumerate() {
        if i > 0 {
            write!(out, ", ").unwrap();
        }
        let local = &function.locals[param];
        write!(out, "{}: {}", local.name, render_type(&local.ty)).unwrap();
    }
    write!(out, ")").unwrap();
    if let Some(ret) = &function.return_type {
        write!(out, " -> {}", render_type(ret)).unwrap();
    }
    if !function.effects.is_empty() {
        for effect in &function.effects {
            write!(out, " !{effect}").unwrap();
        }
    }
    writeln!(out, " {{").unwrap();

    // locals
    for (id, local) in function.locals.iter().enumerate() {
        if function.params.contains(&id) {
            continue;
        }
        let kind = if local.synthetic {
            "tmp"
        } else if local.mutable {
            "var"
        } else {
            "let"
        };
        writeln!(out, "  {kind} _{id}: {}  // {}", render_type(&local.ty), local.name).unwrap();
    }
    if function.locals.len() > function.params.len() {
        writeln!(out).unwrap();
    }

    // blocks
    for (id, block) in function.blocks.iter().enumerate() {
        writeln!(out, "  bb{id}:").unwrap();
        for stmt in &block.statements {
            write!(out, "    ").unwrap();
            render_statement(stmt, out);
            writeln!(out).unwrap();
        }
        write!(out, "    ").unwrap();
        render_terminator(&block.terminator, out);
        writeln!(out).unwrap();
    }

    writeln!(out, "}}").unwrap();
}

fn render_statement(stmt: &mir::Statement, out: &mut String) {
    match stmt {
        mir::Statement::Assign { place, value } => {
            write!(out, "_{place} = ").unwrap();
            render_rvalue(value, out);
        }
    }
}

fn render_rvalue(rv: &mir::Rvalue, out: &mut String) {
    match rv {
        mir::Rvalue::Use(operand) => render_operand(operand, out),
        mir::Rvalue::BinaryOp { op, lhs, rhs } => {
            render_operand(lhs, out);
            write!(out, " {} ", op_str(op)).unwrap();
            render_operand(rhs, out);
        }
        mir::Rvalue::Member { base, name } => {
            render_operand(base, out);
            write!(out, "'{name}").unwrap();
        }
        mir::Rvalue::Call {
            target,
            callee,
            args,
            effects,
            ..
        } => {
            let tag = match target {
                CallTarget::NativeFunction { name } => format!("[native:{name}]"),
                CallTarget::PythonFunction { name } => format!("[py:{name}]"),
                CallTarget::PythonMember { name } => format!("[py.{name}]"),
                CallTarget::PythonDynamic => "[py:?]".to_string(),
                CallTarget::LocalClosure { name } => format!("[closure:{name}]"),
                CallTarget::Dynamic => "[dyn]".to_string(),
            };
            write!(out, "call{tag} ").unwrap();
            render_operand(callee, out);
            write!(out, "(").unwrap();
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ").unwrap();
                }
                match arg {
                    mir::CallArg::Positional(v) => render_operand(v, out),
                    mir::CallArg::Named { name, value } => {
                        write!(out, "{name}: ").unwrap();
                        render_operand(value, out);
                    }
                }
            }
            write!(out, ")").unwrap();
            if !effects.is_empty() {
                for effect in effects {
                    write!(out, " !{effect}").unwrap();
                }
            }
        }
        mir::Rvalue::Closure {
            param_types,
            return_type,
            effects,
        } => {
            write!(out, "closure(").unwrap();
            for (i, ty) in param_types.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ").unwrap();
                }
                write!(out, "{}", render_type(ty)).unwrap();
            }
            write!(out, ") -> {}", render_type(return_type)).unwrap();
            for effect in effects {
                write!(out, " !{effect}").unwrap();
            }
        }
    }
}

fn render_operand(operand: &mir::Operand, out: &mut String) {
    match operand {
        mir::Operand::Copy(id) => write!(out, "_{id}").unwrap(),
        mir::Operand::Const(c) => match c {
            mir::Constant::Int(v) => write!(out, "{v}").unwrap(),
            mir::Constant::String(v) => write!(out, "\"{v}\"").unwrap(),
            mir::Constant::Unit => write!(out, "()").unwrap(),
        },
    }
}

fn render_terminator(term: &mir::Terminator, out: &mut String) {
    match term {
        mir::Terminator::Return(Some(v)) => {
            write!(out, "return ").unwrap();
            render_operand(v, out);
        }
        mir::Terminator::Return(None) => write!(out, "return").unwrap(),
        mir::Terminator::Goto(bb) => write!(out, "goto bb{bb}").unwrap(),
        mir::Terminator::SwitchBool {
            cond,
            then_bb,
            else_bb,
        } => {
            write!(out, "switch ").unwrap();
            render_operand(cond, out);
            write!(out, " -> [true: bb{then_bb}, false: bb{else_bb}]").unwrap();
        }
        mir::Terminator::Match {
            scrutinee,
            arms,
            fallback,
        } => {
            write!(out, "match ").unwrap();
            render_operand(scrutinee, out);
            write!(out, " -> [").unwrap();
            for (i, (pattern, bb)) in arms.iter().enumerate() {
                if i > 0 {
                    write!(out, ", ").unwrap();
                }
                write!(out, "{}: bb{bb}", pattern_str(pattern)).unwrap();
            }
            write!(out, ", _: bb{fallback}]").unwrap();
        }
        mir::Terminator::Unreachable => write!(out, "unreachable").unwrap(),
    }
}

fn op_str(op: &dx_parser::BinOp) -> &'static str {
    match op {
        dx_parser::BinOp::Add => "+",
        dx_parser::BinOp::Sub => "-",
        dx_parser::BinOp::Mul => "*",
        dx_parser::BinOp::Lt => "<",
        dx_parser::BinOp::LtEq => "<=",
        dx_parser::BinOp::Gt => ">",
        dx_parser::BinOp::GtEq => ">=",
        dx_parser::BinOp::EqEq => "==",
    }
}

fn pattern_str(pattern: &dx_hir::Pattern) -> String {
    match pattern {
        dx_hir::Pattern::Name(name) => name.clone(),
        dx_hir::Pattern::Wildcard => "_".to_string(),
        dx_hir::Pattern::Constructor { name, args } => {
            if args.is_empty() {
                name.clone()
            } else {
                let inner: Vec<String> = args.iter().map(pattern_str).collect();
                format!("{}({})", name, inner.join(", "))
            }
        }
    }
}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Str => "Str".to_string(),
        Type::Unit => "Unit".to_string(),
        Type::PyObj => "PyObj".to_string(),
        Type::Named(name) => name.clone(),
        Type::Function {
            params,
            ret,
            effects,
        } => {
            let params_str: Vec<String> = params.iter().map(render_type).collect();
            let mut s = format!("({}) -> {}", params_str.join(", "), render_type(ret));
            for effect in effects {
                write!(s, " !{effect}").unwrap();
            }
            s
        }
        Type::Unknown => "?".to_string(),
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
        let mir = lower_module(&typed.module);
        render_module(&mir)
    }

    #[test]
    fn snapshot_straight_line() {
        let out = render("fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n");
        assert!(out.contains("fn add(a: Int, b: Int) -> Int"), "got:\n{out}");
        assert!(out.contains("return"), "got:\n{out}");
        assert!(out.contains("+"), "got:\n{out}");
    }

    #[test]
    fn snapshot_if_else() {
        let out = render(
            "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
        );
        assert!(out.contains("switch"), "got:\n{out}");
        assert!(out.contains("true:"), "got:\n{out}");
        assert!(out.contains("false:"), "got:\n{out}");
        assert!(out.contains("goto"), "got:\n{out}");
    }

    #[test]
    fn snapshot_match() {
        let out = render(
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        Err(_):\n            0\n    .\n.\n",
        );
        assert!(out.contains("match"), "got:\n{out}");
        assert!(out.contains("Ok(v)"), "got:\n{out}");
        assert!(out.contains("Err(_)"), "got:\n{out}");
    }

    #[test]
    fn snapshot_python_call() {
        let out = render(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        );
        assert!(out.contains("from py pandas import read_csv"), "got:\n{out}");
        assert!(out.contains("[py:read_csv]"), "got:\n{out}");
        assert!(out.contains("!py"), "got:\n{out}");
    }

    #[test]
    fn snapshot_python_member_call() {
        let out = render(
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)'head()\n.\n",
        );
        assert!(out.contains("[py.head]"), "got:\n{out}");
    }

    #[test]
    fn snapshot_closure() {
        let out = render(
            "from py pandas import read_csv\n\nfun make(path: Str) -> lazy PyObj !py:\n    lazy read_csv(path)\n.\n",
        );
        assert!(out.contains("closure("), "got:\n{out}");
    }

    // ── type rendering ────────────────────────────────────────────

    #[test]
    fn render_type_primitives() {
        assert_eq!(render_type(&Type::Int), "Int");
        assert_eq!(render_type(&Type::Str), "Str");
        assert_eq!(render_type(&Type::Bool), "Bool");
        assert_eq!(render_type(&Type::Unit), "Unit");
        assert_eq!(render_type(&Type::PyObj), "PyObj");
        assert_eq!(render_type(&Type::Unknown), "?");
        assert_eq!(render_type(&Type::Named("Foo".into())), "Foo");
    }

    #[test]
    fn render_type_function() {
        let ty = Type::Function {
            params: vec![Type::Int, Type::Str],
            ret: Box::new(Type::Bool),
            effects: vec![],
        };
        assert_eq!(render_type(&ty), "(Int, Str) -> Bool");
    }

    #[test]
    fn render_type_function_with_effects() {
        let ty = Type::Function {
            params: vec![],
            ret: Box::new(Type::PyObj),
            effects: vec!["py".into(), "throw".into()],
        };
        assert_eq!(render_type(&ty), "() -> PyObj !py !throw");
    }

    // ── locals and temporaries ───────────────────────────────────

    #[test]
    fn snapshot_locals_and_temps() {
        let out = render(
            "fun f(x: Int) -> Int:\n    val y = x + 1\n    y\n.\n",
        );
        // params don't show up as locals
        assert!(out.contains("fn f(x: Int) -> Int"), "got:\n{out}");
        // y should be a let local
        assert!(out.contains("let _"), "got:\n{out}");
        assert!(out.contains("// y"), "got:\n{out}");
    }

    #[test]
    fn snapshot_mutable_var() {
        let out = render(
            "fun f() -> Int:\n    var x = 1\n    x = 2\n    x\n.\n",
        );
        assert!(out.contains("var _"), "got:\n{out}");
    }

    // ── function type in signature ───────────────────────────────

    #[test]
    fn snapshot_lazy_param_type() {
        let out = render(
            "fun f(compute: lazy Int !io) -> Int !io:\n    compute()\n.\n",
        );
        // lazy Int !io normalizes to () -> Int !io
        assert!(out.contains("() -> Int !io"), "got:\n{out}");
    }

    // ── Python dynamic call ──────────────────────────────────────

    #[test]
    fn snapshot_python_dynamic_chain() {
        let out = render(
            "from py pandas import read_csv\n\nfun f(path: Str) -> PyObj !py:\n    read_csv(path)'head()'values\n.\n",
        );
        // Should contain both py call and member access
        assert!(out.contains("[py:read_csv]"), "got:\n{out}");
        assert!(out.contains("'head"), "got:\n{out}");
        assert!(out.contains("'values"), "got:\n{out}");
    }

    // ── validation + rendering interaction ───────────────────────

    #[test]
    fn rendered_mir_round_trips_through_validation() {
        let sources = vec![
            "fun add(a: Int, b: Int) -> Int:\n    a + b\n.\n",
            "fun f(x: Bool) -> Int:\n    if x:\n        1\n    else:\n        2\n    .\n.\n",
            "fun f(x: Result) -> Int:\n    match x:\n        Ok(v):\n            v\n        _:\n            0\n    .\n.\n",
            "from py pandas import read_csv\n\nfun load(path: Str) -> PyObj !py:\n    read_csv(path)\n.\n",
        ];
        for src in sources {
            let tokens = Lexer::new(src).tokenize();
            let mut parser = Parser::new(tokens);
            let ast = parser.parse_module().expect("parse");
            let hir = lower_hir(&ast);
            let typed = typecheck_module(&hir);
            let mir = lower_module(&typed.module);
            let report = crate::validate::validate_module(&mir);
            assert!(
                report.diagnostics.is_empty(),
                "validation failed for:\n{}\ndiagnostics: {:?}",
                render_module(&mir),
                report.diagnostics
            );
        }
    }
}
