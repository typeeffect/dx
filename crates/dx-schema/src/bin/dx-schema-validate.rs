use dx_schema::{load_artifact, render_artifact_summary};
use std::path::PathBuf;

fn print_usage() {
    eprintln!("usage: dx-schema-validate <path.dxschema>");
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let Some(path) = args.next() else {
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
            println!("{}", render_artifact_summary(&artifact));
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
