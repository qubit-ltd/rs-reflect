// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Versioned registration contracts used by generated code and contexts.
//!
//! The runtime implementation modules remain private. This hidden v4 module
//! re-exports `RegistrationFragment`, `Registration`, trait-object and type-
//! definition registration types, and `TypeDefinitionMetadata`, and provides
//! typed submission macros. These entry points let generated code associate
//! registrations with concrete types and definitions without exposing the
//! inventory dependency. Their accessibility does not make the whole module a
//! supported compatibility surface; see the architecture and user guides for
//! the items the project currently supports.

pub use crate::runtime::context::handle::TraitObjectDescriptor;
pub use crate::runtime::context::handle::registration::Registration;
pub use crate::runtime::context::handle::registration::RegistrationFragment;
pub use crate::runtime::context::handle::registration::TraitObjectRegistration;
pub use crate::runtime::context::handle::registration::TypeDefinitionRegistration;
pub use crate::runtime::descriptor::generic::TypeDefinitionMetadata;

/// Runs the production fragment selector on a synthetic ordered fragment slice.
///
/// This entry point exists only for the crate's Criterion benchmark and does
/// not form a supported API.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub fn benchmark_select_fragments(
    fragments: &[(Option<usize>, bool, Vec<usize>)],
    root: usize,
) -> (Vec<bool>, Vec<(usize, usize)>) {
    crate::runtime::context::handle::builder::select_fragment_slice(
        fragments,
        root,
        |fragment| fragment.0,
        |fragment| fragment.2.as_slice(),
        |fragment| fragment.1,
    )
}

/// Builds an ordered synthetic graph and checks its adverse dependency order.
#[cfg(any(test, feature = "bench-internals"))]
#[doc(hidden)]
pub fn benchmark_fragment_graph(
    count: usize,
    depth: usize,
    shape: &str,
) -> Vec<(Option<usize>, bool, Vec<usize>)> {
    let active = depth.min(count);
    let mut fragments: Vec<_> = (0..count)
        .map(|index| {
            let dependencies = match shape {
                "chain" | "reverse_chain" if index + 1 < active => vec![index + 1],
                "branch" if index < active && index + 2 < active => vec![index + 1, index + 2],
                "cycle" if index < active => vec![(index + 1) % active],
                "shared" if index > 0 && index < active => vec![0],
                _ => Vec::new(),
            };
            (Some(index), false, dependencies)
        })
        .collect();
    if active > 1 && matches!(shape, "reverse_chain" | "branch") {
        fragments.reverse();
        let active_targets: Vec<_> = fragments
            .iter()
            .filter_map(|fragment| fragment.0.filter(|target| *target < active))
            .collect();
        assert_eq!(active_targets, (0..active).rev().collect::<Vec<_>>());
        assert_eq!(active_targets.last(), Some(&0));
        for (source_index, fragment) in fragments.iter().enumerate() {
            for dependency in &fragment.2 {
                let dependency_index = fragments
                    .iter()
                    .position(|candidate| candidate.0 == Some(*dependency))
                    .expect("each synthetic dependency has a target fragment");
                assert!(dependency_index < source_index);
            }
        }
    }
    fragments
}

#[cfg(test)]
mod benchmark_graph_tests {
    use super::benchmark_fragment_graph;

    #[test]
    fn reverse_chain_and_branch_use_descending_target_order() {
        for shape in ["reverse_chain", "branch"] {
            let fragments = benchmark_fragment_graph(512, 32, shape);
            let targets: Vec<_> = fragments
                .iter()
                .filter_map(|fragment| fragment.0.filter(|target| *target < 32))
                .collect();
            assert_eq!(targets, (0..32).rev().collect::<Vec<_>>());
            assert_eq!(targets.last(), Some(&0));
        }
    }
}

/// Measures the production selection phase over the actual linked inventory.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub fn benchmark_select_linked_inventory<T: crate::runtime::descriptor::Reflect>() -> (usize, usize)
{
    let mut fragments: Vec<_> = crate::__private::__inventory::iter::<RegistrationFragment>
        .into_iter()
        .copied()
        .collect();
    fragments.sort_by_key(|fragment| fragment.sort_key());
    let (selected, edges) = crate::runtime::context::handle::builder::select_fragment_slice(
        &fragments,
        std::any::TypeId::of::<T>(),
        |fragment| fragment.target_type_id(),
        |fragment| fragment.dependencies(),
        |fragment| fragment.include_in_linked_context(),
    );
    (
        selected.iter().filter(|include| **include).count(),
        edges.len(),
    )
}

/// Returns the canonical metadata associated with a generated definition.
///
/// # Type Parameters
/// - `T`: generated marker type for the definition.
///
/// # Parameters
/// - `registration`: static registration that owns the generated metadata.
pub fn type_definition_metadata<T: 'static>(
    registration: &'static TypeDefinitionRegistration<T>,
) -> &'static TypeDefinitionMetadata {
    registration.metadata()
}

/// Combines generic definition metadata with a capability targeting its marker.
///
/// This keeps the definition identity and capability target tied to the same
/// generated marker, so downstream code does not need to construct or expose a
/// `TypeDefinitionId`.
///
/// # Type Parameters
/// - `TMarker`: generated marker identifying the generic definition.
/// - `C`: capability contract implemented by the adapter.
///
/// # Parameters
/// - `definition`: static registration for the generic definition.
/// - `adapter`: implementation of the declared capability contract.
pub fn type_definition_with_capability<
    TMarker: 'static,
    C: crate::runtime::capability::CapabilitySpec,
>(
    definition: &'static TypeDefinitionRegistration<TMarker>,
    adapter: C::Adapter,
) -> Registration {
    Registration::metadata_only()
        .with_type_definition(definition)
        .with_capability(
            crate::runtime::capability::CapabilityRegistration::for_type_definition::<TMarker, C>(
                adapter,
            ),
        )
}

/// Creates a linked capability-only fragment for a generic definition.
///
/// # Parameters
/// - `label`: stable label for the linked registration fragment.
/// - `registration`: function that creates the capability registration.
#[doc(hidden)]
pub const fn linked_definition_capability_fragment(
    label: &'static str,
    registration: fn() -> Registration,
) -> RegistrationFragment {
    RegistrationFragment::for_linked_capability(label, registration)
}

/// Creates a global registration fragment for a compiler-checked trait object.
///
/// # Parameters
/// - `label`: stable label for the global registration fragment.
/// - `registration`: function that creates the trait object registration.
#[doc(hidden)]
pub const fn trait_object_fragment(
    label: &'static str,
    registration: fn() -> Registration,
) -> RegistrationFragment {
    RegistrationFragment::for_global_registration(label, registration)
}

/// A method binding whose target type is fixed by `T`.
///
/// # Type Parameters
/// - `T`: reflected target type for the bound method.
pub struct MethodBinding<T: crate::runtime::descriptor::Reflect> {
    /// Method metadata combined with its generated adapters and policies.
    effective: crate::runtime::descriptor::method::EffectiveMethod,
    /// Associates the binding with its target without storing a value.
    _target: std::marker::PhantomData<fn() -> T>,
}

impl<T: crate::runtime::descriptor::Reflect> MethodBinding<T> {
    /// Adds statically typed generic method specializations to this binding.
    ///
    /// # Parameters
    /// - `specializations`: specialization metadata to attach to the method.
    pub const fn with_specializations(
        mut self,
        specializations: &'static [crate::runtime::descriptor::method::MethodSpecializationSpec],
    ) -> Self {
        self.effective = self.effective.with_bound_specializations(specializations);
        self
    }

    /// Marks this binding unavailable with typed reasons.
    ///
    /// # Parameters
    /// - `reasons`: reasons that prevent invocation of the method.
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
    /// Converts the binding into a registration fragment for its target type.
    ///
    /// # Parameters
    /// - `binding`: statically typed method binding to register.
    fn from(binding: MethodBinding<T>) -> Self {
        crate::runtime::context::handle::registration::Registration::metadata_only()
            .with_method_binding::<T>(binding.effective)
    }
}

/// Binds method metadata and generated adapters to their concrete target type.
///
/// # Type Parameters
/// - `T`: reflected target type that owns the method.
///
/// # Parameters
/// - `metadata`: generated metadata describing the method.
/// - `adapters`: invocation adapters generated for the method.
pub const fn bind_method<T: crate::runtime::descriptor::Reflect>(
    metadata: &'static crate::runtime::descriptor::method::MethodMetadata,
    adapters: crate::runtime::descriptor::method::InvocationAdapters,
) -> MethodBinding<T> {
    MethodBinding {
        effective: crate::runtime::descriptor::method::EffectiveMethod::for_type::<T>(metadata)
            .with_adapters(adapters),
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
