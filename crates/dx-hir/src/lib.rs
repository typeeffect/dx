pub mod effects;
pub mod hir;
pub mod lower;
pub mod resolve;
pub mod typecheck;
pub mod typed;
pub mod types;

pub use effects::{check_module_effects, Diagnostic, FunctionEffectReport, ModuleEffectReport};
pub use hir::*;
pub use lower::lower_module;
pub use resolve::{resolve_module, BindingDiagnostic, NameResolutionReport};
pub use typecheck::{typecheck_module, TypeCheckDiagnostic, TypeCheckReport};
pub use typed as typed_hir;
pub use types::Type;
