use super::{QualifiedName, TypeQualifier, TypeResolver};
use crate::common::*;

pub trait NameTrait: Sized + std::fmt::Debug + Clone {
    fn bare_name(&self) -> &CaseInsensitiveString;
    fn opt_qualifier(&self) -> Option<TypeQualifier>;

    fn is_qualified(&self) -> bool {
        self.opt_qualifier().is_some()
    }

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
