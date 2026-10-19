// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
// =============================================================================

//! Typed trait declaration and application metadata for generated code.

use std::marker::PhantomData;

use crate::runtime::descriptor::generic::{GenericArgumentMetadata, GenericDefinitionMetadata};
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::trait_descriptor::{
    AssociatedConstDefinitionMetadata, AssociatedTypeDefinitionMetadata, SupertraitCompleteness,
    TraitApplicationMetadata, TraitArgumentIdentity, TraitCompleteness, TraitDefinitionMetadata,
    TraitExpressionMetadata, Visibility,
};
use crate::runtime::expression::{ExpressionData, PredicateData, TraitExpressionData};

pub use crate::runtime::descriptor::trait_descriptor::{
    AssociatedConstDefinitionMetadata as AssociatedConst,
    AssociatedTypeDefinitionMetadata as AssociatedType, SupertraitCompleteness as SupertraitStatus,
    TraitArgumentIdentity as ArgumentIdentity, TraitCompleteness as Completeness,
    TraitExpressionMetadata as TraitExpression, Visibility as TraitVisibility,
};

/// Marker-bound static metadata for one trait declaration.
pub struct TraitDefinitionSpec<D: 'static> {
    metadata: TraitDefinitionMetadata,
    _marker: PhantomData<fn() -> D>,
}

impl<D: 'static> TraitDefinitionSpec<D> {
    /// Creates a trait definition without exposing or accepting its identity token.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: &'static str,
        path: &'static str,
        visibility: Visibility,
        completeness: TraitCompleteness,
        generics: &'static GenericDefinitionMetadata,
        supertraits: &'static [TraitExpressionMetadata],
        methods: &'static [MethodMetadata],
        associated_types: &'static [AssociatedTypeDefinitionMetadata],
        associated_consts: &'static [AssociatedConstDefinitionMetadata],
    ) -> Self {
        Self {
            metadata: TraitDefinitionMetadata::new::<D>(
                name,
                path,
                visibility,
                completeness,
                generics,
                supertraits,
                methods,
                associated_types,
                associated_consts,
            ),
            _marker: PhantomData,
        }
    }

    /// Borrows this static declaration for another typed metadata node.
    pub const fn metadata(&'static self) -> &'static TraitDefinitionMetadata {
        &self.metadata
    }

    /// Returns an associated type declaration by its source-order index.
    pub const fn associated_type_at(
        &'static self,
        index: usize,
    ) -> &'static AssociatedTypeDefinitionMetadata {
        if index >= self.metadata.associated_types.len() {
            panic!("associated type index is out of range");
        }
        &self.metadata.associated_types[index]
    }

    /// Returns an associated constant declaration by its source-order index.
    pub const fn associated_const_at(
        &'static self,
        index: usize,
    ) -> &'static AssociatedConstDefinitionMetadata {
        if index >= self.metadata.associated_consts.len() {
            panic!("associated constant index is out of range");
        }
        &self.metadata.associated_consts[index]
    }
}

/// Marker-bound static metadata for one compiled trait application.
pub struct TraitApplicationSpec<T: 'static> {
    metadata: TraitApplicationMetadata,
    _target: PhantomData<fn() -> T>,
}

impl<T: crate::runtime::descriptor::Reflect + 'static> TraitApplicationSpec<T> {
    /// Creates an application whose declaration and target are both type-bound.
    pub const fn for_target<D: 'static>(
        definition: &'static TraitDefinitionSpec<D>,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        direct_supertraits: &'static [&'static TraitApplicationMetadata],
        all_supertraits: &'static [&'static TraitApplicationMetadata],
        supertrait_completeness: SupertraitCompleteness,
    ) -> Self {
        Self {
            metadata: TraitApplicationMetadata::new(
                definition.metadata(),
                arguments,
                identity,
                direct_supertraits,
                all_supertraits,
                supertrait_completeness,
            ),
            _target: PhantomData,
        }
    }

    /// Borrows this static application metadata for an implementation binding.
    pub const fn metadata(&'static self) -> &'static TraitApplicationMetadata {
        &self.metadata
    }
}

/// Creates an unresolved source trait expression.
pub const fn unresolved_trait_expression(
    expression: &'static TraitExpressionData,
) -> TraitExpressionMetadata {
    TraitExpressionMetadata::unresolved(expression)
}

/// Associates a source trait expression with a typed local definition.
pub const fn trait_expression_for<D: 'static>(
    expression: &'static TraitExpressionData,
    definition: &'static TraitDefinitionSpec<D>,
) -> TraitExpressionMetadata {
    TraitExpressionMetadata::for_definition(expression, definition.metadata())
}

/// Creates metadata for a trait associated type.
pub const fn associated_type(
    name: &'static str,
    visibility: Visibility,
    bounds: &'static [PredicateData],
    default: Option<&'static ExpressionData>,
) -> AssociatedTypeDefinitionMetadata {
    AssociatedTypeDefinitionMetadata::new(name, visibility, bounds, default)
}

/// Creates metadata for a trait associated constant.
pub const fn associated_const(
    name: &'static str,
    visibility: Visibility,
    ty: &'static ExpressionData,
    default: Option<crate::runtime::expression::ConstExpression>,
) -> AssociatedConstDefinitionMetadata {
    AssociatedConstDefinitionMetadata::new(name, visibility, ty, default)
}

const fn no_dependencies() -> &'static [std::any::TypeId] {
    &[]
}

/// Builds a global trait definition registration from marker-bound metadata.
#[doc(hidden)]
pub fn register_trait_definition<D: 'static>(
    definition: &'static TraitDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::metadata_only()
        .with_trait_definition(definition.metadata())
}

/// Builds a concrete trait application registration for target `T`.
#[doc(hidden)]
pub fn register_trait_application<T: crate::runtime::descriptor::Reflect + 'static>(
    application: &'static TraitApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::of::<T>()
        .with_trait_application(application.metadata())
}

/// Creates a global metadata fragment without exposing raw inventory payloads.
#[doc(hidden)]
pub const fn trait_definition_fragment(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_global_registration(
        label,
        registration,
    )
}

/// Creates an application fragment bound to target `T`.
#[doc(hidden)]
pub const fn trait_application_fragment<T: crate::runtime::descriptor::Reflect + 'static>(
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

/// Submits a marker-bound trait definition without exposing inventory internals.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_trait_definition {
    ($marker:ty, $definition:expr) => {
        const _: () = {
            static DEFINITION: $crate::__private::codegen_v4::traits::TraitDefinitionSpec<$marker> =
                $definition;
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::traits::register_trait_definition(&DEFINITION)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::traits::trait_definition_fragment(
                    stringify!($marker),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_trait_definition as submit_trait_definition;

/// Submits a typed trait application for a concrete reflected target.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_trait_application {
    ($target:ty, $application:expr) => {
        const _: () = {
            static APPLICATION: $crate::__private::codegen_v4::traits::TraitApplicationSpec<
                $target,
            > = $application;
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::traits::register_trait_application(&APPLICATION)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::traits::trait_application_fragment::<$target>(
                    concat!(stringify!($target), "::trait-application"),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_trait_application as submit_trait_application;
