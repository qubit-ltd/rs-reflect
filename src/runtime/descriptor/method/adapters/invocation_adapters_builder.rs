// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Builder for an immutable set of generated invocation adapters.
pub struct InvocationAdaptersBuilder {
    /// Adapter for calls using local value storage.
    local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Adapter for calls using thread-safe value storage.
    thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the catching local operation used to resolve or dispatch
    /// reflected methods.
    catching_local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Provides the catching thread safe operation used to resolve or dispatch
    /// reflected methods.
    catching_thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the pinned local operation used to resolve or dispatch
    /// reflected methods.
    pinned_local: Option<&'static dyn PinnedInvocationAdapter<Local>>,
    /// Provides the pinned thread safe operation used to resolve or dispatch
    /// reflected methods.
    pinned_thread_safe: Option<&'static dyn PinnedInvocationAdapter<ThreadSafe>>,
}

impl InvocationAdaptersBuilder {
    /// Sets the local adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn local(mut self, adapter: &'static dyn InvocationAdapter<Local>) -> Self {
        self.local = Some(adapter);
        self
    }

    /// Sets the thread-safe adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn thread_safe(mut self, adapter: &'static dyn InvocationAdapter<ThreadSafe>) -> Self {
        self.thread_safe = Some(adapter);
        self
    }

    /// Sets the local adapter used by explicit panic-catching invocation.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn catching_local(mut self, adapter: &'static dyn InvocationAdapter<Local>) -> Self {
        self.catching_local = Some(adapter);
        self
    }

    /// Sets the thread-safe adapter used by explicit panic-catching invocation.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn catching_thread_safe(mut self, adapter: &'static dyn InvocationAdapter<ThreadSafe>) -> Self {
        self.catching_thread_safe = Some(adapter);
        self
    }

    /// Sets the local pinned receiver adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn pinned_local<T: ?Sized + 'static>(
        mut self,
        adapter: &'static crate::runtime::invoke::PinnedFunctionInvocationAdapter<Local, T>,
    ) -> Self {
        self.pinned_local = Some(adapter);
        self
    }

    /// Sets the thread-safe pinned receiver adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn pinned_thread_safe<T: ?Sized + 'static>(
        mut self,
        adapter: &'static crate::runtime::invoke::PinnedFunctionInvocationAdapter<ThreadSafe, T>,
    ) -> Self {
        self.pinned_thread_safe = Some(adapter);
        self
    }

    /// Finishes the immutable adapter set.
    pub const fn build(self) -> InvocationAdapters {
        InvocationAdapters {
            local: self.local,
            thread_safe: self.thread_safe,
            catching_local: self.catching_local,
            catching_thread_safe: self.catching_thread_safe,
            pinned_local: self.pinned_local,
            pinned_thread_safe: self.pinned_thread_safe,
        }
    }
}

impl InvocationAdapters {
    /// Starts building a typed adapter set with no mutable slots exposed.
    pub const fn builder() -> InvocationAdaptersBuilder {
        InvocationAdaptersBuilder {
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            pinned_local: None,
            pinned_thread_safe: None,
        }
    }

    /// Returns the generated invocation adapter for the selected value mode.
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    pub(super) fn adapter<M: ValueMode + ModeAdapter>(&self) -> Option<&'static dyn InvocationAdapter<M>> {
        M::select_adapter(self)
    }
}
