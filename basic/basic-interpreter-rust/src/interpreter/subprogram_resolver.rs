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
    let mut reduced_program = vec![];
    let mut type_resolver = TypeResolverImpl::new();

    for top_level_token in program {
        match top_level_token {
            TopLevelTokenNode::DefType(d, pos) => {
                type_resolver.set(&d);
                // still need it
                reduced_program.push(TopLevelTokenNode::DefType(d, pos));
            }
            TopLevelTokenNode::FunctionDeclaration(f_name, f_params, f_pos) => {
                function_context.add_declaration(f_name, f_params, f_pos, &type_resolver)?;
            }
            TopLevelTokenNode::SubDeclaration(s_name, s_params, s_pos) => {
                sub_context.add_declaration(s_name, s_params, s_pos, &type_resolver)?;
            }
            TopLevelTokenNode::FunctionImplementation(f_name, f_params, f_body, f_pos) => {
                function_context.add_implementation(
                    f_name,
                    f_params,
                    f_body,
                    f_pos,
                    &type_resolver,
                )?;
            }
            TopLevelTokenNode::SubImplementation(s_name, s_params, s_body, s_pos) => {
                sub_context.add_implementation(s_name, s_params, s_body, s_pos, &type_resolver)?;
            }
            _ => reduced_program.push(top_level_token),
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

impl AllSubsKnown for ProgramNode {
    fn all_subs_known(program: &Self, sub_context: &SubContext) -> Result<()> {
        for top_level_token in program {
            match top_level_token {
                TopLevelTokenNode::Statement(s) => StatementNode::all_subs_known(s, sub_context)?,
                _ => (),
            }
        }
        Ok(())
    }
}

impl AllSubsKnown for BlockNode {
    fn all_subs_known(block: &Self, sub_context: &SubContext) -> Result<()> {
        for statement in block {
            StatementNode::all_subs_known(statement, sub_context)?;
        }
        Ok(())
    }
}

impl AllSubsKnown for StatementNode {
    fn all_subs_known(statement: &Self, sub_context: &SubContext) -> Result<()> {
        match statement {
            StatementNode::SubCall(n, _) => {
                // TODO validate argument count and type if possible
                if built_in_subs::is_built_in_sub(n) || sub_context.has_implementation(n.as_ref()) {
                    Ok(())
                } else {
                    err_pre_process(format!("Unknown SUB {}", n.as_ref()), n.location())
                }
            }
            StatementNode::ForLoop(f) => BlockNode::all_subs_known(&f.statements, sub_context),
            StatementNode::IfBlock(i) => {
                BlockNode::all_subs_known(&i.if_block.statements, sub_context)?;
                for else_if_block in &i.else_if_blocks {
                    BlockNode::all_subs_known(&else_if_block.statements, sub_context)?;
                }
                match &i.else_block {
                    Some(x) => BlockNode::all_subs_known(x, sub_context),
                    None => Ok(()),
                }
            }
            StatementNode::While(w) => BlockNode::all_subs_known(&w.statements, sub_context),
            _ => Ok(()),
        }
    }
}

pub trait AllFunctionsKnown {
    fn all_functions_known(node: &Self, function_context: &FunctionContext) -> Result<()>;
}

impl AllFunctionsKnown for ProgramNode {
    fn all_functions_known(program: &Self, function_context: &FunctionContext) -> Result<()> {
        for top_level_token in program {
            match top_level_token {
                TopLevelTokenNode::Statement(s) => {
                    StatementNode::all_functions_known(s, function_context)?
                }
                _ => (),
            }
        }
        Ok(())
    }
}

impl AllFunctionsKnown for BlockNode {
    fn all_functions_known(block: &Self, function_context: &FunctionContext) -> Result<()> {
        for statement in block {
            StatementNode::all_functions_known(statement, function_context)?;
        }
        Ok(())
    }
}

impl AllFunctionsKnown for StatementNode {
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
                BlockNode::all_functions_known(&f.statements, function_context)
            }
            Self::IfBlock(i) => {
                ConditionalBlockNode::all_functions_known(&i.if_block, function_context)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::all_functions_known(&else_if_block, function_context)?;
                }
                match &i.else_block {
                    Some(x) => BlockNode::all_functions_known(x, function_context),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::all_functions_known(w, function_context),
            Self::Assignment(_, right) => {
                ExpressionNode::all_functions_known(right, function_context)
            }
            Self::Const(_, right, _) => {
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
        BlockNode::all_functions_known(&e.statements, function_context)
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
    fn all_functions_known(e: &Self, function_context: &FunctionContext) -> Result<()> {
        match e {
            Self::FunctionCall(n, args) => {
                for a in args {
                    Self::all_functions_known(a, function_context)?;
                }

                if built_in_functions::is_built_in_function(n) {
                    // TODO: validate ENVIRON$
                    Ok(())
                } else if function_context.has_implementation(n.as_ref()) {
                    let func_impl = function_context.get_implementation_ref(n.as_ref()).unwrap();
                    check_function_return_type_on_call(n, func_impl)?;
                    check_function_args_on_call(n, args, func_impl)
                } else {
                    // Unknown function, will fallback to undefined behavior later
                    Ok(())
                }
            }
            Self::BinaryExpression(_, left, right) => {
                let unboxed_left: &Self = left;
                let unboxed_right: &Self = right;
                Self::all_functions_known(unboxed_left, function_context)?;
                Self::all_functions_known(unboxed_right, function_context)
            }
            Self::UnaryExpression(_, child) => {
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

impl NoFunctionInConst for ProgramNode {
    fn no_function_in_const(program: &Self) -> Result<()> {
        for top_level_token in program {
            match top_level_token {
                TopLevelTokenNode::Statement(s) => StatementNode::no_function_in_const(s)?,
                TopLevelTokenNode::FunctionImplementation(_, _, b, _) => {
                    BlockNode::no_function_in_const(b)?
                }
                TopLevelTokenNode::SubImplementation(_, _, b, _) => {
                    BlockNode::no_function_in_const(b)?
                }
                _ => (),
            }
        }
        Ok(())
    }
}

impl NoFunctionInConst for BlockNode {
    fn no_function_in_const(block: &Self) -> Result<()> {
        for statement in block {
            StatementNode::no_function_in_const(statement)?;
        }
        Ok(())
    }
}

impl NoFunctionInConst for StatementNode {
    fn no_function_in_const(statement: &Self) -> Result<()> {
        match statement {
            Self::ForLoop(f) => BlockNode::no_function_in_const(&f.statements),
            Self::IfBlock(i) => {
                ConditionalBlockNode::no_function_in_const(&i.if_block)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::no_function_in_const(&else_if_block)?;
                }
                match &i.else_block {
                    Some(x) => BlockNode::no_function_in_const(x),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::no_function_in_const(w),
            Self::Const(_, right, _) => ExpressionNode::no_function_in_const(right),
            _ => Ok(()),
        }
    }
}

impl NoFunctionInConst for ConditionalBlockNode {
    fn no_function_in_const(e: &Self) -> Result<()> {
        BlockNode::no_function_in_const(&e.statements)
    }
}

impl NoFunctionInConst for ExpressionNode {
    fn no_function_in_const(e: &Self) -> Result<()> {
        match e {
            Self::FunctionCall(_, _) => err_pre_process("Invalid constant", e.location()),
            Self::BinaryExpression(_, left, right) => {
                let unboxed_left: &Self = left;
                let unboxed_right: &Self = right;
                Self::no_function_in_const(unboxed_left)?;
                Self::no_function_in_const(unboxed_right)
            }
            Self::UnaryExpression(_, child) => {
                let unboxed_child: &Self = child;
                Self::no_function_in_const(unboxed_child)
            }
            _ => Ok(()),
        }
    }
}

trait ForNextCounterMatch {
    fn for_next_counter_match<T: TypeResolver>(node: &Self, resolver: &T) -> Result<()>;
}

pub fn for_next_counter_match(program: &ProgramNode) -> Result<()> {
    let mut resolver = TypeResolverImpl::new();
    for top_level_token in program {
        match top_level_token {
            TopLevelTokenNode::Statement(s) => StatementNode::for_next_counter_match(s, &resolver)?,
            TopLevelTokenNode::FunctionImplementation(_, _, b, _) => {
                BlockNode::for_next_counter_match(b, &resolver)?
            }
            TopLevelTokenNode::SubImplementation(_, _, b, _) => {
                BlockNode::for_next_counter_match(b, &resolver)?
            }
            TopLevelTokenNode::DefType(d, _) => resolver.set(d),
            _ => (),
        }
    }
    Ok(())
}

impl ForNextCounterMatch for BlockNode {
    fn for_next_counter_match<T: TypeResolver>(block: &Self, resolver: &T) -> Result<()> {
        for statement in block {
            StatementNode::for_next_counter_match(statement, resolver)?;
        }
        Ok(())
    }
}

impl ForNextCounterMatch for StatementNode {
    fn for_next_counter_match<T: TypeResolver>(statement: &Self, resolver: &T) -> Result<()> {
        match statement {
            Self::ForLoop(f) => {
                BlockNode::for_next_counter_match(&f.statements, resolver)?;

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
                    Some(x) => BlockNode::for_next_counter_match(x, resolver),
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
        BlockNode::for_next_counter_match(&c.statements, resolver)
    }
}
