use super::{HasQualifier, NameTrait, TypeQualifier, TypeResolver};
use crate::common::CaseInsensitiveString;
use std::fmt::Display;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QualifiedName {
    name: CaseInsensitiveString,
    qualifier: TypeQualifier,
}

impl QualifiedName {
    pub fn new(name: CaseInsensitiveString, qualifier: TypeQualifier) -> Self {
        QualifiedName { name, qualifier }
    }

    pub fn consume(self) -> (CaseInsensitiveString, TypeQualifier) {
        (self.name, self.qualifier)
    }
}

impl HasQualifier for QualifiedName {
    fn qualifier(&self) -> TypeQualifier {
        self.qualifier
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.name, self.qualifier)
    }
}

impl NameTrait for QualifiedName {
    fn bare_name(&self) -> &CaseInsensitiveString {
        &self.name
    }

    fn opt_qualifier(&self) -> Option<TypeQualifier> {
        Some(self.qualifier)
    }

    fn eq_resolve<T: TypeResolver, U: NameTrait>(&self, other: &U, resolver: &T) -> bool {
        self == &other.to_qualified_name(resolver)
    }
}
