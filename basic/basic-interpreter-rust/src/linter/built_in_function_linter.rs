use super::error::*;
use super::post_conversion_linter::PostConversionLinter;
use super::types::*;
use crate::common::*;
use crate::parser::{QualifiedName, TypeQualifier};
use std::convert::TryFrom;

pub struct BuiltInFunctionLinter;

impl BuiltInFunctionLinter {
    fn visit_function(&self, name: &QualifiedName, args: &Vec<ExpressionNode>) -> Result<()> {
        if name == &QualifiedName::try_from("ENVIRON$").unwrap() {
            self.visit_environ(args)
        } else {
            Ok(())
        }
    }

    fn visit_environ(&self, args: &Vec<ExpressionNode>) -> Result<()> {
        if args.len() != 1 {
            err("Argument count mismatch", Location::zero())
        } else if args[0].as_ref().try_qualifier()? != TypeQualifier::DollarString {
            err("Argument type mismatch", args[0].location())
        } else {
            Ok(())
        }
    }
}

impl PostConversionLinter for BuiltInFunctionLinter {
    fn visit_expression(&self, expr_node: &ExpressionNode) -> Result<()> {
        let pos = expr_node.location();
        let e = expr_node.as_ref();
        match e {
            Expression::FunctionCall(n, args) => {
                for x in args {
                    self.visit_expression(x)?;
                }
                self.visit_function(n, args)
                    .map_err(|e| e.at_non_zero_location(pos))
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
