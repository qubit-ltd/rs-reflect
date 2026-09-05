// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Exact capability surface consumed by codegen v2.

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
type CapabilityCell = std::sync::OnceLock<Result<TypeCapabilities, crate::capability::CapabilityConflict>>;

#[doc(hidden)]
pub fn intern_capabilities<T: ?Sized + 'static>(
    build: fn() -> Result<TypeCapabilities, crate::capability::CapabilityConflict>,
) -> TypeCapabilitiesResult {
    use std::any::TypeId;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::OnceLock;

    static CACHE: OnceLock<Mutex<HashMap<TypeId, &'static CapabilityCell>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let cell = {
        let mut cache = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        *cache
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::leak(Box::new(OnceLock::new())))
    };
    cell.get_or_init(build).as_ref().map_err(Clone::clone)
}
