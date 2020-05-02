use super::{Instruction, InstructionGenerator, Result, Variant};
use crate::common::*;
use crate::parser::{ForLoopNode, Name, StatementNodes};

impl InstructionGenerator {
    pub fn generate_for_loop_instructions(&mut self, f: ForLoopNode, pos: Location) -> Result<()> {
        let ForLoopNode {
            variable_name,
            lower_bound,
            upper_bound,
            step,
            statements,
            next_counter: _,
        } = f;
        let counter_var_name: Name = variable_name.strip_location();

        // lower bound to A
        self.generate_expression_instructions(lower_bound)?;
        // A to variable
        self.push(Instruction::Store(counter_var_name.clone()), pos);
        // upper bound to A
        self.generate_expression_instructions(upper_bound)?;
        // A to hidden variable
        self.store_temp_var("upper-bound", pos); // TODO dispose temp vars later

        // load the step expression
        match step {
            Some(s) => {
                let step_location = s.location();
                // load 0 to B
                self.push(Instruction::Load(Variant::VInteger(0)), pos);
                self.push(Instruction::CopyAToB, pos);
                // load step to A
                self.generate_expression_instructions(s)?;
                self.store_temp_var("step", pos);
                // is step < 0 ?
                self.push(Instruction::LessThan, pos);
                self.jump_if_false("test-positive-or-zero", pos);
                // negative step
                self.generate_for_loop_instructions_positive_or_negative_step(
                    counter_var_name.clone(),
                    statements.clone(),
                    false,
                    pos,
                )?;
                // jump out
                self.jump("out-of-for", pos);
                // PositiveOrZero: ?
                self.label("test-positive-or-zero", pos);
                // need to load it again into A because the previous "LessThan" op overwrote A
                self.copy_temp_var_to_a("step", pos);
                // is step > 0 ?
                self.push(Instruction::GreaterThan, pos);
                self.jump_if_false("zero", pos);
                // positive step
                self.generate_for_loop_instructions_positive_or_negative_step(
                    counter_var_name,
                    statements,
                    true,
                    pos,
                )?;
                // jump out
                self.jump("out-of-for", pos);
                // Zero step
                self.label("zero", pos);
                self.push(
                    Instruction::Throw(format!("Step cannot be zero")),
                    step_location,
                );
                self.label("out-of-for", pos);
                Ok(())
            }
            None => {
                self.push(Instruction::Load(Variant::VInteger(1)), pos);
                self.store_temp_var("step", pos);
                self.generate_for_loop_instructions_positive_or_negative_step(
                    counter_var_name,
                    statements,
                    true,
                    pos,
                )?;
                self.label("out-of-for", pos);
                Ok(())
            }
        }
    }

    fn generate_for_loop_instructions_positive_or_negative_step(
        &mut self,
        counter_var_name: Name,
        statements: StatementNodes,
        is_positive: bool,
        pos: Location,
    ) -> Result<()> {
        let loop_label = if is_positive {
            "positive-loop"
        } else {
            "negative-loop"
        };
        // loop point
        self.label(loop_label, pos);
        // upper bound to B
        self.copy_temp_var_to_b("upper-bound", pos);
        // counter to A
        self.push(Instruction::CopyVarToA(counter_var_name.clone()), pos);
        if is_positive {
            self.push(Instruction::LessOrEqualThan, pos);
        } else {
            self.push(Instruction::GreaterOrEqualThan, pos);
        }
        self.jump_if_false("out-of-for", pos);
        self.generate_block_instructions(statements)?;

        // increment step
        self.push(Instruction::CopyVarToA(counter_var_name.clone()), pos);
        self.copy_temp_var_to_b("step", pos);
        self.push(Instruction::Plus, pos);
        self.push(Instruction::Store(counter_var_name), pos);

        // back to loop
        self.jump(loop_label, pos);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use super::*;
    use crate::assert_has_variable;
    use crate::assert_pre_process_err;
    use crate::common::Location;
    use crate::interpreter::InterpreterError;

    #[test]
    fn test_simple_for_loop_untyped() {
        let input = "
        FOR I = 1 TO 5
            PRINT I
        NEXT
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_simple_for_loop_typed() {
        let input = "
        FOR i% = 1 TO 5
            PRINT i%
        NEXT
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_simple_for_loop_lowercase() {
        let input = "
        FOR i% = 1 TO 5
            PRINT I%
        NEXT
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_simple_for_loop_value_of_variable_after_loop() {
        let input = "
        FOR i% = 1 TO 5
            PRINT i%
        NEXT
        ";
        let interpreter = interpret(input);
        assert_has_variable!(interpreter, "i%", 6);
    }

    #[test]
    fn test_simple_for_loop_value_of_variable_after_loop_never_entering() {
        let input = "
        FOR i% = 1 TO -1
            PRINT i%
        NEXT
        ";
        let interpreter = interpret(input);
        assert_has_variable!(interpreter, "i%", 1);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, Vec::<String>::new());
    }

    #[test]
    fn test_for_loop_with_positive_step() {
        let input = "
        FOR i% = 1 TO 7 STEP 2
            PRINT i%
        NEXT
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "3", "5", "7"]);
    }

    #[test]
    fn test_for_loop_with_negative_step() {
        let input = "
        FOR i% = 7 TO -6 STEP -3
            PRINT i%
        NEXT
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["7", "4", "1", "-2", "-5"]);
    }

    #[test]
    fn test_for_loop_with_zero_step() {
        let input = "
        FOR i% = 7 TO -6 STEP 0
            PRINT i%
        NEXT
        ";
        assert_eq!(
            interpret_err(input),
            InterpreterError::new_with_pos("Step cannot be zero", Location::new(2, 31))
        );
    }

    #[test]
    fn test_for_loop_with_negative_step_minus_one() {
        let input = "
        FOR i% = 3 TO -3 STEP -1
            PRINT i%
        NEXT
        ";
        let interpreter = interpret(input);
        assert_has_variable!(interpreter, "i%", -4);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["3", "2", "1", "0", "-1", "-2", "-3"]);
    }

    #[test]
    fn test_for_loop_with_specified_next_counter() {
        let input = "
        FOR i% = 1 TO 5
            PRINT i%
        NEXT i%
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_for_loop_with_specified_next_counter_lower_case() {
        let input = "
        FOR i% = 1 TO 5
            PRINT i%
        NEXT I%
        ";
        let interpreter = interpret(input);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "2", "3", "4", "5"]);
    }

    #[test]
    fn test_for_loop_with_wrong_next_counter() {
        let input = "
        FOR i% = 1 TO 5
            PRINT i%
        NEXT i
        ";
        assert_pre_process_err!(input, "NEXT without FOR", 4, 14);
    }

    #[test]
    fn test_for_loop_end_expression_evaluated_only_once() {
        let input = "
        N% = 3
        FOR I% = 1 TO N%
            PRINT I%
            N% = N% - 1
        NEXT
        ";
        let interpreter = interpret(input);
        assert_has_variable!(interpreter, "I%", 4);
        assert_has_variable!(interpreter, "N%", 0);
        let stdlib = interpreter.stdlib;
        assert_eq!(stdlib.output, vec!["1", "2", "3"]);
    }
}
