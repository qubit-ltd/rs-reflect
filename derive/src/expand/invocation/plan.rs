// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Parser-independent invocation plan produced before token emission.

#![allow(dead_code, reason = "T5 fills and consumes the plan")]

/// Complete invocation facts shared by impl and trait expansion.
pub(crate) struct InvocationPlan {
    pub(crate) receiver: Option<ReceiverPlan>,
    pub(crate) parameters: Vec<ParameterPlan>,
    pub(crate) output: OutputPlan,
    pub(crate) modes: AdapterModes,
    pub(crate) availability: AvailabilityPlan,
}

/// Receiver facts retained for adapter emission.
pub(crate) struct ReceiverPlan;

/// One parameter's validated invocation facts.
pub(crate) struct ParameterPlan;

/// Validated output facts retained for adapter emission.
pub(crate) struct OutputPlan;

/// Requested local, thread-safe, and catching adapters.
pub(crate) struct AdapterModes;

/// Whether code can emit an executable adapter.
pub(crate) enum AvailabilityPlan {
    Executable,
    DescribedOnly(Vec<UnavailableReasonPlan>),
}

/// One stable reason an adapter cannot be emitted.
pub(crate) struct UnavailableReasonPlan;
