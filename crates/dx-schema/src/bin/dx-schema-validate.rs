use dx_schema::{
    artifact_source_is_canonical, load_artifact, render_artifact_canonical, render_artifact_json,
    render_artifact_summary, validate_artifact_contract, SchemaArtifactContract,
};
use std::path::PathBuf;

fn print_usage() {
    eprintln!(
        "usage: dx-schema-validate [--json|--canonical|--check-canonical] [--expect-name <name>] [--expect-provider <provider>] [--expect-source <source>] <path.dxschema>"
    );
}

fn main() {
    let Some(options) = parse_args(std::env::args_os().skip(1)) else {
        print_usage();
        std::process::exit(2);
    };
    match options.mode {
        OutputMode::CheckCanonical => match std::fs::read_to_string(&options.path) {
            Ok(src) => match artifact_source_is_canonical(&src) {
                Ok(true) => println!("OK canonical {}", options.path.display()),
                Ok(false) => {
                    eprintln!("artifact is not canonical: {}", options.path.display());
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("i/o error: {err}");
                std::process::exit(1);
            }
        },
        _ => match load_artifact(&options.path) {
            Ok(artifact) => {
                if let Some(contract) = options.contract() {
                    if let Err(err) = validate_artifact_contract(&artifact, &contract) {
                        eprintln!("{err}");
                        std::process::exit(1);
                    }
                }
                let rendered = match options.mode {
                    OutputMode::Summary => render_artifact_summary(&artifact),
                    OutputMode::Json => render_artifact_json(&artifact),
                    OutputMode::Canonical => render_artifact_canonical(&artifact),
                    OutputMode::CheckCanonical => unreachable!(),
                };
                match options.mode {
                    OutputMode::Canonical => print!("{rendered}"),
                    _ => println!("{rendered}"),
                }
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Summary,
    Json,
    Canonical,
    CheckCanonical,
}

#[derive(Debug, PartialEq, Eq)]
struct CliOptions {
    mode: OutputMode,
    path: PathBuf,
    contract: Option<OwnedContract>,
}

#[derive(Debug, PartialEq, Eq)]
struct OwnedContract {
    name: String,
    provider: String,
    source: String,
}

impl OwnedContract {
    fn as_contract(&self) -> SchemaArtifactContract<'_> {
        SchemaArtifactContract {
            name: &self.name,
            provider: &self.provider,
            source: &self.source,
        }
    }
}

impl CliOptions {
    fn contract(&self) -> Option<SchemaArtifactContract<'_>> {
        self.contract.as_ref().map(OwnedContract::as_contract)
    }
}

fn parse_args<I>(args: I) -> Option<CliOptions>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    let mut mode = OutputMode::Summary;
    let mut path = None;
    let mut expect_name = None;
    let mut expect_provider = None;
    let mut expect_source = None;
    let mut args = args.into_iter().map(Into::into).peekable();

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            return None;
        }
        if arg == "--json" {
            mode = OutputMode::Json;
            continue;
        }
        if arg == "--canonical" {
            mode = OutputMode::Canonical;
            continue;
        }
        if arg == "--check-canonical" {
            mode = OutputMode::CheckCanonical;
            continue;
        }
        if arg == "--expect-name" {
            expect_name = args.next().map(PathBuf::from).and_then(path_to_string);
            if expect_name.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--expect-provider" {
            expect_provider = args.next().map(PathBuf::from).and_then(path_to_string);
            if expect_provider.is_none() {
                return None;
            }
            continue;
        }
        if arg == "--expect-source" {
            expect_source = args.next().map(PathBuf::from).and_then(path_to_string);
            if expect_source.is_none() {
                return None;
            }
            continue;
        }
        if path.is_some() {
            return None;
        }
        path = Some(PathBuf::from(arg));
    }

    let path = path?;
    let contract = match (expect_name, expect_provider, expect_source) {
        (None, None, None) => None,
        (Some(name), Some(provider), Some(source)) => Some(OwnedContract {
            name,
            provider,
            source,
        }),
        _ => return None,
    };

    Some(CliOptions {
        mode,
        path,
        contract,
    })
}

fn path_to_string(path: PathBuf) -> Option<String> {
    path.as_os_str().to_str().map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_args_defaults_to_summary_without_contract() {
        let opts = parse_args(["example.dxschema"]).expect("options");
        assert_eq!(
            opts,
            CliOptions {
                mode: OutputMode::Summary,
                path: PathBuf::from("example.dxschema"),
                contract: None,
            }
        );
    }

    #[test]
    fn parse_args_supports_full_contract_expectations() {
        let opts = parse_args([
            "--expect-name",
            "Customers",
            "--expect-provider",
            "csv",
            "--expect-source",
            "data/customers.csv",
            "customers.dxschema",
        ])
        .expect("options");
        assert_eq!(
            opts,
            CliOptions {
                mode: OutputMode::Summary,
                path: PathBuf::from("customers.dxschema"),
                contract: Some(OwnedContract {
                    name: "Customers".to_string(),
                    provider: "csv".to_string(),
                    source: "data/customers.csv".to_string(),
                }),
            }
        );
    }

    #[test]
    fn parse_args_rejects_partial_contract() {
        assert!(parse_args(["--expect-name", "Customers", "customers.dxschema"]).is_none());
    }

    #[test]
    fn contract_helper_exposes_borrowed_contract() {
        let opts = CliOptions {
            mode: OutputMode::Summary,
            path: PathBuf::from("customers.dxschema"),
            contract: Some(OwnedContract {
                name: "Customers".to_string(),
                provider: "csv".to_string(),
                source: "data/customers.csv".to_string(),
            }),
        };

        let contract = opts.contract().expect("contract");
        assert_eq!(contract.name, "Customers");
        assert_eq!(contract.provider, "csv");
        assert_eq!(contract.source, "data/customers.csv");
    }

    #[test]
    fn path_to_string_accepts_plain_utf8_path() {
        assert_eq!(
            path_to_string(PathBuf::from("data/customers.csv")).as_deref(),
            Some("data/customers.csv")
        );
    }
}
