use dx_mir::mir;
use dx_runtime::{RuntimeExternAbiType, ThrowBoundaryKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowModule {
    pub externs: Vec<LowExtern>,
    pub functions: Vec<LowFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowExtern {
    pub symbol: &'static str,
    pub params: Vec<LowType>,
    pub ret: LowType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowFunction {
    pub name: String,
    pub params: Vec<LowParam>,
    pub ret: LowType,
    pub blocks: Vec<LowBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowParam {
    pub local: mir::LocalId,
    pub ty: LowType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowBlock {
    pub label: String,
    pub steps: Vec<LowStep>,
    pub terminator: LowTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowStep {
    RuntimeCall {
        statement: usize,
        destination: Option<mir::LocalId>,
        symbol: &'static str,
        ret: Option<LowType>,
        kind: LowRuntimeCallKind,
    },
    ThrowCheck {
        statement: usize,
        symbol: &'static str,
        boundary: ThrowBoundaryKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowValue {
    Local(mir::LocalId, LowType),
    ConstInt(i64),
    ConstString(String),
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowTerminator {
    Return(Option<LowValue>),
    Goto(String),
    SwitchBool {
        cond: LowValue,
        then_label: String,
        else_label: String,
    },
    Match {
        scrutinee: LowValue,
        arms: Vec<(String, String)>,
        fallback: String,
    },
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowRuntimeCallKind {
    PyCall {
        arg_count: u32,
    },
    ClosureCreate {
        captures: Vec<LowValue>,
        arity: usize,
    },
    ClosureInvoke {
        closure: Box<LowValue>,
        arg_count: u32,
        thunk: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowType {
    I64,
    F64,
    I1,
    Ptr,
    Void,
}

impl LowType {
    pub fn from_runtime_abi(ty: RuntimeExternAbiType) -> Self {
        match ty {
            RuntimeExternAbiType::PyObjHandle
            | RuntimeExternAbiType::Utf8Ptr
            | RuntimeExternAbiType::ClosureHandle
            | RuntimeExternAbiType::EnvHandle
            | RuntimeExternAbiType::Ptr => LowType::Ptr,
            RuntimeExternAbiType::I64 => LowType::I64,
            RuntimeExternAbiType::F64 => LowType::F64,
            RuntimeExternAbiType::I1 => LowType::I1,
            RuntimeExternAbiType::U32 => LowType::I64,
            RuntimeExternAbiType::Void => LowType::Void,
        }
    }
}
