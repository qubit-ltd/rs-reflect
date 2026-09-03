// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Integration tests for descriptor APIs.

#[cfg(feature = "derive")]
mod associated_item_impl_tests;
mod builtin_tests;
#[cfg(feature = "derive")]
mod capability_tests;
#[cfg(feature = "derive")]
mod default_trait_invocation_tests;
#[cfg(feature = "derive")]
mod derive_enum_tests;
#[cfg(feature = "derive")]
mod derive_struct_tests;
mod expression_tests;
#[cfg(feature = "derive")]
mod generic_tests;
mod identity_tests;
#[cfg(feature = "derive")]
mod reflect_impl_tests;
#[cfg(feature = "derive")]
mod reflect_trait_tests;
mod trait_tests;
#[cfg(feature = "derive")]
mod type_descriptor_tests;
