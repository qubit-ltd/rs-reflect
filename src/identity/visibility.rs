//! Stable visibility categories for source declarations.

/// A normalized source visibility category.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VisibilityKind {
    /// Public outside the declaring crate.
    Public,
    /// Visible throughout the declaring crate.
    Crate,
    /// Visible to the immediate parent module.
    Super,
    /// Visible only in an explicitly named source scope.
    Restricted,
    /// Visible only in the declaring module.
    Private,
}

/// A normalized source visibility with diagnostic data for restricted paths.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Visibility {
    /// Public outside the declaring crate.
    Public,
    /// Visible throughout the declaring crate.
    Crate,
    /// Visible to the immediate parent module.
    Super,
    /// Visible only in the retained source path.
    Restricted(Box<str>),
    /// Visible only in the declaring module.
    Private,
}

impl Visibility {
    /// Normalizes a Rust source visibility spelling while retaining restricted paths.
    ///
    /// Unknown spellings are treated as private because they do not grant a
    /// recognized outward visibility.
    pub fn from_source(source: &str) -> Self {
        match source.trim() {
            "pub" => Self::Public,
            "pub(crate)" => Self::Crate,
            "pub(super)" => Self::Super,
            "pub(self)" | "" => Self::Private,
            value if value.starts_with("pub(in ") && value.ends_with(')') => {
                let path = &value[7..value.len() - 1];
                Self::Restricted(path.into())
            }
            _ => Self::Private,
        }
    }

    /// Returns this visibility's stable category.
    pub const fn kind(&self) -> VisibilityKind {
        match self {
            Self::Public => VisibilityKind::Public,
            Self::Crate => VisibilityKind::Crate,
            Self::Super => VisibilityKind::Super,
            Self::Restricted(_) => VisibilityKind::Restricted,
            Self::Private => VisibilityKind::Private,
        }
    }

    /// Returns the retained restricted path, or `None` for other visibility kinds.
    pub fn restricted_path(&self) -> Option<&str> {
        match self {
            Self::Restricted(path) => Some(path),
            _ => None,
        }
    }
}
