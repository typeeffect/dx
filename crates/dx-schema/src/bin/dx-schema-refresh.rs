use dx_schema::{
    parse_schema_refresh_args, refresh_schema_artifact, render_schema_refresh_success,
    SchemaRefreshRequest,
};

fn print_usage() {
    eprintln!(
        "usage: dx-schema-refresh [--name <SchemaName>] [--output <path>] <source.dx> --source-fingerprint <sha256:...> --schema-fingerprint <sha256:...> --generated-at <timestamp> --field <name=Type[?]> [--field ...]"
    );
}

fn main() {
    let Some(options) = parse_args(std::env::args_os().skip(1)) else {
        print_usage();
        std::process::exit(2);
    };

    match refresh_schema_artifact(options) {
        Ok(result) => println!("{}", render_schema_refresh_success(&result)),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn parse_args<I>(args: I) -> Option<SchemaRefreshRequest>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString>,
{
    parse_schema_refresh_args(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_args_accepts_minimal_valid_command() {
        let opts = parse_args([
            "customer_analysis.dx",
            "--source-fingerprint",
            "sha256:source",
            "--schema-fingerprint",
            "sha256:schema",
            "--generated-at",
            "2026-03-29T10:00:00Z",
            "--field",
            "id=Int",
        ])
        .expect("options");

        assert_eq!(
            opts,
            SchemaRefreshRequest {
                name: None,
                source_path: PathBuf::from("customer_analysis.dx"),
                output: None,
                source_fingerprint: "sha256:source".to_string(),
                schema_fingerprint: "sha256:schema".to_string(),
                generated_at: "2026-03-29T10:00:00Z".to_string(),
                fields: vec!["id=Int".to_string()],
            }
        );
    }
}
