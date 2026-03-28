use dx_hir::{typed, Type};
use dx_parser::{BinOp, ImportPyDecl};

pub type LocalId = usize;
pub type BlockId = usize;

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    ImportPy(ImportPyDecl),
    Function(Function),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<LocalId>,
    pub locals: Vec<Local>,
    pub blocks: Vec<BasicBlock>,
    pub return_type: Option<Type>,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub synthetic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Assign {
        place: LocalId,
        value: Rvalue,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Return(Option<Operand>),
    Goto(BlockId),
    SwitchBool {
        cond: Operand,
        then_bb: BlockId,
        else_bb: BlockId,
    },
    Match {
        scrutinee: Operand,
        arms: Vec<(dx_hir::Pattern, BlockId)>,
        fallback: BlockId,
    },
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Copy(LocalId),
    Const(Constant),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(String),
    String(String),
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp {
        op: BinOp,
        lhs: Operand,
        rhs: Operand,
    },
    Member {
        base: Operand,
        name: String,
    },
    Call {
        target: typed::CallTarget,
        callee: Operand,
        args: Vec<CallArg>,
        ty: Type,
        effects: Vec<String>,
    },
    Closure {
        captures: Vec<ClosureCapture>,
        param_types: Vec<Type>,
        return_type: Type,
        effects: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureCapture {
    pub name: String,
    pub source: LocalId,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArg {
    Positional(Operand),
    Named { name: String, value: Operand },
}
