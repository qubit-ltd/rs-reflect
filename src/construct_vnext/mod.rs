// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public construction and owned-update facade over the crate-private runtime.

pub use crate::__private::codegen_v4::construct::ConstructField;
pub use crate::__private::codegen_v4::construct::ConstructionAvailability;
pub use crate::__private::codegen_v4::construct::ConstructionFieldPolicy;
pub use crate::__private::codegen_v4::construct::ConstructionPlan;
pub use crate::__private::codegen_v4::construct::ConstructionProviders;
pub use crate::__private::codegen_v4::construct::ConstructionShape;
pub use crate::__private::codegen_v4::construct::ConstructionUnavailableReason;
pub use crate::__private::codegen_v4::construct::UpdateFieldPolicy;
pub use crate::__private::codegen_v4::construct::UpdateUnavailableReason;

// The construction facade keeps public value shapes, failure recovery, and
// descriptor operations in separate modules while preserving its public paths.
mod construction_target;
pub use construction_target::ConstructionTarget;
mod construction_fields;
pub use construction_fields::ConstructionFields;
mod named_fields;
pub use named_fields::NamedFields;
mod positional_fields;
pub use positional_fields::PositionalFields;
mod unit_fields;
pub use unit_fields::UnitFields;
mod construction_mode;
pub use construction_mode::ConstructionMode;
mod construction_input_mode;
pub use construction_input_mode::ConstructionInputMode;
mod runtime_construction_input;
pub use runtime_construction_input::RuntimeConstructionInput;
mod construction_input;
pub use construction_input::ConstructionInput;
mod construction_error;
pub use construction_error::ConstructionError;
mod update_error;
pub use update_error::UpdateError;
mod construction_failure;
pub use construction_failure::ConstructionFailure;
mod recovered_construction_input;
pub use recovered_construction_input::RecoveredConstructionInput;
mod construction_recovery;
pub use construction_recovery::ConstructionRecovery;
mod update_failure;
pub use update_failure::UpdateFailure;
mod update_input;
pub use update_input::UpdateInput;
mod update_recovery;
pub use update_recovery::UpdateRecovery;
mod construction_descriptor;
pub use construction_descriptor::ConstructionDescriptor;
mod internal;

#[cfg(all(test, feature = "derive"))]
mod tests;
