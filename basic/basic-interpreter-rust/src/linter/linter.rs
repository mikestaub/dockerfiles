// Convert assignment to set return value (needs resolver)
// No function in const
// For - Next match (needs resolver)

// Stage 1 : convert program node into (statements, subprograms)
// all subs known
// all functions known

// Mission: remove the need for TypeResolver in Interpreter

use crate::common::*;
use crate::parser;
use crate::parser::type_resolver_impl::TypeResolverImpl;
use crate::parser::{
    BareName, BareNameNode, HasQualifier, Name, NameNode, NameTrait, Operand, QualifiedName,
    TypeQualifier, TypeResolver, UnaryOperand,
};

use std::collections::{HashMap, HashSet};

//
// Result and error of this module
//

pub type Error = Locatable<String>;
pub type Result<T> = std::result::Result<T, Error>;
fn err<T, S: AsRef<str>>(msg: S, pos: Location) -> Result<T> {
    Err(Locatable::new(format!("[L] {}", msg.as_ref()), pos))
}

//
// Visitor trait
//

/// A visitor visits an object. It might update itself on each visit.
pub trait Visitor<A> {
    fn visit(&mut self, a: &A) -> Result<()>;
}

pub trait PostVisitor<A> {
    fn post_visit(&mut self, a: &A) -> Result<()>;
}

/// Blanket visitor implementation for vectors.
impl<T, A> Visitor<Vec<A>> for T
where
    T: Visitor<A> + PostVisitor<Vec<A>>,
{
    fn visit(&mut self, a: &Vec<A>) -> Result<()> {
        for x in a.iter() {
            self.visit(x)?;
        }
        self.post_visit(a)
    }
}

//
// Pass1 collect declared and implemented functions and subs
//

type ParamTypes = Vec<TypeQualifier>;
type FunctionMap = HashMap<CaseInsensitiveString, (TypeQualifier, ParamTypes, Location)>;

#[derive(Debug, Default)]
struct FunctionContext {
    resolver: TypeResolverImpl,
    declarations: FunctionMap,
    implementations: FunctionMap,
}

impl FunctionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_declaration(
        &mut self,
        name: &NameNode,
        params: &Vec<NameNode>,
        pos: Location,
    ) -> Result<()> {
        // name does not have to be unique (duplicate identical declarations okay)
        // conflicting declarations to previous declaration or implementation not okay
        let q_params: Vec<TypeQualifier> =
            params.iter().map(|p| self.resolver.resolve(p)).collect();
        let q_name: TypeQualifier = self.resolver.resolve(name);
        let bare_name = name.bare_name().clone();
        self.check_implementation_type(&bare_name, &q_name, &q_params, pos)?;
        match self.declarations.get(&bare_name) {
            Some(_) => self.check_declaration_type(&bare_name, &q_name, &q_params, pos),
            None => {
                self.declarations.insert(bare_name, (q_name, q_params, pos));
                Ok(())
            }
        }
    }

    pub fn add_implementation(
        &mut self,
        name: &NameNode,
        params: &Vec<NameNode>,
        pos: Location,
    ) -> Result<()> {
        // type must match declaration
        // param count must match declaration
        // param types must match declaration
        // name needs to be unique
        let q_params: Vec<TypeQualifier> =
            params.iter().map(|p| self.resolver.resolve(p)).collect();
        let q_name: TypeQualifier = self.resolver.resolve(name);
        let bare_name = name.bare_name().clone();
        match self.implementations.get(&bare_name) {
            Some(_) => err("Duplicate definition", pos),
            None => {
                self.check_declaration_type(&bare_name, &q_name, &q_params, pos)?;
                self.implementations
                    .insert(bare_name, (q_name, q_params, pos));
                Ok(())
            }
        }
    }

    fn check_declaration_type(
        &self,
        name: &CaseInsensitiveString,
        q_name: &TypeQualifier,
        q_params: &Vec<TypeQualifier>,
        pos: Location,
    ) -> Result<()> {
        match self.declarations.get(name) {
            Some((e_name, e_params, _)) => {
                if e_name == q_name && e_params == q_params {
                    Ok(())
                } else {
                    err("Type mismatch", pos)
                }
            }
            None => Ok(()),
        }
    }

    fn check_implementation_type(
        &self,
        name: &CaseInsensitiveString,
        q_name: &TypeQualifier,
        q_params: &Vec<TypeQualifier>,
        pos: Location,
    ) -> Result<()> {
        match self.implementations.get(name) {
            Some((e_name, e_params, _)) => {
                if e_name == q_name && e_params == q_params {
                    Ok(())
                } else {
                    err("Type mismatch", pos)
                }
            }
            None => Ok(()),
        }
    }
}

impl Visitor<parser::TopLevelTokenNode> for FunctionContext {
    fn visit(&mut self, a: &parser::TopLevelTokenNode) -> Result<()> {
        let pos = a.location();
        match a.as_ref() {
            parser::TopLevelToken::DefType(d) => {
                self.resolver.set(d);
                Ok(())
            }
            parser::TopLevelToken::FunctionDeclaration(n, params) => {
                self.add_declaration(n, params, pos)
            }
            parser::TopLevelToken::FunctionImplementation(n, params, _) => {
                self.add_implementation(n, params, pos)
            }
            _ => Ok(()),
        }
    }
}

impl PostVisitor<parser::ProgramNode> for FunctionContext {
    fn post_visit(&mut self, _: &parser::ProgramNode) -> Result<()> {
        for (k, v) in self.declarations.iter() {
            if !self.implementations.contains_key(k) {
                return err("Subprogram not defined", v.2);
            }
        }
        Ok(())
    }
}

type SubMap = HashMap<CaseInsensitiveString, (ParamTypes, Location)>;

#[derive(Debug, Default)]
struct SubContext {
    resolver: TypeResolverImpl,
    declarations: SubMap,
    implementations: SubMap,
}

impl SubContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_declaration(
        &mut self,
        name: &CaseInsensitiveString,
        params: &Vec<NameNode>,
        pos: Location,
    ) -> Result<()> {
        // name does not have to be unique (duplicate identical declarations okay)
        // conflicting declarations to previous declaration or implementation not okay
        let q_params: Vec<TypeQualifier> =
            params.iter().map(|p| self.resolver.resolve(p)).collect();
        self.check_implementation_type(name, &q_params, pos)?;
        match self.declarations.get(name) {
            Some(_) => self.check_declaration_type(name, &q_params, pos),
            None => {
                self.declarations.insert(name.clone(), (q_params, pos));
                Ok(())
            }
        }
    }

    pub fn add_implementation(
        &mut self,
        name: &CaseInsensitiveString,
        params: &Vec<NameNode>,
        pos: Location,
    ) -> Result<()> {
        // param count must match declaration
        // param types must match declaration
        // name needs to be unique
        let q_params: Vec<TypeQualifier> =
            params.iter().map(|p| self.resolver.resolve(p)).collect();
        match self.implementations.get(name) {
            Some(_) => err("Duplicate definition", pos),
            None => {
                self.check_declaration_type(name, &q_params, pos)?;
                self.implementations.insert(name.clone(), (q_params, pos));
                Ok(())
            }
        }
    }

    fn check_declaration_type(
        &self,
        name: &CaseInsensitiveString,
        q_params: &Vec<TypeQualifier>,
        pos: Location,
    ) -> Result<()> {
        match self.declarations.get(name) {
            Some((e_params, _)) => {
                if e_params == q_params {
                    Ok(())
                } else {
                    err("Type mismatch", pos)
                }
            }
            None => Ok(()),
        }
    }

    fn check_implementation_type(
        &self,
        name: &CaseInsensitiveString,
        q_params: &Vec<TypeQualifier>,
        pos: Location,
    ) -> Result<()> {
        match self.implementations.get(name) {
            Some((e_params, _)) => {
                if e_params == q_params {
                    Ok(())
                } else {
                    err("Type mismatch", pos)
                }
            }
            None => Ok(()),
        }
    }
}

impl Visitor<parser::TopLevelTokenNode> for SubContext {
    fn visit(&mut self, a: &parser::TopLevelTokenNode) -> Result<()> {
        let pos = a.location();
        match a.as_ref() {
            parser::TopLevelToken::DefType(d) => {
                self.resolver.set(d);
                Ok(())
            }
            parser::TopLevelToken::SubDeclaration(n, params) => {
                self.add_declaration(n.as_ref(), params, pos)
            }
            parser::TopLevelToken::SubImplementation(n, params, _) => {
                self.add_implementation(n.as_ref(), params, pos)
            }
            _ => Ok(()),
        }
    }
}

impl PostVisitor<parser::ProgramNode> for SubContext {
    fn post_visit(&mut self, _: &parser::ProgramNode) -> Result<()> {
        for (k, v) in self.declarations.iter() {
            if !self.implementations.contains_key(k) {
                return err("Missing implementation", v.1);
            }
        }
        Ok(())
    }
}

/// Collects subprograms of the given program.
/// Ensures that:
/// - All declared subprograms are implemented
/// - No duplicate implementations
/// - No conflicts between declarations and implementations
/// - Resolves types of parameters and functions
fn collect_subprograms(p: &parser::ProgramNode) -> Result<(FunctionMap, SubMap)> {
    let mut f_c = FunctionContext::new();
    f_c.visit(p)?;
    let mut s_c = SubContext::new();
    s_c.visit(p)?;
    Ok((f_c.implementations, s_c.implementations))
}

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
        let mut result: Vec<B> = vec![];
        for x in a {
            result.push(self.convert(x)?);
        }
        Ok(result)
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

    pub fn get_constant_type_recursively(&self, n: &parser::Name) -> Result<Option<TypeQualifier>> {
        match self.get_constant_type(n)? {
            Some(q) => Ok(Some(q)),
            None => match &self.parent {
                Some(p) => p.get_constant_type_recursively(n),
                None => Ok(None),
            },
        }
    }
}

#[derive(Debug, Default)]
pub struct Linter {
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

    pub fn resolve_expression_type(&self, e_node: &ExpressionNode) -> Result<TypeQualifier> {
        let pos = e_node.location();
        let e: &Expression = e_node.as_ref();
        match e {
            Expression::SingleLiteral(_) => Ok(TypeQualifier::BangSingle),
            Expression::DoubleLiteral(_) => Ok(TypeQualifier::HashDouble),
            Expression::StringLiteral(_) => Ok(TypeQualifier::DollarString),
            Expression::IntegerLiteral(_) => Ok(TypeQualifier::PercentInteger),
            Expression::LongLiteral(_) => Ok(TypeQualifier::AmpersandLong),
            Expression::Variable(name)
            | Expression::Constant(name)
            | Expression::FunctionCall(name, _) => Ok(name.qualifier()),
            Expression::BinaryExpression(op, l, r) => {
                let q_left = self.resolve_expression_type(l)?;
                let q_right = self.resolve_expression_type(r)?;
                if q_left.can_cast_to(q_right) {
                    match op {
                        Operand::Plus | Operand::Minus => Ok(q_left),
                        Operand::LessThan | Operand::LessOrEqualThan => {
                            Ok(TypeQualifier::PercentInteger)
                        }
                    }
                } else {
                    err("Type mismatch", pos)
                }
            }
            Expression::UnaryExpression(op, c) => {
                let q_child = self.resolve_expression_type(c)?;
                if q_child == TypeQualifier::DollarString {
                    // no unary operator currently applicable to strings
                    err("Type mismatch", pos)
                } else {
                    Ok(q_child)
                }
            }
        }
    }
}

pub type QNameNode = Locatable<QualifiedName>;

#[derive(Clone, Debug, PartialEq)]
pub enum Expression {
    SingleLiteral(f32),
    DoubleLiteral(f64),
    StringLiteral(String),
    IntegerLiteral(i32),
    #[allow(dead_code)]
    LongLiteral(i64),
    Constant(QualifiedName),
    Variable(QualifiedName),
    FunctionCall(QualifiedName, Vec<ExpressionNode>),
    BinaryExpression(Operand, Box<ExpressionNode>, Box<ExpressionNode>),
    UnaryExpression(UnaryOperand, Box<ExpressionNode>),
}

pub type ExpressionNode = Locatable<Expression>;

#[derive(Clone, Debug, PartialEq)]
pub struct ForLoopNode {
    pub variable_name: QNameNode,
    pub lower_bound: ExpressionNode,
    pub upper_bound: ExpressionNode,
    pub step: Option<ExpressionNode>,
    pub statements: StatementNodes,
    pub next_counter: Option<QNameNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConditionalBlockNode {
    pub condition: ExpressionNode,
    pub statements: StatementNodes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IfBlockNode {
    pub if_block: ConditionalBlockNode,
    pub else_if_blocks: Vec<ConditionalBlockNode>,
    pub else_block: Option<StatementNodes>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    SubCall(BareName, Vec<ExpressionNode>),
    ForLoop(ForLoopNode),
    IfBlock(IfBlockNode),
    Assignment(QualifiedName, ExpressionNode),
    While(ConditionalBlockNode),
    Const(QNameNode, ExpressionNode),
    ErrorHandler(CaseInsensitiveString),
    Label(CaseInsensitiveString),
    GoTo(CaseInsensitiveString),
    SetReturnValue(ExpressionNode),
}

pub type StatementNode = Locatable<Statement>;
pub type StatementNodes = Vec<StatementNode>;

#[derive(Clone, Debug, PartialEq)]
pub enum TopLevelToken {
    /// A function implementation
    FunctionImplementation(QNameNode, Vec<QNameNode>, StatementNodes),

    /// A simple or compound statement
    Statement(Statement),

    /// A sub implementation
    SubImplementation(BareNameNode, Vec<QNameNode>, StatementNodes),
}

pub type TopLevelTokenNode = Locatable<TopLevelToken>;
pub type ProgramNode = Vec<TopLevelTokenNode>;

pub fn lint(program: parser::ProgramNode) -> Result<ProgramNode> {
    let mut linter = Linter::default();
    linter.convert(program)
}

impl Converter<parser::ProgramNode, ProgramNode> for Linter {
    fn convert(&mut self, a: parser::ProgramNode) -> Result<ProgramNode> {
        let (f, s) = collect_subprograms(&a)?;
        self.functions = f;
        self.subs = s;

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

        NoDynamicConst::no_dynamic_const(&result)?;
        ForNextCounterMatch::for_next_counter_match(&result)?;

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
                let mapped = TopLevelToken::FunctionImplementation(
                    mapped_name,
                    mapped_params,
                    self.convert(block)?,
                );
                self.pop_context();
                Ok(Some(mapped))
            }
            parser::TopLevelToken::SubImplementation(n, params, block) => {
                let mapped_params = self.convert(params)?;
                self.push_sub_context(n.bare_name());
                for q_n_n in mapped_params.iter() {
                    self.context.variables.insert(q_n_n.bare_name().clone());
                }
                let mapped =
                    TopLevelToken::SubImplementation(n, mapped_params, self.convert(block)?);
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
                    let e_type = self.resolve_expression_type(&converted_expression_node)?;
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
                // types match?
                Ok(Expression::BinaryExpression(
                    op,
                    self.convert(l)?,
                    self.convert(r)?,
                ))
            }
            parser::Expression::UnaryExpression(op, c) => {
                // is it a legal op? e.g. -"hello" isn't
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

pub trait NoDynamicConst {
    fn no_dynamic_const(node: &Self) -> Result<()>;
}

impl<T: std::fmt::Debug + Sized + NoDynamicConst> NoDynamicConst for Locatable<T> {
    fn no_dynamic_const(node: &Self) -> Result<()> {
        T::no_dynamic_const(node.as_ref()).map_err(|e| e.at_non_zero_location(node.location()))
    }
}

impl<T: std::fmt::Debug + Sized + NoDynamicConst> NoDynamicConst for Vec<T> {
    fn no_dynamic_const(block: &Self) -> Result<()> {
        for statement in block {
            T::no_dynamic_const(statement)?;
        }
        Ok(())
    }
}

impl NoDynamicConst for TopLevelToken {
    fn no_dynamic_const(top_level_token: &Self) -> Result<()> {
        match top_level_token {
            TopLevelToken::Statement(s) => Statement::no_dynamic_const(s),
            TopLevelToken::FunctionImplementation(_, _, b) => StatementNodes::no_dynamic_const(b),
            TopLevelToken::SubImplementation(_, _, b) => StatementNodes::no_dynamic_const(b),
        }
    }
}

impl NoDynamicConst for Statement {
    fn no_dynamic_const(statement: &Self) -> Result<()> {
        match statement {
            Self::ForLoop(f) => StatementNodes::no_dynamic_const(&f.statements),
            Self::IfBlock(i) => {
                ConditionalBlockNode::no_dynamic_const(&i.if_block)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::no_dynamic_const(&else_if_block)?;
                }
                match &i.else_block {
                    Some(x) => StatementNodes::no_dynamic_const(x),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::no_dynamic_const(w),
            Self::Const(_, right) => ExpressionNode::no_dynamic_const(right),
            _ => Ok(()),
        }
    }
}

impl NoDynamicConst for ConditionalBlockNode {
    fn no_dynamic_const(e: &Self) -> Result<()> {
        StatementNodes::no_dynamic_const(&e.statements)
    }
}

impl NoDynamicConst for ExpressionNode {
    fn no_dynamic_const(e_node: &Self) -> Result<()> {
        let e: &Expression = e_node.as_ref();
        match e {
            Expression::FunctionCall(_, _) | Expression::Variable(_) => {
                err("Invalid constant", e_node.location())
            }
            Expression::BinaryExpression(_, left, right) => {
                let unboxed_left: &Self = left;
                let unboxed_right: &Self = right;
                Self::no_dynamic_const(unboxed_left)?;
                Self::no_dynamic_const(unboxed_right)
            }
            Expression::UnaryExpression(_, child) => {
                let unboxed_child: &Self = child;
                Self::no_dynamic_const(unboxed_child)
            }
            _ => Ok(()),
        }
    }
}

pub trait ForNextCounterMatch {
    fn for_next_counter_match(node: &Self) -> Result<()>;
}

impl<T: std::fmt::Debug + Sized + ForNextCounterMatch> ForNextCounterMatch for Locatable<T> {
    fn for_next_counter_match(node: &Self) -> Result<()> {
        T::for_next_counter_match(node.as_ref())
            .map_err(|e| e.at_non_zero_location(node.location()))
    }
}

impl<T: std::fmt::Debug + Sized + ForNextCounterMatch> ForNextCounterMatch for Vec<T> {
    fn for_next_counter_match(block: &Self) -> Result<()> {
        for statement in block {
            T::for_next_counter_match(statement)?;
        }
        Ok(())
    }
}

impl ForNextCounterMatch for TopLevelToken {
    fn for_next_counter_match(top_level_token: &Self) -> Result<()> {
        match top_level_token {
            TopLevelToken::Statement(s) => Statement::for_next_counter_match(s),
            TopLevelToken::FunctionImplementation(_, _, b) => {
                StatementNodes::for_next_counter_match(b)
            }
            TopLevelToken::SubImplementation(_, _, b) => StatementNodes::for_next_counter_match(b),
        }
    }
}

impl ForNextCounterMatch for Statement {
    fn for_next_counter_match(statement: &Self) -> Result<()> {
        match statement {
            Self::ForLoop(f) => ForLoopNode::for_next_counter_match(f),
            Self::IfBlock(i) => {
                ConditionalBlockNode::for_next_counter_match(&i.if_block)?;
                for else_if_block in &i.else_if_blocks {
                    ConditionalBlockNode::for_next_counter_match(&else_if_block)?;
                }
                match &i.else_block {
                    Some(x) => StatementNodes::for_next_counter_match(x),
                    None => Ok(()),
                }
            }
            Self::While(w) => ConditionalBlockNode::for_next_counter_match(w),
            _ => Ok(()),
        }
    }
}

impl ForNextCounterMatch for ForLoopNode {
    fn for_next_counter_match(f: &Self) -> Result<()> {
        StatementNodes::for_next_counter_match(&f.statements)?;

        // for and next counters must match
        match &f.next_counter {
            Some(n) => {
                let next_var_name: &QualifiedName = n.as_ref();
                if next_var_name == f.variable_name.as_ref() {
                    Ok(())
                } else {
                    err("NEXT without FOR", n.location())
                }
            }
            None => Ok(()),
        }
    }
}

impl ForNextCounterMatch for ConditionalBlockNode {
    fn for_next_counter_match(c: &Self) -> Result<()> {
        StatementNodes::for_next_counter_match(&c.statements)
    }
}
