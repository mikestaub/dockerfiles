mod common;
mod interpreter;
mod lexer;
mod linter;
mod parser;
mod reader;

use std::env;
use std::fs::File;

use interpreter::instruction_generator;
use interpreter::{DefaultStdlib, Interpreter};
use parser::Parser;

fn main() {
    let filename = env::args()
        .skip(1)
        .take(1)
        .last()
        .expect("The first argument should be the program to run");
    let f = File::open(&filename).expect(format!("Could not find program {}", filename).as_ref());
    let mut parser = Parser::from(f);
    match parser.parse() {
        Ok(program) => {
            let mut interpreter = Interpreter::new(DefaultStdlib {});
            match instruction_generator::generate_instructions(program) {
                Ok(instructions) => match interpreter.interpret(instructions) {
                    Ok(_) => (),
                    Err(e) => eprintln!("Runtime error. {:?}", e),
                },
                Err(e) => eprintln!("Could not generate instructions {:?}", e),
            }
        }
        Err(e) => eprintln!("Could not parse program. {:?}", e),
    }
}
