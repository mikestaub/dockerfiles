use super::{Instruction, InstructionContext, Interpreter, Result, Stdlib, Variant};
use crate::common::*;
use crate::parser::{ForLoopNode, Name, StatementNodes};

impl<S: Stdlib> Interpreter<S> {
    pub fn generate_for_loop_instructions(
        &self,
        result: &mut InstructionContext,
        f: ForLoopNode,
        pos: Location,
    ) -> Result<()> {
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
        self.generate_expression_instructions(result, lower_bound)?;
        // A to variable
        result
            .instructions
            .push(Instruction::Store(counter_var_name.clone()).at(pos));
        // upper bound to A
        self.generate_expression_instructions(result, upper_bound)?;
        // A to hidden variable
        result.store_temp_var("upper-bound", pos); // TODO dispose temp vars later

        // load the step expression
        match step {
            Some(s) => {
                let step_location = s.location();
                // load 0 to B
                result
                    .instructions
                    .push(Instruction::Load(Variant::VInteger(0)).at(pos));
                result.instructions.push(Instruction::CopyAToB.at(pos));
                // load step to A
                self.generate_expression_instructions(result, s)?;
                result.store_temp_var("step", pos);
                // is step < 0 ?
                result.instructions.push(Instruction::LessThan.at(pos));
                result.jump_if_false("test-positive-or-zero", pos);
                // negative step
                self.generate_for_loop_instructions_positive_or_negative_step(
                    result,
                    counter_var_name.clone(),
                    statements.clone(),
                    false,
                    pos,
                )?;
                // jump out
                result.jump("out-of-for", pos);
                // PositiveOrZero: ?
                result.label("test-positive-or-zero", pos);
                // need to load it again into A because the previous "LessThan" op overwrote A
                result.copy_temp_var_to_a("step", pos);
                // is step > 0 ?
                result.instructions.push(Instruction::GreaterThan.at(pos));
                result.jump_if_false("zero", pos);
                // positive step
                self.generate_for_loop_instructions_positive_or_negative_step(
                    result,
                    counter_var_name,
                    statements,
                    true,
                    pos,
                )?;
                // jump out
                result.jump("out-of-for", pos);
                // Zero step
                result.label("zero", pos);
                result
                    .instructions
                    .push(Instruction::Throw(format!("Step cannot be zero")).at(step_location));
                result.label("out-of-for", pos);
                Ok(())
            }
            None => {
                result
                    .instructions
                    .push(Instruction::Load(Variant::VInteger(1)).at(pos));
                result.store_temp_var("step", pos);
                self.generate_for_loop_instructions_positive_or_negative_step(
                    result,
                    counter_var_name,
                    statements,
                    true,
                    pos,
                )?;
                result.label("out-of-for", pos);
                Ok(())
            }
        }
    }

    fn generate_for_loop_instructions_positive_or_negative_step(
        &self,
        result: &mut InstructionContext,
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
        result.label(loop_label, pos);
        // upper bound to B
        result.copy_temp_var_to_b("upper-bound", pos);
        // counter to A
        result
            .instructions
            .push(Instruction::CopyVarToA(counter_var_name.clone()).at(pos));
        if is_positive {
            result
                .instructions
                .push(Instruction::LessOrEqualThan.at(pos));
        } else {
            result
                .instructions
                .push(Instruction::GreaterOrEqualThan.at(pos));
        }
        result.jump_if_false("out-of-for", pos);
        self.generate_block_instructions(result, statements)?;

        // increment step
        result
            .instructions
            .push(Instruction::CopyVarToA(counter_var_name.clone()).at(pos));
        result.copy_temp_var_to_b("step", pos);
        result.instructions.push(Instruction::Plus.at(pos));
        result
            .instructions
            .push(Instruction::Store(counter_var_name).at(pos));

        // back to loop
        result.jump(loop_label, pos);
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
