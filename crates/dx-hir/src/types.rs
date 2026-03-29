use dx_parser::TypeExpr;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    PyObj,
    SchemaRow(String),
    Option(Box<Type>),
    Named(String),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: Vec<String>,
    },
    Unknown,
}

impl Type {
    pub fn from_type_expr(ty: &TypeExpr) -> Self {
        Self::from_type_expr_with_schema_rows(ty, &HashSet::new())
    }

    pub fn from_type_expr_with_schema_rows(ty: &TypeExpr, known_schema_rows: &HashSet<String>) -> Self {
        match ty {
            TypeExpr::Name(name) => resolve_named_type(name, known_schema_rows),
            TypeExpr::Function {
                params,
                ret,
                effects,
            } => Self::Function {
                params: params
                    .iter()
                    .map(|param| Self::from_type_expr_with_schema_rows(param, known_schema_rows))
                    .collect(),
                ret: Box::new(Self::from_type_expr_with_schema_rows(ret, known_schema_rows)),
                effects: effects.clone(),
            },
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

fn resolve_named_type(name: &str, known_schema_rows: &HashSet<String>) -> Type {
    if let Some(inner) = name.strip_prefix("Option(").and_then(|rest| rest.strip_suffix(')')) {
        return Type::Option(Box::new(resolve_named_type(inner, known_schema_rows)));
    }
    match name {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Bool" => Type::Bool,
        "Str" => Type::Str,
        "Unit" => Type::Unit,
        "PyObj" => Type::PyObj,
        _ => match name.strip_suffix(".Row") {
            Some(schema) if known_schema_rows.contains(schema) => Type::SchemaRow(schema.to_string()),
            _ => Type::Named(name.to_string()),
        },
    }
}
