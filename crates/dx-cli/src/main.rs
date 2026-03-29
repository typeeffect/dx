fn print_usage() {
    eprintln!(
        "usage:\n  dx schema refresh [--name <SchemaName>] [--output <path>] <source.dx> --source-fingerprint <sha256:...> --schema-fingerprint <sha256:...> --generated-at <timestamp> --field <name=Type[?]> [--field ...]"
    );
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(domain) = args.next() else {
        print_usage();
        std::process::exit(2);
    };

    if domain == "--help" || domain == "-h" {
        print_usage();
        return;
    }

    let Some(command) = args.next() else {
        print_usage();
        std::process::exit(2);
    };

    if domain == "schema" && command == "refresh" {
        let Some(request) = dx_schema::parse_schema_refresh_args(args) else {
            print_usage();
            std::process::exit(2);
        };

        match dx_schema::refresh_schema_artifact(request) {
            Ok(result) => println!("{}", dx_schema::render_schema_refresh_success(&result)),
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        return;
    }

    print_usage();
    std::process::exit(2);
}
