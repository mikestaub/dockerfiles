mod linter;
pub use self::linter::*;
pub use crate::parser::{
    BareName, BareNameNode, HasQualifier, NameTrait, Operand, QualifiedName, TypeQualifier,
    UnaryOperand,
};
