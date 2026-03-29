use crate::emit::{emit_module, EmitError};
use crate::toolchain::{LlvmToolchain, ToolchainError};
use dx_codegen::lower_module as lower_low;
use dx_hir::{
    bind_locked_schema_artifacts_from_fs, load_bound_schema_catalog_from_fs,
    lower_module as lower_hir, typecheck_module, typecheck_module_with_bound_schemas,
    LockedSchemaArtifactBindingDiagnostic, TypeCheckDiagnostic, TypeCheckReport,
};
use dx_llvm::{lower_module as lower_llvm_like, validate_module, ValidationDiagnostic};
use dx_mir::lower_module as lower_mir;
use dx_parser::{Lexer, Parser};
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub enum PipelineError {
    Io(std::io::Error),
    Parse(String),
    SchemaLocks(Vec<LockedSchemaArtifactBindingDiagnostic>),
    TypeCheck(Vec<TypeCheckDiagnostic>),
    Validation(Vec<ValidationDiagnostic>),
    Emit(EmitError),
    Toolchain(ToolchainError),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipelineError::Io(err) => write!(f, "i/o error: {err}"),
            PipelineError::Parse(message) => write!(f, "parse error: {message}"),
            PipelineError::SchemaLocks(diagnostics) => {
                writeln!(f, "schema lock validation failed:")?;
                for diag in diagnostics {
                    writeln!(
                        f,
                        "- {} ({}): {}",
                        diag.schema, diag.artifact_path, diag.message
                    )?;
                }
                Ok(())
            }
            PipelineError::TypeCheck(diagnostics) => {
                writeln!(f, "typecheck failed:")?;
                for diag in diagnostics {
                    writeln!(f, "- {}: {}", diag.function, diag.message)?;
                }
                Ok(())
            }
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
            PipelineError::Toolchain(err) => write!(f, "toolchain error: {err}"),
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

impl From<ToolchainError> for PipelineError {
    fn from(value: ToolchainError) -> Self {
        Self::Toolchain(value)
    }
}

pub fn check_locked_schema_artifacts_in_file(path: &Path) -> Result<(), PipelineError> {
    let src = fs::read_to_string(path)?;
    let tokens = Lexer::new(&src).tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse_module()
        .map_err(|err| PipelineError::Parse(err.message))?;
    let hir = lower_hir(&ast);
    let source_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let report = bind_locked_schema_artifacts_from_fs(&hir, source_dir);
    if report.diagnostics.is_empty() {
        Ok(())
    } else {
        Err(PipelineError::SchemaLocks(report.diagnostics))
    }
}

pub fn emit_source_to_string(src: &str) -> Result<String, PipelineError> {
    let tokens = Lexer::new(src).tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse_module()
        .map_err(|err| PipelineError::Parse(err.message))?;
    let hir = lower_hir(&ast);
    emit_typechecked_report(typecheck_module(&hir))
}

pub fn emit_source_to_string_unchecked(src: &str) -> Result<String, PipelineError> {
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
    Ok(emit_module(&llvm)?)
}

pub fn emit_file_to_string(path: &Path) -> Result<String, PipelineError> {
    let src = fs::read_to_string(path)?;
    let tokens = Lexer::new(&src).tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse_module()
        .map_err(|err| PipelineError::Parse(err.message))?;
    let hir = lower_hir(&ast);
    let source_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let catalog = load_bound_schema_catalog_from_fs(&hir, source_dir);
    if !catalog.diagnostics.is_empty() {
        return Err(PipelineError::SchemaLocks(catalog.diagnostics));
    }
    emit_typechecked_report(typecheck_module_with_bound_schemas(&hir, &catalog))
}

pub fn emit_file_to_string_unchecked(path: &Path) -> Result<String, PipelineError> {
    let src = fs::read_to_string(path)?;
    emit_source_to_string_unchecked(&src)
}

pub fn emit_file_to_path(input: &Path, output: &Path) -> Result<(), PipelineError> {
    let ir = emit_file_to_string(input)?;
    fs::write(output, ir)?;
    Ok(())
}

pub fn verify_ll_path(path: &Path) -> Result<(), PipelineError> {
    let toolchain = LlvmToolchain::discover().ok_or(ToolchainError::MissingTool("llvm-as"))?;
    verify_ll_path_with_toolchain(path, &toolchain)
}

pub fn emit_file_to_path_and_verify(input: &Path, output: &Path) -> Result<(), PipelineError> {
    emit_file_to_path(input, output)?;
    verify_ll_path(output)
}

pub fn verify_ll_path_with_toolchain(
    path: &Path,
    toolchain: &LlvmToolchain,
) -> Result<(), PipelineError> {
    toolchain.verify_ll_file(path)?;
    Ok(())
}

fn emit_typechecked_report(report: TypeCheckReport) -> Result<String, PipelineError> {
    if !report.diagnostics.is_empty() {
        return Err(PipelineError::TypeCheck(report.diagnostics));
    }
    let mir = lower_mir(&report.module);
    let low = lower_low(&mir);
    let llvm = lower_llvm_like(&low);
    let validation = validate_module(&llvm);
    if !validation.is_ok() {
        return Err(PipelineError::Validation(validation.diagnostics));
    }
    Ok(emit_module(&llvm)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_schema::{build_artifact, render_artifact_canonical, DxSchemaType, SchemaField, SchemaMetadata};
    use crate::toolchain::LlvmToolchain;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dx-llvm-ir-{nonce}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    fn artifact(name: &str, provider: &str, source: &str) -> String {
        let mut fields = BTreeMap::new();
        fields.insert(
            "id".to_string(),
            SchemaField {
                ty: DxSchemaType::Int,
                nullable: false,
            },
        );
        let artifact = build_artifact(
            SchemaMetadata {
                format_version: "0.1.0".to_string(),
                name: name.to_string(),
                provider: provider.to_string(),
                source: source.to_string(),
                source_fingerprint: "sha256:source".to_string(),
                schema_fingerprint: "sha256:schema".to_string(),
                generated_at: "2026-03-29T10:00:00Z".to_string(),
            },
            fields,
        )
        .expect("artifact");
        render_artifact_canonical(&artifact)
    }

    #[cfg(unix)]
    fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write script");
        let mut perms = fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).expect("chmod");
        path
    }

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
    fn surfaces_typecheck_errors_for_normal_emit() {
        let err = emit_source_to_string("fun f() -> Int:\n    \"oops\"\n.\n")
            .expect_err("typecheck should fail");
        match err {
            PipelineError::TypeCheck(diagnostics) => {
                assert_eq!(diagnostics.len(), 1);
                assert!(diagnostics[0].message.contains("declared return type"));
            }
            other => panic!("expected typecheck error, got {other:?}"),
        }
    }

    #[test]
    fn checks_locked_schema_artifacts_from_source_file() {
        let base = temp_dir();
        let source = base.join("input.dx");
        let schemas = base.join("schemas");
        fs::create_dir_all(&schemas).expect("mkdir schemas");
        fs::write(
            &source,
            "schema Customers = csv.schema(\"data/customers.csv\") using \"schemas/customers.dxschema\"\n\nfun main() -> Int:\n    0\n.\n",
        )
        .expect("write source");
        fs::write(
            schemas.join("customers.dxschema"),
            artifact("Customers", "csv", "data/customers.csv"),
        )
        .expect("write artifact");

        check_locked_schema_artifacts_in_file(&source).expect("schema locks should validate");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn reports_missing_locked_schema_artifact_from_source_file() {
        let base = temp_dir();
        let source = base.join("input.dx");
        fs::write(
            &source,
            "schema Customers = csv.schema(\"data/customers.csv\") using \"schemas/customers.dxschema\"\n\nfun main() -> Int:\n    0\n.\n",
        )
        .expect("write source");

        let err = check_locked_schema_artifacts_in_file(&source).expect_err("missing artifact");
        match err {
            PipelineError::SchemaLocks(diagnostics) => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].schema, "Customers");
                assert_eq!(diagnostics[0].artifact_path, "schemas/customers.dxschema");
            }
            other => panic!("expected schema lock error, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn checks_default_schema_artifact_path_from_source_file() {
        let base = temp_dir();
        let source = base.join("input.dx");
        let schemas = base.join("schemas");
        fs::create_dir_all(&schemas).expect("mkdir schemas");
        fs::write(
            &source,
            "schema Customers = csv.schema(\"data/customers.csv\")\n\nfun main() -> Int:\n    0\n.\n",
        )
        .expect("write source");
        fs::write(
            schemas.join("customers.dxschema"),
            artifact("Customers", "csv", "data/customers.csv"),
        )
        .expect("write artifact");

        check_locked_schema_artifacts_in_file(&source).expect("default schema lock should validate");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn emits_file_when_locked_schema_artifact_is_present() {
        let base = temp_dir();
        let source = base.join("input.dx");
        let schemas = base.join("schemas");
        fs::create_dir_all(&schemas).expect("mkdir schemas");
        fs::write(
            &source,
            "schema Customers = csv.schema(\"data/customers.csv\") using \"schemas/customers.dxschema\"\n\nfun main() -> Int:\n    0\n.\n",
        )
        .expect("write source");
        fs::write(
            schemas.join("customers.dxschema"),
            artifact("Customers", "csv", "data/customers.csv"),
        )
        .expect("write artifact");

        let ir = emit_file_to_string(&source).expect("emit file should validate locked artifact");
        assert!(ir.contains("define i64 @main()"), "got:\n{ir}");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn file_emit_fails_when_locked_schema_artifact_is_missing() {
        let base = temp_dir();
        let source = base.join("input.dx");
        fs::write(
            &source,
            "schema Customers = csv.schema(\"data/customers.csv\") using \"schemas/customers.dxschema\"\n\nfun main() -> Int:\n    0\n.\n",
        )
        .expect("write source");

        let err = emit_file_to_string(&source).expect_err("missing artifact should fail normal emit");
        match err {
            PipelineError::SchemaLocks(diagnostics) => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].schema, "Customers");
                assert_eq!(diagnostics[0].artifact_path, "schemas/customers.dxschema");
            }
            other => panic!("expected schema lock error, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn file_emit_fails_when_default_schema_artifact_is_missing() {
        let base = temp_dir();
        let source = base.join("input.dx");
        fs::write(
            &source,
            "schema Customers = csv.schema(\"data/customers.csv\")\n\nfun main() -> Int:\n    0\n.\n",
        )
        .expect("write source");

        let err =
            emit_file_to_string(&source).expect_err("missing default artifact should fail normal emit");
        match err {
            PipelineError::SchemaLocks(diagnostics) => {
                assert_eq!(diagnostics.len(), 1);
                assert_eq!(diagnostics[0].schema, "Customers");
                assert_eq!(diagnostics[0].artifact_path, "schemas/customers.dxschema");
            }
            other => panic!("expected schema lock error, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn file_emit_surfaces_schema_row_field_typecheck_errors() {
        let base = temp_dir();
        let source = base.join("input.dx");
        let schemas = base.join("schemas");
        fs::create_dir_all(&schemas).expect("mkdir schemas");
        fs::write(
            &source,
            concat!(
                "schema Customers = csv.schema(\"data/customers.csv\") using \"schemas/customers.dxschema\"\n\n",
                "fun pick(c: Customers.Row) -> Str:\n",
                "    c'missing\n",
                ".\n",
            ),
        )
        .expect("write source");
        fs::write(
            schemas.join("customers.dxschema"),
            artifact("Customers", "csv", "data/customers.csv"),
        )
        .expect("write artifact");

        let err = emit_file_to_string(&source).expect_err("missing schema field should fail emit");
        match err {
            PipelineError::TypeCheck(diagnostics) => {
                assert_eq!(diagnostics.len(), 1);
                assert!(diagnostics[0].message.contains("unknown field `missing`"));
                assert!(diagnostics[0].message.contains("Customers.Row"));
            }
            other => panic!("expected typecheck error, got {other:?}"),
        }

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn writes_ir_to_output_path() {
        let base = temp_dir();
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

    #[test]
    #[cfg(unix)]
    fn verifies_emitted_output_with_given_toolchain() {
        let base = temp_dir();
        let input = base.join("input.dx");
        let output = base.join("output.ll");
        let log = base.join("log.txt");
        fs::write(&input, "fun f() -> Int:\n    1\n.\n").expect("write input");

        let llvm_as = write_script(
            &base,
            "llvm-as",
            &format!("echo llvm-as >> {}\nexit 0", log.display()),
        );
        let opt = write_script(
            &base,
            "opt",
            &format!("echo opt >> {}\nexit 0", log.display()),
        );
        let toolchain = LlvmToolchain {
            llvm_as,
            opt: Some(opt),
            llc: None,
        };

        emit_file_to_path(&input, &output).expect("emit file");
        verify_ll_path_with_toolchain(&output, &toolchain).expect("verify");

        let contents = fs::read_to_string(&log).expect("read log");
        assert!(contents.contains("llvm-as"), "got:\n{contents}");
        assert!(contents.contains("opt"), "got:\n{contents}");

        let _ = fs::remove_dir_all(&base);
    }
}
