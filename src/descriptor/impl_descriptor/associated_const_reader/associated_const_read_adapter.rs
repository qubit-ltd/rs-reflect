// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Private storage forms for generated associated constant readers.

use crate::value::ReflectedOwned;

/// Adapter retained by one associated constant reader to produce owned values.
///
/// Function adapters borrow no state; closure adapters retain their captures
/// for the process lifetime and produce a fresh value on each invocation.
pub(super) enum AssociatedConstReadAdapter {
    /// Non-capturing generated function returning a fresh owned value.
    Function(fn() -> ReflectedOwned),
    /// Process-lifetime closure adapting a concrete value getter.
    Closure(&'static (dyn Fn() -> ReflectedOwned + Send + Sync)),
}
