use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("dx-schema-{name}-{nonce}"));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

#[test]
fn schema_cli_pipeline_creates_validates_and_matches_artifact() {
    let temp_dir = std::env::temp_dir().join(format!("dx-schema-pipeline-{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).expect("temp dir");
    let output = temp_dir.join("customers.dxschema");

    let new_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-new"))
        .args([
            "--name",
            "Customers",
            "--provider",
            "csv",
            "--source",
            "data/customers.csv",
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "id=Int",
            "--field",
            "email=Str?",
            "--output",
        ])
        .arg(&output)
        .output()
        .expect("run dx-schema-new");
    assert!(
        new_output.status.success(),
        "dx-schema-new failed: {}",
        String::from_utf8_lossy(&new_output.stderr)
    );

    let canonical_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-validate"))
        .args(["--check-canonical"])
        .arg(&output)
        .output()
        .expect("run dx-schema-validate");
    assert!(
        canonical_output.status.success(),
        "dx-schema-validate failed: {}",
        String::from_utf8_lossy(&canonical_output.stderr)
    );

    let match_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-match"))
        .args([
            "--name",
            "Customers",
            "--provider",
            "csv",
            "--source",
            "data/customers.csv",
        ])
        .arg(&output)
        .output()
        .expect("run dx-schema-match");
    assert!(
        match_output.status.success(),
        "dx-schema-match failed: {}",
        String::from_utf8_lossy(&match_output.stderr)
    );
}

#[test]
fn schema_validate_and_match_accept_example_artifact() {
    let example = repo_root().join("examples/schema/customers.dxschema.example");

    let validate_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-validate"))
        .args([
            "--expect-name",
            "Customers",
            "--expect-provider",
            "csv",
            "--expect-source",
            "data/customers.csv",
        ])
        .arg(&example)
        .output()
        .expect("run dx-schema-validate");
    assert!(
        validate_output.status.success(),
        "dx-schema-validate failed: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );

    let match_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-match"))
        .args([
            "--name",
            "Customers",
            "--provider",
            "csv",
            "--source",
            "data/customers.csv",
        ])
        .arg(&example)
        .output()
        .expect("run dx-schema-match");
    assert!(
        match_output.status.success(),
        "dx-schema-match failed: {}",
        String::from_utf8_lossy(&match_output.stderr)
    );
}

#[test]
fn schema_source_check_matches_example_source_to_artifact() {
    let source = repo_root().join("examples/schema/customer_analysis.dx.example");
    let artifact = repo_root().join("examples/schema/customers.dxschema.example");

    let output = Command::new(env!("CARGO_BIN_EXE_dx-schema-check-source"))
        .args(["--name", "Customers"])
        .arg(&source)
        .arg(&artifact)
        .output()
        .expect("run dx-schema-check-source");

    assert!(
        output.status.success(),
        "dx-schema-check-source failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn schema_source_check_locked_resolves_artifacts_from_source_dir() {
    let dir = temp_dir("locked-check");
    let schemas_dir = dir.join("schemas");
    std::fs::create_dir_all(&schemas_dir).expect("schemas dir");

    let source_path = dir.join("customer_analysis.dx");
    std::fs::write(
        &source_path,
        r#"
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
"#,
    )
    .expect("write source");

    let artifact_output = schemas_dir.join("customers.dxschema");
    let new_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-new"))
        .args([
            "--name",
            "Customers",
            "--provider",
            "csv",
            "--source",
            "data/customers.csv",
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "id=Int",
            "--output",
        ])
        .arg(&artifact_output)
        .output()
        .expect("run dx-schema-new");
    assert!(
        new_output.status.success(),
        "dx-schema-new failed: {}",
        String::from_utf8_lossy(&new_output.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_dx-schema-check-source"))
        .arg("--locked")
        .arg(&source_path)
        .output()
        .expect("run dx-schema-check-source --locked");

    assert!(
        output.status.success(),
        "dx-schema-check-source --locked failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn schema_source_check_locked_fails_when_artifact_is_missing() {
    let dir = temp_dir("locked-check-missing");
    let source_path = dir.join("customer_analysis.dx");
    std::fs::write(
        &source_path,
        r#"
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
"#,
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_dx-schema-check-source"))
        .arg("--locked")
        .arg(&source_path)
        .output()
        .expect("run dx-schema-check-source --locked");

    assert!(!output.status.success(), "expected locked mode failure");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("i/o error"), "stderr was: {stderr}");
}

#[test]
fn schema_refresh_writes_locked_artifact_from_source_declaration() {
    let dir = temp_dir("refresh-locked");
    let source_path = dir.join("customer_analysis.dx");
    std::fs::write(
        &source_path,
        r#"
schema Customers = csv.schema("data/customers.csv") using "schemas/customers.dxschema"
"#,
    )
    .expect("write source");

    let refresh_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-refresh"))
        .arg(&source_path)
        .args([
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "id=Int",
            "--field",
            "email=Str?",
        ])
        .output()
        .expect("run dx-schema-refresh");
    assert!(
        refresh_output.status.success(),
        "dx-schema-refresh failed: {}",
        String::from_utf8_lossy(&refresh_output.stderr)
    );

    let artifact = dir.join("schemas/customers.dxschema");
    assert!(artifact.exists(), "artifact should be written");

    let check_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-check-source"))
        .arg("--locked")
        .arg(&source_path)
        .output()
        .expect("run dx-schema-check-source --locked");
    assert!(
        check_output.status.success(),
        "dx-schema-check-source --locked failed: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );
}

#[test]
fn schema_refresh_accepts_explicit_output_for_non_locked_declaration() {
    let dir = temp_dir("refresh-explicit-output");
    let source_path = dir.join("events_refresh.dx");
    std::fs::write(
        &source_path,
        r#"
schema Events = parquet.schema("data/events.parquet") refresh
"#,
    )
    .expect("write source");

    let artifact = dir.join("schemas/events.dxschema");
    let refresh_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-refresh"))
        .arg(&source_path)
        .args([
            "--name",
            "Events",
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "event_id=Int",
            "--output",
        ])
        .arg(&artifact)
        .output()
        .expect("run dx-schema-refresh");
    assert!(
        refresh_output.status.success(),
        "dx-schema-refresh failed: {}",
        String::from_utf8_lossy(&refresh_output.stderr)
    );

    let validate_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-validate"))
        .args([
            "--expect-name",
            "Events",
            "--expect-provider",
            "parquet",
            "--expect-source",
            "data/events.parquet",
        ])
        .arg(&artifact)
        .output()
        .expect("run dx-schema-validate");
    assert!(
        validate_output.status.success(),
        "dx-schema-validate failed: {}",
        String::from_utf8_lossy(&validate_output.stderr)
    );
}

#[test]
fn schema_refresh_uses_default_artifact_path_when_using_is_absent() {
    let dir = temp_dir("refresh-default-output");
    let source_path = dir.join("customer_analysis.dx");
    std::fs::write(
        &source_path,
        r#"
schema Customers = csv.schema("data/customers.csv")
"#,
    )
    .expect("write source");

    let refresh_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-refresh"))
        .arg(&source_path)
        .args([
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "id=Int",
        ])
        .output()
        .expect("run dx-schema-refresh");
    assert!(
        refresh_output.status.success(),
        "dx-schema-refresh failed: {}",
        String::from_utf8_lossy(&refresh_output.stderr)
    );

    let artifact = dir.join("schemas/customers.dxschema");
    assert!(artifact.exists(), "default artifact path should be written");

    let check_output = Command::new(env!("CARGO_BIN_EXE_dx-schema-check-source"))
        .arg("--locked")
        .arg(&source_path)
        .output()
        .expect("run dx-schema-check-source --locked");
    assert!(
        check_output.status.success(),
        "dx-schema-check-source --locked failed: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );
}
