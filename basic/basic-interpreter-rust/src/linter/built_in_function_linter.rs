use super::error::*;
use super::post_conversion_linter::PostConversionLinter;
use super::types::*;
use crate::common::*;
use crate::parser::{QualifiedName, TypeQualifier};
use std::convert::TryFrom;

pub struct BuiltInFunctionLinter;

pub fn is_built_in_function(function_name: &QualifiedName) -> bool {
    function_name == &QualifiedName::new("ENVIRON", TypeQualifier::DollarString)
}

impl BuiltInFunctionLinter {
    fn visit_function(
        &self,
        name: &QualifiedName,
        args: &Vec<ExpressionNode>,
    ) -> Result<(), Error> {
        if name == &QualifiedName::try_from("ENVIRON$").unwrap() {
            self.visit_environ(args)
        } else {
            Ok(())
        }
    }

    fn visit_environ(&self, args: &Vec<ExpressionNode>) -> Result<(), Error> {
        if args.len() != 1 {
            err_no_pos(LinterError::ArgumentCountMismatch)
        } else {
            let q = args[0].as_ref().try_qualifier()?;
            if q != TypeQualifier::DollarString {
                err_l(LinterError::ArgumentTypeMismatch, &args[0])
            } else {
                Ok(())
            }
        }
    }
}

impl PostConversionLinter for BuiltInFunctionLinter {
    fn visit_expression(&self, expr_node: &ExpressionNode) -> Result<(), Error> {
        let pos = expr_node.location();
        let e = expr_node.as_ref();
        match e {
            Expression::FunctionCall(n, args) => {
                for x in args {
                    self.visit_expression(x)?;
                }
                self.visit_function(n, args).with_err_pos(pos)
            }
            Expression::BinaryExpression(_, left, right) => {
                self.visit_expression(left)?;
                self.visit_expression(right)
            }
            Expression::UnaryExpression(_, child) => self.visit_expression(child),
            _ => Ok(()),
        }
    }
}
