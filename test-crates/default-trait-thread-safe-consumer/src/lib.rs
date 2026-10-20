#![forbid(unsafe_code)]

//! Consumer-side fixtures for reflected default trait methods across crates.

use std::rc::Rc;

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

#[Reflect]
/// A thread-safe target used to verify cross-crate default-method adapters.
pub struct SharedTarget;

#[ReflectImpl(reflected_trait = default_trait_thread_safe_producer::DefaultService)]
impl default_trait_thread_safe_producer::DefaultService for SharedTarget {}

#[Reflect]
/// A local-only target whose `Rc` marker makes it non-thread-safe.
pub struct LocalTarget {
    #[reflect(skip)]
    marker: Rc<()>,
}

#[ReflectImpl(reflected_trait = default_trait_thread_safe_producer::DefaultService)]
impl default_trait_thread_safe_producer::DefaultService for LocalTarget {}

/// Returns the number of strong references to the target's local marker.
pub fn local_marker_count(target: &LocalTarget) -> usize {
    Rc::strong_count(&target.marker)
}

/// Creates a local target with a fresh, uniquely owned marker.
pub fn new_local_target() -> LocalTarget {
    LocalTarget { marker: Rc::new(()) }
}
