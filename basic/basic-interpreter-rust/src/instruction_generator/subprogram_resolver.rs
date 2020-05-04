use super::built_in_functions;
use super::built_in_subs;
use super::subprogram_context::*;
use super::{err, Result};
use crate::common::*;
use crate::linter::*;

pub fn resolve(program: ProgramNode) -> Result<(ProgramNode, FunctionContext, SubContext)> {
    let mut function_context = FunctionContext::new();
    let mut sub_context = SubContext::new();
    let mut reduced_program: ProgramNode = vec![];

    for top_level_token_node in program {
        let (top_level_token, pos) = top_level_token_node.consume();
        match top_level_token {
            TopLevelToken::FunctionImplementation(f_name, f_params, f_body) => {
                function_context.add_implementation(f_name, f_params, f_body, pos)?;
            }
            TopLevelToken::SubImplementation(s_name, s_params, s_body) => {
                sub_context.add_implementation(s_name, s_params, s_body, pos)?;
            }
            _ => reduced_program.push(top_level_token.at(pos)),
        }
    }
    // TODO ensure no clash with built-ins
    Ok((reduced_program, function_context, sub_context))
}

pub trait AllSubsKnown {
    fn all_subs_known(node: &Self, sub_context: &SubContext) -> Result<()>;
}

impl<T: std::fmt::Debug + Sized + AllSubsKnown> AllSubsKnown for Locatable<T> {
    fn all_subs_known(node: &Self, sub_context: &SubContext) -> Result<()> {
        T::all_subs_known(node.as_ref(), sub_context)
            .map_err(|e| e.at_non_zero_location(node.location()))
    }
}

impl<T: std::fmt::Debug + Sized + AllSubsKnown> AllSubsKnown for Vec<T> {
    fn all_subs_known(block: &Self, sub_context: &SubContext) -> Result<()> {
        for statement in block {
            T::all_subs_known(statement, sub_context)?;
        }
        Ok(())
    }
}

impl AllSubsKnown for TopLevelToken {
    fn all_subs_known(top_level_token: &Self, sub_context: &SubContext) -> Result<()> {
        match top_level_token {
            TopLevelToken::Statement(s) => Statement::all_subs_known(s, sub_context),
            _ => Ok(()),
        }
    }
}

impl AllSubsKnown for Statement {
    fn all_subs_known(statement: &Self, sub_context: &SubContext) -> Result<()> {
        match statement {
            Statement::SubCall(n, _) => {
                // TODO validate argument count and type if possible
                if built_in_subs::is_built_in_sub(n) || sub_context.has_implementation(n) {
                    Ok(())
                } else {
                    err(format!("Unknown SUB {}", n), Location::zero())
                }
            }
            Statement::ForLoop(f) => StatementNodes::all_subs_known(&f.statements, sub_context),
            Statement::IfBlock(i) => {
                StatementNodes::all_subs_known(&i.if_block.statements, sub_context)?;
                for else_if_block in &i.else_if_blocks {
                    StatementNodes::all_subs_known(&else_if_block.statements, sub_context)?;
                }
                match &i.else_block {
                    Some(x) => StatementNodes::all_subs_known(x, sub_context),
                    None => Ok(()),
                }
            }
            Statement::While(w) => StatementNodes::all_subs_known(&w.statements, sub_context),
            _ => Ok(()),
        }
    }
}

pub trait AllFunctionsKnown {
    fn all_functions_known(node: &Self, function_context: &FunctionContext) -> Result<()>;
}

impl<T: std::fmt::Debug + Sized + AllFunctionsKnown> AllFunctionsKnown for Locatable<T> {
    fn all_functions_known(node: &Self, function_context: &FunctionContext) -> Result<()> {
        T::all_functions_known(node.as_ref(), function_context)
    }
}

impl<T: std::fmt::Debug + Sized + AllFunctionsKnown> AllFunctionsKnown for Vec<T> {
    fn all_functions_known(block: &Self, function_context: &FunctionContext) -> Result<()> {
        for statement in block {
            T::all_functions_known(statement, function_context)?;
        }
        Ok(())
    }
}

impl AllFunctionsKnown for TopLevelToken {
    fn all_functions_known(
        top_level_token: &Self,
        function_context: &FunctionContext,
    ) -> Result<()> {
        match top_level_token {
            TopLevelToken::Statement(s) => Statement::all_functions_known(s, function_context),
            _ => Ok(()),
        }
    }
}

impl AllFunctionsKnown for Statement {
    fn all_functions_known(statement: &Self, function_context: &FunctionContext) -> Result<()> {
        match statement {
            Self::SubCall(_, args) => {
                for a in args {
                    ExpressionNode::all_functions_known(a, function_context)?;
                }
                Ok(())
            }
            Self::ForLoop(f) => {
                ExpressionNode::all_functions_known(&f.lower_bound, function_context)?;
                ExpressionNode::all_functions_known(&f.upper_bound, function_context)?;
                if let Some(step) = &f.step {
                    ExpressionNode::all_functions_known(step, function_context)?;
                }
                StatementNodes::all_functions_known(&f.statements, function_context)
            }
            Self::IfBlock(i) => {
                ConditionalBlockNode::all_functions_known(&i.if_block, function_context)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::all_functions_known(&else_if_block, function_context)?;
                }
                match &i.else_block {
                    Some(x) => StatementNodes::all_functions_known(x, function_context),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::all_functions_known(w, function_context),
            Self::Assignment(_, right) => {
                ExpressionNode::all_functions_known(right, function_context)
            }
            Self::Const(_, right) => {
                // TODO probably remove later as the const should not have functions anyway
                ExpressionNode::all_functions_known(right, function_context)
            }
            _ => Ok(()),
        }
    }
}

impl AllFunctionsKnown for ConditionalBlockNode {
    fn all_functions_known(e: &Self, function_context: &FunctionContext) -> Result<()> {
        ExpressionNode::all_functions_known(&e.condition, function_context)?;
        StatementNodes::all_functions_known(&e.statements, function_context)
    }
}

fn check_function_return_type_on_call(
    call_name: &QNameNode,
    func_impl: &QualifiedFunctionImplementationNode,
) -> Result<()> {
    match call_name.opt_qualifier() {
        None => Ok(()),
        Some(q) => {
            if q == func_impl.qualifier() {
                Ok(())
            } else {
                err("Duplicate definition", call_name.location())
            }
        }
    }
}

fn check_function_args_on_call(
    call_name: &QNameNode,
    args: &Vec<ExpressionNode>,
    func_impl: &QualifiedFunctionImplementationNode,
) -> Result<()> {
    if func_impl.parameters.len() != args.len() {
        err("Argument count mismatch", call_name.location())
    } else {
        Ok(())
    }
}

impl AllFunctionsKnown for ExpressionNode {
    fn all_functions_known(e_node: &Self, function_context: &FunctionContext) -> Result<()> {
        let e: &Expression = e_node.as_ref();
        match e {
            Expression::FunctionCall(n, args) => {
                for a in args {
                    Self::all_functions_known(a, function_context)?;
                }

                if built_in_functions::is_built_in_function(n) {
                    // TODO: validate ENVIRON$
                    Ok(())
                } else if function_context.has_implementation(n) {
                    let func_impl = function_context.get_implementation_ref(n).unwrap();
                    let name_node: QNameNode = n.clone().at(e_node.location());
                    check_function_return_type_on_call(&name_node, func_impl)?;
                    check_function_args_on_call(&name_node, args, func_impl)
                } else {
                    // Unknown function, will fallback to undefined behavior later
                    Ok(())
                }
            }
            Expression::BinaryExpression(_, left, right) => {
                let unboxed_left: &Self = left;
                let unboxed_right: &Self = right;
                Self::all_functions_known(unboxed_left, function_context)?;
                Self::all_functions_known(unboxed_right, function_context)
            }
            Expression::UnaryExpression(_, child) => {
                let unboxed_child: &Self = child;
                Self::all_functions_known(unboxed_child, function_context)
            }
            _ => Ok(()),
        }
    }
}
