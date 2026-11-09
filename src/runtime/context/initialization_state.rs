// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::thread::ThreadId;

use crate::runtime::context::handle::data::ContextData;
use crate::runtime::error::ContextError;

/// Synchronization state for one-time global context initialization.
#[derive(Default)]
pub(super) struct InitializationState {
    /// Indicates whether an initializer currently owns the build operation.
    pub(super) building: bool,
    /// Thread that currently owns initialization, if any.
    pub(super) owner: Option<ThreadId>,
    /// Cached initialization result once a build finishes.
    pub(super) result: Option<Result<Arc<ContextData>, ContextError>>,
    /// Number of threads currently waiting for initialization in tests.
    #[cfg(test)]
    pub(super) waiters: usize,
}
