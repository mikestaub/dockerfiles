#[cfg(test)]
mod tests {
    use super::super::test_utils::*;

    #[test]
    fn go_to_label_go_to_is_before_label_declaration() {
        let input = r#"
        PRINT "a"
        GOTO Jump
        PRINT "b"
        Jump:
        PRINT "c"
        "#;
        let interpreter = interpret(input);
        assert_eq!(interpreter.stdlib.output, vec!["a", "c"]);
    }

    #[test]
    fn go_to_label_go_to_is_after_label_declaration() {
        let input = r#"
        X = 0
        Jump:
        PRINT X
        X = X + 1
        IF X <= 1 THEN
            GOTO Jump
        END IF
        "#;
        let interpreter = interpret(input);
        assert_eq!(interpreter.stdlib.output, vec!["0", "1"]);
    }

    #[test]
    fn go_to_label_go_to_in_single_line_if_then() {
        let input = r#"
        X = 0
        Jump:
        PRINT X
        X = X + 1
        IF X <= 1 THEN GOTO Jump
        "#;
        let interpreter = interpret(input);
        assert_eq!(interpreter.stdlib.output, vec!["0", "1"]);
    }
}
