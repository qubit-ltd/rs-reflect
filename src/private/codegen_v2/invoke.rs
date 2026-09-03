// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Exact invocation surface consumed by codegen v2.

#[doc(hidden)]
pub use crate::invoke::ArgumentExpectation;
#[doc(hidden)]
pub use crate::invoke::BorrowOrigin;
#[doc(hidden)]
pub use crate::invoke::CatchingInvocationAdapter;
#[doc(hidden)]
pub use crate::invoke::Invocation;
#[doc(hidden)]
pub use crate::invoke::InvocationArg;
#[doc(hidden)]
pub use crate::invoke::InvocationFailure;
#[doc(hidden)]
pub use crate::invoke::InvocationOutput;
#[doc(hidden)]
pub use crate::invoke::InvocationPanic;
#[doc(hidden)]
pub use crate::invoke::InvocationReceiver;
#[doc(hidden)]
pub use crate::invoke::PinnedMutAdapter;
#[doc(hidden)]
pub use crate::invoke::PinnedMutInvocation;
#[doc(hidden)]
pub use crate::invoke::PinnedMutInvocationFailure;
#[doc(hidden)]
pub use crate::invoke::PinnedRefAdapter;
#[doc(hidden)]
pub use crate::invoke::PinnedRefInvocation;
#[doc(hidden)]
pub use crate::invoke::PinnedRefInvocationFailure;
#[doc(hidden)]
pub use crate::invoke::ReceiverAdapter;
#[doc(hidden)]
pub use crate::invoke::ReceiverExpectation;
#[doc(hidden)]
pub use crate::invoke::ReflectedFuture;
#[doc(hidden)]
pub use crate::invoke::receiver_adapter_key;
