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

/// A normalized source visibility category.
pub type Visibility = VisibilityKind;

impl VisibilityKind {
    /// Normalizes a Rust source visibility spelling to its stable category.
    ///
    /// Unknown spellings are treated as private because they do not grant a
    /// recognized outward visibility.
    pub fn from_source(source: &str) -> Self {
        match source.trim() {
            "pub" => Self::Public,
            "pub(crate)" => Self::Crate,
            "pub(super)" => Self::Super,
            "pub(self)" | "" => Self::Private,
            value if value.starts_with("pub(in ") && value.ends_with(')') => Self::Restricted,
            _ => Self::Private,
        }
    }

    /// Returns this visibility's stable category.
    pub const fn kind(self) -> Self {
        self
    }
}
