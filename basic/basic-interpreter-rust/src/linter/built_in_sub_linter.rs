use super::error::*;
use super::post_conversion_linter::PostConversionLinter;
use super::types::*;
use crate::common::*;
use crate::parser::TypeQualifier;

pub struct BuiltInSubLinter;

pub fn is_built_in_sub(sub_name: &CaseInsensitiveString) -> bool {
    sub_name == "ENVIRON" || sub_name == "PRINT" || sub_name == "INPUT" || sub_name == "SYSTEM"
}

impl PostConversionLinter for BuiltInSubLinter {
    fn visit_sub_call(&self, n: &CaseInsensitiveString, args: &Vec<ExpressionNode>) -> Result<()> {
        if n == "SYSTEM" {
            if args.len() != 0 {
                err("Argument count mismatch", Location::zero())
            } else {
                Ok(())
            }
        } else if n == "ENVIRON" {
            if args.len() != 1 {
                err("Argument count mismatch", Location::zero())
            } else if args[0].as_ref().try_qualifier()? != TypeQualifier::DollarString {
                err("Argument type mismatch", args[0].location())
            } else {
                Ok(())
            }
        } else if n == "INPUT" {
            if args.len() == 0 {
                err("Argument count mismatch", Location::zero())
            } else {
                args.iter()
                    .map(|a| match a.as_ref() {
                        Expression::Variable(_) => Ok(()),
                        _ => err("Expected variable argument", a.location()),
                    })
                    .collect()
            }
        } else {
            Ok(())
        }
    }
}
