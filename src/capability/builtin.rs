// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Built-in capability identities and safe dynamic operation adapters.

use std::any::Any;
use std::any::TypeId;

use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;
use crate::error::TypeMismatch;
use crate::value::DynamicOwned;
use crate::value::Local;

const SEND_ID: &str = "qubit.reflect.send";
const SYNC_ID: &str = "qubit.reflect.sync";
const CLONE_ID: &str = "qubit.reflect.clone";
const DEFAULT_ID: &str = "qubit.reflect.default";

/// A safe dynamic clone operation for one exact concrete Rust type.
#[derive(Clone, Copy)]
pub struct CloneAdapter {
    clone_owned:
        fn(&DynamicOwned<Local>) -> Result<DynamicOwned<Local>, TypeMismatch>,
}

impl CloneAdapter {
    /// Creates an adapter after statically proving that `T` implements `Clone`.
    pub fn new<T: Clone + 'static>() -> Self {
        Self {
            clone_owned: clone_owned::<T>,
        }
    }

    /// Clones a local dynamic value when it contains the registered exact type.
    ///
    /// Returns [`TypeMismatch`] without changing `value` when its concrete type
    /// differs from the type captured by this adapter.
    pub fn clone_owned(
        &self,
        value: &DynamicOwned<Local>,
    ) -> Result<DynamicOwned<Local>, TypeMismatch> {
        (self.clone_owned)(value)
    }
}

/// A safe dynamic default constructor for one exact concrete Rust type.
#[derive(Clone, Copy)]
pub struct DefaultAdapter {
    create: fn() -> DynamicOwned<Local>,
}

impl DefaultAdapter {
    /// Creates an adapter after statically proving that `T` implements
    /// `Default`.
    pub fn new<T: Default + 'static>() -> Self {
        Self {
            create: create_default::<T>,
        }
    }

    /// Creates a local dynamic value containing `T::default()`.
    #[must_use]
    #[inline(always)]
    pub fn create(&self) -> DynamicOwned<Local> {
        (self.create)()
    }
}

/// Returns the built-in fact key for a statically verified `Send` declaration.
pub fn send_key() -> CapabilityKey<()> {
    CapabilityKey::new_core(SEND_ID)
}

/// Returns the built-in fact key for a statically verified `Sync` declaration.
pub fn sync_key() -> CapabilityKey<()> {
    CapabilityKey::new_core(SYNC_ID)
}

/// Returns the built-in typed key for dynamic clone operations.
pub fn clone_key() -> CapabilityKey<CloneAdapter> {
    CapabilityKey::new_core(CLONE_ID)
}

/// Returns the built-in typed key for dynamic default construction.
pub fn default_key() -> CapabilityKey<DefaultAdapter> {
    CapabilityKey::new_core(DEFAULT_ID)
}

/// Creates a `Send` fact after statically proving the concrete bound.
pub fn send_descriptor<T: Send + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::without_adapter(send_key())
}

/// Creates a `Sync` fact after statically proving the concrete bound.
pub fn sync_descriptor<T: Sync + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::without_adapter(sync_key())
}

/// Creates a clone descriptor and its exact-type dynamic adapter.
pub fn clone_descriptor<T: Clone + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(clone_key(), CloneAdapter::new::<T>())
}

/// Creates a default descriptor and its exact-type dynamic adapter.
pub fn default_descriptor<T: Default + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(
        default_key(),
        DefaultAdapter::new::<T>(),
    )
}

/// Clones one exact dynamic concrete type after checking its runtime identity.
fn clone_owned<T: Clone + 'static>(
    value: &DynamicOwned<Local>,
) -> Result<DynamicOwned<Local>, TypeMismatch> {
    let Some(value) = value.downcast_ref::<T>() else {
        let actual = value.as_any().map(Any::type_id).expect(
            "DynamicOwned<Local> always contains Any-compatible storage",
        );
        return Err(TypeMismatch::new(TypeId::of::<T>(), actual));
    };
    Ok(DynamicOwned::<Local>::new(value.clone()))
}

/// Creates one default value of the statically registered concrete type.
fn create_default<T: Default + 'static>() -> DynamicOwned<Local> {
    DynamicOwned::<Local>::new(T::default())
}
