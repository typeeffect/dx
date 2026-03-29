//! Schema package validation using the real `dx-schema` parser/validator.
//!
//! These tests verify that the packaged `.dxschema` examples parse and validate
//! correctly through the actual implementation, and that the CLI produces
//! expected output.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

fn schema_dir() -> PathBuf {
    repo_root().join("examples/schema")
}

fn dx_schema_validate_bin() -> PathBuf {
    repo_root().join("target/debug/dx-schema-validate")
}

// ── file existence guards ───────────────────────────────────────

#[test]
fn schema_readme_exists() {
    assert!(schema_dir().join("README.md").exists());
}

#[test]
fn schema_source_example_exists() {
    assert!(schema_dir().join("customer_analysis.dx.example").exists());
}

#[test]
fn schema_artifact_customers_exists() {
    assert!(schema_dir().join("customers.dxschema.example").exists());
}

#[test]
fn schema_artifact_sales_exists() {
    assert!(schema_dir().join("sales.dxschema.example").exists());
}

// ── implementation-backed parse + validate ──────────────────────

#[test]
fn customers_artifact_parses_and_validates() {
    let artifact = dx_schema::load_artifact(&schema_dir().join("customers.dxschema.example"))
        .expect("customers.dxschema.example should parse and validate");

    assert_eq!(artifact.schema.format_version, dx_schema::SUPPORTED_FORMAT_VERSION);
    assert_eq!(artifact.schema.name, "Customers");
    assert_eq!(artifact.schema.provider, "csv");
    assert!(!artifact.fields.is_empty());
}

#[test]
fn sales_artifact_parses_and_validates() {
    let artifact = dx_schema::load_artifact(&schema_dir().join("sales.dxschema.example"))
        .expect("sales.dxschema.example should parse and validate");

    assert_eq!(artifact.schema.format_version, dx_schema::SUPPORTED_FORMAT_VERSION);
    assert_eq!(artifact.schema.name, "Sales");
    assert_eq!(artifact.schema.provider, "parquet");
    assert!(!artifact.fields.is_empty());
}

#[test]
fn all_artifacts_use_supported_format_version() {
    for name in &["customers.dxschema.example", "sales.dxschema.example"] {
        let artifact = dx_schema::load_artifact(&schema_dir().join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            artifact.schema.format_version,
            dx_schema::SUPPORTED_FORMAT_VERSION,
            "{name} uses unsupported format version"
        );
    }
}

#[test]
fn canonical_render_roundtrips_for_all_artifacts() {
    for name in &["customers.dxschema.example", "sales.dxschema.example"] {
        let artifact = dx_schema::load_artifact(&schema_dir().join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let canonical = dx_schema::render_artifact_canonical(&artifact);
        let reparsed = dx_schema::parse_artifact(&canonical)
            .unwrap_or_else(|e| panic!("{name} canonical roundtrip failed: {e}"));
        assert_eq!(artifact, reparsed, "{name} canonical roundtrip mismatch");
    }
}

// ── spec/doc existence ──────────────────────────────────────────

#[test]
fn schema_artifact_spec_doc_exists() {
    assert!(repo_root().join("docs/DX_SCHEMA_ARTIFACT_SPEC.md").exists());
}

#[test]
fn schema_provider_plan_doc_exists() {
    assert!(repo_root().join("docs/DX_SCHEMA_PROVIDER_PLAN.md").exists());
}

// ── README cross-links ──────────────────────────────────────────

#[test]
fn schema_readme_references_all_examples() {
    let readme = std::fs::read_to_string(schema_dir().join("README.md"))
        .expect("read README.md");
    assert!(readme.contains("customer_analysis.dx.example"));
    assert!(readme.contains("customers.dxschema.example"));
    assert!(readme.contains("sales.dxschema.example"));
    assert!(readme.contains("DX_SCHEMA_ARTIFACT_SPEC.md"));
    assert!(readme.contains("DX_SCHEMA_PROVIDER_PLAN.md"));
}

// ── CLI: dx-schema-validate ─────────────────────────────────────

fn has_schema_validate_bin() -> bool {
    dx_schema_validate_bin().exists()
}

#[test]
fn cli_validate_summary_customers() {
    if !has_schema_validate_bin() {
        eprintln!("skipping: dx-schema-validate binary not built");
        return;
    }
    let output = Command::new(dx_schema_validate_bin())
        .arg(schema_dir().join("customers.dxschema.example"))
        .output()
        .expect("run dx-schema-validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("OK"), "summary should start with OK: {stdout}");
    assert!(stdout.contains("Customers"), "summary should include name: {stdout}");
    assert!(stdout.contains("csv"), "summary should include provider: {stdout}");
}

#[test]
fn cli_validate_summary_sales() {
    if !has_schema_validate_bin() {
        eprintln!("skipping: dx-schema-validate binary not built");
        return;
    }
    let output = Command::new(dx_schema_validate_bin())
        .arg(schema_dir().join("sales.dxschema.example"))
        .output()
        .expect("run dx-schema-validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("OK"), "stdout: {stdout}");
    assert!(stdout.contains("Sales"), "stdout: {stdout}");
    assert!(stdout.contains("parquet"), "stdout: {stdout}");
}

#[test]
fn cli_validate_json() {
    if !has_schema_validate_bin() {
        eprintln!("skipping");
        return;
    }
    let output = Command::new(dx_schema_validate_bin())
        .args(["--json"])
        .arg(schema_dir().join("customers.dxschema.example"))
        .output()
        .expect("run dx-schema-validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("\"schema\""), "JSON should have schema: {stdout}");
    assert!(stdout.contains("\"fields\""), "JSON should have fields: {stdout}");
    assert!(stdout.contains("\"Customers\""), "JSON should have name: {stdout}");
}

#[test]
fn cli_validate_canonical() {
    if !has_schema_validate_bin() {
        eprintln!("skipping");
        return;
    }
    let output = Command::new(dx_schema_validate_bin())
        .args(["--canonical"])
        .arg(schema_dir().join("customers.dxschema.example"))
        .output()
        .expect("run dx-schema-validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("[schema]"), "canonical should have [schema]: {stdout}");
    assert!(stdout.contains("[fields]"), "canonical should have [fields]: {stdout}");
}

#[test]
fn cli_validate_check_canonical() {
    if !has_schema_validate_bin() {
        eprintln!("skipping");
        return;
    }
    // First render canonical, write to temp, then check it
    let canonical_output = Command::new(dx_schema_validate_bin())
        .args(["--canonical"])
        .arg(schema_dir().join("customers.dxschema.example"))
        .output()
        .expect("run dx-schema-validate --canonical");
    assert!(canonical_output.status.success());

    let temp_dir = std::env::temp_dir().join("dx-schema-check-canonical-test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let canonical_file = temp_dir.join("customers.dxschema");
    std::fs::write(&canonical_file, &canonical_output.stdout).expect("write canonical");

    let check_output = Command::new(dx_schema_validate_bin())
        .args(["--check-canonical"])
        .arg(&canonical_file)
        .output()
        .expect("run dx-schema-validate --check-canonical");

    let stdout = String::from_utf8_lossy(&check_output.stdout);
    assert!(check_output.status.success(), "canonical output should pass check: {stdout}");
    assert!(stdout.contains("OK canonical"), "stdout: {stdout}");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn cli_validate_rejects_invalid_artifact() {
    if !has_schema_validate_bin() {
        eprintln!("skipping");
        return;
    }
    let temp_dir = std::env::temp_dir().join("dx-schema-invalid-test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let bad_file = temp_dir.join("bad.dxschema");
    std::fs::write(&bad_file, "not valid schema content").expect("write bad file");

    let output = Command::new(dx_schema_validate_bin())
        .arg(&bad_file)
        .output()
        .expect("run dx-schema-validate");

    assert!(!output.status.success(), "invalid artifact should fail");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

// ── canonical artifact files ────────────────────────────────────

#[test]
fn canonical_customers_artifact_exists() {
    assert!(schema_dir().join("customers.dxschema.canonical").exists());
}

#[test]
fn canonical_sales_artifact_exists() {
    assert!(schema_dir().join("sales.dxschema.canonical").exists());
}

#[test]
fn canonical_artifacts_parse_and_validate() {
    for name in &["customers.dxschema.canonical", "sales.dxschema.canonical"] {
        dx_schema::load_artifact(&schema_dir().join(name))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn cli_check_canonical_accepts_canonical_files() {
    if !has_schema_validate_bin() {
        eprintln!("skipping");
        return;
    }
    for name in &["customers.dxschema.canonical", "sales.dxschema.canonical"] {
        let output = Command::new(dx_schema_validate_bin())
            .args(["--check-canonical"])
            .arg(schema_dir().join(name))
            .output()
            .unwrap_or_else(|e| panic!("run dx-schema-validate for {name}: {e}"));

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{name} should be canonical.\nstderr: {}", String::from_utf8_lossy(&output.stderr));
        assert!(stdout.contains("OK canonical"), "{name}: {stdout}");
    }
}

#[test]
fn cli_check_canonical_rejects_example_files() {
    if !has_schema_validate_bin() {
        eprintln!("skipping");
        return;
    }
    // The .example files contain comments, making them non-canonical.
    for name in &["customers.dxschema.example", "sales.dxschema.example"] {
        let output = Command::new(dx_schema_validate_bin())
            .args(["--check-canonical"])
            .arg(schema_dir().join(name))
            .output()
            .unwrap_or_else(|e| panic!("run dx-schema-validate for {name}: {e}"));

        assert!(
            !output.status.success(),
            "{name} should be rejected as non-canonical"
        );
    }
}
