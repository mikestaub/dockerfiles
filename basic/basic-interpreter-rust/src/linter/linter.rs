// Convert assignment to set return value (needs resolver)
// No function in const
// For - Next match (needs resolver)

// Stage 1 : convert program node into (statements, subprograms)
// all subs known
// all functions known

// Mission: remove the need for TypeResolver in Interpreter

use crate::common::*;
use crate::parser::type_resolver_impl::TypeResolverImpl;
use crate::parser::*;

use std::collections::{HashMap, HashSet};

//
// Result and error of this module
//

pub type Error = Locatable<String>;
pub type Result<T> = std::result::Result<T, Error>;
fn err<T, S: AsRef<str>>(msg: S, pos: Location) -> Result<T> {
    Err(Locatable::new(msg.as_ref().to_string(), pos))
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

impl Visitor<TopLevelTokenNode> for FunctionContext {
    fn visit(&mut self, a: &TopLevelTokenNode) -> Result<()> {
        let pos = a.location();
        match a.as_ref() {
            TopLevelToken::DefType(d) => {
                self.resolver.set(d);
                Ok(())
            }
            TopLevelToken::FunctionDeclaration(n, params) => self.add_declaration(n, params, pos),
            TopLevelToken::FunctionImplementation(n, params, _) => {
                self.add_implementation(n, params, pos)
            }
            _ => Ok(()),
        }
    }
}

impl PostVisitor<ProgramNode> for FunctionContext {
    fn post_visit(&mut self, _: &ProgramNode) -> Result<()> {
        for (k, v) in self.declarations.iter() {
            if !self.implementations.contains_key(k) {
                return err("Missing implementation", v.2);
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

impl Visitor<TopLevelTokenNode> for SubContext {
    fn visit(&mut self, a: &TopLevelTokenNode) -> Result<()> {
        let pos = a.location();
        match a.as_ref() {
            TopLevelToken::DefType(d) => {
                self.resolver.set(d);
                Ok(())
            }
            TopLevelToken::SubDeclaration(n, params) => {
                self.add_declaration(n.as_ref(), params, pos)
            }
            TopLevelToken::SubImplementation(n, params, _) => {
                self.add_implementation(n.as_ref(), params, pos)
            }
            _ => Ok(()),
        }
    }
}

impl PostVisitor<ProgramNode> for SubContext {
    fn post_visit(&mut self, _: &ProgramNode) -> Result<()> {
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
fn collect_subprograms(p: &ProgramNode) -> Result<(FunctionMap, SubMap)> {
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
    pub fn get_constant_type<N: NameTrait + HasLocation>(
        &self,
        n: &N,
    ) -> Result<Option<TypeQualifier>> {
        let bare_name: &CaseInsensitiveString = n.bare_name();
        match self.constants.get(bare_name) {
            Some(const_type) => {
                // it's okay to reference a const unqualified
                if n.bare_or_eq(*const_type) {
                    Ok(Some(*const_type))
                } else {
                    err("Duplicate definition", n.location())
                }
            }
            None => Ok(None),
        }
    }

    pub fn get_parent_constant_type<N: NameTrait + HasLocation>(
        &self,
        n: &N,
    ) -> Result<Option<TypeQualifier>> {
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

    pub fn get_constant_type_recursively<N: NameTrait + HasLocation>(
        &self,
        n: &N,
    ) -> Result<Option<TypeQualifier>> {
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

    pub fn resolve_args_type(&self, args: &Vec<ExpressionNode>) -> Result<Vec<TypeQualifier>> {
        let mut result: Vec<TypeQualifier> = vec![];
        for a in args.iter() {
            result.push(self.resolve_expression_type(a)?);
        }
        Ok(result)
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
            Expression::VariableName(name) => {
                let name_node: NameNode = name.clone().at(pos);
                match self.context.get_constant_type_recursively(&name_node)? {
                    Some(q) => Ok(q),
                    None => Ok(self.resolver.resolve(name)),
                }
            }
            Expression::FunctionCall(n, args) => {
                let bare_name = n.bare_name();
                match self.functions.get(bare_name) {
                    Some((udf_type, udf_param_types, _)) => {
                        if args.len() != udf_param_types.len() {
                            err("Argument count mismatch", pos)
                        } else if !n.bare_or_eq(*udf_type) {
                            err("Type mismatch", pos)
                        } else if udf_param_types != &self.resolve_args_type(args)? {
                            // TODO specify the location of the offending argument
                            err("Type mismatch", pos)
                        } else {
                            Ok(*udf_type)
                        }
                    }
                    None => {
                        // TODO support built-in and undefined functions
                        Ok(self.resolver.resolve(n))
                    }
                }
            }
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

#[derive(Clone, Debug, PartialEq)]
pub struct QForLoopNode {
    pub variable_name: QNameNode,
    pub lower_bound: QExpressionNode,
    pub upper_bound: QExpressionNode,
    pub step: Option<QExpressionNode>,
    pub statements: QStatementNodes,
    pub next_counter: Option<QNameNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QConditionalBlockNode {
    pub condition: QExpressionNode,
    pub statements: QStatementNodes,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QIfBlockNode {
    pub if_block: QConditionalBlockNode,
    pub else_if_blocks: Vec<QConditionalBlockNode>,
    pub else_block: Option<QStatementNodes>,
}

pub type QNameNode = Locatable<QualifiedName>;

#[derive(Clone, Debug, PartialEq)]
pub enum QStatement {
    SubCall(BareName, Vec<QExpressionNode>),
    ForLoop(QForLoopNode),
    IfBlock(QIfBlockNode),
    Assignment(QualifiedName, QExpressionNode),
    While(QConditionalBlockNode),
    Const(QNameNode, QExpressionNode),
    ErrorHandler(CaseInsensitiveString),
    Label(CaseInsensitiveString),
    GoTo(CaseInsensitiveString),
    SetReturnValue(QExpressionNode),
}

pub type QStatementNode = Locatable<QStatement>;
pub type QStatementNodes = Vec<QStatementNode>;

#[derive(Clone, Debug, PartialEq)]
pub enum QExpression {
    SingleLiteral(f32),
    DoubleLiteral(f64),
    StringLiteral(String),
    IntegerLiteral(i32),
    #[allow(dead_code)]
    LongLiteral(i64),
    Constant(QualifiedName),
    Variable(QualifiedName),
    FunctionCall(QualifiedName, Vec<QExpressionNode>),
    BinaryExpression(Operand, Box<QExpressionNode>, Box<QExpressionNode>),
    UnaryExpression(UnaryOperand, Box<QExpressionNode>),
}

pub type QExpressionNode = Locatable<QExpression>;

#[derive(Clone, Debug, PartialEq)]
pub enum QTopLevelToken {
    /// A function implementation
    FunctionImplementation(QNameNode, Vec<QNameNode>, QStatementNodes),

    /// A simple or compound statement
    Statement(QStatement),

    /// A sub implementation
    SubImplementation(BareNameNode, Vec<QNameNode>, QStatementNodes),
}

pub type QTopLevelTokenNode = Locatable<QTopLevelToken>;
pub type QProgramNode = Vec<QTopLevelTokenNode>;

impl Converter<ProgramNode, QProgramNode> for Linter {
    fn convert(&mut self, a: ProgramNode) -> Result<QProgramNode> {
        let (f, s) = collect_subprograms(&a)?;
        self.functions = f;
        self.subs = s;

        let mut result: Vec<QTopLevelTokenNode> = vec![];
        for top_level_token_node in a.into_iter() {
            // will contain None where DefInt and declarations used to be
            let (top_level_token, pos) = top_level_token_node.consume();
            let opt: Option<QTopLevelToken> = self.convert(top_level_token)?;
            match opt {
                Some(t) => {
                    let r: QTopLevelTokenNode = t.at(pos);
                    result.push(r);
                }
                _ => (),
            }
        }
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
impl Converter<TopLevelToken, Option<QTopLevelToken>> for Linter {
    fn convert(&mut self, a: TopLevelToken) -> Result<Option<QTopLevelToken>> {
        match a {
            TopLevelToken::DefType(d) => {
                self.resolver.set(&d);
                Ok(None)
            }
            TopLevelToken::FunctionDeclaration(_, _) | TopLevelToken::SubDeclaration(_, _) => {
                Ok(None)
            }
            TopLevelToken::FunctionImplementation(n, params, block) => {
                let mapped_name = self.convert(n)?;
                let mapped_params = self.convert(params)?;
                // register variables
                self.push_function_context(mapped_name.bare_name());
                let mapped = QTopLevelToken::FunctionImplementation(
                    mapped_name,
                    mapped_params,
                    self.convert(block)?,
                );
                self.pop_context();
                Ok(Some(mapped))
            }
            TopLevelToken::SubImplementation(n, params, block) => {
                let mapped_params = self.convert(params)?;
                self.push_sub_context(n.bare_name());
                // register variables
                let mapped =
                    QTopLevelToken::SubImplementation(n, mapped_params, self.convert(block)?);
                self.pop_context();
                Ok(Some(mapped))
            }
            TopLevelToken::Statement(s) => Ok(Some(QTopLevelToken::Statement(self.convert(s)?))),
        }
    }
}

impl Converter<Statement, QStatement> for Linter {
    fn convert(&mut self, a: Statement) -> Result<QStatement> {
        match a {
            Statement::SubCall(n, args) => Ok(QStatement::SubCall(n, self.convert(args)?)),
            Statement::ForLoop(f) => Ok(QStatement::ForLoop(self.convert(f)?)),
            Statement::IfBlock(i) => Ok(QStatement::IfBlock(self.convert(i)?)),
            Statement::Assignment(n, e) => {
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
                        Ok(QStatement::SetReturnValue(self.convert(e)?))
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
                    // TODO: it is possible to overwrite a const inside a function/sub
                    // register variable
                    Ok(QStatement::Assignment(self.convert(n)?, self.convert(e)?))
                }
            }
            Statement::While(c) => Ok(QStatement::While(self.convert(c)?)),
            Statement::Const(n, e) => {
                // bare name resolves from right side, not resolver
                // register constant
                Ok(QStatement::Const(self.convert(n)?, self.convert(e)?))
            }
            Statement::ErrorHandler(l) => Ok(QStatement::ErrorHandler(l)),
            Statement::Label(l) => Ok(QStatement::Label(l)),
            Statement::GoTo(l) => Ok(QStatement::GoTo(l)),
            Statement::InternalSetReturnValue(_) => unimplemented!(),
        }
    }
}

impl Converter<Expression, QExpression> for Linter {
    fn convert(&mut self, a: Expression) -> Result<QExpression> {
        match a {
            Expression::SingleLiteral(f) => Ok(QExpression::SingleLiteral(f)),
            Expression::DoubleLiteral(f) => Ok(QExpression::DoubleLiteral(f)),
            Expression::StringLiteral(f) => Ok(QExpression::StringLiteral(f)),
            Expression::IntegerLiteral(f) => Ok(QExpression::IntegerLiteral(f)),
            Expression::LongLiteral(f) => Ok(QExpression::LongLiteral(f)),
            Expression::VariableName(n) => {
                // or constant?
                Ok(QExpression::Variable(self.convert(n)?))
            }
            Expression::FunctionCall(n, args) => {
                // validate arg count, arg types, name type
                // for built-in and for user-defined
                // for undefined, resolve to literal 0, as long as the arguments do not contain a string
                Ok(QExpression::FunctionCall(
                    self.convert(n)?,
                    self.convert(args)?,
                ))
            }
            Expression::BinaryExpression(op, l, r) => {
                // types match?
                Ok(QExpression::BinaryExpression(
                    op,
                    self.convert(l)?,
                    self.convert(r)?,
                ))
            }
            Expression::UnaryExpression(op, c) => {
                // is it a legal op? e.g. -"hello" isn't
                Ok(QExpression::UnaryExpression(op, self.convert(c)?))
            }
        }
    }
}

impl Converter<ForLoopNode, QForLoopNode> for Linter {
    fn convert(&mut self, a: ForLoopNode) -> Result<QForLoopNode> {
        Ok(QForLoopNode {
            variable_name: self.convert(a.variable_name)?,
            lower_bound: self.convert(a.lower_bound)?,
            upper_bound: self.convert(a.upper_bound)?,
            step: self.convert(a.step)?,
            statements: self.convert(a.statements)?,
            next_counter: self.convert(a.next_counter)?,
        })
    }
}

impl Converter<ConditionalBlockNode, QConditionalBlockNode> for Linter {
    fn convert(&mut self, a: ConditionalBlockNode) -> Result<QConditionalBlockNode> {
        Ok(QConditionalBlockNode {
            condition: self.convert(a.condition)?,
            statements: self.convert(a.statements)?,
        })
    }
}

impl Converter<IfBlockNode, QIfBlockNode> for Linter {
    fn convert(&mut self, a: IfBlockNode) -> Result<QIfBlockNode> {
        Ok(QIfBlockNode {
            if_block: self.convert(a.if_block)?,
            else_if_blocks: self.convert(a.else_if_blocks)?,
            else_block: self.convert(a.else_block)?,
        })
    }
}
