use super::{InstructionGenerator, Interpreter, Result, Stdlib};
use crate::common::*;
use crate::parser::IfBlockNode;

impl InstructionGenerator {
    pub fn generate_if_block_instructions(
        &mut self,
        if_block_statement: IfBlockNode,
        pos: Location,
    ) -> Result<()> {
        let IfBlockNode {
            if_block,
            else_if_blocks,
            else_block,
        } = if_block_statement;

        // evaluate condition into A
        self.generate_expression_instructions(if_block.condition)?;

        // if false, jump to next one (first else-if or else or end-if)
        let next_label = if else_if_blocks.len() > 0 {
            "else-if-0"
        } else if else_block.is_some() {
            "else"
        } else {
            "end-if"
        };
        self.jump_if_false(next_label, pos);

        // if true, run statements and jump out
        self.generate_block_instructions(if_block.statements)?;
        self.jump("end-if", pos);

        for i in 0..else_if_blocks.len() {
            let else_if_block = else_if_blocks[i].clone();
            self.label(format!("else-if-{}", i), pos);

            // evaluate condition into A
            self.generate_expression_instructions(else_if_block.condition)?;

            // if false, jump to next one (next else-if or else or end-if)
            let next_label = if i + 1 < else_if_blocks.len() {
                format!("else-if-{}", i + 1)
            } else if else_block.is_some() {
                format!("else")
            } else {
                format!("end-if")
            };
            self.jump_if_false(next_label, pos);

            // if true, run statements and jump out
            self.generate_block_instructions(else_if_block.statements)?;
            self.jump("end-if", pos);
        }

        match else_block {
            Some(e) => {
                self.label("else", pos);
                self.generate_block_instructions(e)?;
            }
            None => (),
        }
        self.label("end-if", pos);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;

    #[test]
    fn test_if_block_true() {
        let input = "
        IF 1 < 2 THEN
            PRINT \"hello\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
    }

    #[test]
    fn test_if_block_false() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, Vec::<String>::new());
    }

    #[test]
    fn test_if_else_block_true() {
        let input = "
        IF 1 < 2 THEN
            PRINT \"hello\"
        ELSE
            PRINT \"bye\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
    }

    #[test]
    fn test_if_else_block_false() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        ELSE
            PRINT \"bye\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["bye"]);
    }

    #[test]
    fn test_if_elseif_block_true_true() {
        let input = "
        IF 1 < 2 THEN
            PRINT \"hello\"
        ELSEIF 1 < 2 THEN
            PRINT \"bye\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
    }

    #[test]
    fn test_if_elseif_block_true_false() {
        let input = "
        IF 1 < 2 THEN
            PRINT \"hello\"
        ELSEIF 2 < 1 THEN
            PRINT \"bye\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
    }

    #[test]
    fn test_if_elseif_block_false_true() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        ELSEIF 1 < 2 THEN
            PRINT \"bye\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["bye"]);
    }

    #[test]
    fn test_if_elseif_block_false_false() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        ELSEIF 2 < 1 THEN
            PRINT \"bye\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, Vec::<String>::new());
    }

    #[test]
    fn test_if_elseif_else_block_true_true() {
        let input = "
        IF 1 < 2 THEN
            PRINT \"hello\"
        ELSEIF 1 < 2 THEN
            PRINT \"bye\"
        ELSE
            PRINT \"else\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
    }

    #[test]
    fn test_if_elseif_else_block_true_false() {
        let input = "
        IF 1 < 2 THEN
            PRINT \"hello\"
        ELSEIF 2 < 1 THEN
            PRINT \"bye\"
        ELSE
            PRINT \"else\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
    }

    #[test]
    fn test_if_elseif_else_block_false_true() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        ELSEIF 1 < 2 THEN
            PRINT \"bye\"
        ELSE
            PRINT \"else\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["bye"]);
    }

    #[test]
    fn test_if_elseif_else_block_false_false() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        ELSEIF 2 < 1 THEN
            PRINT \"bye\"
        ELSE
            PRINT \"else\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["else"]);
    }

    #[test]
    fn test_if_multiple_elseif_block() {
        let input = "
        IF 2 < 1 THEN
            PRINT \"hello\"
        ELSEIF 1 < 2 THEN
            PRINT \"bye\"
        ELSEIF 1 < 2 THEN
            PRINT \"else if 2\"
        END IF
        ";
        assert_eq!(interpret(input).stdlib.output, vec!["bye"]);
    }

    #[test]
    fn test_single_line_if() {
        let input = r#"
        IF 1 THEN PRINT "hello"
        "#;
        assert_eq!(interpret(input).stdlib.output, vec!["hello"]);
        let input = r#"
        IF 0 THEN PRINT "hello"
        "#;
        assert_eq!(interpret(input).stdlib.output.len(), 0);
        let input = r#"
        PRINT "before"
        IF 1 THEN PRINT "hello"
        PRINT "after"
        "#;
        assert_eq!(
            interpret(input).stdlib.output,
            vec!["before", "hello", "after"]
        );
        let input = r#"
        PRINT "before"
        IF 0 THEN PRINT "hello"
        PRINT "after"
        "#;
        assert_eq!(interpret(input).stdlib.output, vec!["before", "after"]);
    }
}
