use super::{Instruction, InstructionContext, Interpreter, Result, Stdlib};
use crate::common::*;
use crate::parser::{BlockNode, StatementNode};

impl<S: Stdlib> Interpreter<S> {
    pub fn generate_block_instructions(
        &self,
        result: &mut InstructionContext,
        block: BlockNode,
    ) -> Result<()> {
        for s in block {
            self.generate_statement_instructions(result, s)?;
        }
        Ok(())
    }

    pub fn generate_statement_instructions(
        &self,
        result: &mut InstructionContext,
        statement: StatementNode,
    ) -> Result<()> {
        match statement {
            StatementNode::Assignment(left_side, right_side) => {
                self.generate_assignment_instructions(result, left_side, right_side)
            }
            StatementNode::SubCall(n, args) => self.generate_sub_call_instructions(result, n, args),
            StatementNode::ForLoop(f) => self.generate_for_loop_instructions(result, f),
            StatementNode::IfBlock(i) => self.generate_if_block_instructions(result, i),
            StatementNode::While(w) => self.generate_while_instructions(result, w),
            StatementNode::Const(n, e, _) => self.generate_const_instructions(result, n, e),
            StatementNode::ErrorHandler(label, pos) => {
                result
                    .instructions
                    .push(Instruction::SetUnresolvedErrorHandler(label).at(pos));
                Ok(())
            }
            StatementNode::Label(name, pos) => {
                result
                    .instructions
                    .push(Instruction::Label(name.clone()).at(pos));
                Ok(())
            }
            StatementNode::GoTo(name, pos) => {
                result
                    .instructions
                    .push(Instruction::UnresolvedJump(name.clone()).at(pos));
                Ok(())
            }
            StatementNode::InternalSetReturnValue(e) => {
                let pos = e.location();
                self.generate_expression_instructions(result, e)?;
                result
                    .instructions
                    .push(Instruction::StoreAToResult.at(pos));
                Ok(())
            }
        }
    }
}
