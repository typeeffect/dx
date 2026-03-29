pub mod capture;
pub mod effects;
pub mod hir;
pub mod lower;
pub mod resolve;
pub mod schema;
pub mod typecheck;
pub mod typed;
pub mod types;

pub use capture::annotate_module_captures;
pub use effects::{check_module_effects, Diagnostic, FunctionEffectReport, ModuleEffectReport};
pub use hir::*;
pub use lower::lower_module;
pub use resolve::{resolve_module, BindingDiagnostic, NameResolutionReport};
pub use schema::{
    analyze_schema_declarations, bind_locked_schema_artifacts,
    bind_locked_schema_artifacts_from_fs,
    load_bound_schema_catalog, load_bound_schema_catalog_from_fs, BoundSchemaArtifact,
    BoundSchemaCatalog, LockedSchemaArtifactBindingDiagnostic,
    LockedSchemaArtifactBindingReport, LockedSchemaRequirement,
    SchemaDeclarationDiagnostic, SchemaDeclarationReport, SchemaRefreshRequest,
};
pub use typecheck::{
    typecheck_module, typecheck_module_with_bound_schemas, TypeCheckDiagnostic, TypeCheckReport,
};
pub use typed as typed_hir;
pub use types::Type;
