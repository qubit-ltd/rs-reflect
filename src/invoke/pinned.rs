// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed invocation inputs that retain a borrowed receiver's pin guarantee.

mod pinned_mut_adapter;
mod pinned_mut_invocation;
mod pinned_mut_invocation_failure;
mod pinned_mut_invocation_recovery;
mod pinned_ref_adapter;
mod pinned_ref_invocation;
mod pinned_ref_invocation_failure;
mod pinned_ref_invocation_recovery;
mod pinned_validated_mut_invocation;
mod pinned_validated_ref_invocation;

pub use pinned_mut_adapter::PinnedMutAdapter;
pub use pinned_mut_invocation::PinnedMutInvocation;
pub use pinned_mut_invocation_failure::PinnedMutInvocationFailure;
pub use pinned_mut_invocation_recovery::PinnedMutInvocationRecovery;
pub use pinned_ref_adapter::PinnedRefAdapter;
pub use pinned_ref_invocation::PinnedRefInvocation;
pub use pinned_ref_invocation_failure::PinnedRefInvocationFailure;
pub use pinned_ref_invocation_recovery::PinnedRefInvocationRecovery;
pub use pinned_validated_mut_invocation::PinnedValidatedMutInvocation;
pub use pinned_validated_ref_invocation::PinnedValidatedRefInvocation;
