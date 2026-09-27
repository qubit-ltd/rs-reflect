// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Built-in capability identities and safe dynamic operation adapters.

use super::clone_adapter::CloneAdapter;
use super::default_adapter::DefaultAdapter;
use crate::capability::CapabilityDescriptor;
use crate::capability::CapabilityKey;

const SEND_ID: &str = "qubit.reflect.send";
const SYNC_ID: &str = "qubit.reflect.sync";
const CLONE_ID: &str = "qubit.reflect.clone";
const DEFAULT_ID: &str = "qubit.reflect.default";

/// Returns the built-in fact key for a statically verified `Send` declaration.
///
/// # Returns
///
/// Returns the key identifying the `Send` fact.
#[must_use]
#[inline]
pub fn send_key() -> CapabilityKey<()> {
    CapabilityKey::new_core(SEND_ID)
}

/// Returns the built-in fact key for a statically verified `Sync` declaration.
///
/// # Returns
///
/// Returns the key identifying the `Sync` fact.
#[must_use]
#[inline]
pub fn sync_key() -> CapabilityKey<()> {
    CapabilityKey::new_core(SYNC_ID)
}

/// Returns the built-in typed key for dynamic clone operations.
///
/// # Returns
///
/// Returns the key for clone adapters.
#[must_use]
#[inline]
pub fn clone_key() -> CapabilityKey<CloneAdapter> {
    CapabilityKey::new_core(CLONE_ID)
}

/// Returns the built-in typed key for dynamic default construction.
///
/// # Returns
///
/// Returns the key for default-construction adapters.
#[must_use]
#[inline]
pub fn default_key() -> CapabilityKey<DefaultAdapter> {
    CapabilityKey::new_core(DEFAULT_ID)
}

/// Creates a `Send` fact after statically proving the concrete bound.
///
/// # Type Parameters
///
/// - `T`: The concrete type whose `Send` implementation is verified.
///
/// # Returns
///
/// Returns the `Send` capability descriptor.
#[must_use]
#[inline]
pub fn send_descriptor<T: Send + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::without_adapter(send_key())
}

/// Creates a `Sync` fact after statically proving the concrete bound.
///
/// # Type Parameters
///
/// - `T`: The concrete type whose `Sync` implementation is verified.
///
/// # Returns
///
/// Returns the `Sync` capability descriptor.
#[must_use]
#[inline]
pub fn sync_descriptor<T: Sync + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::without_adapter(sync_key())
}

/// Creates a clone descriptor and its exact-type dynamic adapter.
///
/// # Type Parameters
///
/// - `T`: The concrete type whose `Clone` implementation is verified.
///
/// # Returns
///
/// Returns the clone capability descriptor.
#[must_use]
#[inline]
pub fn clone_descriptor<T: Clone + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(clone_key(), CloneAdapter::new::<T>())
}

/// Creates a default descriptor and its exact-type dynamic adapter.
///
/// # Type Parameters
///
/// - `T`: The concrete type whose `Default` implementation is verified.
///
/// # Returns
///
/// Returns the default capability descriptor.
#[must_use]
#[inline]
pub fn default_descriptor<T: Default + 'static>() -> CapabilityDescriptor {
    CapabilityDescriptor::with_adapter(default_key(), DefaultAdapter::new::<T>())
}
