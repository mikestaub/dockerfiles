use super::{BareNameNode, ExpressionNode, NameNode};
use crate::common::{CaseInsensitiveString, Location};

pub type BlockNode = Vec<StatementNode>;

#[derive(Clone, Debug, PartialEq)]
pub enum StatementNode {
    SubCall(BareNameNode, Vec<ExpressionNode>),
    ForLoop(ForLoopNode),
    IfBlock(IfBlockNode),
    Assignment(NameNode, ExpressionNode),
    While(ConditionalBlockNode),
    Const(NameNode, ExpressionNode, Location),
    ErrorHandler(CaseInsensitiveString, Location),
    Label(CaseInsensitiveString, Location),
    GoTo(CaseInsensitiveString, Location),
    // TODO remove out of here
    InternalSetReturnValue(ExpressionNode),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForLoopNode {
    pub variable_name: NameNode,
    pub lower_bound: ExpressionNode,
    pub upper_bound: ExpressionNode,
    pub step: Option<ExpressionNode>,
    pub statements: BlockNode,
    pub next_counter: Option<NameNode>,
    pub pos: Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalBlockNode {
    pub pos: Location,
    pub condition: ExpressionNode,
    pub statements: BlockNode,
}

impl ConditionalBlockNode {
    pub fn new(
        pos: Location,
        condition: ExpressionNode,
        statements: BlockNode,
    ) -> ConditionalBlockNode {
        ConditionalBlockNode {
            pos,
            condition,
            statements,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfBlockNode {
    pub if_block: ConditionalBlockNode,
    pub else_if_blocks: Vec<ConditionalBlockNode>,
    pub else_block: Option<BlockNode>,
}