use dx_runtime_stub::build_plan::render_runtime_stub_build_plan;
use std::env;
use std::path::PathBuf;

fn main() {
    let mut args = env::args().skip(1);
    let profile = args.next().unwrap_or_else(|| "debug".to_string());
    let target_dir = args.next().map(PathBuf::from);
    println!(
        "{}",
        render_runtime_stub_build_plan(&profile, target_dir.as_deref())
    );
}
