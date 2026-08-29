//! Composite identities for reflected members.

use crate::identity::FragmentIdentity;

/// The stable composite identity of a reflected member.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MemberId {
    declaring_identity: Box<str>,
    kind: Box<str>,
    index: usize,
    fragment: FragmentIdentity,
}

impl MemberId {
    /// Creates a member ID from its declaring identity, category, position, and
    /// fragment.
    pub fn new(declaring_identity: &str, kind: &str, index: usize, fragment: FragmentIdentity) -> Self {
        Self {
            declaring_identity: declaring_identity.into(),
            kind: kind.into(),
            index,
            fragment,
        }
    }

    /// Returns the identity of the descriptor declaring this member.
    pub fn declaring_identity(&self) -> &str {
        &self.declaring_identity
    }
    /// Returns the member category.
    pub fn kind(&self) -> &str {
        &self.kind
    }
    /// Returns the member's declaration index within its category.
    pub fn index(&self) -> usize {
        self.index
    }
    /// Returns the fragment that contributed this member.
    pub fn fragment(&self) -> &FragmentIdentity {
        &self.fragment
    }
}
