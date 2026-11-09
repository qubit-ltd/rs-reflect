// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;

/// A thread-safe target used to verify cross-crate default-method adapters.
#[Reflect]
pub struct SharedTarget;

#[ReflectImpl(reflected_trait = default_trait_thread_safe_producer::DefaultService)]
impl default_trait_thread_safe_producer::DefaultService for SharedTarget {}
