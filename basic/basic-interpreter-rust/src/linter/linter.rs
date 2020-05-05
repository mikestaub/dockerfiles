// Convert assignment to set return value (needs resolver)
// No function in const
// For - Next match (needs resolver)

// Stage 1 : convert program node into (statements, subprograms)
// all subs known
// all functions known

// Mission: remove the need for TypeResolver in Interpreter

use super::error::*;
use super::post_conversion_linter::PostConversionLinter;
use super::subprogram_context::{collect_subprograms, FunctionMap, SubMap};
use super::types::*;
use crate::common::*;
use crate::parser;
use crate::parser::type_resolver_impl::TypeResolverImpl;
use crate::parser::{HasQualifier, Name, NameTrait, QualifiedName, TypeQualifier, TypeResolver};
use std::collections::{HashMap, HashSet};

//
// Converter trait
//

trait Converter<A, B> {
    fn convert(&mut self, a: A) -> Result<B>;
}

// blanket for Vec
impl<T, A, B> Converter<Vec<A>, Vec<B>> for T
where
    T: Converter<A, B>,
{
    fn convert(&mut self, a: Vec<A>) -> Result<Vec<B>> {
        a.into_iter().map(|x| self.convert(x)).collect()
    }
}

// blanket for Option
impl<T, A, B> Converter<Option<A>, Option<B>> for T
where
    T: Converter<A, B>,
{
    fn convert(&mut self, a: Option<A>) -> Result<Option<B>> {
        match a {
            Some(x) => self.convert(x).map(|r| Some(r)),
            None => Ok(None),
        }
    }
}

// blanket for Box
impl<T, A, B> Converter<Box<A>, Box<B>> for T
where
    T: Converter<A, B>,
{
    fn convert(&mut self, a: Box<A>) -> Result<Box<B>> {
        let unboxed_a: A = *a;
        self.convert(unboxed_a).map(|unboxed_b| Box::new(unboxed_b))
    }
}

// blanket for Locatable
impl<T, A, B> Converter<Locatable<A>, Locatable<B>> for T
where
    A: std::fmt::Debug + Sized,
    B: std::fmt::Debug + Sized,
    T: Converter<A, B>,
{
    fn convert(&mut self, a: Locatable<A>) -> Result<Locatable<B>> {
        let (element, pos) = a.consume();
        self.convert(element)
            .map(|x| x.at(pos))
            .map_err(|e| e.at_non_zero_location(pos))
    }
}

//
// Linter
//

#[derive(Debug, Default)]
struct LinterContext {
    parent: Option<Box<LinterContext>>,
    constants: HashMap<CaseInsensitiveString, TypeQualifier>,
    variables: HashSet<CaseInsensitiveString>,
    function_name: Option<CaseInsensitiveString>,
    sub_name: Option<CaseInsensitiveString>,
}

impl LinterContext {
    pub fn get_constant_type(&self, n: &parser::Name) -> Result<Option<TypeQualifier>> {
        let bare_name: &CaseInsensitiveString = n.bare_name();
        match self.constants.get(bare_name) {
            Some(const_type) => {
                // it's okay to reference a const unqualified
                if n.bare_or_eq(*const_type) {
                    Ok(Some(*const_type))
                } else {
                    err("Duplicate definition", Location::zero())
                }
            }
            None => Ok(None),
        }
    }

    pub fn get_parent_constant_type(&self, n: &parser::Name) -> Result<Option<TypeQualifier>> {
        match &self.parent {
            Some(p) => {
                let x = p.get_constant_type(n)?;
                match x {
                    Some(q) => Ok(Some(q)),
                    None => p.get_parent_constant_type(n),
                }
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug, Default)]
struct Linter {
    resolver: TypeResolverImpl,
    context: LinterContext,
    functions: FunctionMap,
    subs: SubMap,
}

impl Linter {
    pub fn push_function_context(&mut self, name: &CaseInsensitiveString) {
        let old = std::mem::take(&mut self.context);
        let mut new = LinterContext::default();
        new.parent = Some(Box::new(old));
        new.function_name = Some(name.clone());
        self.context = new;
    }

    pub fn push_sub_context(&mut self, name: &CaseInsensitiveString) {
        let old = std::mem::take(&mut self.context);
        let mut new = LinterContext::default();
        new.parent = Some(Box::new(old));
        new.sub_name = Some(name.clone());
        self.context = new;
    }

    pub fn pop_context(&mut self) {
        let old = std::mem::take(&mut self.context);
        match old.parent {
            Some(p) => {
                self.context = *p;
            }
            None => panic!("Stack underflow!"),
        }
    }
}

pub fn lint(program: parser::ProgramNode) -> Result<ProgramNode> {
    let mut linter = Linter::default();
    let (f_c, s_c) = collect_subprograms(&program)?;
    linter.functions = f_c;
    linter.subs = s_c;
    linter.convert(program)
}

impl Converter<parser::ProgramNode, ProgramNode> for Linter {
    fn convert(&mut self, a: parser::ProgramNode) -> Result<ProgramNode> {
        let mut result: Vec<TopLevelTokenNode> = vec![];
        for top_level_token_node in a.into_iter() {
            // will contain None where DefInt and declarations used to be
            let (top_level_token, pos) = top_level_token_node.consume();
            let opt: Option<TopLevelToken> = self
                .convert(top_level_token)
                .map_err(|e| e.at_non_zero_location(pos))?;
            match opt {
                Some(t) => {
                    let r: TopLevelTokenNode = t.at(pos);
                    result.push(r);
                }
                _ => (),
            }
        }

        let linter = super::no_dynamic_const::NoDynamicConst {};
        linter.visit_program(&result)?;
        let linter = super::for_next_counter_match::ForNextCounterMatch {};
        linter.visit_program(&result)?;
        let linter = super::built_in_function_linter::BuiltInFunctionLinter {};
        linter.visit_program(&result)?;
        let linter = super::built_in_sub_linter::BuiltInSubLinter {};
        linter.visit_program(&result)?;

        Ok(result)
    }
}

impl Converter<Name, QualifiedName> for Linter {
    fn convert(&mut self, a: Name) -> Result<QualifiedName> {
        match a {
            Name::Bare(b) => {
                let qualifier = self.resolver.resolve(&b);
                Ok(QualifiedName::new(b, qualifier))
            }
            Name::Qualified(q) => Ok(q),
        }
    }
}

// Option because we filter out DefType
impl Converter<parser::TopLevelToken, Option<TopLevelToken>> for Linter {
    fn convert(&mut self, a: parser::TopLevelToken) -> Result<Option<TopLevelToken>> {
        match a {
            parser::TopLevelToken::DefType(d) => {
                self.resolver.set(&d);
                Ok(None)
            }
            parser::TopLevelToken::FunctionDeclaration(_, _)
            | parser::TopLevelToken::SubDeclaration(_, _) => Ok(None),
            parser::TopLevelToken::FunctionImplementation(n, params, block) => {
                let mapped_name = self.convert(n)?;
                let mapped_params = self.convert(params)?;
                self.push_function_context(mapped_name.bare_name());
                for q_n_n in mapped_params.iter() {
                    self.context.variables.insert(q_n_n.bare_name().clone());
                }
                let mapped = TopLevelToken::FunctionImplementation(FunctionImplementation {
                    name: mapped_name,
                    params: mapped_params,
                    body: self.convert(block)?,
                });
                self.pop_context();
                Ok(Some(mapped))
            }
            parser::TopLevelToken::SubImplementation(n, params, block) => {
                let mapped_params = self.convert(params)?;
                self.push_sub_context(n.bare_name());
                for q_n_n in mapped_params.iter() {
                    self.context.variables.insert(q_n_n.bare_name().clone());
                }
                let mapped = TopLevelToken::SubImplementation(SubImplementation {
                    name: n,
                    params: mapped_params,
                    body: self.convert(block)?,
                });
                self.pop_context();
                Ok(Some(mapped))
            }
            parser::TopLevelToken::Statement(s) => {
                Ok(Some(TopLevelToken::Statement(self.convert(s)?)))
            }
        }
    }
}

impl Converter<parser::Statement, Statement> for Linter {
    fn convert(&mut self, a: parser::Statement) -> Result<Statement> {
        match a {
            parser::Statement::SubCall(n, args) => Ok(Statement::SubCall(n, self.convert(args)?)),
            parser::Statement::ForLoop(f) => Ok(Statement::ForLoop(self.convert(f)?)),
            parser::Statement::IfBlock(i) => Ok(Statement::IfBlock(self.convert(i)?)),
            parser::Statement::Assignment(n, e) => {
                if self
                    .context
                    .function_name
                    .as_ref()
                    .map(|x| x == n.bare_name())
                    .unwrap_or_default()
                {
                    // trying to assign to the function
                    let function_type: TypeQualifier = self.functions.get(n.bare_name()).unwrap().0;
                    if n.bare_or_eq(function_type) {
                        // TODO check if casting is possible
                        Ok(Statement::SetReturnValue(self.convert(e)?))
                    } else {
                        err("Duplicate definition", Location::zero())
                    }
                } else if self
                    .context
                    .sub_name
                    .as_ref()
                    .map(|x| x == n.bare_name())
                    .unwrap_or_default()
                {
                    // trying to assign to the sub name should always be an error hopefully
                    err("Cannot assign to sub", Location::zero())
                } else {
                    if self.context.constants.contains_key(n.bare_name()) {
                        // cannot overwrite local constant
                        err("Duplicate definition", Location::zero())
                    } else {
                        // TODO check if casting is possible
                        self.context.variables.insert(n.bare_name().clone());
                        Ok(Statement::Assignment(self.convert(n)?, self.convert(e)?))
                    }
                }
            }
            parser::Statement::While(c) => Ok(Statement::While(self.convert(c)?)),
            parser::Statement::Const(n, e) => {
                let (name, pos) = n.consume();
                if self.context.variables.contains(name.bare_name())
                    || self.context.constants.contains_key(name.bare_name())
                {
                    // local variable or local constant already present by that name
                    err("Duplicate definition", pos)
                } else {
                    let converted_expression_node = self.convert(e)?;
                    let e_type = converted_expression_node.as_ref().try_qualifier()?;
                    match name {
                        Name::Bare(b) => {
                            // bare name resolves from right side, not resolver
                            self.context.constants.insert(b.clone(), e_type);
                            Ok(Statement::Const(
                                QualifiedName::new(b, e_type).at(pos),
                                converted_expression_node,
                            ))
                        }
                        Name::Qualified(q) => {
                            if e_type.can_cast_to(q.qualifier()) {
                                self.context
                                    .constants
                                    .insert(q.bare_name().clone(), q.qualifier());
                                Ok(Statement::Const(q.at(pos), converted_expression_node))
                            } else {
                                err("Type mismatch", converted_expression_node.location())
                            }
                        }
                    }
                }
            }
            parser::Statement::ErrorHandler(l) => Ok(Statement::ErrorHandler(l)),
            parser::Statement::Label(l) => Ok(Statement::Label(l)),
            parser::Statement::GoTo(l) => Ok(Statement::GoTo(l)),
        }
    }
}

impl Converter<parser::Expression, Expression> for Linter {
    fn convert(&mut self, a: parser::Expression) -> Result<Expression> {
        match a {
            parser::Expression::SingleLiteral(f) => Ok(Expression::SingleLiteral(f)),
            parser::Expression::DoubleLiteral(f) => Ok(Expression::DoubleLiteral(f)),
            parser::Expression::StringLiteral(f) => Ok(Expression::StringLiteral(f)),
            parser::Expression::IntegerLiteral(f) => Ok(Expression::IntegerLiteral(f)),
            parser::Expression::LongLiteral(f) => Ok(Expression::LongLiteral(f)),
            parser::Expression::VariableName(n) => {
                // check for a local constant
                match self.context.get_constant_type(&n)? {
                    Some(q) => Ok(Expression::Constant(QualifiedName::new(
                        n.bare_name().clone(),
                        q,
                    ))),
                    None => {
                        // check for an already defined local variable or parameter
                        // TODO: type might be important, but it is ignored on the next check
                        if self.context.variables.contains(n.bare_name()) {
                            Ok(Expression::Variable(self.convert(n)?))
                        } else {
                            // parent constant?
                            match self.context.get_parent_constant_type(&n)? {
                                Some(q) => Ok(Expression::Constant(QualifiedName::new(
                                    n.bare_name().clone(),
                                    q,
                                ))),
                                None => {
                                    // e.g. INPUT N, where N has not been declared in advance
                                    // TODO: register N as a variable?
                                    Ok(Expression::Variable(self.convert(n)?))
                                }
                            }
                        }
                    }
                }
            }
            parser::Expression::FunctionCall(n, args) => {
                // validate arg count, arg types, name type
                // for built-in and for user-defined
                // for undefined, resolve to literal 0, as long as the arguments do not contain a string
                Ok(Expression::FunctionCall(
                    self.convert(n)?,
                    self.convert(args)?,
                ))
            }
            parser::Expression::BinaryExpression(op, l, r) => {
                // TODO types match?
                Ok(Expression::BinaryExpression(
                    op,
                    self.convert(l)?,
                    self.convert(r)?,
                ))
            }
            parser::Expression::UnaryExpression(op, c) => {
                // TODO is it a legal op? e.g. -"hello" isn't
                Ok(Expression::UnaryExpression(op, self.convert(c)?))
            }
        }
    }
}

impl Converter<parser::ForLoopNode, ForLoopNode> for Linter {
    fn convert(&mut self, a: parser::ForLoopNode) -> Result<ForLoopNode> {
        Ok(ForLoopNode {
            variable_name: self.convert(a.variable_name)?,
            lower_bound: self.convert(a.lower_bound)?,
            upper_bound: self.convert(a.upper_bound)?,
            step: self.convert(a.step)?,
            statements: self.convert(a.statements)?,
            next_counter: self.convert(a.next_counter)?,
        })
    }
}

impl Converter<parser::ConditionalBlockNode, ConditionalBlockNode> for Linter {
    fn convert(&mut self, a: parser::ConditionalBlockNode) -> Result<ConditionalBlockNode> {
        Ok(ConditionalBlockNode {
            condition: self.convert(a.condition)?,
            statements: self.convert(a.statements)?,
        })
    }
}

impl Converter<parser::IfBlockNode, IfBlockNode> for Linter {
    fn convert(&mut self, a: parser::IfBlockNode) -> Result<IfBlockNode> {
        Ok(IfBlockNode {
            if_block: self.convert(a.if_block)?,
            else_if_blocks: self.convert(a.else_if_blocks)?,
            else_block: self.convert(a.else_block)?,
        })
    }
}
