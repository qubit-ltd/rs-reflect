// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed linked-registration contracts consumed by reflection contexts.
//!
//! The current runtime stores `RegistrationFragment` and its inventory
//! collector under `runtime::context`; both are crate-private. Derive needs a
//! safe submit macro or typed registration constructor that binds a concrete
//! `Reflect` type to its generated registration and source origin. It must not
//! expose caller-constructed fragment identities, raw payload variants, or the
//! inventory macro as part of this protocol.

pub use crate::runtime::context::handle::TraitObjectDescriptor;
pub use crate::runtime::context::handle::registration::Registration;
pub use crate::runtime::context::handle::registration::RegistrationFragment;
pub use crate::runtime::context::handle::registration::TraitObjectRegistration;
pub use crate::runtime::context::handle::registration::TypeDefinitionRegistration;

/// Combines generic definition metadata with a capability targeting its marker.
///
/// This keeps the definition identity and capability target tied to the same
/// generated marker, so downstream code does not need to construct or expose a
/// `TypeDefinitionId`.
pub fn type_definition_with_capability<TMarker: 'static, C: crate::runtime::capability::CapabilitySpec>(
    definition: &'static TypeDefinitionRegistration<TMarker>,
    adapter: C::Adapter,
) -> Registration {
    Registration::metadata_only()
        .with_type_definition(definition)
        .with_capability(crate::runtime::capability::CapabilityRegistration::for_type_definition::<TMarker, C>(adapter))
}

/// Creates a linked capability-only fragment for a generic definition.
#[doc(hidden)]
pub const fn linked_definition_capability_fragment(
    label: &'static str,
    registration: fn() -> Registration,
) -> RegistrationFragment {
    RegistrationFragment::for_linked_capability(label, registration)
}

/// Creates a global registration fragment for a compiler-checked trait object.
#[doc(hidden)]
pub const fn trait_object_fragment(label: &'static str, registration: fn() -> Registration) -> RegistrationFragment {
    RegistrationFragment::for_global_registration(label, registration)
}

/// A method binding whose target type is fixed by `T`.
pub struct MethodBinding<T: crate::runtime::descriptor::Reflect> {
    effective: crate::runtime::descriptor::method::EffectiveMethod,
    _target: std::marker::PhantomData<fn() -> T>,
}

impl<T: crate::runtime::descriptor::Reflect> MethodBinding<T> {
    /// Adds statically typed generic method specializations to this binding.
    pub const fn with_specializations(
        mut self,
        specializations: &'static [crate::runtime::descriptor::method::MethodSpecializationSpec],
    ) -> Self {
        self.effective = self.effective.with_specializations(specializations);
        self
    }

    /// Marks this binding unavailable with typed reasons.
    pub const fn with_unavailable(
        mut self,
        reasons: &'static [crate::runtime::invoke::InvocationUnavailableReason],
    ) -> Self {
        self.effective = self.effective.with_unavailable(reasons);
        self
    }

    /// Returns the effective entry for inclusion in `T`'s method catalog.
    pub const fn effective(&self) -> crate::runtime::descriptor::method::EffectiveMethod {
        self.effective
    }
}

impl<T: crate::runtime::descriptor::Reflect> From<MethodBinding<T>>
    for crate::runtime::context::handle::registration::Registration
{
    fn from(binding: MethodBinding<T>) -> Self {
        crate::runtime::context::handle::registration::Registration::metadata_only()
            .with_method_binding::<T>(binding.effective)
    }
}

/// Binds method metadata and generated adapters to their concrete target type.
pub const fn bind_method<T: crate::runtime::descriptor::Reflect>(
    metadata: &'static crate::runtime::descriptor::method::MethodMetadata,
    adapters: crate::runtime::descriptor::method::InvocationAdapters,
) -> MethodBinding<T> {
    MethodBinding {
        effective: crate::runtime::descriptor::method::EffectiveMethod::for_type::<T>(metadata).with_adapters(adapters),
        _target: std::marker::PhantomData,
    }
}

/// Submits a type registration without exposing the inventory dependency.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_type {
    ($ty:ty) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::registration::RegistrationFragment::for_reflected_type::<$ty>(stringify!($ty))
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_type as submit_type;

/// Submits a generic type definition independently of any concrete instance.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_type_definition {
    ($label:expr, $registration:path) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::registration::RegistrationFragment::for_type_definition(
                $label,
                $registration,
            )
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_type_definition as submit_type_definition;

/// Submits a capability for a definition already registered by its derive.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_linked_definition_capability {
    ($label:expr, $registration:path) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::registration::linked_definition_capability_fragment(
                $label,
                $registration,
            )
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_linked_definition_capability as submit_linked_definition_capability;

/// Creates a statically typed method binding for the exact target type.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_bind_method {
    ($ty:ty, $metadata:expr, $adapters:expr) => {
        $crate::__private::codegen_v4::registration::bind_method::<$ty>($metadata, $adapters)
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_bind_method as bind_method;

/// Submits a target's method catalog without exposing the inventory dependency.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_register_method_binding {
    ($ty:ty, $methods:expr) => {
        $crate::__private::__inventory::submit! {
            $crate::__private::codegen_v4::invoke::MethodCatalogRegistration::new::<$ty>($methods)
        }
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_register_method_binding as register_method_binding;
