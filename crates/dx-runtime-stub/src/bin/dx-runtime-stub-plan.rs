use dx_runtime_stub::plan::{render_runtime_stub_plan, render_runtime_stub_plan_json};
use std::env;
use std::path::PathBuf;

fn main() {
    let mut json = false;
    let mut positional = Vec::new();
    for arg in env::args().skip(1) {
        if arg == "--json" {
            json = true;
        } else {
            positional.push(arg);
        }
    }
    let mut args = positional.into_iter();
    let object = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("build/program.o"));
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("build/program"));

    if json {
        println!("{}", render_runtime_stub_plan_json(&object, &output));
    } else {
        println!("{}", render_runtime_stub_plan(&object, &output));
    }
}
