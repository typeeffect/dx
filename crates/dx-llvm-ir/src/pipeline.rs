use crate::emit::{emit_module, EmitError};
use dx_codegen::lower_module as lower_low;
use dx_hir::{lower_module as lower_hir, typecheck_module};
use dx_llvm::{lower_module as lower_llvm_like, validate_module, ValidationDiagnostic};
use dx_mir::lower_module as lower_mir;
use dx_parser::{Lexer, Parser};
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum PipelineError {
    Io(std::io::Error),
    Parse(String),
    Validation(Vec<ValidationDiagnostic>),
    Emit(EmitError),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::Io(err) => write!(f, "i/o error: {err}"),
            PipelineError::Parse(message) => write!(f, "parse error: {message}"),
            PipelineError::Validation(diagnostics) => {
                writeln!(f, "llvm validation failed:")?;
                for diag in diagnostics {
                    match (&diag.function, &diag.block) {
                        (Some(function), Some(block)) => {
                            writeln!(f, "- {function}/{block}: {}", diag.message)?;
                        }
                        (Some(function), None) => {
                            writeln!(f, "- {function}: {}", diag.message)?;
                        }
                        _ => {
                            writeln!(f, "- {}", diag.message)?;
                        }
                    }
                }
                Ok(())
            }
            PipelineError::Emit(err) => write!(f, "emit error: {err:?}"),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<std::io::Error> for PipelineError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<EmitError> for PipelineError {
    fn from(value: EmitError) -> Self {
        Self::Emit(value)
    }
}

pub fn emit_source_to_string(src: &str) -> Result<String, PipelineError> {
    let tokens = Lexer::new(src).tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse_module()
        .map_err(|err| PipelineError::Parse(err.message))?;
    let hir = lower_hir(&ast);
    let typed = typecheck_module(&hir);
    let mir = lower_mir(&typed.module);
    let low = lower_low(&mir);
    let llvm = lower_llvm_like(&low);
    let report = validate_module(&llvm);
    if !report.is_ok() {
        return Err(PipelineError::Validation(report.diagnostics));
    }
    Ok(emit_module(&llvm)?)
}

pub fn emit_file_to_string(path: &Path) -> Result<String, PipelineError> {
    let src = fs::read_to_string(path)?;
    emit_source_to_string(&src)
}

pub fn emit_file_to_path(input: &Path, output: &Path) -> Result<(), PipelineError> {
    let ir = emit_file_to_string(input)?;
    fs::write(output, ir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn emits_plain_arithmetic_source() {
        let ir = emit_source_to_string("fun f(x: Int) -> Int:\n    val y = x + 1\n    y\n.\n")
            .expect("emit");
        assert!(ir.contains("define i64 @f"), "got:\n{ir}");
        assert!(ir.contains("add i64"), "got:\n{ir}");
    }

    #[test]
    fn surfaces_parse_errors() {
        let err = emit_source_to_string("fun f(x Int) -> Int:\n    x\n.\n").expect_err("parse should fail");
        match err {
            PipelineError::Parse(message) => {
                assert!(message.contains("expected `:`"), "got: {message}");
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[test]
    fn writes_ir_to_output_path() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("dx-llvm-ir-{nonce}"));
        fs::create_dir_all(&base).expect("mkdir");
        let input = base.join("input.dx");
        let output = base.join("output.ll");
        fs::write(&input, "fun f() -> Int:\n    1\n.\n").expect("write input");

        emit_file_to_path(&input, &output).expect("emit file");

        let ir = fs::read_to_string(&output).expect("read output");
        assert!(ir.contains("define i64 @f()"), "got:\n{ir}");

        let _ = fs::remove_file(&input);
        let _ = fs::remove_file(&output);
        let _ = fs::remove_dir(&base);
    }
}
