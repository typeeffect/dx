use dx_schema::{load_artifact, render_artifact_canonical, render_artifact_json, render_artifact_summary};
use std::path::PathBuf;

fn print_usage() {
    eprintln!("usage: dx-schema-validate [--json|--canonical] <path.dxschema>");
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let mut mode = OutputMode::Summary;
    let Some(first) = args.next() else {
        print_usage();
        std::process::exit(2);
    };
    let path = if first == "--json" {
        mode = OutputMode::Json;
        args.next()
    } else if first == "--canonical" {
        mode = OutputMode::Canonical;
        args.next()
    } else {
        Some(first)
    };
    let Some(path) = path else {
        print_usage();
        std::process::exit(2);
    };
    if args.next().is_some() {
        print_usage();
        std::process::exit(2);
    }

    let path = PathBuf::from(path);
    match load_artifact(&path) {
        Ok(artifact) => {
            let rendered = match mode {
                OutputMode::Summary => render_artifact_summary(&artifact),
                OutputMode::Json => render_artifact_json(&artifact),
                OutputMode::Canonical => render_artifact_canonical(&artifact),
            };
            println!("{rendered}");
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[derive(Clone, Copy)]
enum OutputMode {
    Summary,
    Json,
    Canonical,
}
