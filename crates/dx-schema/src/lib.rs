use dx_parser::{Item as DxItem, Lexer, Parser};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
pub struct SchemaArtifactContract<'a> {
    pub name: &'a str,
    pub provider: &'a str,
    pub source: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSourceDeclaration {
    pub name: String,
    pub provider: String,
    pub source: String,
    pub using_artifact: Option<String>,
    pub refresh: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSchemaArtifactCheck {
    pub schema: String,
    pub artifact_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSchemaArtifactDiagnostic {
    pub schema: String,
    pub artifact_path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedSchemaArtifactReport {
    pub checks: Vec<LockedSchemaArtifactCheck>,
    pub diagnostics: Vec<LockedSchemaArtifactDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRefreshRequest {
    pub name: Option<String>,
    pub source_path: PathBuf,
    pub output: Option<PathBuf>,
    pub source_fingerprint: String,
    pub schema_fingerprint: String,
    pub generated_at: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRefreshResult {
    pub name: String,
    pub provider: String,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaArtifactError {
    Io(String),
    MissingSection(&'static str),
    MissingField(&'static str),
    UnknownSection(String),
    UnknownSchemaKey(String),
    DuplicateKey(String),
    InvalidFingerprint(String),
    InvalidTimestamp(String),
    InvalidSchemaName(String),
    InvalidFieldName(String),
    InvalidSource(String),
    InvalidLine(String),
    InvalidSourceDeclaration(String),
    InvalidStringValue(String),
    InvalidFieldSpec(String),
    InvalidBool(String),
    UnsupportedProvider(String),
    UnsupportedType(String),
    UnsupportedFormatVersion(String),
    SchemaSelectionRequired,
    SchemaDeclarationNotFound(String),
    NameMismatch { expected: String, actual: String },
    ProviderMismatch { expected: String, actual: String },
    SourceMismatch { expected: String, actual: String },
}

impl std::fmt::Display for SchemaArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaArtifactError::Io(message) => write!(f, "i/o error: {message}"),
            SchemaArtifactError::MissingSection(name) => write!(f, "missing section `{name}`"),
            SchemaArtifactError::MissingField(name) => write!(f, "missing field `{name}`"),
            SchemaArtifactError::UnknownSection(name) => write!(f, "unknown section `{name}`"),
            SchemaArtifactError::UnknownSchemaKey(name) => {
                write!(f, "unknown schema key: {name}")
            }
            SchemaArtifactError::DuplicateKey(name) => write!(f, "duplicate key: {name}"),
            SchemaArtifactError::InvalidFingerprint(value) => {
                write!(f, "invalid fingerprint: {value}")
            }
            SchemaArtifactError::InvalidTimestamp(value) => {
                write!(f, "invalid timestamp: {value}")
            }
            SchemaArtifactError::InvalidSchemaName(value) => {
                write!(f, "invalid schema name: {value}")
            }
            SchemaArtifactError::InvalidFieldName(value) => {
                write!(f, "invalid field name: {value}")
            }
            SchemaArtifactError::InvalidSource(value) => {
                write!(f, "invalid source: {value}")
            }
            SchemaArtifactError::InvalidLine(line) => write!(f, "invalid line: {line}"),
            SchemaArtifactError::InvalidSourceDeclaration(line) => {
                write!(f, "invalid source declaration: {line}")
            }
            SchemaArtifactError::InvalidStringValue(value) => {
                write!(f, "invalid string value: {value}")
            }
            SchemaArtifactError::InvalidFieldSpec(spec) => {
                write!(f, "invalid field spec: {spec}")
            }
            SchemaArtifactError::InvalidBool(value) => write!(f, "invalid bool: {value}"),
            SchemaArtifactError::UnsupportedProvider(name) => {
                write!(f, "unsupported provider: {name}")
            }
            SchemaArtifactError::UnsupportedType(name) => write!(f, "unsupported type: {name}"),
            SchemaArtifactError::UnsupportedFormatVersion(version) => {
                write!(f, "unsupported format_version: {version}")
            }
            SchemaArtifactError::SchemaSelectionRequired => {
                write!(f, "could not select schema declaration")
            }
            SchemaArtifactError::SchemaDeclarationNotFound(name) => {
                write!(f, "schema declaration not found: {name}")
            }
            SchemaArtifactError::NameMismatch { expected, actual } => {
                write!(f, "schema name mismatch: expected {expected}, got {actual}")
            }
            SchemaArtifactError::ProviderMismatch { expected, actual } => {
                write!(f, "provider mismatch: expected {expected}, got {actual}")
            }
            SchemaArtifactError::SourceMismatch { expected, actual } => {
                write!(f, "source mismatch: expected {expected}, got {actual}")
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
                let key = parse_schema_key(key)?;
                let value = parse_string_literal(value)?;
                if schema_pairs.insert(key.to_string(), value).is_some() {
                    return Err(SchemaArtifactError::DuplicateKey(key.to_string()));
                }
            }
            Some("fields") => {
                let (name, value) = parse_assignment(line)?;
                if !is_valid_field_name(name) {
                    return Err(SchemaArtifactError::InvalidFieldName(name.to_string()));
                }
                if fields
                    .insert(name.to_string(), parse_field_spec(value)?)
                    .is_some()
                {
                    return Err(SchemaArtifactError::DuplicateKey(name.to_string()));
                }
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
    if !is_valid_schema_name(&artifact.schema.name) {
        return Err(SchemaArtifactError::InvalidSchemaName(
            artifact.schema.name.clone(),
        ));
    }
    if artifact.schema.provider.trim().is_empty() {
        return Err(SchemaArtifactError::MissingField("provider"));
    }
    if !is_supported_provider(&artifact.schema.provider) {
        return Err(SchemaArtifactError::UnsupportedProvider(
            artifact.schema.provider.clone(),
        ));
    }
    if artifact.schema.source.trim().is_empty() {
        return Err(SchemaArtifactError::MissingField("source"));
    }
    if !source_matches_provider(&artifact.schema.provider, &artifact.schema.source) {
        return Err(SchemaArtifactError::InvalidSource(
            artifact.schema.source.clone(),
        ));
    }
    if !is_valid_fingerprint(&artifact.schema.source_fingerprint) {
        return Err(SchemaArtifactError::InvalidFingerprint(
            artifact.schema.source_fingerprint.clone(),
        ));
    }
    if !is_valid_fingerprint(&artifact.schema.schema_fingerprint) {
        return Err(SchemaArtifactError::InvalidFingerprint(
            artifact.schema.schema_fingerprint.clone(),
        ));
    }
    if !is_valid_timestamp(&artifact.schema.generated_at) {
        return Err(SchemaArtifactError::InvalidTimestamp(
            artifact.schema.generated_at.clone(),
        ));
    }
    if artifact.fields.is_empty() {
        return Err(SchemaArtifactError::MissingSection("fields"));
    }
    Ok(())
}

pub fn validate_artifact_contract(
    artifact: &SchemaArtifact,
    expected: &SchemaArtifactContract<'_>,
) -> Result<(), SchemaArtifactError> {
    if artifact.schema.name != expected.name {
        return Err(SchemaArtifactError::NameMismatch {
            expected: expected.name.to_string(),
            actual: artifact.schema.name.clone(),
        });
    }
    if artifact.schema.provider != expected.provider {
        return Err(SchemaArtifactError::ProviderMismatch {
            expected: expected.provider.to_string(),
            actual: artifact.schema.provider.clone(),
        });
    }
    if artifact.schema.source != expected.source {
        return Err(SchemaArtifactError::SourceMismatch {
            expected: expected.source.to_string(),
            actual: artifact.schema.source.clone(),
        });
    }
    Ok(())
}

pub fn parse_source_declarations(
    src: &str,
) -> Result<Vec<SchemaSourceDeclaration>, SchemaArtifactError> {
    let mut schema_src = String::new();
    for raw_line in src.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() || !line.starts_with("schema ") {
            continue;
        }
        schema_src.push_str(line);
        schema_src.push('\n');
    }

    let tokens = Lexer::new(&schema_src).tokenize();
    let mut parser = Parser::new(tokens);
    let module = parser
        .parse_module()
        .map_err(|err| SchemaArtifactError::InvalidSourceDeclaration(err.message))?;

    let mut declarations = Vec::new();
    for item in module.items {
        if let DxItem::Schema(schema) = item {
            if !is_valid_schema_name(&schema.name) {
                return Err(SchemaArtifactError::InvalidSchemaName(schema.name));
            }
            if !is_supported_provider(&schema.provider) {
                return Err(SchemaArtifactError::UnsupportedProvider(schema.provider));
            }
            if !source_matches_provider(&schema.provider, &schema.source) {
                return Err(SchemaArtifactError::InvalidSource(schema.source));
            }
            declarations.push(SchemaSourceDeclaration {
                name: schema.name,
                provider: schema.provider,
                source: schema.source,
                using_artifact: schema.using_artifact,
                refresh: schema.refresh,
            });
        }
    }
    Ok(declarations)
}

pub fn validate_source_declaration_contract(
    decl: &SchemaSourceDeclaration,
    artifact: &SchemaArtifact,
) -> Result<(), SchemaArtifactError> {
    validate_artifact_contract(
        artifact,
        &SchemaArtifactContract {
            name: &decl.name,
            provider: &decl.provider,
            source: &decl.source,
        },
    )
}

pub fn default_schema_artifact_rel_path(schema_name: &str) -> String {
    format!("schemas/{}.dxschema", schema_name_to_artifact_stem(schema_name))
}

pub fn schema_artifact_rel_path(decl: &SchemaSourceDeclaration) -> String {
    decl.using_artifact
        .clone()
        .unwrap_or_else(|| default_schema_artifact_rel_path(&decl.name))
}

pub fn analyze_locked_source_artifacts<F>(
    src: &str,
    mut load_artifact_for: F,
) -> Result<LockedSchemaArtifactReport, SchemaArtifactError>
where
    F: FnMut(&str) -> Result<SchemaArtifact, SchemaArtifactError>,
{
    let declarations = parse_source_declarations(src)?;
    let mut checks = Vec::new();
    let mut diagnostics = Vec::new();

    for decl in declarations {
        let artifact_path = schema_artifact_rel_path(&decl);

        checks.push(LockedSchemaArtifactCheck {
            schema: decl.name.clone(),
            artifact_path: artifact_path.clone(),
        });

        match load_artifact_for(&artifact_path) {
            Ok(artifact) => {
                if let Err(err) = validate_source_declaration_contract(&decl, &artifact) {
                    diagnostics.push(LockedSchemaArtifactDiagnostic {
                        schema: decl.name.clone(),
                        artifact_path: artifact_path.clone(),
                        message: err.to_string(),
                    });
                }
            }
            Err(err) => diagnostics.push(LockedSchemaArtifactDiagnostic {
                schema: decl.name.clone(),
                artifact_path: artifact_path.clone(),
                message: err.to_string(),
            }),
        }
    }

    Ok(LockedSchemaArtifactReport { checks, diagnostics })
}

pub fn build_artifact(
    schema: SchemaMetadata,
    fields: BTreeMap<String, SchemaField>,
) -> Result<SchemaArtifact, SchemaArtifactError> {
    let artifact = SchemaArtifact { schema, fields };
    validate_artifact(&artifact)?;
    Ok(artifact)
}

pub fn parse_dx_type_name(name: &str) -> Result<DxSchemaType, SchemaArtifactError> {
    parse_dx_type(name)
}

pub fn parse_schema_refresh_args<I>(args: I) -> Option<SchemaRefreshRequest>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut name = None;
    let mut source_path = None;
    let mut output = None;
    let mut source_fingerprint = None;
    let mut schema_fingerprint = None;
    let mut generated_at = None;
    let mut fields = Vec::new();
    let mut args = args.into_iter().map(Into::into);

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return None;
        }
        if arg == "--name" {
            name = args.next().map(PathBuf::from).and_then(path_buf_to_string);
            if name.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--output" {
            output = args.next().map(PathBuf::from);
            if output.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--source-fingerprint" {
            source_fingerprint = args.next().map(PathBuf::from).and_then(path_buf_to_string);
            if source_fingerprint.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--schema-fingerprint" {
            schema_fingerprint = args.next().map(PathBuf::from).and_then(path_buf_to_string);
            if schema_fingerprint.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--generated-at" {
            generated_at = args.next().map(PathBuf::from).and_then(path_buf_to_string);
            if generated_at.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--field" {
            let field = args.next().map(PathBuf::from).and_then(path_buf_to_string)?;
            fields.push(field);
            continue;
        }
        if source_path.is_none() {
            source_path = Some(PathBuf::from(arg));
            continue;
        }
        return None;
    }

    if fields.is_empty() {
        return None;
    }

    Some(SchemaRefreshRequest {
        name,
        source_path: source_path?,
        output,
        source_fingerprint: source_fingerprint?,
        schema_fingerprint: schema_fingerprint?,
        generated_at: generated_at?,
        fields,
    })
}

pub fn refresh_schema_artifact(
    request: SchemaRefreshRequest,
) -> Result<SchemaRefreshResult, SchemaArtifactError> {
    let src = std::fs::read_to_string(&request.source_path)
        .map_err(|err| SchemaArtifactError::Io(err.to_string()))?;
    let declarations = parse_source_declarations(&src)?;
    let decl = select_schema_declaration(&declarations, request.name.as_deref())?;
    let output = resolve_schema_output_path(decl, &request.source_path, request.output.as_ref())?;
    let fields = parse_cli_schema_fields(&request.fields)?;

    let artifact = build_artifact(
        SchemaMetadata {
            format_version: SUPPORTED_FORMAT_VERSION.to_string(),
            name: decl.name.clone(),
            provider: decl.provider.clone(),
            source: decl.source.clone(),
            source_fingerprint: request.source_fingerprint,
            schema_fingerprint: request.schema_fingerprint,
            generated_at: request.generated_at,
        },
        fields,
    )?;

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|err| SchemaArtifactError::Io(err.to_string()))?;
    }

    std::fs::write(&output, render_artifact_canonical(&artifact))
        .map_err(|err| SchemaArtifactError::Io(err.to_string()))?;

    Ok(SchemaRefreshResult {
        name: decl.name.clone(),
        provider: decl.provider.clone(),
        output_path: output,
    })
}

pub fn render_schema_refresh_success(result: &SchemaRefreshResult) -> String {
    format!(
        "OK refreshed name={} provider={} output={}",
        result.name,
        result.provider,
        result.output_path.display()
    )
}

pub fn render_artifact_summary(artifact: &SchemaArtifact) -> String {
    format!(
        "OK name={} provider={} fields={} format_version={}",
        artifact.schema.name,
        artifact.schema.provider,
        artifact.fields.len(),
        artifact.schema.format_version
    )
}

pub fn render_artifact_canonical(artifact: &SchemaArtifact) -> String {
    let mut out = String::new();
    out.push_str("[schema]\n");
    out.push_str(&format!(
        "format_version = \"{}\"\n",
        artifact.schema.format_version
    ));
    out.push_str(&format!("name = \"{}\"\n", artifact.schema.name));
    out.push_str(&format!("provider = \"{}\"\n", artifact.schema.provider));
    out.push_str(&format!("source = \"{}\"\n", artifact.schema.source));
    out.push_str(&format!(
        "source_fingerprint = \"{}\"\n",
        artifact.schema.source_fingerprint
    ));
    out.push_str(&format!(
        "schema_fingerprint = \"{}\"\n",
        artifact.schema.schema_fingerprint
    ));
    out.push_str(&format!(
        "generated_at = \"{}\"\n\n",
        artifact.schema.generated_at
    ));
    out.push_str("[fields]\n");
    for (name, field) in &artifact.fields {
        out.push_str(&format!(
            "{} = {{ type = \"{}\", nullable = {} }}\n",
            name,
            render_dx_type(field.ty),
            if field.nullable { "true" } else { "false" }
        ));
    }
    out
}

pub fn render_artifact_json(artifact: &SchemaArtifact) -> String {
    let mut out = String::new();
    out.push_str("{\"schema\":{");
    out.push_str(&format!(
        "\"format_version\":\"{}\",\"name\":\"{}\",\"provider\":\"{}\",\"source\":\"{}\",\"source_fingerprint\":\"{}\",\"schema_fingerprint\":\"{}\",\"generated_at\":\"{}\"",
        escape_json(&artifact.schema.format_version),
        escape_json(&artifact.schema.name),
        escape_json(&artifact.schema.provider),
        escape_json(&artifact.schema.source),
        escape_json(&artifact.schema.source_fingerprint),
        escape_json(&artifact.schema.schema_fingerprint),
        escape_json(&artifact.schema.generated_at),
    ));
    out.push_str("},\"fields\":{");
    let mut first = true;
    for (name, field) in &artifact.fields {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!(
            "\"{}\":{{\"type\":\"{}\",\"nullable\":{}}}",
            escape_json(name),
            render_dx_type(field.ty),
            if field.nullable { "true" } else { "false" }
        ));
    }
    out.push_str("}}");
    out
}

pub fn artifact_source_is_canonical(src: &str) -> Result<bool, SchemaArtifactError> {
    let artifact = parse_artifact(src)?;
    Ok(src == render_artifact_canonical(&artifact))
}

fn strip_comment(line: &str) -> &str {
    match line.find('#') {
        Some(index) => &line[..index],
        None => line,
    }
}

fn schema_name_to_artifact_stem(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();

    for (idx, ch) in chars.iter().copied().enumerate() {
        if ch.is_ascii_uppercase() {
            let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
            let next = chars.get(idx + 1).copied();
            let should_insert_separator = !out.is_empty()
                && (prev.is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                    || next.is_some_and(|c| c.is_ascii_lowercase()));
            if should_insert_separator && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }

    out.trim_matches('_').to_string()
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

fn parse_schema_key(key: &str) -> Result<&str, SchemaArtifactError> {
    match key {
        "format_version"
        | "name"
        | "provider"
        | "source"
        | "source_fingerprint"
        | "schema_fingerprint"
        | "generated_at" => Ok(key),
        _ => Err(SchemaArtifactError::UnknownSchemaKey(key.to_string())),
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

fn render_dx_type(ty: DxSchemaType) -> &'static str {
    match ty {
        DxSchemaType::Int => "Int",
        DxSchemaType::Float => "Float",
        DxSchemaType::Str => "Str",
        DxSchemaType::Bool => "Bool",
    }
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn path_buf_to_string(path: PathBuf) -> Option<String> {
    path.as_os_str().to_str().map(ToOwned::to_owned)
}

fn select_schema_declaration<'a>(
    declarations: &'a [SchemaSourceDeclaration],
    expected_name: Option<&str>,
) -> Result<&'a SchemaSourceDeclaration, SchemaArtifactError> {
    match expected_name {
        Some(name) => declarations
            .iter()
            .find(|decl| decl.name == name)
            .ok_or_else(|| SchemaArtifactError::SchemaDeclarationNotFound(name.to_string())),
        None if declarations.len() == 1 => declarations
            .first()
            .ok_or(SchemaArtifactError::SchemaSelectionRequired),
        _ => Err(SchemaArtifactError::SchemaSelectionRequired),
    }
}

fn resolve_schema_output_path(
    decl: &SchemaSourceDeclaration,
    source_path: &Path,
    explicit_output: Option<&PathBuf>,
) -> Result<PathBuf, SchemaArtifactError> {
    if let Some(path) = explicit_output {
        return Ok(path.clone());
    }

    let source_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(source_dir.join(schema_artifact_rel_path(decl)))
}

fn parse_cli_schema_fields(
    specs: &[String],
) -> Result<BTreeMap<String, SchemaField>, SchemaArtifactError> {
    let mut fields = BTreeMap::new();
    for spec in specs {
        let (name, field) = parse_cli_schema_field(spec)?;
        if fields.insert(name.clone(), field).is_some() {
            return Err(SchemaArtifactError::DuplicateKey(name));
        }
    }
    Ok(fields)
}

fn parse_cli_schema_field(spec: &str) -> Result<(String, SchemaField), SchemaArtifactError> {
    let (name, raw_ty) = spec
        .split_once('=')
        .ok_or_else(|| SchemaArtifactError::InvalidFieldSpec(spec.to_string()))?;
    let nullable = raw_ty.ends_with('?');
    let ty_name = if nullable {
        &raw_ty[..raw_ty.len() - 1]
    } else {
        raw_ty
    };
    let ty = parse_dx_type_name(ty_name)?;
    Ok((
        name.to_string(),
        SchemaField {
            ty,
            nullable,
        },
    ))
}

fn is_valid_fingerprint(value: &str) -> bool {
    if !value.starts_with("sha256:") {
        return false;
    }
    let digest = &value["sha256:".len()..];
    !digest.is_empty()
}

fn is_valid_schema_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_valid_field_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn is_supported_provider(value: &str) -> bool {
    matches!(value, "csv" | "parquet")
}

fn source_matches_provider(provider: &str, source: &str) -> bool {
    match provider {
        "csv" => source.ends_with(".csv"),
        "parquet" => source.ends_with(".parquet"),
        _ => false,
    }
}

fn is_valid_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(|(idx, b)| match idx {
                4 | 7 | 10 | 13 | 16 | 19 => true,
                _ => b.is_ascii_digit(),
            })
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

    #[test]
    fn rejects_duplicate_schema_key() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
name = "Other"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "duplicate key: name");
    }

    #[test]
    fn rejects_duplicate_field_name() {
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
id = { type = "Int", nullable = false }
id = { type = "Int", nullable = true }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "duplicate key: id");
    }

    #[test]
    fn rejects_unknown_schema_key() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"
owner = "analytics"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "unknown schema key: owner");
    }

    #[test]
    fn rejects_invalid_fingerprint_shape() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "md5:abc"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "invalid fingerprint: md5:abc");
    }

    #[test]
    fn rejects_invalid_timestamp_shape() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29 10:00:00"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "invalid timestamp: 2026-03-29 10:00:00");
    }

    #[test]
    fn rejects_invalid_schema_name_shape() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "customer-data"
provider = "csv"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "invalid schema name: customer-data");
    }

    #[test]
    fn rejects_unsupported_provider() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Sales"
provider = "postgres"
source = "postgres://db"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "unsupported provider: postgres");
    }

    #[test]
    fn rejects_invalid_field_name_shape() {
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
CustomerId = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "invalid field name: CustomerId");
    }

    #[test]
    fn rejects_source_that_does_not_match_provider() {
        let src = r#"
[schema]
format_version = "0.1.0"
name = "Customers"
provider = "csv"
source = "data/customers.parquet"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
id = { type = "Int", nullable = false }
"#;

        let err = parse_artifact(src).expect_err("should reject");
        assert_eq!(err.to_string(), "invalid source: data/customers.parquet");
    }

    #[test]
    fn renders_summary_for_example() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        assert_eq!(
            render_artifact_summary(&artifact),
            "OK name=Customers provider=csv fields=6 format_version=0.1.0"
        );
    }

    #[test]
    fn canonical_render_roundtrips() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        let rendered = render_artifact_canonical(&artifact);
        let reparsed = parse_artifact(&rendered).expect("reparse");

        assert_eq!(artifact, reparsed);
    }

    #[test]
    fn json_render_contains_core_shape() {
        let artifact = load_artifact(&example_path("sales.dxschema.example")).expect("parse");
        let rendered = render_artifact_json(&artifact);

        assert!(rendered.contains("\"schema\""));
        assert!(rendered.contains("\"fields\""));
        assert!(rendered.contains("\"provider\":\"parquet\""));
        assert!(rendered.contains("\"revenue\":{\"type\":\"Float\",\"nullable\":false}"));
    }

    #[test]
    fn canonical_source_check_accepts_canonical_render() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        let canonical = render_artifact_canonical(&artifact);

        assert!(artifact_source_is_canonical(&canonical).expect("check canonical"));
    }

    #[test]
    fn canonical_source_check_rejects_non_canonical_source() {
        let src = r#"
[schema]
provider = "csv"
format_version = "0.1.0"
name = "Customers"
source = "data/customers.csv"
source_fingerprint = "sha256:1"
schema_fingerprint = "sha256:2"
generated_at = "2026-03-29T10:00:00Z"

[fields]
name = { nullable = false, type = "Str" }
id = { type = "Int", nullable = false }
"#;

        assert!(!artifact_source_is_canonical(src).expect("check canonical"));
    }

    #[test]
    fn artifact_contract_accepts_matching_example() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        let contract = SchemaArtifactContract {
            name: "Customers",
            provider: "csv",
            source: "data/customers.csv",
        };

        validate_artifact_contract(&artifact, &contract).expect("matching contract");
    }

    #[test]
    fn artifact_contract_rejects_name_mismatch() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        let contract = SchemaArtifactContract {
            name: "Sales",
            provider: "csv",
            source: "data/customers.csv",
        };

        let err = validate_artifact_contract(&artifact, &contract).expect_err("name mismatch");
        assert_eq!(
            err.to_string(),
            "schema name mismatch: expected Sales, got Customers"
        );
    }

    #[test]
    fn artifact_contract_rejects_provider_mismatch() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        let contract = SchemaArtifactContract {
            name: "Customers",
            provider: "parquet",
            source: "data/customers.csv",
        };

        let err =
            validate_artifact_contract(&artifact, &contract).expect_err("provider mismatch");
        assert_eq!(
            err.to_string(),
            "provider mismatch: expected parquet, got csv"
        );
    }

    #[test]
    fn artifact_contract_rejects_source_mismatch() {
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("parse");
        let contract = SchemaArtifactContract {
            name: "Customers",
            provider: "csv",
            source: "data/other.csv",
        };

        let err = validate_artifact_contract(&artifact, &contract).expect_err("source mismatch");
        assert_eq!(
            err.to_string(),
            "source mismatch: expected data/other.csv, got data/customers.csv"
        );
    }

    #[test]
    fn build_artifact_accepts_valid_shape() {
        let artifact = build_artifact(
            SchemaMetadata {
                format_version: SUPPORTED_FORMAT_VERSION.to_string(),
                name: "Customers".to_string(),
                provider: "csv".to_string(),
                source: "data/customers.csv".to_string(),
                source_fingerprint: "sha256:source".to_string(),
                schema_fingerprint: "sha256:schema".to_string(),
                generated_at: "2026-03-29T10:00:00Z".to_string(),
            },
            BTreeMap::from([
                (
                    "id".to_string(),
                    SchemaField {
                        ty: DxSchemaType::Int,
                        nullable: false,
                    },
                ),
                (
                    "email".to_string(),
                    SchemaField {
                        ty: DxSchemaType::Str,
                        nullable: true,
                    },
                ),
            ]),
        )
        .expect("artifact");

        assert_eq!(artifact.schema.name, "Customers");
        assert_eq!(artifact.fields.len(), 2);
    }

    #[test]
    fn parse_dx_type_name_supports_public_lookup() {
        assert_eq!(parse_dx_type_name("Float").expect("type"), DxSchemaType::Float);
    }

    #[test]
    fn parses_source_declarations_from_example_surface() {
        let src = std::fs::read_to_string(example_path("customer_analysis.dx.example")).expect("src");
        let decls = parse_source_declarations(&src).expect("parse");

        assert_eq!(decls.len(), 3);
        assert_eq!(decls[0].name, "Customers");
        assert_eq!(decls[0].provider, "csv");
        assert_eq!(decls[0].source, "data/customers.csv");
        assert_eq!(decls[0].using_artifact, None);
        assert!(!decls[0].refresh);

        assert_eq!(decls[1].name, "Sales");
        assert_eq!(decls[1].using_artifact.as_deref(), Some("schemas/sales.dxschema"));
        assert!(!decls[1].refresh);

        assert_eq!(decls[2].name, "Events");
        assert_eq!(decls[2].provider, "parquet");
        assert!(decls[2].refresh);
    }

    #[test]
    fn rejects_invalid_source_declaration_shape() {
        let err = parse_source_declarations("schema Customers = postgres.schema(\"db\")")
            .expect_err("invalid");
        assert_eq!(err.to_string(), "unsupported provider: postgres");

        let err = parse_source_declarations("schema Customers = csv.schema(data/customers.csv)")
            .expect_err("invalid");
        assert_eq!(
            err.to_string(),
            "invalid source declaration: expected string literal, found identifier `data`"
        );
    }

    #[test]
    fn validates_source_declaration_against_artifact() {
        let src = std::fs::read_to_string(example_path("customer_analysis.dx.example")).expect("src");
        let decls = parse_source_declarations(&src).expect("parse");
        let artifact = load_artifact(&example_path("customers.dxschema.example")).expect("artifact");

        validate_source_declaration_contract(&decls[0], &artifact).expect("match");
    }

    #[test]
    fn default_schema_artifact_rel_path_uses_snake_case_name() {
        assert_eq!(
            default_schema_artifact_rel_path("CustomerEvents"),
            "schemas/customer_events.dxschema"
        );
        assert_eq!(
            default_schema_artifact_rel_path("Sales"),
            "schemas/sales.dxschema"
        );
    }

    #[test]
    fn analyzes_locked_source_artifacts_with_matching_artifact() {
        let src = r#"
schema Customers = csv.schema("data/customers.csv")
schema Sales = parquet.schema("data/sales.parquet") using "schemas/sales.dxschema"
"#;
        let report = analyze_locked_source_artifacts(&src, |artifact_path| match artifact_path {
            "schemas/customers.dxschema" => {
                load_artifact(&example_path("customers.dxschema.example"))
            }
            "schemas/sales.dxschema" => load_artifact(&example_path("sales.dxschema.example")),
            other => Err(SchemaArtifactError::Io(format!("unexpected artifact: {other}"))),
        })
        .expect("analyze");

        assert_eq!(
            report.checks,
            vec![
                LockedSchemaArtifactCheck {
                    schema: "Customers".to_string(),
                    artifact_path: "schemas/customers.dxschema".to_string(),
                },
                LockedSchemaArtifactCheck {
                    schema: "Sales".to_string(),
                    artifact_path: "schemas/sales.dxschema".to_string(),
                },
            ]
        );
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn analyzes_locked_source_artifacts_reports_loader_error() {
        let src = r#"
schema Customers = csv.schema("data/customers.csv")
"#;
        let report = analyze_locked_source_artifacts(&src, |_| {
            Err(SchemaArtifactError::Io("artifact missing".to_string()))
        })
        .expect("analyze");

        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Customers");
        assert_eq!(report.diagnostics[0].artifact_path, "schemas/customers.dxschema");
        assert!(report.diagnostics[0].message.contains("artifact missing"));
    }

    #[test]
    fn analyzes_locked_source_artifacts_reports_contract_mismatch() {
        let src = r#"
schema Customers = csv.schema("data/customers.csv")
"#;
        let report = analyze_locked_source_artifacts(&src, |_| {
            load_artifact(&example_path("sales.dxschema.example"))
        })
        .expect("analyze");

        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].schema, "Customers");
        assert_eq!(report.diagnostics[0].artifact_path, "schemas/customers.dxschema");
        assert!(
            report.diagnostics[0]
                .message
                .contains("schema name mismatch")
        );
    }
}
