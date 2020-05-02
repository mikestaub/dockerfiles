use super::{Instruction, InstructionContext, Interpreter, Result, Stdlib};
use crate::common::*;
use crate::parser::{Statement, StatementNode, StatementNodes};

impl<S: Stdlib> Interpreter<S> {
    pub fn generate_block_instructions(
        &self,
        result: &mut InstructionContext,
        block: StatementNodes,
    ) -> Result<()> {
        for s in block {
            self.generate_statement_node_instructions(result, s)?;
        }
        Ok(())
    }

    pub fn generate_statement_node_instructions(
        &self,
        result: &mut InstructionContext,
        statement_node: StatementNode,
    ) -> Result<()> {
        let (statement, pos) = statement_node.consume();
        match statement {
            Statement::Assignment(left_side, right_side) => {
                self.generate_assignment_instructions(result, left_side.at(pos), right_side)
            }
            Statement::SubCall(n, args) => {
                self.generate_sub_call_instructions(result, n.at(pos), args)
            }
            Statement::ForLoop(f) => self.generate_for_loop_instructions(result, f, pos),
            Statement::IfBlock(i) => self.generate_if_block_instructions(result, i, pos),
            Statement::While(w) => self.generate_while_instructions(result, w, pos),
            Statement::Const(n, e) => self.generate_const_instructions(result, n, e),
            Statement::ErrorHandler(label) => {
                result
                    .instructions
                    .push(Instruction::SetUnresolvedErrorHandler(label).at(pos));
                Ok(())
            }
            Statement::Label(name) => {
                result
                    .instructions
                    .push(Instruction::Label(name.clone()).at(pos));
                Ok(())
            }
            Statement::GoTo(name) => {
                result
                    .instructions
                    .push(Instruction::UnresolvedJump(name.clone()).at(pos));
                Ok(())
            }
            Statement::InternalSetReturnValue(e) => {
                self.generate_expression_instructions(result, e)?;
                result
                    .instructions
                    .push(Instruction::StoreAToResult.at(pos));
                Ok(())
            }
        }
    }
}
