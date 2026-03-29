use dx_schema::{
    load_artifact, parse_source_declarations, schema_artifact_rel_path,
    validate_source_declaration_contract,
    SchemaSourceDeclaration,
};
use std::path::PathBuf;

fn print_usage() {
    eprintln!(
        "usage: dx-schema-check-source [--name <SchemaName>] <source.dx> <artifact.dxschema>\n       dx-schema-check-source [--name <SchemaName>] --locked <source.dx>"
    );
}

fn main() {
    let Some(options) = parse_args(std::env::args_os().skip(1)) else {
        print_usage();
        std::process::exit(2);
    };

    let src = match std::fs::read_to_string(&options.source_path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("i/o error: {err}");
            std::process::exit(1);
        }
    };

    let declarations = match parse_source_declarations(&src) {
        Ok(decls) => decls,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    if options.locked {
        match check_locked_declarations(&declarations, &options) {
            Ok(summary) => println!("{summary}"),
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
        }
    } else {
        let Some(decl) = select_declaration(&declarations, options.name.as_deref()) else {
            eprintln!("could not select schema declaration");
            std::process::exit(1);
        };

        let artifact_path = options
            .artifact_path
            .as_ref()
            .expect("artifact path required outside --locked mode");
        let artifact = match load_artifact(artifact_path) {
            Ok(artifact) => artifact,
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        };

        match validate_source_declaration_contract(decl, &artifact) {
            Ok(()) => println!(
                "OK source-match name={} provider={} source={}",
                decl.name, decl.provider, decl.source
            ),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    name: Option<String>,
    locked: bool,
    source_path: PathBuf,
    artifact_path: Option<PathBuf>,
}

fn parse_args<I>(args: I) -> Option<CliOptions>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut name = None;
    let mut locked = false;
    let mut source_path = None;
    let mut artifact_path = None;
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
        if arg == "--locked" {
            locked = true;
            continue;
        }
        if source_path.is_none() {
            source_path = Some(PathBuf::from(arg));
            continue;
        }
        if !locked && artifact_path.is_none() {
            artifact_path = Some(PathBuf::from(arg));
            continue;
        }
        return None;
    }

    if !locked && artifact_path.is_none() {
        return None;
    }

    Some(CliOptions {
        name,
        locked,
        source_path: source_path?,
        artifact_path,
    })
}

fn select_declaration<'a>(
    declarations: &'a [SchemaSourceDeclaration],
    expected_name: Option<&str>,
) -> Option<&'a SchemaSourceDeclaration> {
    match expected_name {
        Some(name) => declarations.iter().find(|decl| decl.name == name),
        None if declarations.len() == 1 => declarations.first(),
        _ => None,
    }
}

fn path_to_string(path: PathBuf) -> Option<String> {
    path.as_os_str().to_str().map(ToOwned::to_owned)
}

fn check_locked_declarations(
    declarations: &[SchemaSourceDeclaration],
    options: &CliOptions,
) -> Result<String, String> {
    let source_dir = options
        .source_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let selected: Vec<&SchemaSourceDeclaration> = match options.name.as_deref() {
        Some(name) => declarations.iter().filter(|decl| decl.name == name).collect(),
        None => declarations.iter().collect(),
    };

    if selected.is_empty() {
        return Err("could not select locked schema declaration".to_string());
    }

    for decl in &selected {
        let artifact_rel = schema_artifact_rel_path(decl);
        let artifact_path = source_dir.join(&artifact_rel);
        let artifact = load_artifact(&artifact_path).map_err(|err| err.to_string())?;
        validate_source_declaration_contract(decl, &artifact).map_err(|err| err.to_string())?;
    }

    if selected.len() == 1 {
        let decl = selected[0];
        Ok(format!(
            "OK locked-source-match name={} provider={} source={}",
            decl.name, decl.provider, decl.source
        ))
    } else {
        Ok(format!(
            "OK locked-source-match count={} source={}",
            selected.len(),
            options.source_path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_accepts_named_source_check() {
        let opts = parse_args([
            "--name",
            "Customers",
            "customer_analysis.dx.example",
            "customers.dxschema.example",
        ])
        .expect("options");

        assert_eq!(
            opts,
            CliOptions {
                name: Some("Customers".to_string()),
                locked: false,
                source_path: PathBuf::from("customer_analysis.dx.example"),
                artifact_path: Some(PathBuf::from("customers.dxschema.example")),
            }
        );
    }

    #[test]
    fn parse_args_accepts_locked_mode() {
        let opts = parse_args(["--locked", "customer_analysis.dx.example"]).expect("options");

        assert_eq!(
            opts,
            CliOptions {
                name: None,
                locked: true,
                source_path: PathBuf::from("customer_analysis.dx.example"),
                artifact_path: None,
            }
        );
    }

    #[test]
    fn select_declaration_requires_name_for_multi_decl_source() {
        let declarations = vec![
            SchemaSourceDeclaration {
                name: "A".to_string(),
                provider: "csv".to_string(),
                source: "data/a.csv".to_string(),
                using_artifact: None,
                refresh: false,
            },
            SchemaSourceDeclaration {
                name: "B".to_string(),
                provider: "csv".to_string(),
                source: "data/b.csv".to_string(),
                using_artifact: None,
                refresh: false,
            },
        ];

        assert!(select_declaration(&declarations, None).is_none());
        assert_eq!(
            select_declaration(&declarations, Some("B")).map(|decl| decl.name.as_str()),
            Some("B")
        );
    }
}
