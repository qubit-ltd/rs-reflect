// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generic model facade fixture declaration.

use model_facade_derive::model_reflect;

/// A generic facade fixture that exercises generated expression metadata.
#[model_reflect]
pub struct FacadeEnvelope<T> {
    /// The reflected payload.
    pub value: T,
}
