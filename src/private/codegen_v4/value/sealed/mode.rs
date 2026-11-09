// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::value as runtime_value;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;

/// Selects the runtime representation used by a public value mode.
pub trait Mode: 'static {
    /// Runtime value mode that implements this marker's representation.
    type Runtime: runtime_value::RuntimeValueMode;
}

impl Mode for Local {
    /// Runtime representation for local values and borrows.
    type Runtime = runtime_value::Local;
}

impl Mode for ThreadSafe {
    /// Runtime representation for thread-safe values and borrows.
    type Runtime = runtime_value::ThreadSafe;
}
