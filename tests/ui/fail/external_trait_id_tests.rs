// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================


struct Subject;
trait External {}

#[qubit_reflect::ReflectImpl(external_trait_id = "invalid..identifier")]
impl External for Subject {}

fn main() {}
