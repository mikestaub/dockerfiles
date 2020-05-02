#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use crate::assert_has_variable;
    use crate::assert_pre_process_err;
    use crate::common::*;
    use crate::interpreter::InterpreterError;
    use crate::variant::Variant;

    #[test]
    fn test_function_call_declared_and_implemented() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        X = Add(1, 2)
        FUNCTION Add(A, B)
            Add = A + B
        END FUNCTION
        ";
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", 3.0_f32);
    }

    #[test]
    fn test_function_call_without_implementation() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        X = Add(1, 2)
        ";
        assert_pre_process_err!(program, "Subprogram not defined", 2, 9);
    }

    #[test]
    fn test_function_call_without_declaration() {
        let program = "
        X = Add(1, 2)
        FUNCTION Add(A, B)
            Add = A + B
        END FUNCTION
        ";
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", 3.0_f32);
    }

    #[test]
    fn test_function_call_not_setting_return_value_defaults_to_zero() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        X = Add(1, 2)
        FUNCTION Add(A, B)
            PRINT A + B
        END FUNCTION
        ";
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", 0.0_f32);
        assert_eq!(interpreter.stdlib.output, vec!["3"]);
    }

    #[test]
    fn test_function_call_missing_returns_zero() {
        let program = "
        X = Add(1, 2)
        ";
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", 0.0_f32);
    }

    #[test]
    fn test_function_call_missing_with_string_arguments_gives_type_mismatch() {
        let program = "
        X = Add(\"1\", \"2\")
        ";
        assert_eq!(
            interpret_err(program),
            // TODO 13 should be 17 with an additional linter
            InterpreterError::new_with_pos("Type mismatch", Location::new(2, 13))
        );
    }

    #[test]
    fn test_function_call_lowercase() {
        let program = "
        DECLARE FUNCTION Add(A, B, c)
        X = add(1, 2, 3)
        FUNCTION ADD(a, B, C)
            aDd = a + b + c
        END FUNCTION
        ";
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", 6.0_f32);
    }

    #[test]
    fn test_function_call_defint() {
        let program = "
        DEFINT A-Z
        DECLARE FUNCTION Add(A, B, c)
        X = add(1, 2, 3)
        FUNCTION ADD(a, B, C)
            aDd = a + b + c
        END FUNCTION
        ";
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", 6);
    }

    #[test]
    fn test_function_call_defstr() {
        let program = r#"
        DEFSTR A-Z
        DECLARE FUNCTION Add(A, B, c)
        X = add("1", "2", "3")
        FUNCTION ADD(a, B, C)
            aDd = a + b + c
        END FUNCTION
        "#;
        let interpreter = interpret(program);
        assert_has_variable!(interpreter, "X", "123");
    }

    #[test]
    fn test_interpret_function_call_user_defined_literal_arg() {
        let program = r#"
        DECLARE FUNCTION Hello(X)
        A = 1
        B = Hello(A + 1)
        PRINT A
        PRINT B
        FUNCTION Hello(X)
            X = X + 1
            Hello = X + 1
        END FUNCTION
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["1", "4"]);
    }

    #[test]
    fn test_interpret_function_call_user_defined_var_arg_is_by_ref() {
        let program = r#"
        DECLARE FUNCTION Hello(X)
        A = 1
        B = Hello(A)
        PRINT A
        PRINT B
        FUNCTION Hello(X)
            X = X + 1
            Hello = X + 1
        END FUNCTION
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["2", "3"]);
    }

    #[test]
    fn test_interpret_function_call_user_defined_var_arg_is_by_ref_assign_to_self() {
        let program = r#"
        DECLARE FUNCTION Hello(X)
        A = 1
        A = Hello(A)
        PRINT A
        FUNCTION Hello(X)
            X = X + 1
            Hello = X + 1
        END FUNCTION
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["3"]);
    }

    #[test]
    fn test_recursive_function() {
        let program = r#"
        DECLARE FUNCTION Sum(X)

        PRINT Sum(3)

        FUNCTION Sum(X)
            IF 1 < X THEN
                Sum = Sum(X - 1) + X
            ELSE
                Sum = 1
            END IF
        END FUNCTION
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["6"]);
    }
}
