use dx_schema::load_artifact;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("dx-cli-schema-refresh-{nonce}"));
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

#[test]
fn dx_schema_refresh_writes_locked_artifact() {
    let dir = temp_dir();
    let source = dir.join("input.dx");
    let schemas = dir.join("schemas");
    std::fs::create_dir_all(&schemas).expect("mkdir schemas");
    std::fs::write(
        &source,
        "schema Customers = csv.schema(\"data/customers.csv\") using \"schemas/customers.dxschema\"\n",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_dx"))
        .args([
            "schema",
            "refresh",
            source.to_str().expect("source str"),
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
        .expect("run dx schema refresh");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let artifact_path = schemas.join("customers.dxschema");
    let artifact = load_artifact(&artifact_path).expect("artifact");
    assert_eq!(artifact.schema.name, "Customers");
    assert_eq!(artifact.schema.provider, "csv");
    assert!(artifact.fields["email"].nullable);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn dx_schema_refresh_uses_default_artifact_path_without_using() {
    let dir = temp_dir();
    let source = dir.join("events.dx");
    std::fs::write(
        &source,
        "schema Events = parquet.schema(\"data/events.parquet\")\n",
    )
    .expect("write source");

    let output = Command::new(env!("CARGO_BIN_EXE_dx"))
        .args([
            "schema",
            "refresh",
            source.to_str().expect("source str"),
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "event_id=Int",
        ])
        .output()
        .expect("run dx schema refresh");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let artifact = load_artifact(&dir.join("schemas/events.dxschema")).expect("artifact");
    assert_eq!(artifact.schema.name, "Events");
    assert_eq!(artifact.schema.provider, "parquet");

    let _ = std::fs::remove_dir_all(&dir);
}
