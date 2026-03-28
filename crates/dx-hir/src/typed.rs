use crate::{hir, types::Type};
use dx_parser::ImportPyDecl;

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    ImportPy(ImportPyDecl),
    Function(Function),
    Statement(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub effects: Vec<String>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub result: Option<Box<Expr>>,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        mutable: bool,
        value: Expr,
        synthetic: bool,
    },
    Rebind {
        name: String,
        value: Expr,
    },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub ty: Type,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Unit,
    Name(String),
    Integer(String),
    String(String),
    Member {
        base: Box<Expr>,
        name: String,
    },
    Call {
        target: CallTarget,
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Closure {
        params: Vec<ClosureParam>,
        body: Box<ClosureBody>,
    },
    If {
        branches: Vec<(Expr, Block)>,
        else_branch: Option<Block>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    BinaryOp {
        op: dx_parser::BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallTarget {
    NativeFunction { name: String },
    PythonFunction { name: String },
    PythonMember { name: String },
    PythonDynamic,
    LocalClosure { name: String },
    Dynamic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClosureBody {
    Expr(Box<Expr>),
    Block(Box<Block>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Positional(Expr),
    Named { name: String, value: Expr },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: hir::Pattern,
    pub body: Block,
}
