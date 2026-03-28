#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub externs: Vec<ExternDecl>,
    pub functions: Vec<Function>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    CallExtern {
        result: Option<String>,
        symbol: &'static str,
        ret: Type,
        args: Vec<Operand>,
        comment: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Register(String, Type),
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
