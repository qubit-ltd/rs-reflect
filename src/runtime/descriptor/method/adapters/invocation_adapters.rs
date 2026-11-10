// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Mode-specific adapters generated for a concrete specialization.
#[derive(Clone, Copy)]
pub struct InvocationAdapters {
    /// Adapter for calls using local value storage.
    pub(crate) local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Adapter for calls using thread-safe value storage.
    pub(crate) thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the catching local operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) catching_local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Provides the catching thread safe operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) catching_thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the pinned local operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) pinned_local: Option<&'static dyn PinnedInvocationAdapter<Local>>,
    /// Provides the pinned thread safe operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) pinned_thread_safe: Option<&'static dyn PinnedInvocationAdapter<ThreadSafe>>,
}

pub(super) static NO_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: None,
    thread_safe: None,
    catching_local: None,
    catching_thread_safe: None,
    pinned_local: None,
    pinned_thread_safe: None,
};
