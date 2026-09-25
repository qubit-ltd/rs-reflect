// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe dynamic default constructors for concrete Rust types.

use crate::value::DynamicOwned;
use crate::value::Local;

/// A safe dynamic default constructor for one exact concrete Rust type.
#[derive(Clone, Copy)]
pub struct DefaultAdapter {
    create: fn() -> DynamicOwned<Local>,
}

impl DefaultAdapter {
    /// Creates an adapter after statically proving that `T` implements
    /// `Default`.
    #[must_use]
    pub fn new<T: Default + 'static>() -> Self {
        Self {
            create: create_default::<T>,
        }
    }

    /// Creates a local dynamic value containing `T::default()`.
    #[must_use]
    pub fn create(&self) -> DynamicOwned<Local> {
        (self.create)()
    }
}

/// Creates one default value of the statically registered concrete type.
fn create_default<T: Default + 'static>() -> DynamicOwned<Local> {
    DynamicOwned::<Local>::new(T::default())
}
