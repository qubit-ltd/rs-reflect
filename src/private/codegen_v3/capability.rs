// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Exact capability surface consumed by codegen v3.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::PoisonError;

use crate::capability::CapabilityConflict;
#[doc(hidden)]
pub use crate::capability::TypeCapabilities;
#[doc(hidden)]
pub use crate::capability::TypeCapabilitiesResult;
#[doc(hidden)]
pub use crate::capability::clone_descriptor;
#[doc(hidden)]
pub use crate::capability::default_descriptor;
#[doc(hidden)]
pub use crate::capability::send_descriptor;
#[doc(hidden)]
pub use crate::capability::sync_descriptor;

/// Interns generated capabilities for one exact concrete type.
///
/// The factory runs outside the map lock and at most once after a successful
/// initialization. A panic leaves the cell available for retry. Cells live for
/// the process lifetime, just like the concrete descriptors that use them.
#[doc(hidden)]
type CapabilityCell = OnceLock<Result<TypeCapabilities, CapabilityConflict>>;

/// Interns one concrete type's validated capability set for the process
/// lifetime.
///
/// # Type Parameters
///
/// - `T`: The exact target type whose capability set is cached.
///
/// # Parameters
///
/// - `build`: Factory that validates and constructs the immutable set.
///
/// # Returns
///
/// Returns the shared capability set, or the cached validation error.
///
/// # Panics
///
/// Propagates a panic from `build`; the cell remains available for retry.
#[doc(hidden)]
pub fn intern_capabilities<T: ?Sized + 'static>(
    build: fn() -> Result<TypeCapabilities, CapabilityConflict>,
) -> TypeCapabilitiesResult {
    static CACHE: OnceLock<Mutex<HashMap<TypeId, &'static CapabilityCell>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let cell = {
        let mut cache = cache.lock().unwrap_or_else(PoisonError::into_inner);
        *cache
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::leak(Box::new(OnceLock::new())))
    };
    cell.get_or_init(build).as_ref().map_err(Clone::clone)
}
