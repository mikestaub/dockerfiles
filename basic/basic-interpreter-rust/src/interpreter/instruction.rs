use crate::common::*;
use crate::interpreter::Variant;
use crate::parser::{DefType, Name, QualifiedName, TypeQualifier};

#[derive(Debug, PartialEq)]
pub enum Instruction {
    /// Loads a value into register A
    Load(Variant),
    /// Stores a value from register A
    Store(Name),
    /// Stores a value from register A into a constant
    StoreConst(CaseInsensitiveString),
    /// Casts register A to the desired type
    Cast(TypeQualifier),
    CopyAToB,
    /// Adds registers A and B and stores the results into register A
    Plus,
    Minus,
    EqualTo,
    LessOrEqualThan,
    LessThan,
    GreaterThan,
    GreaterOrEqualThan,
    NegateA,
    NotA,
    Jump(usize),
    JumpIfFalse(usize),
    Label(CaseInsensitiveString),
    UnresolvedJump(CaseInsensitiveString),
    UnresolvedJumpIfFalse(CaseInsensitiveString),
    CopyVarToA(Name),
    CopyVarToB(Name),
    BuiltInSub(CaseInsensitiveString),
    BuiltInFunction(Name),
    DefType(DefType),
    Halt,

    PushRegisters,
    PopRegisters,

    PushRet(usize),
    PopRet,

    PreparePush,
    PushStack,
    PopStack,

    PushUnnamedRefParam(Name),

    /// Pushes the contents of register A at the end of the unnamed stack
    PushUnnamedValParam,

    SetNamedRefParam(QualifiedName, Name),
    SetNamedValParam(QualifiedName),

    Throw(String),

    /// Stores A as the result of a function
    StoreAToResult,
    /// Copies the result of a function to A
    CopyResultToA,

    SetUnresolvedErrorHandler(CaseInsensitiveString),
    SetErrorHandler(usize),
}

pub type InstructionNode = Locatable<Instruction>;
