pub mod emit;
pub mod pipeline;
pub mod toolchain;

pub use emit::{emit_module, EmitError};
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
