// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[path = "../access/tests.rs"]
mod access;
#[path = "../capability/tests.rs"]
mod capability;
#[path = "construct.rs"]
mod construct;
#[path = "../context/tests.rs"]
mod context;
#[path = "expression.rs"]
mod expression;
#[path = "invoke.rs"]
mod invoke;
mod lookup_error;
mod seam;
#[path = "trait_impl.rs"]
mod trait_impl;
#[path = "value.rs"]
mod value;
