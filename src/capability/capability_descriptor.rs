// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Type-erased descriptors for capability facts and operation adapters.

use std::any::Any;
use std::any::TypeId;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::sync::Arc;

use crate::capability::CapabilityKey;
use crate::identity::CapabilityId;

/// An immutable capability fact with an optional type-checked adapter value.
///
/// The stable ID and adapter contract remain available even when the adapter
/// type is unknown to a caller or no executable operation is attached.
///
/// # Examples
///
/// ```
/// use qubit_reflect::capability::CapabilityDescriptor;
/// use qubit_reflect::capability::CapabilityKey;
/// use qubit_reflect::identity::CapabilityId;
///
/// let id = CapabilityId::new("example.fact").expect("valid capability ID");
/// let descriptor = CapabilityDescriptor::without_adapter(CapabilityKey::<()>::new(id));
/// assert!(!descriptor.has_adapter());
/// ```
pub struct CapabilityDescriptor {
    /// Stable capability identity used together with `adapter_type` for lookup.
    id: CapabilityId,
    /// Process-local Rust type identity expected for the associated adapter.
    adapter_type: TypeId,
    /// Optional type-erased adapter whose concrete type must match
    /// `adapter_type`.
    adapter: Option<Arc<dyn Any + Send + Sync>>,
}

impl CapabilityDescriptor {
    /// Creates a capability fact without an executable adapter.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The adapter contract represented by `key`.
    ///
    /// # Parameters
    ///
    /// - `key`: The typed capability key.
    ///
    /// # Returns
    ///
    /// Returns a descriptor without an executable adapter.
    #[must_use]
    pub fn without_adapter<A: 'static>(key: CapabilityKey<A>) -> Self {
        Self {
            id: *key.id(),
            adapter_type: key.adapter_type(),
            adapter: None,
        }
    }

    /// Creates a capability fact carrying an adapter of the key's exact type.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The thread-safe adapter contract represented by `key`.
    ///
    /// # Parameters
    ///
    /// - `key`: The typed capability key.
    /// - `adapter`: The executable adapter value.
    ///
    /// # Returns
    ///
    /// Returns a descriptor that owns the adapter in a shared allocation.
    #[must_use]
    pub fn with_adapter<A: Send + Sync + 'static>(key: CapabilityKey<A>, adapter: A) -> Self {
        Self {
            id: *key.id(),
            adapter_type: key.adapter_type(),
            adapter: Some(Arc::new(adapter)),
        }
    }

    /// Returns the stable capability identity.
    ///
    /// # Returns
    ///
    /// Returns the descriptor's stable capability ID.
    #[must_use]
    #[inline]
    pub const fn id(&self) -> &CapabilityId {
        &self.id
    }

    /// Returns the process-local identity of the adapter contract.
    ///
    /// # Returns
    ///
    /// Returns the adapter contract's process-local `TypeId`.
    #[must_use]
    #[inline]
    pub const fn adapter_type(&self) -> TypeId {
        self.adapter_type
    }

    /// Returns whether this descriptor carries an executable adapter.
    ///
    /// # Returns
    ///
    /// Returns `true` when an executable adapter is present.
    #[must_use]
    #[inline]
    pub const fn has_adapter(&self) -> bool {
        self.adapter.is_some()
    }

    /// Retrieves the adapter only when both ID and Rust contract type match.
    ///
    /// # Type Parameters
    ///
    /// - `A`: The requested adapter contract.
    ///
    /// # Parameters
    ///
    /// - `key`: The key whose ID and adapter type must match this descriptor.
    ///
    /// # Returns
    ///
    /// Returns the adapter when both the ID and contract match, or `None`
    /// otherwise.
    #[must_use]
    pub(crate) fn get<A: 'static>(&self, key: &CapabilityKey<A>) -> Option<&A> {
        if self.id != *key.id() || self.adapter_type != key.adapter_type() {
            return None;
        }
        self.adapter.as_deref()?.downcast_ref::<A>()
    }
}

impl Clone for CapabilityDescriptor {
    /// Shares the immutable adapter while cloning portable descriptor facts.
    ///
    /// # Returns
    ///
    /// Returns a descriptor sharing the same immutable adapter allocation.
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            adapter_type: self.adapter_type,
            adapter: self.adapter.clone(),
        }
    }
}

impl Debug for CapabilityDescriptor {
    /// Formats portable descriptor facts without inspecting the erased adapter.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the descriptor.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter
            .debug_struct("CapabilityDescriptor")
            .field("id", &self.id)
            .field("adapter_type", &self.adapter_type)
            .field("has_adapter", &self.has_adapter())
            .finish()
    }
}
