use dx_parser::TypeExpr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    PyObj,
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
        match ty {
            TypeExpr::Name(name) => match name.as_str() {
                "Int" => Self::Int,
                "Float" => Self::Float,
                "Bool" => Self::Bool,
                "Str" => Self::Str,
                "Unit" => Self::Unit,
                "PyObj" => Self::PyObj,
                other => Self::Named(other.to_string()),
            },
            TypeExpr::Function {
                params,
                ret,
                effects,
            } => Self::Function {
                params: params.iter().map(Self::from_type_expr).collect(),
                ret: Box::new(Self::from_type_expr(ret)),
                effects: effects.clone(),
            },
        }
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}
