pub mod emit;
pub mod pipeline;

pub use emit::{emit_module, EmitError};
pub use pipeline::{emit_file_to_path, emit_file_to_string, emit_source_to_string, PipelineError};
