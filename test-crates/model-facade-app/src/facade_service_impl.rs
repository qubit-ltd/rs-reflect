// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Impl model facade fixture declaration.

use model_facade_derive::model_reflect_impl;

use crate::FacadeService;
use crate::FacadeUser;

#[model_reflect_impl]
impl FacadeService for FacadeUser {}
