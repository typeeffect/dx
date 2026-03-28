pub mod lower;
pub mod mir;
pub mod validate;

pub use lower::lower_module;
pub use mir::*;
pub use validate::{validate_module, ValidationDiagnostic, ValidationReport};
