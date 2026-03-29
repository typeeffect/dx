pub mod exec;
pub mod emit;
pub mod link;
pub mod pipeline;
pub mod toolchain;

pub use exec::{
    build_executable_plan_from_ll,
    build_executable_plan_from_source,
    build_source_executable_plan,
    default_runtime_archive_path,
    ExecutablePlan,
    SourceExecutablePlan,
};
pub use emit::{emit_module, EmitError};
pub use link::{build_link_command_plan, render_link_plan, LinkCommandPlan};
pub use pipeline::{
    emit_file_to_path,
    emit_file_to_path_and_verify,
    emit_file_to_string,
    emit_source_to_string,
    verify_ll_path,
    verify_ll_path_with_toolchain,
    PipelineError,
};
pub use toolchain::{LlvmToolchain, ToolchainError};
