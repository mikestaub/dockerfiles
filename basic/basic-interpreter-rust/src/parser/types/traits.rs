use super::{QualifiedName, TypeQualifier};
use crate::common::*;

pub trait TypeResolver {
    fn resolve<T: NameTrait>(&self, name: &T) -> TypeQualifier;
}

pub trait HasQualifier {
    fn qualifier(&self) -> TypeQualifier;
}

pub trait NameTrait: Sized + std::fmt::Debug + Clone {
    fn bare_name(&self) -> &CaseInsensitiveString;
    fn is_qualified(&self) -> bool;
    fn opt_qualifier(&self) -> Option<TypeQualifier>;

    fn to_qualified_name<T: TypeResolver>(&self, resolver: &T) -> QualifiedName {
        QualifiedName::new(self.bare_name().clone(), resolver.resolve(self))
    }

    fn eq_resolve<T: TypeResolver, U: NameTrait>(&self, other: &U, resolver: &T) -> bool {
        self.to_qualified_name(resolver) == other.to_qualified_name(resolver)
    }

    /// Checks if the type of this instance is unspecified (bare) or equal to the parameter.
    fn bare_or_eq(&self, other: TypeQualifier) -> bool {
        match self.opt_qualifier() {
            Some(q) => q == other,
            None => true,
        }
    }
}

impl<T: std::fmt::Debug + Sized + HasQualifier> HasQualifier for Locatable<T> {
    fn qualifier(&self) -> TypeQualifier {
        self.as_ref().qualifier()
    }
}
