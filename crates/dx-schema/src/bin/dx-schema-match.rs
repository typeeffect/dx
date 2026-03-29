use dx_schema::{load_artifact, validate_artifact_contract, SchemaArtifactContract};
use std::path::PathBuf;

fn print_usage() {
    eprintln!(
        "usage: dx-schema-match --name <name> --provider <provider> --source <source> <path.dxschema>"
    );
}

fn main() {
    let Some(options) = parse_args(std::env::args_os().skip(1)) else {
        print_usage();
        std::process::exit(2);
    };

    match load_artifact(&options.path) {
        Ok(artifact) => {
            let contract = SchemaArtifactContract {
                name: &options.name,
                provider: &options.provider,
                source: &options.source,
            };
            match validate_artifact_contract(&artifact, &contract) {
                Ok(()) => println!(
                    "OK match name={} provider={} source={}",
                    options.name, options.provider, options.source
                ),
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    name: String,
    provider: String,
    source: String,
    path: PathBuf,
}

fn parse_args<I>(args: I) -> Option<CliOptions>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut name = None;
    let mut provider = None;
    let mut source = None;
    let mut path = None;
    let mut args = args.into_iter().map(Into::into).peekable();

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
        if path.is_some() {
            return None;
        }
        path = Some(PathBuf::from(arg));
    }

    Some(CliOptions {
        name: name?,
        provider: provider?,
        source: source?,
        path: path?,
    })
}

fn path_to_string(path: PathBuf) -> Option<String> {
    path.as_os_str().to_str().map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_accepts_full_contract() {
        let opts = parse_args([
            "--name",
            "Customers",
            "--provider",
            "csv",
            "--source",
            "data/customers.csv",
            "customers.dxschema",
        ])
        .expect("options");

        assert_eq!(
            opts,
            CliOptions {
                name: "Customers".to_string(),
                provider: "csv".to_string(),
                source: "data/customers.csv".to_string(),
                path: PathBuf::from("customers.dxschema"),
            }
        );
    }

    #[test]
    fn parse_args_rejects_missing_provider() {
        assert!(
            parse_args([
                "--name",
                "Customers",
                "--source",
                "data/customers.csv",
                "customers.dxschema",
            ])
            .is_none()
        );
    }
}
