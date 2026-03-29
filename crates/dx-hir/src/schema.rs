use crate::hir;
use dx_parser::{SchemaDecl, TypeExpr};
use dx_schema::{
    default_schema_artifact_rel_path, load_artifact, SchemaArtifact, SchemaArtifactContract,
    SchemaArtifactError,
};
use std::path::Path;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDeclarationDiagnostic {
    pub schema: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSchemaRequirement {
    pub schema: String,
    pub artifact_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRefreshRequest {
    pub schema: String,
    pub provider: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaDeclarationReport {
    pub declarations: Vec<SchemaDecl>,
    pub locked_artifacts: Vec<LockedSchemaRequirement>,
    pub refresh_requests: Vec<SchemaRefreshRequest>,
    pub diagnostics: Vec<SchemaDeclarationDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSchemaArtifactBindingDiagnostic {
    pub schema: String,
    pub artifact_path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSchemaArtifactBindingReport {
    pub bindings: Vec<LockedSchemaRequirement>,
    pub diagnostics: Vec<LockedSchemaArtifactBindingDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundSchemaArtifact {
    pub schema: String,
    pub artifact_path: String,
    pub artifact: SchemaArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundSchemaCatalog {
    pub bindings: Vec<BoundSchemaArtifact>,
    pub diagnostics: Vec<LockedSchemaArtifactBindingDiagnostic>,
}

pub fn analyze_schema_declarations(module: &hir::Module) -> SchemaDeclarationReport {
    let declarations: Vec<SchemaDecl> = module
        .items
        .iter()
        .filter_map(|item| match item {
            hir::Item::Schema(schema) => Some(schema.clone()),
            _ => None,
        })
        .collect();
    let declared_schemas: HashSet<&str> = declarations.iter().map(|schema| schema.name.as_str()).collect();
    let locked_artifacts: Vec<LockedSchemaRequirement> = declarations
        .iter()
        .map(|schema| {
            let artifact_path = schema
                .using_artifact
                .clone()
                .unwrap_or_else(|| default_schema_artifact_rel_path(&schema.name));
            LockedSchemaRequirement {
                schema: schema.name.clone(),
                artifact_path,
            }
        })
        .collect();
    let refresh_requests: Vec<SchemaRefreshRequest> = declarations
        .iter()
        .filter(|schema| schema.refresh)
        .map(|schema| SchemaRefreshRequest {
            schema: schema.name.clone(),
            provider: schema.provider.clone(),
            source: schema.source.clone(),
        })
        .collect();

    let mut diagnostics = Vec::new();
    for schema in &declarations {
        if !matches!(schema.provider.as_str(), "csv" | "parquet") {
            diagnostics.push(SchemaDeclarationDiagnostic {
                schema: schema.name.clone(),
                message: format!("unsupported schema provider `{}`", schema.provider),
            });
        }
        if schema.using_artifact.is_some() && schema.refresh {
            diagnostics.push(SchemaDeclarationDiagnostic {
                schema: schema.name.clone(),
                message: "schema declaration cannot combine `using` and `refresh` in v0"
                    .to_string(),
            });
        }
    }
    collect_schema_type_reference_diagnostics(module, &declared_schemas, &mut diagnostics);

    SchemaDeclarationReport {
        declarations,
        locked_artifacts,
        refresh_requests,
        diagnostics,
    }
}

pub fn bind_locked_schema_artifacts<F>(
    module: &hir::Module,
    mut load_artifact_for: F,
) -> LockedSchemaArtifactBindingReport
where
    F: FnMut(&str) -> Result<SchemaArtifact, SchemaArtifactError>,
{
    let report = analyze_schema_declarations(module);
    let mut diagnostics = Vec::new();

    for requirement in &report.locked_artifacts {
        let declaration = report
            .declarations
            .iter()
            .find(|decl| decl.name == requirement.schema)
            .expect("locked artifact requirement must correspond to a schema declaration");

        match load_artifact_for(&requirement.artifact_path) {
            Ok(artifact) => {
                let contract = SchemaArtifactContract {
                    name: &declaration.name,
                    provider: &declaration.provider,
                    source: &declaration.source,
                };
                if let Err(err) = dx_schema::validate_artifact_contract(&artifact, &contract) {
                    diagnostics.push(LockedSchemaArtifactBindingDiagnostic {
                        schema: requirement.schema.clone(),
                        artifact_path: requirement.artifact_path.clone(),
                        message: err.to_string(),
                    });
                }
            }
            Err(err) => diagnostics.push(LockedSchemaArtifactBindingDiagnostic {
                schema: requirement.schema.clone(),
                artifact_path: requirement.artifact_path.clone(),
                message: err.to_string(),
            }),
        }
    }

    LockedSchemaArtifactBindingReport {
        bindings: report.locked_artifacts,
        diagnostics,
    }
}

pub fn bind_locked_schema_artifacts_from_fs(
    module: &hir::Module,
    source_dir: &Path,
) -> LockedSchemaArtifactBindingReport {
    bind_locked_schema_artifacts(module, |artifact_path| {
        let path = source_dir.join(artifact_path);
        load_artifact(&path)
    })
}

pub fn load_bound_schema_catalog<F>(module: &hir::Module, mut load_artifact_for: F) -> BoundSchemaCatalog
where
    F: FnMut(&str) -> Result<SchemaArtifact, SchemaArtifactError>,
{
    let report = analyze_schema_declarations(module);
    let mut bindings = Vec::new();
    let mut diagnostics = Vec::new();

    for requirement in &report.locked_artifacts {
        let declaration = report
            .declarations
            .iter()
            .find(|decl| decl.name == requirement.schema)
            .expect("locked artifact requirement must correspond to a schema declaration");

        match load_artifact_for(&requirement.artifact_path) {
            Ok(artifact) => {
                let contract = SchemaArtifactContract {
                    name: &declaration.name,
                    provider: &declaration.provider,
                    source: &declaration.source,
                };
                match dx_schema::validate_artifact_contract(&artifact, &contract) {
                    Ok(()) => bindings.push(BoundSchemaArtifact {
                        schema: requirement.schema.clone(),
                        artifact_path: requirement.artifact_path.clone(),
                        artifact,
                    }),
                    Err(err) => diagnostics.push(LockedSchemaArtifactBindingDiagnostic {
                        schema: requirement.schema.clone(),
                        artifact_path: requirement.artifact_path.clone(),
                        message: err.to_string(),
                    }),
                }
            }
            Err(err) => diagnostics.push(LockedSchemaArtifactBindingDiagnostic {
                schema: requirement.schema.clone(),
                artifact_path: requirement.artifact_path.clone(),
                message: err.to_string(),
            }),
        }
    }

    BoundSchemaCatalog {
        bindings,
        diagnostics,
    }
}

pub fn load_bound_schema_catalog_from_fs(module: &hir::Module, source_dir: &Path) -> BoundSchemaCatalog {
    load_bound_schema_catalog(module, |artifact_path| {
        let path = source_dir.join(artifact_path);
        load_artifact(&path)
    })
}

fn collect_schema_type_reference_diagnostics(
    module: &hir::Module,
    declared_schemas: &HashSet<&str>,
    diagnostics: &mut Vec<SchemaDeclarationDiagnostic>,
) {
    for item in &module.items {
        if let hir::Item::Function(function) = item {
            for param in &function.params {
                diagnose_type_expr(&param.ty, declared_schemas, diagnostics);
            }
            if let Some(ret) = &function.return_type {
                diagnose_type_expr(ret, declared_schemas, diagnostics);
            }
            diagnose_block(&function.body, declared_schemas, diagnostics);
        }
    }
}

fn diagnose_block(
    block: &hir::Block,
    declared_schemas: &HashSet<&str>,
    diagnostics: &mut Vec<SchemaDeclarationDiagnostic>,
) {
    for stmt in &block.stmts {
        match stmt {
            hir::Stmt::Let { value, .. } | hir::Stmt::Expr(value) => {
                diagnose_expr(value, declared_schemas, diagnostics)
            }
            hir::Stmt::Rebind { value, .. } => diagnose_expr(value, declared_schemas, diagnostics),
        }
    }
    if let Some(result) = &block.result {
        diagnose_expr(result, declared_schemas, diagnostics);
    }
}

fn diagnose_expr(
    expr: &hir::Expr,
    declared_schemas: &HashSet<&str>,
    diagnostics: &mut Vec<SchemaDeclarationDiagnostic>,
) {
    match expr {
        hir::Expr::Closure { params, body } => {
            for param in params {
                if let Some(ty) = &param.ty {
                    diagnose_type_expr(ty, declared_schemas, diagnostics);
                }
            }
            match body.as_ref() {
                hir::ClosureBody::Expr(expr) => diagnose_expr(expr, declared_schemas, diagnostics),
                hir::ClosureBody::Block(block) => diagnose_block(block, declared_schemas, diagnostics),
            }
        }
        hir::Expr::Call { callee, args } => {
            diagnose_expr(callee, declared_schemas, diagnostics);
            for arg in args {
                match arg {
                    hir::Arg::Positional(expr) => diagnose_expr(expr, declared_schemas, diagnostics),
                    hir::Arg::Named { value, .. } => diagnose_expr(value, declared_schemas, diagnostics),
                }
            }
        }
        hir::Expr::Member { base, .. } => diagnose_expr(base, declared_schemas, diagnostics),
        hir::Expr::If {
            branches,
            else_branch,
        } => {
            for (condition, block) in branches {
                diagnose_expr(condition, declared_schemas, diagnostics);
                diagnose_block(block, declared_schemas, diagnostics);
            }
            if let Some(block) = else_branch {
                diagnose_block(block, declared_schemas, diagnostics);
            }
        }
        hir::Expr::Match { scrutinee, arms } => {
            diagnose_expr(scrutinee, declared_schemas, diagnostics);
            for arm in arms {
                diagnose_block(&arm.body, declared_schemas, diagnostics);
            }
        }
        hir::Expr::BinaryOp { lhs, rhs, .. } => {
            diagnose_expr(lhs, declared_schemas, diagnostics);
            diagnose_expr(rhs, declared_schemas, diagnostics);
        }
        hir::Expr::Unit | hir::Expr::Name(_) | hir::Expr::Integer(_) | hir::Expr::String(_) => {}
    }
}

fn diagnose_type_expr(
    ty: &TypeExpr,
    declared_schemas: &HashSet<&str>,
    diagnostics: &mut Vec<SchemaDeclarationDiagnostic>,
) {
    match ty {
        TypeExpr::Name(name) => {
            for schema in extract_schema_row_refs(name) {
                if !declared_schemas.contains(schema.as_str()) {
                    diagnostics.push(SchemaDeclarationDiagnostic {
                        schema: schema.clone(),
                        message: format!("unknown schema row type `{}.Row`", schema),
                    });
                }
            }
        }
        TypeExpr::Function {
            params,
            ret,
            effects: _,
        } => {
            for param in params {
                diagnose_type_expr(param, declared_schemas, diagnostics);
            }
            diagnose_type_expr(ret, declared_schemas, diagnostics);
        }
    }
}

fn extract_schema_row_refs(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut refs = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphabetic() || chars[i] == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            if i + 3 < chars.len()
                && chars[i] == '.'
                && chars[i + 1] == 'R'
                && chars[i + 2] == 'o'
                && chars[i + 3] == 'w'
            {
                refs.push(ident);
                i += 4;
                continue;
            }
        } else {
            i += 1;
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lower::lower_module;
    use dx_parser::{Lexer, Parser};
    use dx_schema::{build_artifact, DxSchemaType, SchemaField, SchemaMetadata};
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn analyze(src: &str) -> SchemaDeclarationReport {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        let hir = lower_module(&ast);
        analyze_schema_declarations(&hir)
    }

    fn lower(src: &str) -> hir::Module {
        let tokens = Lexer::new(src).tokenize();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_module().expect("module should parse");
        lower_module(&ast)
    }

    fn artifact(name: &str, provider: &str, source: &str) -> SchemaArtifact {
        let mut fields = BTreeMap::new();
        fields.insert(
            "id".to_string(),
            SchemaField {
                ty: DxSchemaType::Int,
                nullable: false,
            },
        );
        build_artifact(
            SchemaMetadata {
                format_version: "0.1.0".to_string(),
                name: name.to_string(),
                provider: provider.to_string(),
                source: source.to_string(),
                source_fingerprint: "sha256:1234".to_string(),
                schema_fingerprint: "sha256:5678".to_string(),
                generated_at: "2025-01-01T00:00:00Z".to_string(),
            },
            fields,
        )
        .expect("artifact should build")
    }

    fn write_artifact_file(dir: &Path, relative_path: &str, artifact: &SchemaArtifact) -> PathBuf {
        let path = dir.join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir should exist");
        }
        fs::write(&path, dx_schema::render_artifact_canonical(artifact))
            .expect("artifact should write");
        path
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dx-hir-{name}-{nonce}"));
        fs::create_dir_all(&dir).expect("temp dir should create");
        dir
    }

    #[test]
    fn collects_top_level_schema_declarations() {
        let report = analyze(
            r#"
schema Customers = csv.schema("data/customers.csv")
schema Sales = parquet.schema("data/sales.parquet") using "schemas/sales.dxschema"
"#,
        );

        assert_eq!(report.declarations.len(), 2);
        assert_eq!(report.declarations[0].name, "Customers");
        assert_eq!(report.declarations[1].name, "Sales");
        assert_eq!(
            report.locked_artifacts,
            vec![
                LockedSchemaRequirement {
                    schema: "Customers".to_string(),
                    artifact_path: "schemas/customers.dxschema".to_string(),
                },
                LockedSchemaRequirement {
                    schema: "Sales".to_string(),
                    artifact_path: "schemas/sales.dxschema".to_string(),
                },
            ]
        );
        assert!(report.refresh_requests.is_empty());
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn reports_unsupported_schema_provider() {
        let report = analyze(
            r#"
schema Sales = postgres.schema("postgres://db/sales")
"#,
        );

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Sales");
        assert!(report.diagnostics[0]
            .message
            .contains("unsupported schema provider `postgres`"));
    }

    #[test]
    fn reports_using_refresh_combination() {
        let report = analyze(
            r#"
schema Events = parquet.schema("data/events.parquet") using "schemas/events.dxschema" refresh
"#,
        );

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Events");
        assert!(report.diagnostics[0]
            .message
            .contains("cannot combine `using` and `refresh`"));
        assert_eq!(
            report.locked_artifacts,
            vec![LockedSchemaRequirement {
                schema: "Events".to_string(),
                artifact_path: "schemas/events.dxschema".to_string(),
            }]
        );
        assert_eq!(
            report.refresh_requests,
            vec![SchemaRefreshRequest {
                schema: "Events".to_string(),
                provider: "parquet".to_string(),
                source: "data/events.parquet".to_string(),
            }]
        );
    }

    #[test]
    fn reports_unknown_schema_row_type_in_function_signature() {
        let report = analyze(
            r#"
schema Customers = csv.schema("data/customers.csv")

fun names(rows: List(Orders.Row)) -> List(Customers.Row):
    rows
.
"#,
        );

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Orders");
        assert!(report.diagnostics[0]
            .message
            .contains("unknown schema row type `Orders.Row`"));
    }

    #[test]
    fn reports_unknown_schema_row_type_in_typed_lambda_param() {
        let report = analyze(
            r#"
schema Customers = csv.schema("data/customers.csv")

fun run() -> Unit:
    val f = (row: Orders.Row) => row
    f
.
"#,
        );

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Orders");
        assert!(report.diagnostics[0]
            .message
            .contains("unknown schema row type `Orders.Row`"));
    }

    #[test]
    fn binds_locked_schema_artifact_successfully() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );

        let report = bind_locked_schema_artifacts(&module, |path| {
            assert_eq!(path, "schemas/customers.dxschema");
            Ok(artifact("Customers", "csv", "data/customers.csv"))
        });

        assert_eq!(
            report.bindings,
            vec![LockedSchemaRequirement {
                schema: "Customers".to_string(),
                artifact_path: "schemas/customers.dxschema".to_string(),
            }]
        );
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn loads_bound_schema_catalog_successfully() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );

        let catalog = load_bound_schema_catalog(&module, |path| {
            assert_eq!(path, "schemas/customers.dxschema");
            Ok(artifact("Customers", "csv", "data/customers.csv"))
        });

        assert!(catalog.diagnostics.is_empty());
        assert_eq!(catalog.bindings.len(), 1);
        assert_eq!(catalog.bindings[0].schema, "Customers");
        assert_eq!(catalog.bindings[0].artifact_path, "schemas/customers.dxschema");
        assert_eq!(catalog.bindings[0].artifact.schema.name, "Customers");
    }

    #[test]
    fn reports_locked_schema_artifact_loader_error() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );

        let report = bind_locked_schema_artifacts(&module, |_| {
            Err(SchemaArtifactError::Io("missing file".to_string()))
        });

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Customers");
        assert_eq!(report.diagnostics[0].artifact_path, "schemas/customers.dxschema");
        assert!(report.diagnostics[0].message.contains("missing file"));
    }

    #[test]
    fn load_bound_schema_catalog_reports_loader_error() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );

        let catalog = load_bound_schema_catalog(&module, |_| {
            Err(SchemaArtifactError::Io("missing file".to_string()))
        });

        assert!(catalog.bindings.is_empty());
        assert_eq!(catalog.diagnostics.len(), 1);
        assert_eq!(catalog.diagnostics[0].schema, "Customers");
        assert_eq!(catalog.diagnostics[0].artifact_path, "schemas/customers.dxschema");
    }

    #[test]
    fn reports_locked_schema_artifact_contract_mismatch() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );

        let report = bind_locked_schema_artifacts(&module, |_| {
            Ok(artifact("Orders", "csv", "data/customers.csv"))
        });

        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Customers");
        assert!(report.diagnostics[0]
            .message
            .contains("schema name mismatch"));
    }

    #[test]
    fn load_bound_schema_catalog_reports_contract_mismatch() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );

        let catalog = load_bound_schema_catalog(&module, |_| {
            Ok(artifact("Orders", "csv", "data/customers.csv"))
        });

        assert!(catalog.bindings.is_empty());
        assert_eq!(catalog.diagnostics.len(), 1);
        assert_eq!(catalog.diagnostics[0].schema, "Customers");
        assert!(catalog.diagnostics[0]
            .message
            .contains("schema name mismatch"));
    }

    #[test]
    fn binds_default_schema_artifact_from_fs_relative_to_source_dir() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );
        let dir = temp_test_dir("bind-default-from-fs-ok");
        write_artifact_file(
            &dir,
            "schemas/customers.dxschema",
            &artifact("Customers", "csv", "data/customers.csv"),
        );

        let report = bind_locked_schema_artifacts_from_fs(&module, &dir);

        assert_eq!(report.bindings.len(), 1);
        assert!(report.diagnostics.is_empty());

        fs::remove_dir_all(&dir).expect("temp dir should clean up");
    }

    #[test]
    fn loads_bound_schema_catalog_from_fs() {
        let module = lower(
            r#"
schema Customers = csv.schema("data/customers.csv")
"#,
        );
        let dir = temp_test_dir("catalog-from-fs-ok");
        write_artifact_file(
            &dir,
            "schemas/customers.dxschema",
            &artifact("Customers", "csv", "data/customers.csv"),
        );

        let catalog = load_bound_schema_catalog_from_fs(&module, &dir);

        assert!(catalog.diagnostics.is_empty());
        assert_eq!(catalog.bindings.len(), 1);
        assert_eq!(catalog.bindings[0].schema, "Customers");

        fs::remove_dir_all(&dir).expect("temp dir should clean up");
    }

    #[test]
    fn binds_locked_schema_artifact_from_fs_relative_to_source_dir() {
        let module = lower(
            r#"
schema Sales = parquet.schema("data/sales.parquet") using "schemas/sales.dxschema"
"#,
        );
        let dir = temp_test_dir("bind-from-fs-ok");
        write_artifact_file(
            &dir,
            "schemas/sales.dxschema",
            &artifact("Sales", "parquet", "data/sales.parquet"),
        );

        let report = bind_locked_schema_artifacts_from_fs(&module, &dir);

        assert_eq!(report.bindings.len(), 1);
        assert!(report.diagnostics.is_empty());

        fs::remove_dir_all(&dir).expect("temp dir should clean up");
    }

    #[test]
    fn reports_missing_locked_schema_artifact_from_fs() {
        let module = lower(
            r#"
schema Sales = parquet.schema("data/sales.parquet") using "schemas/sales.dxschema"
"#,
        );
        let dir = temp_test_dir("bind-from-fs-missing");

        let report = bind_locked_schema_artifacts_from_fs(&module, &dir);

        assert_eq!(report.bindings.len(), 1);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Sales");
        assert_eq!(report.diagnostics[0].artifact_path, "schemas/sales.dxschema");
        assert!(report.diagnostics[0].message.contains("i/o error"));

        fs::remove_dir_all(&dir).expect("temp dir should clean up");
    }
}
