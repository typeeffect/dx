use dx_schema::{
    build_artifact, parse_dx_type_name, render_artifact_canonical, SchemaArtifactError,
    SchemaField, SchemaMetadata, SUPPORTED_FORMAT_VERSION,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn print_usage() {
    eprintln!(
        "usage: dx-schema-new --name <Name> --provider <provider> --source <source> --source-fingerprint <sha256:...> --schema-fingerprint <sha256:...> --generated-at <timestamp> --field <name=Type[?]> [--field ...] [--output <path>]"
    );
}

fn main() {
    let Some(options) = parse_args(std::env::args_os().skip(1)) else {
        print_usage();
        std::process::exit(2);
    };

    let mut fields = BTreeMap::new();
    for spec in &options.fields {
        let (name, field) = match parse_cli_field(spec) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        };
        if fields.insert(name.clone(), field).is_some() {
            eprintln!("duplicate key: {name}");
            std::process::exit(1);
        }
    }

    let artifact = match build_artifact(
        SchemaMetadata {
            format_version: SUPPORTED_FORMAT_VERSION.to_string(),
            name: options.name,
            provider: options.provider,
            source: options.source,
            source_fingerprint: options.source_fingerprint,
            schema_fingerprint: options.schema_fingerprint,
            generated_at: options.generated_at,
        },
        fields,
    ) {
        Ok(artifact) => artifact,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let rendered = render_artifact_canonical(&artifact);
    match options.output {
        Some(path) => {
            if let Err(err) = std::fs::write(&path, rendered) {
                eprintln!("i/o error: {err}");
                std::process::exit(1);
            }
            println!("OK wrote {}", path.display());
        }
        None => print!("{rendered}"),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    name: String,
    provider: String,
    source: String,
    source_fingerprint: String,
    schema_fingerprint: String,
    generated_at: String,
    fields: Vec<String>,
    output: Option<PathBuf>,
}

fn parse_args<I>(args: I) -> Option<CliOptions>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut name = None;
    let mut provider = None;
    let mut source = None;
    let mut source_fingerprint = None;
    let mut schema_fingerprint = None;
    let mut generated_at = None;
    let mut fields = Vec::new();
    let mut output = None;
    let mut args = args.into_iter().map(Into::into);

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return None;
        }
        if arg == "--name" {
            name = args.next().map(PathBuf::from).and_then(path_to_string);
            if name.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--provider" {
            provider = args.next().map(PathBuf::from).and_then(path_to_string);
            if provider.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--source" {
            source = args.next().map(PathBuf::from).and_then(path_to_string);
            if source.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--source-fingerprint" {
            source_fingerprint = args.next().map(PathBuf::from).and_then(path_to_string);
            if source_fingerprint.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--schema-fingerprint" {
            schema_fingerprint = args.next().map(PathBuf::from).and_then(path_to_string);
            if schema_fingerprint.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--generated-at" {
            generated_at = args.next().map(PathBuf::from).and_then(path_to_string);
            if generated_at.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--field" {
            let field = args.next().map(PathBuf::from).and_then(path_to_string)?;
            fields.push(field);
            continue;
        }
        if arg == "--output" {
            output = args.next().map(PathBuf::from);
            if output.is_none() {
                return None;
            }
            continue;
        }
        return None;
    }

    if fields.is_empty() {
        return None;
    }

    Some(CliOptions {
        name: name?,
        provider: provider?,
        source: source?,
        source_fingerprint: source_fingerprint?,
        schema_fingerprint: schema_fingerprint?,
        generated_at: generated_at?,
        fields,
        output,
    })
}

fn parse_cli_field(spec: &str) -> Result<(String, SchemaField), SchemaArtifactError> {
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

fn path_to_string(path: PathBuf) -> Option<String> {
    path.as_os_str().to_str().map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_requires_full_core_shape() {
        assert!(parse_args(["--name", "Customers"]).is_none());
    }

    #[test]
    fn parse_args_accepts_minimal_valid_command() {
        let opts = parse_args([
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
        ])
        .expect("options");

        assert_eq!(opts.name, "Customers");
        assert_eq!(opts.fields, vec!["id=Int".to_string(), "email=Str?".to_string()]);
    }

    #[test]
    fn parse_cli_field_supports_nullable_suffix() {
        let (name, field) = parse_cli_field("email=Str?").expect("field");
        assert_eq!(name, "email");
        assert_eq!(field.ty, dx_schema::DxSchemaType::Str);
        assert!(field.nullable);
    }

    #[test]
    fn parse_cli_field_rejects_missing_assignment() {
        let err = parse_cli_field("email").expect_err("invalid");
        assert!(matches!(err, SchemaArtifactError::InvalidFieldSpec(_)));
    }
}
