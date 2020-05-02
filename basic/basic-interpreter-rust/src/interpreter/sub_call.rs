#[cfg(test)]
mod tests {
    use crate::interpreter::test_utils::*;
    use crate::interpreter::Stdlib;

    mod input {
        mod unqualified_var {
            use crate::interpreter::test_utils::*;

            #[test]
            fn test_input_empty() {
                assert_input("", "N", 0.0_f32);
            }

            #[test]
            fn test_input_zero() {
                assert_input("0", "N", 0.0_f32);
            }

            #[test]
            fn test_input_single() {
                assert_input("1.1", "N", 1.1_f32);
            }

            #[test]
            fn test_input_negative() {
                assert_input("-1.2345", "N", -1.2345_f32);
            }

            #[test]
            fn test_input_explicit_positive() {
                assert_input("+3.14", "N", 3.14_f32);
            }
        }

        mod string_var {
            use crate::interpreter::test_utils::*;

            #[test]
            fn test_input_hello() {
                assert_input("hello", "A$", "hello");
            }

            #[test]
            fn test_input_does_not_trim_new_line() {
                assert_input("hello\r\n", "A$", "hello\r\n");
            }
        }

        mod int_var {
            use crate::interpreter::test_utils::*;

            #[test]
            fn test_input_42() {
                assert_input("42", "A%", 42);
            }
        }
    }

    #[test]
    fn test_sub_call_environ() {
        let program = r#"
        ENVIRON "FOO=BAR"
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.get_env_var(&"FOO".to_string()), "BAR");
    }

    #[test]
    fn test_interpret_sub_call_user_defined_no_args() {
        let program = r#"
        DECLARE SUB Hello
        Hello
        SUB Hello
            ENVIRON "FOO=BAR"
        END SUB
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.get_env_var(&"FOO".to_string()), "BAR");
    }

    #[test]
    fn test_interpret_sub_call_user_defined_two_args() {
        let program = r#"
        DECLARE SUB Hello(N$, V$)
        Hello "FOO", "BAR"
        SUB Hello(N$, V$)
            ENVIRON N$ + "=" + V$
        END SUB
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.get_env_var(&"FOO".to_string()), "BAR");
    }

    #[test]
    fn test_interpret_sub_call_user_defined_literal_arg() {
        let program = r#"
        DECLARE SUB Hello(X)
        A = 1
        Hello 5
        PRINT A
        SUB Hello(X)
            X = 42
        END SUB
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["1"]);
    }

    #[test]
    fn test_interpret_sub_call_user_defined_var_arg_is_by_ref() {
        let program = r#"
        DECLARE SUB Hello(X)
        A = 1
        Hello A
        PRINT A
        SUB Hello(X)
            X = 42
        END SUB
        "#;
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["42"]);
    }

    #[test]
    fn test_interpret_sub_call_user_defined_cannot_access_global_scope() {
        let program = "
        DECLARE SUB Hello
        A = 1
        Hello
        PRINT A
        SUB Hello
            A = 42
        END SUB
        ";
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["1"]);
    }
}
