use super::built_in_sub_linter::is_built_in_sub;
use super::error::*;
use super::post_conversion_linter::PostConversionLinter;
use super::subprogram_context::SubMap;
use super::types::*;
use crate::common::*;

pub struct UserDefinedSubLinter<'a> {
    pub subs: &'a SubMap,
}

impl<'a> PostConversionLinter for UserDefinedSubLinter<'a> {
    fn visit_sub_call(
        &self,
        name: &CaseInsensitiveString,
        args: &Vec<ExpressionNode>,
    ) -> Result<()> {
        if is_built_in_sub(name) {
            // TODO somewhere ensure we can't override built-in subs
            Ok(())
        } else {
            match self.subs.get(name) {
                Some((param_types, _)) => {
                    if args.len() != param_types.len() {
                        err("Argument count mismatch", Location::zero())
                    } else {
                        for i in 0..args.len() {
                            let arg_node = args.get(i).unwrap();
                            let arg = arg_node.as_ref();
                            let arg_q = arg.try_qualifier()?;
                            if !arg_q.can_cast_to(param_types[i]) {
                                return err("Argument type mismatch", arg_node.location());
                            }
                        }
                        Ok(())
                    }
                }
                None => err("Subprogram not defined", Location::zero()),
            }
        }
    }
}
