use super::{HasQualifier, NameTrait, QualifiedName, TypeQualifier};
use crate::common::CaseInsensitiveString;
use std::convert::TryFrom;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub enum Name {
    Bare(CaseInsensitiveString),
    Typed(QualifiedName),
}

impl NameTrait for CaseInsensitiveString {
    fn bare_name(&self) -> &CaseInsensitiveString {
        self
    }

    fn is_qualified(&self) -> bool {
        false
    }

    fn opt_qualifier(&self) -> Option<TypeQualifier> {
        None
    }
}

impl NameTrait for Name {
    fn bare_name(&self) -> &CaseInsensitiveString {
        match self {
            Self::Bare(b) => b,
            Self::Typed(t) => t.bare_name(),
        }
    }

    fn is_qualified(&self) -> bool {
        match self {
            Self::Bare(_) => false,
            Self::Typed(_) => true,
        }
    }

    fn opt_qualifier(&self) -> Option<TypeQualifier> {
        match self {
            Self::Bare(_) => None,
            Self::Typed(t) => Some(t.qualifier()),
        }
    }
}

impl<S: AsRef<str>> From<S> for Name {
    fn from(s: S) -> Self {
        let mut buf = s.as_ref().to_string();
        let last_ch: char = buf.pop().unwrap();
        match TypeQualifier::try_from(last_ch) {
            Ok(qualifier) => Name::Typed(QualifiedName::new(
                CaseInsensitiveString::new(buf),
                qualifier,
            )),
            _ => {
                buf.push(last_ch);
                Name::Bare(CaseInsensitiveString::new(buf))
            }
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Name::Bare(s) => write!(f, "{}", s),
            Name::Typed(t) => write!(f, "{}", t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from() {
        assert_eq!(Name::from("A"), Name::Bare("A".into()));
        assert_eq!(
            Name::from("Pos%"),
            Name::Typed(QualifiedName::new(
                CaseInsensitiveString::new("Pos".to_string()),
                TypeQualifier::PercentInteger
            ))
        );
    }
}
