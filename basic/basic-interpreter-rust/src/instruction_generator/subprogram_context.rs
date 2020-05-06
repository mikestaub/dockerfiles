use crate::common::*;
use crate::linter::*;
use std::collections::HashMap;

pub type FunctionContext = SubprogramContext<QualifiedName>;
pub type SubContext = SubprogramContext<CaseInsensitiveString>;

#[derive(Clone, Debug)]
pub struct QualifiedImplementationNode<T: NameTrait> {
    pub name: T,
    pub parameters: Vec<QualifiedName>,
    pub block: StatementNodes,
    pos: Location,
}

impl<T: Clone + NameTrait> QualifiedImplementationNode<T> {
    pub fn new(
        name: Locatable<T>,
        parameters: Vec<QNameNode>,
        block: StatementNodes,
        pos: Location,
    ) -> Self {
        QualifiedImplementationNode {
            name: name.as_ref().clone(),
            parameters: parameters.into_iter().map(|x| x.as_ref().clone()).collect(),
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

#[derive(Debug)]
pub struct SubprogramContext<T: NameTrait> {
    pub implementations: HashMap<CaseInsensitiveString, QualifiedImplementationNode<T>>,
}

impl<T: NameTrait> SubprogramContext<T> {
    pub fn new() -> Self {
        SubprogramContext {
            implementations: HashMap::new(),
        }
    }

    pub fn get_implementation<U: NameTrait>(
        &self,
        name: &U,
    ) -> Option<QualifiedImplementationNode<T>> {
        self.implementations
            .get(name.bare_name())
            .map(|x| x.clone())
    }

    pub fn add_implementation(
        &mut self,
        name_node: Locatable<T>,
        parameters: Vec<QNameNode>,
        block: StatementNodes,
        pos: Location,
    ) {
        let bare_name: &CaseInsensitiveString = name_node.bare_name();
        self.implementations.insert(
            bare_name.clone(),
            QualifiedImplementationNode::new(name_node, parameters, block, pos),
        );
    }
}
