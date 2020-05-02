use super::{Instruction, InstructionGenerator, Interpreter, Result, Stdlib};
use crate::common::*;
use crate::parser::ConditionalBlockNode;

impl InstructionGenerator {
    pub fn generate_while_instructions(
        &mut self,
        w: ConditionalBlockNode,
        pos: Location,
    ) -> Result<()> {
        let start_idx = self.instructions.len();
        // evaluate condition into register A
        self.generate_expression_instructions(w.condition)?;
        let jump_if_false_idx = self.instructions.len();
        self.push(Instruction::JumpIfFalse(0), pos); // will determine soon
        self.generate_block_instructions(w.statements)?;
        self.push(Instruction::Jump(start_idx), pos);
        let exit_idx = self.instructions.len();
        self.instructions[jump_if_false_idx] = Instruction::JumpIfFalse(exit_idx).at(pos); // patch jump statement with correct index
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;

    #[test]
    fn test_while_wend() {
        let input = "
        A = 1
        WHILE A < 5
            PRINT A
            A = A + 1
        WEND
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["1", "2", "3", "4"]);
    }
}
