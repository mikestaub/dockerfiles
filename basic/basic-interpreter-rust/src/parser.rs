use crate::buf_lexer::BufLexer;
use crate::common::Result;
use crate::lexer::Lexeme;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor};

mod assignment;
mod declaration;
mod expression;
mod for_loop;
mod function_implementation;
mod if_block;
mod qname;
mod statement;
mod sub_call;
mod type_qualifier;

#[cfg(test)]
mod test_utils;

pub use self::expression::*;
pub use self::for_loop::*;
pub use self::if_block::*;
pub use self::qname::*;
pub use self::statement::*;
pub use self::type_qualifier::*;

pub type Block = Vec<Statement>;

#[derive(Debug, PartialEq)]
pub enum TopLevelToken {
    EOF,
    FunctionDeclaration(QName, Vec<QName>),
    Statement(Statement),
    FunctionImplementation(QName, Vec<QName>, Block),
}

pub type Program = Vec<TopLevelToken>;

#[derive(Debug)]
pub struct Parser<T> {
    pub buf_lexer: BufLexer<T>,
}

impl<T: BufRead> Parser<T> {
    pub fn new(buf_lexer: BufLexer<T>) -> Parser<T> {
        Parser { buf_lexer }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut v: Vec<TopLevelToken> = vec![];
        loop {
            let x = self._parse_top_level_token()?;
            match x {
                TopLevelToken::EOF => break,
                _ => v.push(x),
            };
        }
        Ok(v)
    }

    fn _parse_top_level_token(&mut self) -> Result<TopLevelToken> {
        if let Some(d) = self.try_parse_declaration()? {
            Ok(d)
        } else if let Some(f) = self.try_parse_function_implementation()? {
            Ok(f)
        } else if let Some(s) = self._try_parse_statement_as_top_level_token()? {
            Ok(s)
        } else {
            let lexeme = self.buf_lexer.read()?;
            match lexeme {
                Lexeme::EOF => {
                    self.buf_lexer.consume();
                    Ok(TopLevelToken::EOF)
                }
                _ => self.buf_lexer.err("[parser] Unexpected lexeme"),
            }
        }
    }

    fn _try_parse_statement_as_top_level_token(&mut self) -> Result<Option<TopLevelToken>> {
        match self.try_parse_statement()? {
            Some(statement) => Ok(Some(TopLevelToken::Statement(statement))),
            None => Ok(None),
        }
    }
}

// bytes || &str -> Parser
impl<T> From<T> for Parser<BufReader<Cursor<T>>>
where
    T: AsRef<[u8]>,
{
    fn from(input: T) -> Self {
        Parser::new(BufLexer::from(input))
    }
}

// File -> Parser
impl From<File> for Parser<BufReader<File>> {
    fn from(input: File) -> Self {
        Parser::new(BufLexer::from(input))
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::*;
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_parse_sub_call_no_args() {
        let input = "PRINT";
        let program = parse(input);
        assert_eq!(program, vec![top_sub_call("PRINT", vec![])]);
    }

    #[test]
    fn test_parse_sub_call_single_arg_string_literal() {
        let input = "PRINT \"Hello, world!\"";
        let program = parse(input);
        assert_eq!(
            program,
            vec![top_sub_call(
                "PRINT",
                vec![Expression::from("Hello, world!")]
            )]
        );
    }

    #[test]
    fn test_parse_fixture_hello1() {
        let program = parse_file("HELLO1.BAS");
        assert_eq!(
            program,
            vec![top_sub_call(
                "PRINT",
                vec![Expression::from("Hello, world!")]
            )]
        );
    }

    #[test]
    fn test_parse_fixture_hello2() {
        let program = parse_file("HELLO2.BAS");
        assert_eq!(
            program,
            vec![top_sub_call(
                "PRINT",
                vec![Expression::from("Hello"), Expression::from("world!"),]
            )]
        );
    }

    #[test]
    fn test_parse_fixture_hello_system() {
        let program = parse_file("HELLO_S.BAS");
        assert_eq!(
            program,
            vec![
                top_sub_call("PRINT", vec![Expression::from("Hello, world!"),]),
                top_sub_call("SYSTEM", vec![])
            ]
        );
    }

    #[test]
    fn test_parse_fixture_input() {
        let program = parse_file("INPUT.BAS");
        assert_eq!(
            program,
            vec![
                top_sub_call("INPUT", vec![Expression::variable_name_unqualified("N")]),
                top_sub_call("PRINT", vec![Expression::variable_name_unqualified("N")])
            ]
        );
    }

    #[test]
    fn test_parse_fixture_fib() {
        let program = parse_file("FIB.BAS");
        assert_eq!(
            program,
            vec![
                // DECLARE FUNCTION Fib! (N!)
                TopLevelToken::FunctionDeclaration(
                    QName::Typed(QualifiedName::new(
                        "Fib".to_string(),
                        TypeQualifier::BangSingle
                    )),
                    vec![QName::Typed(QualifiedName::new(
                        "N".to_string(),
                        TypeQualifier::BangSingle
                    ))]
                ),
                // PRINT "Enter the number of fibonacci to calculate"
                top_sub_call(
                    "PRINT",
                    vec![Expression::from(
                        "Enter the number of fibonacci to calculate"
                    )]
                ),
                // INPUT N
                top_sub_call("INPUT", vec![Expression::variable_name_unqualified("N")]),
                // FOR I = 0 TO N
                TopLevelToken::Statement(Statement::ForLoop(ForLoop {
                    variable_name: QName::from_str("I").unwrap(),
                    lower_bound: Expression::IntegerLiteral(0),
                    upper_bound: Expression::variable_name_unqualified("N"),
                    step: None,
                    statements: vec![
                        // PRINT "Fibonacci of ", I, " is ", Fib(I)
                        sub_call(
                            "PRINT",
                            vec![
                                Expression::from("Fibonacci of"),
                                Expression::variable_name_unqualified("I"),
                                Expression::from("is"),
                                Expression::FunctionCall(
                                    QName::from_str("Fib").unwrap(),
                                    vec![Expression::variable_name_unqualified("I")]
                                )
                            ]
                        )
                    ],
                    next_counter: None
                })),
                // FUNCTION Fib (N)
                TopLevelToken::FunctionImplementation(
                    QName::Untyped("Fib".to_string()),
                    vec![QName::Untyped("N".to_string())],
                    vec![
                        // IF N <= 1 THEN
                        Statement::IfBlock(IfBlock::new_if_else(
                            // N <= 1
                            Expression::lte(
                                Expression::variable_name_unqualified("N"),
                                Expression::IntegerLiteral(1)
                            ),
                            // Fib = N
                            vec![Statement::Assignment(
                                QName::Untyped("Fib".to_string()),
                                Expression::variable_name_unqualified("N")
                            )],
                            // ELSE Fib = Fib(N - 1) + Fib(N - 2)
                            vec![Statement::Assignment(
                                QName::Untyped("Fib".to_string()),
                                Expression::plus(
                                    Expression::FunctionCall(
                                        QName::Untyped("Fib".to_string()),
                                        vec![Expression::minus(
                                            Expression::variable_name_unqualified("N"),
                                            Expression::IntegerLiteral(1)
                                        )]
                                    ),
                                    Expression::FunctionCall(
                                        QName::Untyped("Fib".to_string()),
                                        vec![Expression::minus(
                                            Expression::variable_name_unqualified("N"),
                                            Expression::IntegerLiteral(2)
                                        )]
                                    )
                                )
                            )]
                        ))
                    ]
                )
            ]
        );
    }
}