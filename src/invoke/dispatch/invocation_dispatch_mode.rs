// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Requested invocation dispatch modes.

/// The invocation entry requested by the caller before input validation.
///
/// # Examples
///
/// ```
/// use qubit_reflect::invoke::InvocationDispatchMode;
///
/// let mode = InvocationDispatchMode::CatchingLocal;
/// assert_ne!(mode, InvocationDispatchMode::Local);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationDispatchMode {
    /// Ordinary invocation using local receivers, arguments and output.
    Local,
    /// Ordinary invocation using thread-safe receivers, arguments and output.
    ThreadSafe,
    /// Local invocation requesting synchronous panic capture.
    CatchingLocal,
    /// Thread-safe invocation requesting synchronous panic capture.
    CatchingThreadSafe,
    /// Local invocation with a pinned shared receiver.
    PinnedRefLocal,
    /// Local invocation with a pinned mutable receiver.
    PinnedMutLocal,
}
