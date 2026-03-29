use dx_runtime_stub::build_plan::{
    render_runtime_stub_build_plan,
    render_runtime_stub_build_plan_json,
};
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
    let profile = args.next().unwrap_or_else(|| "debug".to_string());
    let target_dir = args.next().map(PathBuf::from);
    if json {
        println!(
            "{}",
            render_runtime_stub_build_plan_json(&profile, target_dir.as_deref())
        );
    } else {
        println!(
            "{}",
            render_runtime_stub_build_plan(&profile, target_dir.as_deref())
        );
    }
}
