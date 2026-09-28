// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! AssociatedConstReader metadata and behavior.

use std::fmt;

use crate::value::ReflectedOwned;

/// A safe reader for one concrete associated constant value.
///
/// A reader invokes its generated adapter each time, producing a fresh owned
/// value without exposing a reference to static storage.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstReader;
/// use qubit_reflect::ReflectedOwned;
///
/// fn read_value() -> ReflectedOwned { ReflectedOwned::new(7_u8) }
/// let reader = AssociatedConstReader::new(read_value);
/// assert_eq!(reader.read().downcast_ref::<u8>(), Some(&7));
/// ```
pub struct AssociatedConstReader {
    /// Safe function or closure adapter that reads a fresh owned value.
    read: AssociatedConstReadAdapter,
}

/// Internal storage forms for generated associated constant readers.
enum AssociatedConstReadAdapter {
    /// Non-capturing generated reader.
    Function(fn() -> ReflectedOwned),
    /// Static closure used to adapt a concrete value getter.
    Closure(&'static (dyn Fn() -> ReflectedOwned + Send + Sync)),
}

impl AssociatedConstReader {
    /// Creates a reader from generated safe adapter code.
    ///
    /// # Parameters
    ///
    /// - `read`: Generated function returning an owned reflected value.
    ///
    /// # Returns
    ///
    /// Returns a reader that invokes `read` on each call.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(read: fn() -> ReflectedOwned) -> Self {
        Self {
            read: AssociatedConstReadAdapter::Function(read),
        }
    }

    /// Creates a reader from a compiler-proven sized `'static` value getter.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete associated constant value type.
    ///
    /// # Parameters
    ///
    /// - `getter`: Function that returns the associated constant value.
    ///
    /// # Returns
    ///
    /// Returns a reader that owns each value produced by `getter`.
    /// The generated closure is retained for the process lifetime.
    #[doc(hidden)]
    #[must_use]
    pub fn from_getter<T: 'static>(getter: fn() -> T) -> Self {
        let read = Box::leak(Box::new(move || ReflectedOwned::new(getter())));
        Self {
            read: AssociatedConstReadAdapter::Closure(read),
        }
    }

    /// Reads a fresh owned reflected value.
    ///
    /// This invokes the generated reader adapter on every call.
    ///
    /// # Returns
    ///
    /// Returns a newly owned reflected value.
    #[must_use]
    pub fn read(&self) -> ReflectedOwned {
        match self.read {
            AssociatedConstReadAdapter::Function(read) => read(),
            AssociatedConstReadAdapter::Closure(read) => read(),
        }
    }
}

impl fmt::Debug for AssociatedConstReader {
    /// Formats adapter availability without exposing a process address.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the stable, address-free
    ///   representation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the representation, or the formatter
    /// error.
    ///
    /// # Errors
    ///
    /// Returns the error reported by `formatter`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AssociatedConstReader(..)")
    }
}
