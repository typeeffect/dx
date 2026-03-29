use dx_runtime_stub::link::render_link_command;
use std::env;
use std::path::PathBuf;

fn main() {
    let mut args = env::args().skip(1);
    let object = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("build/program.o"));
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("build/program"));

    println!("{}", render_link_command(&object, &output));
}
