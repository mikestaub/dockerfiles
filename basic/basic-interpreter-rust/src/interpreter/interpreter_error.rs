use crate::common::Location;

/// A collection of coordinates in the program where an error occurred
pub type Stacktrace = Vec<Location>;

/// The error type of the interpreter
#[derive(Debug, PartialEq)]
pub struct InterpreterError {
    message: String,
    stacktrace: Stacktrace,
}

pub type Result<T> = std::result::Result<T, InterpreterError>;

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

    pub fn at_non_zero_location(self, pos: Location) -> Self {
        if self.stacktrace.is_empty() {
            InterpreterError::new_with_pos(self.message, pos)
        } else {
            if self.stacktrace[self.stacktrace.len() - 1] == Location::zero() {
                let mut new_stacktrace = Stacktrace::new();
                for i in 0..self.stacktrace.len() - 1 {
                    new_stacktrace.push(self.stacktrace[i]);
                }
                new_stacktrace.push(pos);
                Self::new(self.message, new_stacktrace)
            } else {
                self
            }
        }
    }

    #[cfg(test)]
    pub fn message(&self) -> &String {
        &self.message
    }
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
