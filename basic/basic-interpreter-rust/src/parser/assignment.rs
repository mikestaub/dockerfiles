use super::{NameNode, Parser, ParserError, StatementNode};
use crate::common::ResultOptionHelper;
use std::io::BufRead;

impl<T: BufRead> Parser<T> {
    pub fn try_parse_assignment(&mut self) -> Result<Option<StatementNode>, ParserError> {
        self.buf_lexer.mark();
        let left_side = self._try_parse_left_side_of_assignment()?;
        match left_side {
            Some(n) => {
                self.buf_lexer.clear();
                let exp = self.demand_expression()?;
                self.buf_lexer.skip_whitespace()?;
                self.buf_lexer.demand_eol_or_eof()?;
                Ok(Some(StatementNode::Assignment(n, exp)))
            }
            None => {
                self.buf_lexer.backtrack();
                Ok(None)
            }
        }
    }

    fn _try_parse_left_side_of_assignment(&mut self) -> Result<Option<NameNode>, ParserError> {
        self.try_parse_name_with_type_qualifier().opt_filter(|_| {
            self.buf_lexer.skip_whitespace()?;
            if self.buf_lexer.try_consume_symbol('=')?.is_some() {
                self.buf_lexer.skip_whitespace()?;
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::*;
    use super::*;
    use crate::common::Location;
    use crate::lexer::LexemeNode;
    use crate::parser::StatementNode;

    macro_rules! assert_top_level_assignment {
        ($input:expr, $name:expr, $value:expr) => {
            match parse($input).demand_single_statement() {
                StatementNode::Assignment(n, v) => {
                    assert_eq!(&n, $name);
                    assert_eq!(v, $value);
                }
                _ => panic!("expected assignment"),
            }
        };
    }

    #[test]
    fn test_numeric_assignment() {
        assert_top_level_assignment!("A = 42", "A", 42);
    }

    #[test]
    fn test_numeric_assignment_to_keyword_not_allowed() {
        assert_eq!(
            parse_err("FOR = 42"),
            ParserError::Unexpected(
                "Expected word".to_string(),
                LexemeNode::Symbol('=', Location::new(1, 5))
            )
        );
    }

    #[test]
    fn test_numeric_assignment_to_keyword_plus_number_allowed() {
        assert_top_level_assignment!("FOR42 = 42", "FOR42", 42);
    }

    #[test]
    fn test_sub_call_is_not_mistaken_for_assignment() {
        let input = "PRINT 42";
        let mut parser = Parser::from(input);
        let result = parser.try_parse_assignment().unwrap();
        match result {
            None => (),
            Some(_) => panic!("should not have mistaken sub call for assignment"),
        };
        parser.try_parse_sub_call().unwrap().unwrap();
    }
}
