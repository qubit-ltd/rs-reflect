use std::any::TypeId;

use crate::capability::CapabilityConflict;
use crate::identity::CapabilityId;

/// An error returned when a typed capability exists but cannot be executed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CapabilityAccessError {
    /// The intrinsic capability declarations conflict.
    #[error("intrinsic capability conflict: {0}")]
    IntrinsicConflict(#[source] CapabilityConflict),
    /// The requested capability is a fact without an executable adapter.
    #[error("capability {id} has no adapter for contract {adapter_type:?}")]
    FactOnly { id: CapabilityId, adapter_type: TypeId },
    /// The requested adapter contract differs from the declared contract.
    #[error("capability {id} adapter mismatch: expected {expected:?}, actual {actual:?}")]
    AdapterTypeMismatch {
        id: CapabilityId,
        expected: TypeId,
        actual: TypeId,
    },
}
