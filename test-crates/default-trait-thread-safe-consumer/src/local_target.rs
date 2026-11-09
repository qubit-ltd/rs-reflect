// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

/// A local-only target whose `Rc` marker makes it non-thread-safe.
#[Reflect]
pub struct LocalTarget {
    /// Keeps the target local to its creating thread.
    #[reflect(skip)]
    marker: Rc<()>,
}

#[ReflectImpl(reflected_trait = default_trait_thread_safe_producer::DefaultService)]
impl default_trait_thread_safe_producer::DefaultService for LocalTarget {}

/// Returns the number of strong references to the target's local marker.
///
/// # Parameters
///
/// - `target`: The target whose marker reference count is returned.
pub fn local_marker_count(target: &LocalTarget) -> usize {
    Rc::strong_count(&target.marker)
}

/// Creates a local target with a fresh, uniquely owned marker.
pub fn new_local_target() -> LocalTarget {
    LocalTarget { marker: Rc::new(()) }
}
