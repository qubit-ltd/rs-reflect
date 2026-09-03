// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Trait model facade fixture declaration.

use model_facade_derive::model_reflect_trait;

/// A reflected facade trait with a dynamically callable default method.
#[model_reflect_trait]
pub trait FacadeService {
    /// Returns a stable label through the generated default-method adapter.
    fn label(&self) -> String {
        String::from("facade")
    }
}
