use crate::common::*;
use crate::interpreter::built_in_functions;
use crate::interpreter::built_in_subs;
use crate::interpreter::err_pre_process;
use crate::interpreter::function_context::*;
use crate::interpreter::sub_context::*;
use crate::interpreter::type_resolver_impl::*;
use crate::interpreter::Result;
use crate::parser::*;

pub fn resolve(program: ProgramNode) -> Result<(ProgramNode, FunctionContext, SubContext)> {
    let mut function_context = FunctionContext::new();
    let mut sub_context = SubContext::new();
    let mut reduced_program: ProgramNode = vec![];
    let mut type_resolver = TypeResolverImpl::new();

    for top_level_token_node in program {
        let (top_level_token, pos) = top_level_token_node.consume();
        match top_level_token {
            TopLevelToken::DefType(d) => {
                type_resolver.set(&d);
                // still need it
                reduced_program.push(TopLevelToken::DefType(d).at(pos));
            }
            TopLevelToken::FunctionDeclaration(f_name, f_params) => {
                function_context.add_declaration(f_name, f_params, pos, &type_resolver)?;
            }
            TopLevelToken::SubDeclaration(s_name, s_params) => {
                sub_context.add_declaration(s_name, s_params, pos, &type_resolver)?;
            }
            TopLevelToken::FunctionImplementation(f_name, f_params, f_body) => {
                function_context.add_implementation(
                    f_name,
                    f_params,
                    f_body,
                    pos,
                    &type_resolver,
                )?;
            }
            TopLevelToken::SubImplementation(s_name, s_params, s_body) => {
                sub_context.add_implementation(s_name, s_params, s_body, pos, &type_resolver)?;
            }
            _ => reduced_program.push(top_level_token.at(pos)),
        }
    }
    function_context.ensure_all_declared_programs_are_implemented()?;
    sub_context.ensure_all_declared_programs_are_implemented()?;
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
                    err_pre_process(format!("Unknown SUB {}", n), Location::zero())
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
    call_name: &NameNode,
    func_impl: &QualifiedFunctionImplementationNode,
) -> Result<()> {
    match call_name.opt_qualifier() {
        None => Ok(()),
        Some(q) => {
            if q == func_impl.qualifier() {
                Ok(())
            } else {
                err_pre_process("Duplicate definition", call_name.location())
            }
        }
    }
}

fn check_function_args_on_call(
    call_name: &NameNode,
    args: &Vec<ExpressionNode>,
    func_impl: &QualifiedFunctionImplementationNode,
) -> Result<()> {
    if func_impl.parameters.len() != args.len() {
        err_pre_process("Argument count mismatch", call_name.location())
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
                    let name_node: NameNode = n.clone().at(e_node.location());
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

pub trait NoFunctionInConst {
    fn no_function_in_const(node: &Self) -> Result<()>;
}

impl<T: std::fmt::Debug + Sized + NoFunctionInConst> NoFunctionInConst for Locatable<T> {
    fn no_function_in_const(node: &Self) -> Result<()> {
        T::no_function_in_const(node.as_ref()).map_err(|e| e.at_non_zero_location(node.location()))
    }
}

impl<T: std::fmt::Debug + Sized + NoFunctionInConst> NoFunctionInConst for Vec<T> {
    fn no_function_in_const(block: &Self) -> Result<()> {
        for statement in block {
            T::no_function_in_const(statement)?;
        }
        Ok(())
    }
}

impl NoFunctionInConst for TopLevelToken {
    fn no_function_in_const(top_level_token: &Self) -> Result<()> {
        match top_level_token {
            TopLevelToken::Statement(s) => Statement::no_function_in_const(s),
            TopLevelToken::FunctionImplementation(_, _, b) => {
                StatementNodes::no_function_in_const(b)
            }
            TopLevelToken::SubImplementation(_, _, b) => StatementNodes::no_function_in_const(b),
            _ => Ok(()),
        }
    }
}

impl NoFunctionInConst for Statement {
    fn no_function_in_const(statement: &Self) -> Result<()> {
        match statement {
            Self::ForLoop(f) => StatementNodes::no_function_in_const(&f.statements),
            Self::IfBlock(i) => {
                ConditionalBlockNode::no_function_in_const(&i.if_block)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::no_function_in_const(&else_if_block)?;
                }
                match &i.else_block {
                    Some(x) => StatementNodes::no_function_in_const(x),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::no_function_in_const(w),
            Self::Const(_, right) => ExpressionNode::no_function_in_const(right),
            _ => Ok(()),
        }
    }
}

impl NoFunctionInConst for ConditionalBlockNode {
    fn no_function_in_const(e: &Self) -> Result<()> {
        StatementNodes::no_function_in_const(&e.statements)
    }
}

impl NoFunctionInConst for ExpressionNode {
    fn no_function_in_const(e_node: &Self) -> Result<()> {
        let e: &Expression = e_node.as_ref();
        match e {
            Expression::FunctionCall(_, _) => {
                err_pre_process("Invalid constant", e_node.location())
            }
            Expression::BinaryExpression(_, left, right) => {
                let unboxed_left: &Self = left;
                let unboxed_right: &Self = right;
                Self::no_function_in_const(unboxed_left)?;
                Self::no_function_in_const(unboxed_right)
            }
            Expression::UnaryExpression(_, child) => {
                let unboxed_child: &Self = child;
                Self::no_function_in_const(unboxed_child)
            }
            _ => Ok(()),
        }
    }
}

trait ForNextCounterMatch {
    fn for_next_counter_match<TR: TypeResolver>(node: &Self, resolver: &TR) -> Result<()>;
}

impl<T: std::fmt::Debug + Sized + ForNextCounterMatch> ForNextCounterMatch for Locatable<T> {
    fn for_next_counter_match<TR: TypeResolver>(node: &Self, resolver: &TR) -> Result<()> {
        T::for_next_counter_match(node.as_ref(), resolver)
            .map_err(|e| e.at_non_zero_location(node.location()))
    }
}

impl<T: std::fmt::Debug + Sized + ForNextCounterMatch> ForNextCounterMatch for Vec<T> {
    fn for_next_counter_match<TR: TypeResolver>(block: &Self, resolver: &TR) -> Result<()> {
        for statement in block {
            T::for_next_counter_match(statement, resolver)?;
        }
        Ok(())
    }
}

pub fn for_next_counter_match(program: &ProgramNode) -> Result<()> {
    let mut resolver = TypeResolverImpl::new();
    for top_level_token_node in program {
        match top_level_token_node.as_ref() {
            TopLevelToken::DefType(d) => resolver.set(d),
            _ => TopLevelTokenNode::for_next_counter_match(top_level_token_node, &resolver)?,
        }
    }
    Ok(())
}

impl ForNextCounterMatch for TopLevelToken {
    fn for_next_counter_match<T: TypeResolver>(top_level_token: &Self, resolver: &T) -> Result<()> {
        match top_level_token {
            TopLevelToken::Statement(s) => Statement::for_next_counter_match(s, resolver),
            TopLevelToken::FunctionImplementation(_, _, b) => {
                StatementNodes::for_next_counter_match(b, resolver)
            }
            TopLevelToken::SubImplementation(_, _, b) => {
                StatementNodes::for_next_counter_match(b, resolver)
            }
            TopLevelToken::DefType(d) => panic!("unexpected, should have been handled earlier"),
            _ => Ok(()),
        }
    }
}

impl ForNextCounterMatch for Statement {
    fn for_next_counter_match<T: TypeResolver>(statement: &Self, resolver: &T) -> Result<()> {
        match statement {
            Self::ForLoop(f) => {
                StatementNodes::for_next_counter_match(&f.statements, resolver)?;

                // for and next counters must match
                match &f.next_counter {
                    Some(n) => {
                        if n.eq_resolve(&f.variable_name, resolver) {
                            Ok(())
                        } else {
                            err_pre_process("NEXT without FOR", n.location())
                        }
                    }
                    None => Ok(()),
                }
            }
            Self::IfBlock(i) => {
                ConditionalBlockNode::for_next_counter_match(&i.if_block, resolver)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::for_next_counter_match(&else_if_block, resolver)?;
                }
                match &i.else_block {
                    Some(x) => StatementNodes::for_next_counter_match(x, resolver),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::for_next_counter_match(w, resolver),
            _ => Ok(()),
        }
    }
}

impl ForNextCounterMatch for ConditionalBlockNode {
    fn for_next_counter_match<T: TypeResolver>(c: &Self, resolver: &T) -> Result<()> {
        StatementNodes::for_next_counter_match(&c.statements, resolver)
    }
}
