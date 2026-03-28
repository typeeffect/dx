use dx_parser::BinOp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub globals: Vec<GlobalString>,
    pub externs: Vec<ExternDecl>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalString {
    pub symbol: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternDecl {
    pub symbol: &'static str,
    pub params: Vec<Type>,
    pub ret: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Type,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub label: String,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Assign {
        result: String,
        ty: Type,
        value: Operand,
    },
    BinaryOp {
        result: String,
        op: BinOp,
        ty: Type,
        lhs: Operand,
        rhs: Operand,
    },
    PackEnv {
        result: String,
        captures: Vec<Operand>,
    },
    CallExtern {
        result: Option<String>,
        symbol: &'static str,
        ret: Type,
        args: Vec<Operand>,
        comment: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Terminator {
    Ret(Option<Operand>),
    Br(String),
    CondBr {
        cond: Operand,
        then_label: String,
        else_label: String,
    },
    MatchBr {
        scrutinee: Operand,
        arms: Vec<(String, String)>,
        fallback: String,
    },
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Register(String, Type),
    Global(String, Type),
    ConstInt(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I64,
    Double,
    I1,
    Ptr,
    Void,
}
