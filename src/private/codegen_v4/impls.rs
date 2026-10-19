// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Typed implementation declaration and application metadata for generated code.

use std::marker::PhantomData;

use crate::runtime::descriptor::generic::{
    GenericArgumentMetadata, GenericDefinitionMetadata, GenericParameterMetadata,
};
use crate::runtime::descriptor::impl_descriptor::{
    AssociatedConstBindingMetadata, AssociatedTypeBindingMetadata, ImplApplicationMetadata,
    ImplDefinitionMetadata, ImplKind, ImplQualifiers,
};
use crate::runtime::descriptor::method::{EffectiveMethod, MethodMetadata};
use crate::runtime::descriptor::trait_descriptor::{
    AssociatedConstDefinitionMetadata, AssociatedTypeDefinitionMetadata, TraitApplicationMetadata,
    TraitArgumentIdentity, TraitExpressionMetadata,
};
use crate::runtime::expression::ExpressionData;

pub use crate::runtime::descriptor::impl_descriptor::{
    AssociatedConstBindingMetadata as AssociatedConstBinding, AssociatedItemSource as ItemSource,
    AssociatedTypeBindingMetadata as AssociatedTypeBinding, ImplKind as Kind,
    ImplQualifiers as Qualifiers,
};
pub use crate::runtime::descriptor::trait_descriptor::{
    AssociatedConstDefinitionMetadata as AssociatedConst,
    AssociatedTypeDefinitionMetadata as AssociatedType,
};

/// Marker-bound static metadata for one source impl block.
pub struct ImplDefinitionSpec<D: 'static> {
    metadata: ImplDefinitionMetadata,
    _marker: PhantomData<fn() -> D>,
}

impl<D: 'static> ImplDefinitionSpec<D> {
    /// Creates definition metadata whose identity is bound to marker type `D`.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        kind: ImplKind,
        target: &'static ExpressionData,
        implemented_trait: Option<&'static TraitExpressionMetadata>,
        generics: &'static GenericDefinitionMetadata,
        qualifiers: ImplQualifiers,
        methods: &'static [MethodMetadata],
        associated_types: &'static [AssociatedTypeDefinitionMetadata],
        associated_consts: &'static [AssociatedConstDefinitionMetadata],
    ) -> Self {
        Self {
            metadata: ImplDefinitionMetadata::new::<D>(
                kind,
                target,
                implemented_trait,
                generics,
                qualifiers,
                methods,
                associated_types,
                associated_consts,
            ),
            _marker: PhantomData,
        }
    }

    /// Borrows this static definition for a typed application.
    pub const fn metadata(&'static self) -> &'static ImplDefinitionMetadata {
        &self.metadata
    }

    /// Returns an impl generic parameter by its source-order index.
    pub const fn generic_parameter_at(
        &'static self,
        index: usize,
    ) -> &'static GenericParameterMetadata {
        if index >= self.metadata.generics.parameters.len() {
            panic!("impl generic parameter index is out of range");
        }
        &self.metadata.generics.parameters[index]
    }
}

/// Static application metadata for one concrete target type.
pub struct ImplApplicationSpec<T: 'static> {
    metadata: ImplApplicationMetadata,
    _target: PhantomData<fn() -> T>,
}

impl<T: crate::runtime::descriptor::Reflect + 'static> ImplApplicationSpec<T> {
    /// Creates a concrete application linked to marker-bound impl metadata.
    #[allow(clippy::too_many_arguments)]
    pub const fn for_target<D: 'static>(
        definition: &'static ImplDefinitionSpec<D>,
        implemented_trait: Option<&'static TraitApplicationMetadata>,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        methods: &'static [EffectiveMethod],
        associated_types: &'static [AssociatedTypeBindingMetadata],
        associated_consts: &'static [AssociatedConstBindingMetadata],
    ) -> Self {
        Self {
            metadata: ImplApplicationMetadata::for_target::<T>(
                definition.metadata(),
                implemented_trait,
                arguments,
                identity,
                methods,
                associated_types,
                associated_consts,
            ),
            _target: PhantomData,
        }
    }

    /// Borrows this static application for the runtime registration closure.
    pub const fn metadata(&'static self) -> &'static ImplApplicationMetadata {
        &self.metadata
    }
}

/// Creates a typed associated type binding from its trait declaration item.
pub const fn associated_type_binding(
    definition: &'static AssociatedType,
    expression: &'static ExpressionData,
    source: crate::runtime::descriptor::impl_descriptor::AssociatedItemSource,
) -> AssociatedTypeBindingMetadata {
    AssociatedTypeBindingMetadata::new(definition, expression, None, source)
}

/// Creates an associated type binding linked to its exact reflected Rust type.
pub const fn associated_type_binding_for<T: crate::runtime::descriptor::Reflect>(
    definition: &'static AssociatedType,
    expression: &'static ExpressionData,
    source: crate::runtime::descriptor::impl_descriptor::AssociatedItemSource,
) -> AssociatedTypeBindingMetadata {
    AssociatedTypeBindingMetadata::new(
        definition,
        expression,
        Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
        source,
    )
}

/// Creates a typed associated constant binding without a dynamic value adapter.
pub const fn associated_const_binding(
    definition: &'static AssociatedConst,
    expression: crate::runtime::expression::ConstExpression,
    source: crate::runtime::descriptor::impl_descriptor::AssociatedItemSource,
) -> AssociatedConstBindingMetadata {
    AssociatedConstBindingMetadata::new(definition, expression, source)
}

/// Builds a registration value from a marker-bound impl definition.
#[doc(hidden)]
pub fn register_impl_definition<D: 'static>(
    definition: &'static ImplDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::metadata_only()
        .with_impl_definition(definition.metadata())
}

/// Builds an impl definition registration from a cached static metadata provider.
#[doc(hidden)]
pub fn register_impl_definition_from<D: 'static>(
    definition: fn() -> &'static ImplDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    register_impl_definition(definition())
}

/// Builds a concrete registration value from an impl application.
#[doc(hidden)]
pub fn register_impl_application<T: crate::runtime::descriptor::Reflect + 'static>(
    application: &'static ImplApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::of::<T>()
        .with_impl_application(application.metadata())
}

/// Builds an impl application registration from a cached static metadata provider.
#[doc(hidden)]
pub fn register_impl_application_from<T: crate::runtime::descriptor::Reflect + 'static>(
    application: fn() -> &'static ImplApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    register_impl_application(application())
}

const fn no_dependencies() -> &'static [std::any::TypeId] {
    &[]
}

/// Creates a global definition fragment without exposing inventory payloads.
#[doc(hidden)]
pub const fn impl_definition_fragment(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_global_registration(
        label,
        registration,
    )
}

/// Creates an impl application fragment bound to `T`.
#[doc(hidden)]
pub const fn impl_application_fragment<T: crate::runtime::descriptor::Reflect + 'static>(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_target::<T>(
        (
            "generated",
            None,
            "generated",
            Some(label),
            None,
            None,
            label,
        ),
        no_dependencies,
        registration,
    )
}

/// Submits one marker-bound impl definition.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_impl_definition {
    ($marker:ty, $definition:path) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::impls::register_impl_definition_from::<$marker>(
                    $definition,
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::impls::impl_definition_fragment(
                    stringify!($marker),
                    registration,
                )
            }
        };
    };
    ($marker:ty, static $definition:expr) => {
        const _: () = {
            static DEFINITION: $crate::__private::codegen_v4::impls::ImplDefinitionSpec<$marker> =
                $definition;
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::impls::register_impl_definition(&DEFINITION)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::impls::impl_definition_fragment(
                    stringify!($marker),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_impl_definition as submit_impl_definition;

/// Submits an impl application linked to the reflected target `T`.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_impl_application {
    ($target:ty, $application:path) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::impls::register_impl_application_from::<$target>(
                    $application,
                )
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::impls::impl_application_fragment::<$target>(
                    concat!(stringify!($target), "::impl-application"),
                    registration,
                )
            }
        };
    };
    ($target:ty, static $application:expr) => {
        const _: () = {
            static APPLICATION: $crate::__private::codegen_v4::impls::ImplApplicationSpec<$target> =
                $application;
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::impls::register_impl_application(&APPLICATION)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::impls::impl_application_fragment::<$target>(
                    concat!(stringify!($target), "::impl-application"),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_impl_application as submit_impl_application;
