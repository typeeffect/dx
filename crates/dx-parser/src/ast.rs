#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    ImportPy(ImportPyDecl),
    Function(FunctionDecl),
    Statement(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportPyDecl {
    pub module: String,
    pub names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub effects: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Name(String),
    Function {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
        effects: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    ValBind { name: String, value: Expr },
    VarBind { name: String, value: Expr },
    Rebind { name: String, value: Expr },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Name(String),
    Integer(String),
    String(String),
    Member {
        base: Box<Expr>,
        name: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Lambda {
        params: Vec<LambdaParam>,
        body: LambdaBody,
    },
    Lazy {
        body: LambdaBody,
    },
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        else_branch: Option<Vec<Stmt>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Placeholder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Name(String),
    Wildcard,
    Constructor {
        name: String,
        args: Vec<Pattern>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    pub name: String,
    pub ty: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Positional(Expr),
    Named { name: String, value: Expr },
}
