use crate::interpreter::subprogram_context::{QualifiedImplementationNode, SubprogramContext};
use crate::parser::QualifiedName;

//pub type QualifiedFunctionDeclarationNode = QualifiedDeclarationNode<QualifiedName>;
pub type QualifiedFunctionImplementationNode = QualifiedImplementationNode<QualifiedName>;
pub type FunctionContext = SubprogramContext<QualifiedName>;

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use crate::assert_pre_process_err;
    use crate::common::Location;
    use crate::interpreter::InterpreterError;

    #[test]
    fn test_duplicate_function_declaration_identical_is_tolerated() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        DECLARE FUNCTION Add(A, B)
        PRINT Add(1, 2)
        FUNCTION Add(A, B)
        Add = A + B
        END FUNCTION
        ";
        let interpreter = interpret(program);
        assert_eq!(interpreter.stdlib.output, vec!["3"]);
    }

    #[test]
    fn test_duplicate_function_same_type_different_argument_count() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        DECLARE FUNCTION Add(A, B, C)
        PRINT Add(1, 2)
        ";
        assert_pre_process_err!(program, "Argument-count mismatch", 3, 9);
    }

    #[test]
    fn test_declaration_implementation_different_argument_count() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        PRINT Add(1, 2)
        FUNCTION Add(A, B, C)
            Add = A + B +C
        END FUNCTION
        ";
        assert_pre_process_err!(program, "Argument-count mismatch", 4, 9);
    }

    #[test]
    fn test_duplicate_function_different_function_type() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        DECLARE FUNCTION Add%(A, B)
        PRINT Add(1, 2)
        ";
        assert_pre_process_err!(program, "Duplicate definition", 3, 9);
    }

    #[test]
    fn test_duplicate_function_implementation() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        PRINT Add(1, 2)
        FUNCTION Add(A, B)
        Add = A + B
        END FUNCTION
        FUNCTION Add(A, B)
        Add = A + B
        END FUNCTION
        ";
        assert_pre_process_err!(program, "Duplicate definition", 7, 9);
    }

    #[test]
    fn test_duplicate_function_different_parameter_type() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        DECLARE FUNCTION Add(A$, B)
        PRINT Add(1, 2)
        ";
        assert_pre_process_err!(program, "Parameter type mismatch", 3, 30);
    }

    #[test]
    fn test_declaration_implementation_different_parameter_type() {
        let program = "
        DECLARE FUNCTION Add(A, B)
        PRINT Add(1, 2)
        FUNCTION Add(A, B$)
        Add = A + B
        END FUNCTION
        ";
        assert_pre_process_err!(program, "Parameter type mismatch", 4, 25);
    }

    #[test]
    fn test_duplicate_definition_on_call() {
        let program = "
        DECLARE FUNCTION Add#(A, B)
        PRINT Add!(1, 2)
        FUNCTION Add#(A, B)
            Add# = A + B
        END FUNCTION
        ";
        assert_pre_process_err!(program, "Duplicate definition", 3, 15);
    }

    #[test]
    fn test_duplicate_definition_on_implementation() {
        let program = "
        DECLARE FUNCTION Add#(A, B)
        PRINT Add#(1, 2)
        FUNCTION Add(A, B)
            Add = A + B
        END FUNCTION
        ";
        assert_pre_process_err!(program, "Duplicate definition", 4, 9);
    }

    #[test]
    fn test_duplicate_definition_on_return_value() {
        let program = "
        DECLARE FUNCTION Add#(A, B)
        PRINT Add#(1, 2)
        FUNCTION Add#(A, B)
            Add! = A + B
        END FUNCTION
        ";
        assert_pre_process_err!(program, "Duplicate definition", 5, 13);
    }

    #[test]
    fn test_able_to_call_function_with_type_qualifier() {
        let program = "
        DECLARE FUNCTION Add#(A, B)
        PRINT Add#(1, 2)
        FUNCTION Add#(A, B)
            Add# = A + B
        END FUNCTION
        ";
        assert_eq!(interpret(program).stdlib.output, vec!["3"]);
    }

    #[test]
    fn test_able_to_call_function_without_type_qualifier() {
        let program = "
        DECLARE FUNCTION Add#(A, B)
        PRINT Add(1, 2)
        FUNCTION Add#(A, B)
            Add# = A + B
        END FUNCTION
        ";
        assert_eq!(interpret(program).stdlib.output, vec!["3"]);
    }

    #[test]
    fn test_able_to_return_value_without_type_qualifier() {
        let program = "
        DECLARE FUNCTION Add#(A, B)
        PRINT Add#(1, 2)
        FUNCTION Add#(A, B)
            Add = A + B
        END FUNCTION
        ";
        assert_eq!(interpret(program).stdlib.output, vec!["3"]);
    }
}
