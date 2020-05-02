use super::{Instruction, InstructionGenerator, Result};
use crate::parser::{ExpressionNode, Name, NameNode};

impl InstructionGenerator {
    pub fn generate_const_instructions(
        &mut self,
        left: NameNode,
        right: ExpressionNode,
    ) -> Result<()> {
        let (name, pos) = left.consume();
        self.generate_const_expression_instructions(right)?;
        match name {
            Name::Bare(bare_name) => {
                self.push(Instruction::StoreConst(bare_name.clone()), pos);
                self.constants.push(bare_name);
            }
            Name::Qualified(qualified_name) => {
                let (bare_name, qualifier) = qualified_name.consume();
                self.push(Instruction::Cast(qualifier), pos);
                self.push(Instruction::StoreConst(bare_name.clone()), pos);
                self.constants.push(bare_name);
            }
        }
        Ok(())
    }
}
