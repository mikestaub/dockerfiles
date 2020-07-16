use super::Name;
use crate::common::{FileHandle, Locatable};
use crate::variant;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Operand {
    // relational
    Less,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    Greater,
    NotEqual,
    // arithmetic
    Plus,
    Minus, // TODO add multiplication and division
    // binary
    And,
    Or,
}

impl Operand {
    pub fn is_relational(&self) -> bool {
        match self {
            Self::Less
            | Self::LessOrEqual
            | Self::Equal
            | Self::GreaterOrEqual
            | Self::Greater
            | Self::NotEqual => true,
            _ => false,
        }
    }

    pub fn is_arithmetic(&self) -> bool {
        match self {
            Self::Plus | Self::Minus => true,
            _ => false,
        }
    }

    pub fn is_binary(&self) -> bool {
        match self {
            Self::And | Self::Or => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnaryOperand {
    // Plus,
    Minus,
    Not,
}

pub type ArgumentNodes = Vec<ExpressionNode>;

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    SingleLiteral(f32),
    DoubleLiteral(f64),
    StringLiteral(String),
    IntegerLiteral(i32),
    #[allow(dead_code)]
    LongLiteral(i64),
    VariableName(Name),
    FunctionCall(Name, ArgumentNodes),
    BinaryExpression(Operand, Box<ExpressionNode>, Box<ExpressionNode>),
    UnaryExpression(UnaryOperand, Box<ExpressionNode>),
    Parenthesis(Box<ExpressionNode>),

    /// A file handle is used in built-in subs such as CLOSE #1
    FileHandle(FileHandle),
}

pub type ExpressionNode = Locatable<Expression>;

impl From<f32> for Expression {
    fn from(f: f32) -> Expression {
        Expression::SingleLiteral(f)
    }
}

impl From<f64> for Expression {
    fn from(f: f64) -> Expression {
        Expression::DoubleLiteral(f)
    }
}

impl From<String> for Expression {
    fn from(f: String) -> Expression {
        Expression::StringLiteral(f)
    }
}

impl From<&str> for Expression {
    fn from(f: &str) -> Expression {
        f.to_string().into()
    }
}

impl From<i32> for Expression {
    fn from(f: i32) -> Expression {
        Expression::IntegerLiteral(f)
    }
}

impl From<i64> for Expression {
    fn from(f: i64) -> Expression {
        Expression::LongLiteral(f)
    }
}

impl Expression {
    pub fn unary(operand: UnaryOperand, child: ExpressionNode) -> Self {
        Self::UnaryExpression(operand, Box::new(child))
    }

    pub fn unary_minus(child: ExpressionNode) -> Self {
        match child.as_ref() {
            Self::SingleLiteral(n) => Self::SingleLiteral(-n),
            Self::DoubleLiteral(n) => Self::DoubleLiteral(-n),
            Self::IntegerLiteral(n) => {
                if *n <= variant::MIN_INTEGER {
                    Self::LongLiteral(-n as i64)
                } else {
                    Self::IntegerLiteral(-n)
                }
            }
            Self::LongLiteral(n) => {
                if *n <= variant::MIN_LONG {
                    Self::DoubleLiteral(-n as f64)
                } else {
                    Self::LongLiteral(-n)
                }
            }
            _ => Self::unary(UnaryOperand::Minus, child),
        }
    }
}
