use dx_runtime_stub::archive::{
    render_runtime_stub_artifact_info,
    render_runtime_stub_artifact_info_json,
};

fn main() {
    let json = std::env::args().skip(1).any(|arg| arg == "--json");
    if json {
        println!("{}", render_runtime_stub_artifact_info_json());
    } else {
        println!("{}", render_runtime_stub_artifact_info());
    }
}
