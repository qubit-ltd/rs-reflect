// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Method parameter, receiver, return, visibility, and qualifier facts.

mod method_qualifiers;
mod method_visibility;
mod parameter_descriptor;
mod parameter_passing_mode;
mod parameter_pattern_descriptor;
mod receiver_descriptor;
mod return_descriptor;
mod return_kind;

pub use method_qualifiers::MethodQualifiers;
pub use method_visibility::MethodVisibility;
pub use parameter_descriptor::ParameterDescriptor;
pub use parameter_passing_mode::ParameterPassingMode;
pub use parameter_pattern_descriptor::ParameterPatternDescriptor;
pub use receiver_descriptor::ReceiverDescriptor;
pub use return_descriptor::ReturnDescriptor;
pub use return_kind::ReturnKind;
