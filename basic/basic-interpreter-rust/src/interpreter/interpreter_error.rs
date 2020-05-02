use crate::common::Location;

/// A collection of coordinates in the program where an error occurred
pub type Stacktrace = Vec<Location>;

/// The error type of the interpreter
#[derive(Debug, PartialEq)]
pub struct InterpreterError {
    message: String,
    stacktrace: Stacktrace,
}

impl InterpreterError {
    pub fn new<S: AsRef<str>>(msg: S, stacktrace: Stacktrace) -> InterpreterError {
        InterpreterError {
            message: msg.as_ref().to_string(),
            stacktrace,
        }
    }

    pub fn new_with_pos<S: AsRef<str>>(msg: S, pos: Location) -> InterpreterError {
        InterpreterError::new(msg, vec![pos])
    }

    // TODO add test with stacktrace demo
    pub fn merge_pos(self, pos: Location) -> InterpreterError {
        let mut new_vec = self.stacktrace;
        new_vec.push(pos);
        InterpreterError::new(self.message, new_vec)
    }

    #[cfg(test)]
    pub fn message(&self) -> &String {
        &self.message
    }
}

pub fn err_pre_process<T, S: AsRef<str>>(msg: S, pos: Location) -> Result<T, InterpreterError> {
    Err(InterpreterError::new(
        format!("[P] {}", msg.as_ref()),
        vec![pos],
    ))
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;

    #[test]
    fn on_error_go_to_label() {
        let input = r#"
        ON ERROR GOTO ErrTrap
        X = 1 + "oops"
        SYSTEM
        ErrTrap:
            PRINT "Saved by the bell"
        "#;
        let interpreter = interpret(input);
        assert_eq!(interpreter.stdlib.output, vec!["Saved by the bell"]);
    }
}
