//! Errors produced while validating stable textual IDs.

/// An invalid stable reflection ID.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdError {
    /// The ID is empty or has a segment that is not an ASCII identifier.
    #[error("invalid namespaced ID `{value}`")]
    InvalidFormat { value: Box<str> },
    /// An external caller attempted to use the library-reserved namespace.
    #[error("the ID `{value}` uses the reserved `qubit.reflect` namespace")]
    ReservedNamespace { value: Box<str> },
}
