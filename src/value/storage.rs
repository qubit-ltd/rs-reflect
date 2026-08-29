//! Internal erased storage for dynamic value wrappers.

use std::any::Any;

/// Internal local-mode owned storage.
#[doc(hidden)]
pub enum LocalOwnedStorage {
    /// An owned `Any` value without thread-safety bounds.
    Any(Box<dyn Any>),
}

/// Internal thread-safe-mode owned storage.
#[doc(hidden)]
pub enum ThreadSafeOwnedStorage {
    /// An owned `Any` value retaining its `Send + Sync` boundary.
    Any(Box<dyn Any + Send + Sync>),
}

/// Internal local-mode shared-borrow storage.
#[doc(hidden)]
pub enum LocalRefStorage<'a> {
    /// A shared `Any` borrow without thread-safety bounds.
    Any(&'a dyn Any),
    /// A shared `str` borrow kept separate from `Any`.
    Str(&'a str),
}

/// Internal thread-safe-mode shared-borrow storage.
#[doc(hidden)]
pub enum ThreadSafeRefStorage<'a> {
    /// A shared `Any` borrow retaining its `Sync` boundary.
    Any(&'a (dyn Any + Sync)),
    /// A shared `str` borrow kept separate from `Any`.
    Str(&'a str),
}

/// Internal local-mode mutable-borrow storage.
#[doc(hidden)]
pub enum LocalMutStorage<'a> {
    /// A mutable `Any` borrow without thread-safety bounds.
    Any(&'a mut dyn Any),
    /// A mutable `str` borrow kept separate from `Any`.
    Str(&'a mut str),
}

/// Internal thread-safe-mode mutable-borrow storage.
#[doc(hidden)]
pub enum ThreadSafeMutStorage<'a> {
    /// A mutable `Any` borrow retaining its `Send + Sync` boundary.
    Any(&'a mut (dyn Any + Send + Sync)),
    /// A mutable `str` borrow kept separate from `Any`.
    Str(&'a mut str),
}
