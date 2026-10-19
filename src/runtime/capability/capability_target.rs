use crate::runtime::identity::{ItemIdentity, TypeIdentity};

/// Identifies the reflection entity that owns a capability.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum CapabilityTargetId {
    Type(TypeIdentity),
    Definition(ItemIdentity),
}
