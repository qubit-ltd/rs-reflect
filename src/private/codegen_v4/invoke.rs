// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed method invocation adapter contracts emitted by derive.
//!
//! Derive needs safe constructors for method metadata and its `InvocationAdapters`
//! table, including local, thread-safe, catching, and pinned dispatch. Runtime
//! counterparts include `MethodMetadata`, `MethodDescriptor`,
//! `InvocationAdapter`, `PinnedInvocationAdapter`, `ModeAdapter`, and
//! `Invocation`; these are currently crate-private. Adapter factories must
//! preserve receiver, argument, output, and value-mode types in their
//! signatures.
