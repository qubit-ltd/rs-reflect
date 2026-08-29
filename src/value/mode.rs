//! Compile-time mode markers for dynamic value wrappers.

use std::marker::PhantomData;
use std::rc::Rc;

use crate::value::storage::{
    LocalMutStorage, LocalOwnedStorage, LocalRefStorage, ThreadSafeMutStorage,
    ThreadSafeOwnedStorage, ThreadSafeRefStorage,
};

mod sealed {
    /// Restricts dynamic value modes to the crate-defined markers.
    pub trait Sealed {}
}

/// A sealed compile-time mode for dynamic value wrappers.
///
/// This trait is only implemented by [`Local`] and [`ThreadSafe`]. Its hidden
/// associated types preserve each mode's erased type and auto-trait boundary.
pub trait Mode: sealed::Sealed {
    /// Internal marker that determines the wrapper's auto traits.
    #[doc(hidden)]
    type Marker;
    /// Internal owned storage selected by this mode.
    #[doc(hidden)]
    type OwnedStorage;
    /// Internal shared-borrow storage selected by this mode.
    #[doc(hidden)]
    type RefStorage<'a>;
    /// Internal mutable-borrow storage selected by this mode.
    #[doc(hidden)]
    type MutStorage<'a>;
}

/// The local dynamic value mode.
///
/// Wrappers in this mode intentionally cannot implement `Send` or `Sync`.
///
/// ```compile_fail
/// use qubit_reflect::value::ReflectedRef;
///
/// fn assert_send<T: Send>(_: T) {}
/// let value = 1_u32;
/// assert_send(ReflectedRef::new(&value));
/// ```
pub enum Local {}

/// The dynamic value mode with compile-time `Send` and `Sync` constraints.
pub enum ThreadSafe {}

impl sealed::Sealed for Local {}

impl Mode for Local {
    type Marker = PhantomData<Rc<()>>;
    type OwnedStorage = LocalOwnedStorage;
    type RefStorage<'a> = LocalRefStorage<'a>;
    type MutStorage<'a> = LocalMutStorage<'a>;
}

impl sealed::Sealed for ThreadSafe {}

impl Mode for ThreadSafe {
    type Marker = ();
    type OwnedStorage = ThreadSafeOwnedStorage;
    type RefStorage<'a> = ThreadSafeRefStorage<'a>;
    type MutStorage<'a> = ThreadSafeMutStorage<'a>;
}
