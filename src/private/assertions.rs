// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compile-time trait assertions shared with generated reflection code.

use crate::descriptor::Reflect;

/// Requires `T` to provide the crate's unique static reflection contract.
#[doc(hidden)]
pub const fn assert_reflect<T: Reflect + ?Sized>() {}

/// Requires `T` to have a process-local runtime identity.
#[doc(hidden)]
pub const fn assert_static<T: ?Sized + 'static>() {}
