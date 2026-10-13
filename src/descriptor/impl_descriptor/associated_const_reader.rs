// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Associated-constant readers returning ordinary owned Rust values.

use std::any::Any;
use std::any::TypeId;
use std::fmt;

use crate::error::TypeMismatch;

/// A safe reader for one concrete associated constant value.
pub struct AssociatedConstReader {
    /// Function or process-lifetime closure producing a fresh erased value.
    read: AssociatedConstAnyGetter,
}

enum AssociatedConstAnyGetter {
    Function(fn() -> Box<dyn Any>),
    Closure(&'static (dyn Fn() -> Box<dyn Any> + Send + Sync)),
}

impl AssociatedConstReader {
    /// Creates a reader from a generated function returning a boxed value.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(read: fn() -> Box<dyn Any>) -> Self {
        Self {
            read: AssociatedConstAnyGetter::Function(read),
        }
    }

    /// Creates a reader from a concrete associated-value getter.
    #[doc(hidden)]
    #[must_use]
    pub fn from_getter<T: Any>(getter: fn() -> T) -> Self {
        let read = Box::leak(Box::new(move || Box::new(getter()) as Box<dyn Any>));
        Self {
            read: AssociatedConstAnyGetter::Closure(read),
        }
    }

    /// Reads a fresh associated value as `Box<dyn Any>`.
    #[must_use]
    pub fn read_any(&self) -> Box<dyn Any> {
        match self.read {
            AssociatedConstAnyGetter::Function(read) => read(),
            AssociatedConstAnyGetter::Closure(read) => read(),
        }
    }

    /// Reads a fresh value and checks that it has exact type `T`.
    ///
    /// # Errors
    ///
    /// Returns [`TypeMismatch`] if the associated value's concrete type is
    /// different from `T`.
    pub fn read<T: Any>(&self) -> Result<T, TypeMismatch> {
        let value = self.read_any();
        let actual = value.as_ref().type_id();
        value
            .downcast::<T>()
            .map(|value| *value)
            .map_err(|_| TypeMismatch::new(TypeId::of::<T>(), actual))
    }
}

impl fmt::Debug for AssociatedConstReader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AssociatedConstReader(..)")
    }
}
