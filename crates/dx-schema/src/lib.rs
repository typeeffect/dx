use std::collections::BTreeMap;
use std::path::Path;

pub const SUPPORTED_FORMAT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaArtifact {
    pub schema: SchemaMetadata,
    pub fields: BTreeMap<String, SchemaField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMetadata {
    pub format_version: String,
    pub name: String,
    pub provider: String,
    pub source: String,
    pub source_fingerprint: String,
    pub schema_fingerprint: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxSchemaType {
    Int,
    Float,
    Str,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaField {
    pub ty: DxSchemaType,
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaArtifactError {
    Io(String),
    MissingSection(&'static str),
    MissingField(&'static str),
    UnknownSection(String),
    InvalidLine(String),
    InvalidStringValue(String),
    InvalidFieldSpec(String),
    InvalidBool(String),
    UnsupportedType(String),
    UnsupportedFormatVersion(String),
}

impl std::fmt::Display for SchemaArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaArtifactError::Io(message) => write!(f, "i/o error: {message}"),
            SchemaArtifactError::MissingSection(name) => write!(f, "missing section `{name}`"),
            SchemaArtifactError::MissingField(name) => write!(f, "missing field `{name}`"),
            SchemaArtifactError::UnknownSection(name) => write!(f, "unknown section `{name}`"),
            SchemaArtifactError::InvalidLine(line) => write!(f, "invalid line: {line}"),
            SchemaArtifactError::InvalidStringValue(value) => {
                write!(f, "invalid string value: {value}")
            }
            SchemaArtifactError::InvalidFieldSpec(spec) => {
                write!(f, "invalid field spec: {spec}")
            }
            SchemaArtifactError::InvalidBool(value) => write!(f, "invalid bool: {value}"),
            SchemaArtifactError::UnsupportedType(name) => write!(f, "unsupported type: {name}"),
            SchemaArtifactError::UnsupportedFormatVersion(version) => {
                write!(f, "unsupported format_version: {version}")
            }
        }
    }
}

impl std::error::Error for SchemaArtifactError {}

pub fn load_artifact(path: &Path) -> Result<SchemaArtifact, SchemaArtifactError> {
    let src = std::fs::read_to_string(path).map_err(|err| SchemaArtifactError::Io(err.to_string()))?;
    parse_artifact(&src)
}

pub fn parse_artifact(src: &str) -> Result<SchemaArtifact, SchemaArtifactError> {
    let mut schema_pairs = BTreeMap::new();
    let mut fields = BTreeMap::new();
    let mut section: Option<&str> = None;

    for raw_line in src.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            match name {
                "schema" | "fields" => section = Some(name),
                _ => return Err(SchemaArtifactError::UnknownSection(name.to_string())),
            }
            continue;
        }

        match section {
            Some("schema") => {
                let (key, value) = parse_assignment(line)?;
                schema_pairs.insert(key.to_string(), parse_string_literal(value)?);
            }
            Some("fields") => {
                let (name, value) = parse_assignment(line)?;
                fields.insert(name.to_string(), parse_field_spec(value)?);
            }
            _ => return Err(SchemaArtifactError::InvalidLine(line.to_string())),
        }
    }

    if schema_pairs.is_empty() {
        return Err(SchemaArtifactError::MissingSection("schema"));
    }
    if fields.is_empty() {
        return Err(SchemaArtifactError::MissingSection("fields"));
    }

    let schema = SchemaMetadata {
        format_version: required_schema_value(&schema_pairs, "format_version")?,
        name: required_schema_value(&schema_pairs, "name")?,
        provider: required_schema_value(&schema_pairs, "provider")?,
        source: required_schema_value(&schema_pairs, "source")?,
        source_fingerprint: required_schema_value(&schema_pairs, "source_fingerprint")?,
        schema_fingerprint: required_schema_value(&schema_pairs, "schema_fingerprint")?,
        generated_at: required_schema_value(&schema_pairs, "generated_at")?,
    };

    validate_artifact(&SchemaArtifact {
        schema: schema.clone(),
        fields: fields.clone(),
    })?;

    Ok(SchemaArtifact { schema, fields })
}

pub fn validate_artifact(artifact: &SchemaArtifact) -> Result<(), SchemaArtifactError> {
    if artifact.schema.format_version != SUPPORTED_FORMAT_VERSION {
        return Err(SchemaArtifactError::UnsupportedFormatVersion(
            artifact.schema.format_version.clone(),
        ));
    }
    if artifact.schema.name.trim().is_empty() {
        return Err(SchemaArtifactError::MissingField("name"));
    }
    if artifact.schema.provider.trim().is_empty() {
        return Err(SchemaArtifactError::MissingField("provider"));
    }
    if artifact.schema.source.trim().is_empty() {
        return Err(SchemaArtifactError::MissingField("source"));
    }
    if artifact.fields.is_empty() {
        return Err(SchemaArtifactError::MissingSection("fields"));
    }
    Ok(())
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(index) => &line[..index],
        None => line,
    }
}

fn parse_assignment(line: &str) -> Result<(&str, &str), SchemaArtifactError> {
    let (lhs, rhs) = line
        .split_once('=')
        .ok_or_else(|| SchemaArtifactError::InvalidLine(line.to_string()))?;
    Ok((lhs.trim(), rhs.trim()))
}

fn parse_string_literal(value: &str) -> Result<String, SchemaArtifactError> {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        Ok(value[1..value.len() - 1].to_string())
    } else {
        Err(SchemaArtifactError::InvalidStringValue(value.to_string()))
    }
}

fn parse_bool(value: &str) -> Result<bool, SchemaArtifactError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(SchemaArtifactError::InvalidBool(value.to_string())),
    }
}

fn parse_dx_type(name: &str) -> Result<DxSchemaType, SchemaArtifactError> {
    match name {
        "Int" => Ok(DxSchemaType::Int),
        "Float" => Ok(DxSchemaType::Float),
        "Str" => Ok(DxSchemaType::Str),
        "Bool" => Ok(DxSchemaType::Bool),
        _ => Err(SchemaArtifactError::UnsupportedType(name.to_string())),
    }
}

fn parse_field_spec(value: &str) -> Result<SchemaField, SchemaArtifactError> {
    let inner = value
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .ok_or_else(|| SchemaArtifactError::InvalidFieldSpec(value.to_string()))?
        .trim();

    let mut ty = None;
    let mut nullable = None;
    for part in inner.split(',') {
        let (key, raw_value) = parse_assignment(part.trim())?;
        match key {
            "type" => ty = Some(parse_dx_type(&parse_string_literal(raw_value)?)?),
            "nullable" => nullable = Some(parse_bool(raw_value)?),
            _ => return Err(SchemaArtifactError::InvalidFieldSpec(part.trim().to_string())),
        }
    }

    Ok(SchemaField {
        ty: ty.ok_or(SchemaArtifactError::MissingField("type"))?,
        nullable: nullable.ok_or(SchemaArtifactError::MissingField("nullable"))?,
    })
}

fn required_schema_value(
    schema_pairs: &BTreeMap<String, String>,
    key: &'static str,
) -> Result<String, SchemaArtifactError> {
    schema_pairs
        .get(key)
        .cloned()
        .ok_or(SchemaArtifactError::MissingField(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn example_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/schema")
            .join(name)
    }

    #[test]
    fn parses_customers_example() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");

        assert_eq!(artifact.schema.format_version, "0.1.0");
        assert_eq!(artifact.schema.name, "Customers");
        assert_eq!(artifact.schema.provider, "csv");
        assert_eq!(artifact.fields["id"].ty, DxSchemaType::Int);
        assert!(!artifact.fields["id"].nullable);
        assert!(artifact.fields["email"].nullable);
    }

    #[test]
    fn parses_sales_example() {
        let artifact = load_artifact(&example_path("sales.dxschema.example")).expect("parse");

        assert_eq!(artifact.schema.name, "Sales");
        assert_eq!(artifact.schema.provider, "parquet");
        assert_eq!(artifact.fields["revenue"].ty, DxSchemaType::Float);
        assert!(artifact.fields["discount"].nullable);
    }

    #[test]
    fn rejects_unsupported_format_version() {
        let src = r#"
[schema]
format_version = "9.9.9"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "unsupported format_version: 9.9.9");
    }

    #[test]
    fn rejects_unsupported_field_type() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Date", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "unsupported type: Date");
    }

    #[test]
    fn rejects_missing_fields_section() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "missing section `fields`");
    }
}
