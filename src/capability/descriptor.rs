//! Type-erased descriptors for capability facts and operation adapters.

use std::any::{Any, TypeId};
use std::sync::Arc;

use crate::capability::CapabilityKey;
use crate::identity::CapabilityId;

/// An immutable capability fact with an optional type-checked adapter value.
///
/// The stable ID and adapter contract remain available even when the adapter
/// type is unknown to a caller or no executable operation is attached.
pub struct CapabilityDescriptor {
    id: CapabilityId,
    adapter_type: TypeId,
    adapter: Option<Arc<dyn Any + Send + Sync>>,
}

impl CapabilityDescriptor {
    /// Creates a capability fact without an executable adapter.
    pub fn without_adapter<A: 'static>(key: CapabilityKey<A>) -> Self {
        Self {
            id: key.id().clone(),
            adapter_type: key.adapter_type(),
            adapter: None,
        }
    }

    /// Creates a capability fact carrying an adapter of the key's exact type.
    pub fn with_adapter<A: Send + Sync + 'static>(key: CapabilityKey<A>, adapter: A) -> Self {
        Self {
            id: key.id().clone(),
            adapter_type: key.adapter_type(),
            adapter: Some(Arc::new(adapter)),
        }
    }

    /// Returns the stable capability identity.
    pub const fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// Returns the process-local identity of the adapter contract.
    pub const fn adapter_type(&self) -> TypeId {
        self.adapter_type
    }

    /// Returns whether this descriptor carries an executable adapter.
    pub const fn has_adapter(&self) -> bool {
        self.adapter.is_some()
    }

    /// Retrieves the adapter only when both ID and Rust contract type match.
    pub(crate) fn get<A: 'static>(&self, key: &CapabilityKey<A>) -> Option<&A> {
        if self.id != *key.id() || self.adapter_type != key.adapter_type() {
            return None;
        }
        self.adapter.as_deref()?.downcast_ref::<A>()
    }
}

impl Clone for CapabilityDescriptor {
    /// Shares the immutable adapter while cloning portable descriptor facts.
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            adapter_type: self.adapter_type,
            adapter: self.adapter.clone(),
        }
    }
}

impl std::fmt::Debug for CapabilityDescriptor {
    /// Formats portable descriptor facts without inspecting the erased adapter.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CapabilityDescriptor")
            .field("id", &self.id)
            .field("adapter_type", &self.adapter_type)
            .field("has_adapter", &self.has_adapter())
            .finish()
    }
}
