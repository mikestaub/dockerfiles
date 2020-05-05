use super::{Instruction, InstructionGenerator, Result};
use crate::common::*;
use crate::linter::{BareNameNode, ExpressionNode};

pub fn is_built_in_sub(sub_name: &CaseInsensitiveString) -> bool {
    sub_name == "ENVIRON" || sub_name == "PRINT" || sub_name == "INPUT" || sub_name == "SYSTEM"
}

impl InstructionGenerator {
    pub fn generate_built_in_sub_call_instructions(
        &mut self,
        name_node: BareNameNode,
        args: Vec<ExpressionNode>,
    ) -> Result<()> {
        let (name, pos) = name_node.consume();
        if &name == "SYSTEM" {
            self.push(Instruction::Halt, pos);
        } else {
            self.generate_push_unnamed_args_instructions(args, pos)?;
            self.push(Instruction::PushStack, pos);
            self.push(Instruction::BuiltInSub(name), pos);
        }
        Ok(())
    }
}
