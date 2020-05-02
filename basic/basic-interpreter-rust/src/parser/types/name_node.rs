use super::{Name, NameTrait, QualifiedName, TypeQualifier};
use crate::common::{CaseInsensitiveString, Locatable, Location};

pub type NameNode = Locatable<Name>;

impl NameNode {
    pub fn from(
        word: String,
        optional_type_qualifier: Option<TypeQualifier>,
        pos: Location,
    ) -> Self {
        let s = CaseInsensitiveString::new(word);
        let n = match optional_type_qualifier {
            Some(q) => Name::Typed(QualifiedName::new(s, q)),
            None => Name::Bare(s),
        };
        NameNode::new(n, pos)
    }
}

impl<T: NameTrait> NameTrait for Locatable<T> {
    fn bare_name(&self) -> &CaseInsensitiveString {
        self.as_ref().bare_name()
    }

    fn opt_qualifier(&self) -> Option<TypeQualifier> {
        self.as_ref().opt_qualifier()
    }
}

impl PartialEq<Name> for NameNode {
    fn eq(&self, other: &Name) -> bool {
        let my_name: &Name = self.as_ref();
        my_name == other
    }
}

impl From<NameNode> for Name {
    fn from(n: NameNode) -> Name {
        n.consume().0
    }
}

#[cfg(test)]
impl PartialEq<str> for NameNode {
    fn eq(&self, other: &str) -> bool {
        self == &Name::from(other)
    }
}
