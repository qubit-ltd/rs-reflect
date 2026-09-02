// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Model facade fixture declaration.

use model_facade_derive::model_reflect;

/// A model-facade fixture whose descriptor is generated through delegation.
#[model_reflect]
pub struct FacadeUser {
    /// The reflected identifier used by facade integration assertions.
    pub id: u64,
}
