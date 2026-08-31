// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::reflect_impl;

struct Subject;
trait External {}

#[reflect_impl(external_trait_id = "invalid..identifier")]
impl External for Subject {}

fn main() {}
