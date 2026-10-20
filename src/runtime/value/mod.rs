// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]
// The value wrappers are used from generated downstream adapters, so local
// crate dead-code analysis cannot see all of their macro expansion call sites.
#![allow(dead_code)]

mod borrowed;
mod builtin;
mod dynamic;
mod failure;
mod mode;
mod owned;

pub use borrowed::DynamicMut;
pub use borrowed::DynamicRef;
pub use borrowed::ValueForMut;
pub(crate) use borrowed::ValueForMut as RuntimeValueForMut;
pub use borrowed::ValueForRef;
pub(crate) use borrowed::ValueForRef as RuntimeValueForRef;
pub use dynamic::DynamicValue;
pub use dynamic::DynamicValueKind;
pub use failure::DowncastFailure;
pub use failure::TypeMismatch;
pub use mode::Local;
pub use mode::ThreadSafe;
pub use mode::ValueMode;
pub(crate) use mode::ValueMode as RuntimeValueMode;
pub use owned::DynamicOwned;
pub(crate) use owned::ValueFor as RuntimeValueFor;
pub use owned::ValueFor;
