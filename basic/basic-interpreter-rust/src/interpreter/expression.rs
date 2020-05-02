use super::{
    Instruction, InstructionGenerator, Interpreter, InterpreterError, Result, Stdlib, Variant,
};
use crate::common::*;
use crate::parser::*;

impl InstructionGenerator {
    pub fn generate_expression_instructions(&mut self, e: ExpressionNode) -> Result<()> {
        self.do_generate_expression_instructions(e, false)
    }

    pub fn generate_const_expression_instructions(&mut self, e: ExpressionNode) -> Result<()> {
        self.do_generate_expression_instructions(e, true)
    }

    fn do_generate_expression_instructions(
        &mut self,
        e_node: ExpressionNode,
        only_const: bool,
    ) -> Result<()> {
        let (e, pos) = e_node.consume();
        match e {
            Expression::SingleLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::DoubleLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::StringLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::IntegerLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::LongLiteral(s) => {
                self.push(Instruction::Load(Variant::from(s)), pos);
                Ok(())
            }
            Expression::VariableName(name) => {
                if !only_const || self.constants.contains(name.bare_name()) {
                    self.push(Instruction::CopyVarToA(name), pos);
                    Ok(())
                } else {
                    Err(InterpreterError::new_with_pos("Invalid constant", pos))
                }
            }
            Expression::FunctionCall(n, args) => {
                if only_const {
                    Err(InterpreterError::new_with_pos("Invalid constant", pos))
                } else {
                    let name_node = n.at(pos);
                    self.generate_function_call_instructions(name_node, args)?;
                    Ok(())
                }
            }
            Expression::BinaryExpression(op, left, right) => {
                self.push(Instruction::PushRegisters, pos);
                // TODO this implies right to left evaluation, double check with QBasic reference implementation
                self.do_generate_expression_instructions(*right, only_const)?;
                self.push(Instruction::CopyAToB, pos);
                self.do_generate_expression_instructions(*left, only_const)?;
                match op {
                    Operand::Plus => self.push(Instruction::Plus, pos),
                    Operand::Minus => self.push(Instruction::Minus, pos),
                    Operand::LessThan => self.push(Instruction::LessThan, pos),
                    Operand::LessOrEqualThan => self.push(Instruction::LessOrEqualThan, pos),
                }
                self.push(Instruction::PopRegisters, pos);
                Ok(())
            }
            Expression::UnaryExpression(op, child) => {
                match op {
                    UnaryOperand::Not => {
                        self.do_generate_expression_instructions(*child, only_const)?;
                        self.push(Instruction::NotA, pos);
                    }
                    UnaryOperand::Minus => {
                        self.do_generate_expression_instructions(*child, only_const)?;
                        self.push(Instruction::NegateA, pos);
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_has_variable;
    use crate::common::Location;
    use crate::interpreter::test_utils::*;

    #[test]
    fn test_literals() {
        assert_has_variable!(interpret("X = 3.14"), "X", 3.14_f32);
        assert_has_variable!(interpret("X# = 3.14"), "X#", 3.14);
        assert_has_variable!(interpret("X$ = \"hello\""), "X$", "hello");
        assert_has_variable!(interpret("X% = 42"), "X%", 42);
        assert_has_variable!(interpret("X& = 42"), "X&", 42_i64);
    }

    mod binary_plus {
        use super::*;

        #[test]
        fn test_left_float() {
            assert_has_variable!(interpret("X = 1.1 + 2.1"), "X", 3.2_f32);
            assert_has_variable!(interpret("X = 1.1 + 2.1#"), "X", 3.2_f32);
            assert_has_variable!(interpret("X = 1.1 + 2"), "X", 3.1_f32);
            assert_eq!(
                interpret_err("X = 1.1 + \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 9))
            );
        }

        #[test]
        fn test_left_double() {
            assert_has_variable!(interpret("X# = 1.1# + 2.1"), "X#", 3.2_f64);
            assert_has_variable!(interpret("X# = 1.1 + 2.1#"), "X#", 3.2_f64);
            assert_has_variable!(interpret("X# = 1.1# + 2"), "X#", 3.1_f64);
            assert_eq!(
                interpret_err("X = 1.1# + \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 10))
            );
        }

        #[test]
        fn test_left_string() {
            assert_has_variable!(interpret(r#"X$ = "hello" + " hi""#), "X$", "hello hi");
            assert_eq!(
                interpret_err("X$ = \"hello\" + 1"),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
            assert_eq!(
                interpret_err("X$ = \"hello\" + 1.1"),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
            assert_eq!(
                interpret_err("X$ = \"hello\" + 1.1#"),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
        }

        #[test]
        fn test_left_integer() {
            assert_has_variable!(interpret("X% = 1 + 2.1"), "X%", 3);
            assert_has_variable!(interpret("X% = 1 + 2.5"), "X%", 4);
            assert_has_variable!(interpret("X% = 1 + 2.1#"), "X%", 3);
            assert_has_variable!(interpret("X% = 1 + 2"), "X%", 3);
            assert_eq!(
                interpret_err("X% = 1 + \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 8))
            );
        }

        #[test]
        fn test_left_long() {
            assert_has_variable!(interpret("X& = 1 + 2.1"), "X&", 3_i64);
            assert_has_variable!(interpret("X& = 1 + 2.5"), "X&", 4_i64);
            assert_has_variable!(interpret("X& = 1 + 2.1#"), "X&", 3_i64);
            assert_has_variable!(interpret("X& = 1 + 2"), "X&", 3_i64);
            assert_eq!(
                interpret_err("X& = 1 + \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 8))
            );
        }

        #[test]
        fn test_function_call_plus_literal() {
            let program = r#"
            DECLARE FUNCTION Sum(A, B)

            PRINT Sum(1, 2) + 1

            FUNCTION Sum(A, B)
                Sum = A + B
            END FUNCTION
            "#;
            let interpreter = interpret(program);
            assert_eq!(interpreter.stdlib.output, vec!["4"]);
        }

        #[test]
        fn test_literal_plus_function_call() {
            let program = r#"
            DECLARE FUNCTION Sum(A, B)

            PRINT 1 + Sum(1, 2)

            FUNCTION Sum(A, B)
                Sum = A + B
            END FUNCTION
            "#;
            let interpreter = interpret(program);
            assert_eq!(interpreter.stdlib.output, vec!["4"]);
        }
    }

    mod binary_minus {
        use super::*;

        #[test]
        fn test_left_float() {
            assert_has_variable!(interpret("X = 5.4 - 2.1"), "X", 3.3_f32);
            assert_has_variable!(interpret("X = 5.4 - 2.1#"), "X", 3.3_f32);
            assert_has_variable!(interpret("X = 5.1 - 2"), "X", 3.1_f32);
            assert_eq!(
                interpret_err("X = 1.1 - \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 9))
            );
        }

        #[test]
        fn test_left_double() {
            assert_has_variable!(interpret("X# = 5.4# - 2.1"), "X#", 3.3_f64);
            assert_has_variable!(interpret("X# = 5.4 - 2.1#"), "X#", 3.3_f64);
            assert_has_variable!(interpret("X# = 5.1# - 2"), "X#", 3.1_f64);
            assert_eq!(
                interpret_err("X = 1.1# - \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 10))
            );
        }

        #[test]
        fn test_left_string() {
            assert_eq!(
                interpret_err("X$ = \"hello\" - \"hi\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
            assert_eq!(
                interpret_err("X$ = \"hello\" - 1"),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
            assert_eq!(
                interpret_err("X$ = \"hello\" - 1.1"),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
            assert_eq!(
                interpret_err("X$ = \"hello\" - 1.1#"),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 14))
            );
        }

        #[test]
        fn test_left_integer() {
            assert_has_variable!(interpret("X% = 5 - 2.1"), "X%", 3);
            assert_has_variable!(interpret("X% = 6 - 2.5"), "X%", 4);
            assert_has_variable!(interpret("X% = 5 - 2.1#"), "X%", 3);
            assert_has_variable!(interpret("X% = 5 - 2"), "X%", 3);
            assert_eq!(
                interpret_err("X% = 1 - \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 8))
            );
        }

        #[test]
        fn test_left_long() {
            assert_has_variable!(interpret("X& = 5 - 2.1"), "X&", 3_i64);
            assert_has_variable!(interpret("X& = 6 - 2.5"), "X&", 4_i64);
            assert_has_variable!(interpret("X& = 5 - 2.1#"), "X&", 3_i64);
            assert_has_variable!(interpret("X& = 5 - 2"), "X&", 3_i64);
            assert_eq!(
                interpret_err("X& = 1 - \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 8))
            );
        }
    }

    mod unary_minus {
        use super::*;

        #[test]
        fn test_unary_minus_float() {
            assert_has_variable!(interpret("X = -1.1"), "X", -1.1_f32);
            assert_has_variable!(interpret("X = -1.1#"), "X", -1.1_f32);
            assert_has_variable!(interpret("X = -1"), "X", -1.0_f32);
            assert_eq!(
                interpret_err("X = -\"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 5))
            );
        }

        #[test]
        fn test_unary_minus_integer() {
            assert_has_variable!(interpret("X% = -1.1"), "X%", -1);
            assert_has_variable!(interpret("X% = -1.1#"), "X%", -1);
            assert_has_variable!(interpret("X% = -1"), "X%", -1);
            assert_eq!(
                interpret_err("X% = -\"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 6))
            );
        }
    }

    mod unary_not {
        use super::*;

        #[test]
        fn test_unary_not_float() {
            assert_has_variable!(interpret("X = NOT 3.14"), "X", -4.0_f32);
            assert_has_variable!(interpret("X = NOT 3.5#"), "X", -5.0_f32);
            assert_has_variable!(interpret("X = NOT -1.1"), "X", 0.0_f32);
            assert_has_variable!(interpret("X = NOT -1.5"), "X", 1.0_f32);
            assert_eq!(
                interpret_err("X = NOT \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 5))
            );
        }

        #[test]
        fn test_unary_not_integer() {
            assert_has_variable!(interpret("X% = NOT 1"), "X%", -2);
            assert_has_variable!(interpret("X% = NOT 0"), "X%", -1);
            assert_has_variable!(interpret("X% = NOT -1"), "X%", 0);
            assert_has_variable!(interpret("X% = NOT -2"), "X%", 1);
            assert_eq!(
                interpret_err("X% = NOT \"hello\""),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 6))
            );
        }
    }

    macro_rules! assert_condition {
        ($condition:expr) => {
            let program = format!(
                "
            IF {} THEN
            ELSE
                PRINT \"hi\"
            END IF
            ",
                $condition
            );
            if interpret(program).stdlib.output.len() > 0 {
                panic!(format!(
                    "Expected condition to be true but was false: {}",
                    $condition
                ))
            }
        };
    }

    macro_rules! assert_condition_false {
        ($condition:expr) => {
            let program = format!(
                "
            IF {} THEN
                PRINT \"hi\"
            END IF
            ",
                $condition
            );
            if interpret(program).stdlib.output.len() > 0 {
                panic!(format!(
                    "Expected condition to be false but was true: {}",
                    $condition
                ))
            }
        };
    }

    macro_rules! assert_condition_err {
        ($condition:expr) => {
            let program = format!(
                "
            IF {} THEN
                PRINT \"hi\"
            END IF
            ",
                $condition
            );
            let e = interpret_err(program);
            assert_eq!("Type mismatch", e.message());
        };
    }

    mod less {
        use super::*;

        #[test]
        fn test_left_float() {
            assert_condition_false!("9.1 < 2.1");
            assert_condition_false!("9.1 < 9.1");
            assert_condition!("9.1 < 19.1");

            assert_condition_false!("9.1 < 2");
            assert_condition_false!("9.1 < 9");
            assert_condition!("9.1 < 19");

            assert_condition_err!("9.1 < \"hello\"");

            assert_condition_false!("9.1 < 2.1#");
            assert_condition_false!("9.1 < 9.1#");
            assert_condition!("9.1 < 19.1#");
        }

        #[test]
        fn test_left_double() {
            assert_condition_false!("9.1# < 2.1");
            assert_condition_false!("9.1# < 9.1");
            assert_condition!("9.1# < 19.1");

            assert_condition_false!("9.1# < 2");
            assert_condition_false!("9.1# < 9");
            assert_condition!("9.1# < 19");

            assert_condition_err!("9.1# < \"hello\"");

            assert_condition_false!("9.1# < 2.1#");
            assert_condition_false!("9.1# < 9.1#");
            assert_condition!("9.1# < 19.1#");
        }

        #[test]
        fn test_left_string() {
            assert_condition_err!("\"hello\" < 3.14");
            assert_condition_err!("\"hello\" < 3");
            assert_condition_err!("\"hello\" < 3.14#");

            assert_condition_false!("\"def\" < \"abc\"");
            assert_condition_false!("\"def\" < \"def\"");
            assert_condition!("\"def\" < \"xyz\"");
        }

        #[test]
        fn test_left_integer() {
            assert_condition_false!("9 < 2.1");
            assert_condition_false!("9 < 8.9");
            assert_condition_false!("9 < 9.0");
            assert_condition!("9 < 9.1");
            assert_condition!("9 < 19.1");

            assert_condition_false!("9 < 2");
            assert_condition_false!("9 < 9");
            assert_condition!("9 < 19");

            assert_condition_err!("9 < \"hello\"");

            assert_condition_false!("9 < 2.1#");
            assert_condition!("9 < 9.1#");
            assert_condition!("9 < 19.1#");
        }
    }

    mod lte {
        use super::*;

        #[test]
        fn test_left_float() {
            assert_condition_false!("9.1 <= 2.1");
            assert_condition!("9.1 <= 9.1");
            assert_condition!("9.1 <= 19.1");

            assert_condition_false!("9.1 <= 2");
            assert_condition_false!("9.1 <= 9");
            assert_condition!("9.1 <= 19");

            assert_condition_err!("9.1 <= \"hello\"");

            assert_condition_false!("9.1 <= 2.1#");
            assert_condition!("9.1 <= 9.1#");
            assert_condition!("9.1 <= 19.1#");
        }

        #[test]
        fn test_left_double() {
            assert_condition_false!("9.1# <= 2.1");
            assert_condition!("9.1# <= 9.1");
            assert_condition!("9.1# <= 19.1");

            assert_condition_false!("9.1# <= 2");
            assert_condition_false!("9.1# <= 9");
            assert_condition!("9.1# <= 19");

            assert_condition_err!("9.1# <= \"hello\"");

            assert_condition_false!("9.1# <= 2.1#");
            assert_condition!("9.1# <= 9.1#");
            assert_condition!("9.1# <= 19.1#");
        }

        #[test]
        fn test_left_string() {
            assert_condition_err!("\"hello\" <= 3.14");
            assert_condition_err!("\"hello\" <= 3");
            assert_condition_err!("\"hello\" <= 3.14#");

            assert_condition_false!("\"def\" <= \"abc\"");
            assert_condition!("\"def\" <= \"def\"");
            assert_condition!("\"def\" <= \"xyz\"");
        }

        #[test]
        fn test_left_integer() {
            assert_condition_false!("9 <= 2.1");
            assert_condition_false!("9 <= 8.9");
            assert_condition!("9 <= 9.0");
            assert_condition!("9 <= 9.1");
            assert_condition!("9 <= 19.1");

            assert_condition_false!("9 <= 2");
            assert_condition!("9 <= 9");
            assert_condition!("9 <= 19");

            assert_condition_err!("9 <= \"hello\"");

            assert_condition_false!("9 <= 2.1#");
            assert_condition!("9 <= 9.1#");
            assert_condition!("9 <= 19.1#");
        }
    }
}
