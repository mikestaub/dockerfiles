use super::{Instruction, InstructionContext, Interpreter, Result, Stdlib};
use crate::common::*;
use crate::parser::ConditionalBlockNode;

impl<S: Stdlib> Interpreter<S> {
    pub fn generate_while_instructions(
        &self,
        result: &mut InstructionContext,
        w: ConditionalBlockNode,
        pos: Location,
    ) -> Result<()> {
        let start_idx = result.instructions.len();
        // evaluate condition into register A
        self.generate_expression_instructions(result, w.condition)?;
        let jump_if_false_idx = result.instructions.len();
        result
            .instructions
            .push(Instruction::JumpIfFalse(0).at(pos)); // will determine soon
        self.generate_block_instructions(result, w.statements)?;
        result
            .instructions
            .push(Instruction::Jump(start_idx).at(pos));
        let exit_idx = result.instructions.len();
        result.instructions[jump_if_false_idx] = Instruction::JumpIfFalse(exit_idx).at(pos); // patch jump statement with correct index
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
