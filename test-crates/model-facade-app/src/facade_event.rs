// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Enum model facade fixture declaration.

use model_facade_derive::model_reflect;

/// An enum facade fixture that exercises variant and construction metadata.
#[model_reflect]
pub enum FacadeEvent {
    /// A field-free event.
    Ready,
    /// An event carrying a reflected value.
    Data(String),
}
