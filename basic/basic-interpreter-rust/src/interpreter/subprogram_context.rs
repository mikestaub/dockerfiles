use crate::common::*;
use crate::interpreter::{err_pre_process, Result};
use crate::parser::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct QualifiedDeclarationNode<T: NameTrait> {
    pub name: T,
    pub parameters: Vec<QualifiedName>,
    pos: Location,
}

pub trait ResolveInto<D: NameTrait> {
    fn resolve_into<TR: TypeResolver>(x: &Self, resolver: &TR) -> D
    where
        Self: NameTrait;
}

impl<T: NameTrait> ResolveInto<CaseInsensitiveString> for T {
    fn resolve_into<TR: TypeResolver>(x: &T, _resolver: &TR) -> CaseInsensitiveString {
        x.bare_name().clone()
    }
}

impl<T: NameTrait> ResolveInto<QualifiedName> for T {
    fn resolve_into<TR: TypeResolver>(x: &T, resolver: &TR) -> QualifiedName {
        x.to_qualified_name(resolver)
    }
}

impl<T: NameTrait> QualifiedDeclarationNode<T> {
    pub fn new<TR: TypeResolver, TName>(
        name: Locatable<TName>,
        parameters: Vec<NameNode>,
        pos: Location,
        resolver: &TR,
    ) -> Self
    where
        TName: NameTrait + ResolveInto<T>,
    {
        QualifiedDeclarationNode {
            // TODO: find all .consume().0
            name: TName::resolve_into(name.as_ref(), resolver),
            parameters: parameters
                .into_iter()
                .map(|x| NameNode::resolve_into(&x, resolver))
                .collect(),
            pos: pos,
        }
    }
}

impl<T: NameTrait> HasLocation for QualifiedDeclarationNode<T> {
    fn location(&self) -> Location {
        self.pos
    }
}

impl<T: NameTrait> NameTrait for QualifiedDeclarationNode<T> {
    fn bare_name(&self) -> &CaseInsensitiveString {
        self.name.bare_name()
    }

    fn opt_qualifier(&self) -> Option<TypeQualifier> {
        self.name.opt_qualifier()
    }
}

#[derive(Clone, Debug)]
pub struct QualifiedImplementationNode<T: NameTrait> {
    pub name: T,
    pub parameters: Vec<QualifiedName>,
    pub block: BlockNode,
    pos: Location,
}

impl<T: Clone + NameTrait> QualifiedImplementationNode<T> {
    pub fn new<TR: TypeResolver, TName>(
        name: Locatable<TName>,
        parameters: Vec<NameNode>,
        block: BlockNode,
        pos: Location,
        resolver: &TR,
    ) -> Self
    where
        TName: NameTrait + ResolveInto<T>,
    {
        QualifiedImplementationNode {
            name: TName::resolve_into(name.as_ref(), resolver),
            parameters: parameters
                .into_iter()
                .map(|x| NameNode::resolve_into(&x, resolver))
                .collect(),
            block: block,
            pos: pos,
        }
    }
}

impl<T: NameTrait> HasLocation for QualifiedImplementationNode<T> {
    fn location(&self) -> Location {
        self.pos
    }
}

impl<T: NameTrait> NameTrait for QualifiedImplementationNode<T> {
    fn bare_name(&self) -> &CaseInsensitiveString {
        self.name.bare_name()
    }

    fn opt_qualifier(&self) -> Option<TypeQualifier> {
        self.name.opt_qualifier()
    }
}

impl<T: HasQualifier + NameTrait> HasQualifier for QualifiedImplementationNode<T> {
    fn qualifier(&self) -> TypeQualifier {
        self.name.qualifier()
    }
}

#[derive(Debug)]
pub struct SubprogramContext<T: NameTrait> {
    pub declarations: HashMap<CaseInsensitiveString, QualifiedDeclarationNode<T>>,
    pub implementations: HashMap<CaseInsensitiveString, QualifiedImplementationNode<T>>,
}

impl<T: NameTrait> SubprogramContext<T> {
    pub fn new() -> Self {
        SubprogramContext {
            declarations: HashMap::new(),
            implementations: HashMap::new(),
        }
    }

    pub fn ensure_all_declared_programs_are_implemented(&self) -> Result<()> {
        for (k, v) in self.declarations.iter() {
            if !self.implementations.contains_key(k) {
                return err_pre_process("Subprogram not defined", v.pos);
            }
        }
        Ok(())
    }

    pub fn has_implementation<U: NameTrait>(&self, name: &U) -> bool {
        self.implementations.contains_key(name.bare_name())
    }

    pub fn get_implementation<U: NameTrait>(
        &self,
        name: &U,
    ) -> Option<QualifiedImplementationNode<T>> {
        self.implementations
            .get(name.bare_name())
            .map(|x| x.clone())
    }

    pub fn get_implementation_ref<U: NameTrait>(
        &self,
        name: &U,
    ) -> Option<&QualifiedImplementationNode<T>> {
        self.implementations.get(name.bare_name())
    }

    pub fn add_declaration<TR: TypeResolver, TName: NameTrait + ResolveInto<T>>(
        &mut self,
        name: Locatable<TName>,
        parameters: Vec<NameNode>,
        pos: Location,
        resolver: &TR,
    ) -> Result<()> {
        match self.validate_against_existing_declaration(&name, &parameters, pos, resolver)? {
            None => {
                let bare_name: &CaseInsensitiveString = name.bare_name();
                self.declarations.insert(
                    bare_name.clone(),
                    QualifiedDeclarationNode::new(name, parameters, pos, resolver),
                );
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn add_implementation<TR: TypeResolver, TName: NameTrait + ResolveInto<T>>(
        &mut self,
        name_node: Locatable<TName>,
        parameters: Vec<NameNode>,
        block: BlockNode,
        pos: Location,
        resolver: &TR,
    ) -> Result<()> {
        let bare_name: &CaseInsensitiveString = name_node.bare_name();
        if self.has_implementation(bare_name) {
            err_pre_process("Duplicate definition", pos)
        } else {
            self.validate_against_existing_declaration(&name_node, &parameters, pos, resolver)?;
            let resolved_name: T = TName::resolve_into(name_node.as_ref(), resolver);
            let modified_block = BlockNode::assignment_to_set_return_value(block, &resolved_name)?;
            self.implementations.insert(
                bare_name.clone(),
                QualifiedImplementationNode::new(
                    name_node,
                    parameters,
                    modified_block,
                    pos,
                    resolver,
                ),
            );
            Ok(())
        }
    }

    fn validate_against_existing_declaration<
        TR: TypeResolver,
        TName: NameTrait + ResolveInto<T>,
    >(
        &self,
        name_node: &Locatable<TName>,
        parameters: &Vec<NameNode>,
        pos: Location,
        resolver: &TR,
    ) -> Result<Option<&QualifiedDeclarationNode<T>>> {
        let bare_name: &CaseInsensitiveString = name_node.bare_name();
        match self.declarations.get(bare_name) {
            Some(existing_declaration) => {
                if existing_declaration.is_qualified()
                    && existing_declaration.opt_qualifier().unwrap() != resolver.resolve(name_node)
                {
                    err_pre_process("Duplicate definition", pos)
                } else {
                    require_parameters_same(
                        &existing_declaration.parameters,
                        &parameters,
                        pos,
                        resolver,
                    )?;
                    Ok(Some(existing_declaration))
                }
            }
            None => Ok(None),
        }
    }
}

fn require_parameters_same<T: TypeResolver>(
    existing: &Vec<QualifiedName>,
    parameters: &Vec<NameNode>,
    pos: Location,
    resolver: &T,
) -> Result<()> {
    if existing.len() != parameters.len() {
        return err_pre_process("Argument-count mismatch", pos);
    }

    for i in 0..existing.len() {
        let e = &existing[i];
        let n = &parameters[i];
        if e.qualifier() != resolver.resolve(n) {
            return err_pre_process("Parameter type mismatch", n.location());
        }
    }

    Ok(())
}

trait AssignmentToSetReturnValue<T: NameTrait> {
    fn assignment_to_set_return_value(node: Self, result_name: &T) -> Result<Self>
    where
        Self: Sized;
}

impl<TElement, TName: NameTrait> AssignmentToSetReturnValue<TName> for Vec<TElement>
where
    TElement: AssignmentToSetReturnValue<TName>,
{
    fn assignment_to_set_return_value(node: Self, result_name: &TName) -> Result<Self> {
        let mut result: Self = vec![];
        for x in node {
            result.push(TElement::assignment_to_set_return_value(x, result_name)?);
        }
        Ok(result)
    }
}

impl<TElement, TName: NameTrait> AssignmentToSetReturnValue<TName> for Option<TElement>
where
    TElement: AssignmentToSetReturnValue<TName>,
{
    fn assignment_to_set_return_value(node: Self, result_name: &TName) -> Result<Self> {
        match node {
            Some(n) => Ok(Some(TElement::assignment_to_set_return_value(
                n,
                result_name,
            )?)),
            None => Ok(None),
        }
    }
}

impl<T: NameTrait> AssignmentToSetReturnValue<T> for ConditionalBlockNode {
    fn assignment_to_set_return_value(node: Self, result_name: &T) -> Result<Self> {
        Ok(ConditionalBlockNode {
            pos: node.pos,
            condition: node.condition,
            statements: BlockNode::assignment_to_set_return_value(node.statements, result_name)?,
        })
    }
}

impl<T: NameTrait> AssignmentToSetReturnValue<T> for StatementNode {
    fn assignment_to_set_return_value(node: Self, result_name: &T) -> Result<Self> {
        match node {
            Self::ForLoop(f) => Ok(Self::ForLoop(ForLoopNode {
                variable_name: f.variable_name,
                lower_bound: f.lower_bound,
                upper_bound: f.upper_bound,
                step: f.step,
                statements: BlockNode::assignment_to_set_return_value(f.statements, result_name)?,
                next_counter: f.next_counter,
                pos: f.pos,
            })),
            Self::IfBlock(i) => Ok(Self::IfBlock(IfBlockNode {
                if_block: ConditionalBlockNode::assignment_to_set_return_value(
                    i.if_block,
                    result_name,
                )?,
                else_if_blocks: Vec::<ConditionalBlockNode>::assignment_to_set_return_value(
                    i.else_if_blocks,
                    result_name,
                )?,
                else_block: Option::<BlockNode>::assignment_to_set_return_value(
                    i.else_block,
                    result_name,
                )?,
            })),
            Self::Assignment(left, right) => {
                let n: &Name = left.as_ref();
                match n {
                    Name::Bare(b) => {
                        if b == result_name.bare_name() {
                            // assigning to function result name
                            // TODO: throw error for SUB
                            Ok(Self::InternalSetReturnValue(right))
                        } else {
                            Ok(Self::Assignment(left, right))
                        }
                    }
                    Name::Qualified(q) => {
                        if q.bare_name() == result_name.bare_name() {
                            // bare name is equal to result name bare name
                            if result_name.is_qualified() {
                                // function
                                if result_name.opt_qualifier().unwrap() == q.qualifier() {
                                    Ok(Self::InternalSetReturnValue(right))
                                } else {
                                    err_pre_process("Duplicate definition", left.location())
                                }
                            } else {
                                // sub
                                err_pre_process("Duplicate definition", left.location())
                            }
                        } else {
                            Ok(Self::Assignment(left, right))
                        }
                    }
                }
            }
            Self::While(w) => Ok(Self::While(
                ConditionalBlockNode::assignment_to_set_return_value(w, result_name)?,
            )),
            Self::Const(left, right, pos) => {
                let n: &Name = left.as_ref();
                match n {
                    Name::Bare(b) => {
                        if b == result_name.bare_name() {
                            // CONST cannot match function result name
                            err_pre_process("Duplicate definition", left.location())
                        } else {
                            Ok(Self::Const(left, right, pos))
                        }
                    }
                    Name::Qualified(q) => {
                        if q.bare_name() == result_name.bare_name() {
                            err_pre_process("Duplicate definition", left.location())
                        } else {
                            Ok(Self::Const(left, right, pos))
                        }
                    }
                }
            }
            _ => Ok(node),
        }
    }
}
