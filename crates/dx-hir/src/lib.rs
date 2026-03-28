pub mod effects;
pub mod hir;
pub mod lower;
pub mod resolve;

pub use effects::{check_module_effects, Diagnostic, FunctionEffectReport, ModuleEffectReport};
pub use hir::*;
pub use lower::lower_module;
pub use resolve::{resolve_module, BindingDiagnostic, NameResolutionReport};
