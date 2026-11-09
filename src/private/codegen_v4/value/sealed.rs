// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod mode;
mod mutable_value;
mod owned_value;
mod shared_value;

pub use mode::Mode;
pub use mutable_value::MutableValue;
pub use owned_value::OwnedValue;
pub use shared_value::SharedValue;

/// Prevents downstream implementations of the value-mode marker.
pub trait Sealed {
    // empty
}
