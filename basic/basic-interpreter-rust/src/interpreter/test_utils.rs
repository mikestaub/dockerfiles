use crate::common::*;
use crate::interpreter::context_owner::ContextOwner;
use crate::interpreter::variant::Variant;
use crate::interpreter::{Instruction, Interpreter, InterpreterError, Result, Stdlib};
use crate::parser::{Name, NameNode, Parser};
use std::collections::HashMap;
use std::fs::File;

pub fn generate_instructions<T>(input: T) -> Vec<Instruction>
where
    T: AsRef<[u8]>,
{
    let mut parser = Parser::from(input);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new(MockStdlib::new());
    interpreter
        .generate_instructions(program)
        .unwrap()
        .into_iter()
        .map(|x| x.consume().0)
        .collect()
}

pub fn interpret<T>(input: T) -> Interpreter<MockStdlib>
where
    T: AsRef<[u8]>,
{
    let mut parser = Parser::from(input);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new(MockStdlib::new());
    interpreter.interpret(program).map(|_| interpreter).unwrap()
}

pub fn interpret_with_stdlib<T, TStdlib>(input: T, stdlib: TStdlib) -> Interpreter<TStdlib>
where
    T: AsRef<[u8]>,
    TStdlib: Stdlib,
{
    let mut parser = Parser::from(input);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new(stdlib);
    interpreter.interpret(program).map(|_| interpreter).unwrap()
}

pub fn interpret_err<T>(input: T) -> InterpreterError
where
    T: AsRef<[u8]>,
{
    let mut parser = Parser::from(input);
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new(MockStdlib::new());
    interpreter.interpret(program).unwrap_err()
}

pub fn interpret_file<S, TStdlib>(filename: S, stdlib: TStdlib) -> Result<Interpreter<TStdlib>>
where
    S: AsRef<str>,
    TStdlib: Stdlib,
{
    let file_path = format!("fixtures/{}", filename.as_ref());
    let mut parser = Parser::from(File::open(file_path).expect("Could not read bas file"));
    let program = parser.parse().unwrap();
    let mut interpreter = Interpreter::new(stdlib);
    interpreter.interpret(program).map(|_| interpreter)
}

#[derive(Debug)]
pub struct MockStdlib {
    next_input: Vec<String>,
    pub output: Vec<String>,
    pub env: HashMap<String, String>,
}

impl MockStdlib {
    pub fn new() -> MockStdlib {
        MockStdlib {
            next_input: vec![],
            output: vec![],
            env: HashMap::new(),
        }
    }

    pub fn add_next_input<S: AsRef<str>>(&mut self, value: S) {
        self.next_input.push(value.as_ref().to_string())
    }
}

impl Stdlib for MockStdlib {
    fn print(&mut self, args: Vec<String>) {
        let mut is_first = true;
        let mut buf = String::new();
        for arg in args {
            if is_first {
                is_first = false;
            } else {
                buf.push(' ');
            }
            buf.push_str(&arg);
        }

        println!("{}", buf);
        self.output.push(buf);
    }

    fn system(&self) {
        println!("would have exited")
    }

    fn input(&mut self) -> std::io::Result<String> {
        Ok(self.next_input.remove(0))
    }

    fn get_env_var(&self, name: &String) -> String {
        match self.env.get(name) {
            Some(x) => x.clone(),
            None => String::new(),
        }
    }

    fn set_env_var(&mut self, name: String, value: String) {
        self.env.insert(name, value);
    }
}

impl<S: Stdlib> Interpreter<S> {
    pub fn get_variable_str(&self, name: &str) -> Result<Variant> {
        let pos = Location::start();
        self.context_ref()
            .get_r_value(&Name::from(name).at(pos))
            .map(|x| x.unwrap())
    }
}

#[macro_export]
macro_rules! assert_has_variable {
    ($int:expr, $name:expr, $expected_value:expr) => {
        assert_eq!(
            $int.get_variable_str($name).unwrap(),
            Variant::from($expected_value)
        );
    };
}

pub struct AssignmentBuilder {
    variable_name_node: NameNode,
    program: String,
}

impl AssignmentBuilder {
    pub fn new(variable_literal: &str) -> AssignmentBuilder {
        AssignmentBuilder {
            variable_name_node: Name::from(variable_literal).at(Location::start()),
            program: String::new(),
        }
    }

    pub fn literal(&mut self, expression_literal: &str) -> &mut Self {
        if self.program.is_empty() {
            let variable_name: Name = self.variable_name_node.clone().strip_location();
            self.program = format!("{} = {}", variable_name, expression_literal);
            self
        } else {
            panic!("Cannot re-assign program")
        }
    }

    pub fn assert_eq<T>(&self, expected_value: T)
    where
        Variant: From<T>,
    {
        if self.program.is_empty() {
            panic!("Program was not set")
        } else {
            let interpreter = interpret(&self.program);
            assert_eq!(
                interpreter
                    .context_ref()
                    .get_r_value(&self.variable_name_node)
                    .unwrap()
                    .unwrap(),
                Variant::from(expected_value)
            );
        }
    }

    pub fn assert_err(&self) {
        if self.program.is_empty() {
            panic!("Program was not set");
        } else {
            assert_eq!(
                interpret_err(&self.program),
                InterpreterError::new_with_pos("Type mismatch", Location::new(1, 1))
            );
        }
    }
}

pub fn assert_assign(variable_literal: &str) -> AssignmentBuilder {
    AssignmentBuilder::new(variable_literal)
}

pub fn assert_input<T>(raw_input: &str, variable_name: &str, expected_value: T)
where
    Variant: From<T>,
{
    let mut stdlib = MockStdlib::new();
    stdlib.add_next_input(raw_input);
    let input = format!("INPUT {}", variable_name);
    let interpreter = interpret_with_stdlib(input, stdlib);
    assert_has_variable!(interpreter, variable_name, expected_value);
}

#[macro_export]
macro_rules! assert_err {
    ($program:expr, $expected_msg:expr, $expected_row:expr, $expected_col:expr) => {
        assert_eq!(
            interpret_err($program),
            InterpreterError::new_with_pos(
                $expected_msg,
                Location::new($expected_row, $expected_col)
            )
        );
    };
}

#[macro_export]
macro_rules! assert_pre_process_err {
    ($program:expr, $expected_msg:expr, $expected_row:expr, $expected_col:expr) => {
        assert_eq!(
            interpret_err($program),
            InterpreterError::new_with_pos(
                format!("[P] {}", $expected_msg),
                Location::new($expected_row, $expected_col)
            )
        );
    };
}
