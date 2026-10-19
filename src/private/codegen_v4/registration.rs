// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed linked-registration contracts consumed by reflection contexts.
//!
//! The current runtime stores `RegistrationFragment` and its inventory
//! collector under `runtime::context`; both are crate-private. Derive needs a
//! safe submit macro or typed registration constructor that binds a concrete
//! `Reflect` type to its generated registration and source origin. It must not
//! expose caller-constructed fragment identities, raw payload variants, or the
//! inventory macro as part of this protocol.

pub use crate::runtime::context::handle::registration::Registration;
pub use crate::runtime::context::handle::registration::RegistrationFragment;

/// Submits a type registration without exposing the inventory dependency.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_type {
    ($ty:ty) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::registration::RegistrationFragment::for_reflected_type::<$ty>(stringify!($ty))
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_type as submit_type;
