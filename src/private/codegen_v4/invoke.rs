// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed method invocation adapter contracts emitted by derive.
//!
//! Derive needs safe constructors for method metadata and its
//! `InvocationAdapters` table, including local, thread-safe, catching, and
//! pinned dispatch. Runtime counterparts include `MethodMetadata`,
//! `MethodDescriptor`, `InvocationAdapter`, `PinnedInvocationAdapter`,
//! `ModeAdapter`, and `Invocation`; these are currently crate-private. Adapter
//! factories must preserve receiver, argument, output, and value-mode types in
//! their signatures.

pub use crate::runtime::descriptor::method::EffectiveMethod;
pub use crate::runtime::descriptor::method::FunctionAbi;
pub use crate::runtime::descriptor::method::InvocationAdapters;
pub use crate::runtime::descriptor::method::InvocationAdaptersBuilder;
pub use crate::runtime::descriptor::method::MethodAvailability;
pub use crate::runtime::descriptor::method::MethodCatalogRegistration;
pub use crate::runtime::descriptor::method::MethodImplementationSource;
pub use crate::runtime::descriptor::method::MethodMetadata;
pub use crate::runtime::descriptor::method::MethodMetadataSpec;
pub use crate::runtime::descriptor::method::MethodNamespaceMetadata;
pub use crate::runtime::descriptor::method::MethodQualifiers;
pub use crate::runtime::descriptor::method::MethodSpecializationArgument;
pub use crate::runtime::descriptor::method::MethodSpecializationSpec;
pub use crate::runtime::descriptor::method::MethodVisibility;
pub use crate::runtime::descriptor::method::ParameterMetadata;
pub use crate::runtime::descriptor::method::ParameterPattern;
pub use crate::runtime::descriptor::method::PassingMode;
pub use crate::runtime::descriptor::method::ReceiverDescriptor;
pub use crate::runtime::descriptor::method::ReceiverKind;
pub use crate::runtime::descriptor::method::ReturnKind;
pub use crate::runtime::descriptor::method::ReturnMetadata;
pub use crate::runtime::invoke::BorrowOrigin;
pub use crate::runtime::invoke::FunctionInvocationAdapter;
pub use crate::runtime::invoke::InvocationAdapter;
pub use crate::runtime::invoke::InvocationOutput;
pub use crate::runtime::invoke::InvocationReceiver;
pub use crate::runtime::invoke::InvocationUnavailableReason;
pub use crate::runtime::invoke::InvocationValue;
pub use crate::runtime::invoke::PinnedFunctionInvocationAdapter;
pub use crate::runtime::invoke::PinnedReceiver;
pub use crate::runtime::invoke::ValidatedInvocation;
pub use crate::runtime::lookup::Lookup;

/// Submits an effective method catalog for its exact target type.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_method_catalog {
    ($ty:ty, $methods:expr) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::invoke::MethodCatalogRegistration::new::<$ty>($methods)
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_method_catalog as submit_method_catalog;

/// Submits a method catalog initialized by a typed provider function.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_method_catalog_provider {
    ($ty:ty, $provider:path) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::invoke::MethodCatalogRegistration::with_provider::<$ty>($provider)
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_method_catalog_provider as submit_method_catalog_provider;
/// Registers a method declaration using its concrete reflected owner type.
#[doc(hidden)]
pub use crate::runtime::descriptor::method::EffectiveMethod as RuntimeEffectiveMethod;
