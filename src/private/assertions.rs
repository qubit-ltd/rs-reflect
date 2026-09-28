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
///
/// # Type Parameters
///
/// - `T`: Reflected type whose static contract is checked at compile time.
///
/// # Returns
///
/// Returns unit when the type satisfies `Reflect`.
#[doc(hidden)]
pub const fn assert_reflect<T: Reflect + ?Sized>() {}

/// Requires `T` to have a process-local runtime identity.
///
/// # Type Parameters
///
/// - `T`: Type whose `'static` lifetime is checked at compile time.
///
/// # Returns
///
/// Returns unit when `T` is `'static`.
#[doc(hidden)]
pub const fn assert_static<T: ?Sized + 'static>() {}
