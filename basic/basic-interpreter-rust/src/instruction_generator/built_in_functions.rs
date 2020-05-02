use super::{Instruction, InstructionGenerator, Result};
use crate::common::*;
use crate::parser::{ExpressionNode, Name, NameNode};

pub fn is_built_in_function(function_name: &Name) -> bool {
    function_name == &Name::from("ENVIRON$")
}

impl InstructionGenerator {
    pub fn generate_built_in_function_call_instructions(
        &mut self,
        function_name: NameNode,
        args: Vec<ExpressionNode>,
    ) -> Result<()> {
        // TODO validate arg len for ENVIRON$
        let pos = function_name.location();
        self.generate_push_unnamed_args_instructions(args, pos)?;
        self.push(Instruction::PushStack, pos);
        self.push(
            Instruction::BuiltInFunction(function_name.strip_location()),
            pos,
        );
        Ok(())
    }
}
