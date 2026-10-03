// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! ImplDescriptor metadata and behavior.

use std::cmp::Ordering;
use std::fmt;

use super::impl_descriptor_builder::ImplDescriptorBuilder;
use crate::descriptor::AssociatedConstBindingDescriptor;
use crate::descriptor::AssociatedTypeBindingDescriptor;
use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplKind;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::MethodInstanceDescriptor;
use crate::descriptor::MethodLookup;
use crate::descriptor::MethodQualifier;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::GenericArgument;

/// One explicitly registered concrete instance of an impl definition.
///
/// # Examples
///
/// ```standalone_crate
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// # #[cfg(feature = "derive")]
/// use qubit_reflect::TypeDescriptor;
/// #[cfg(feature = "derive")]
/// mod example {
///     use qubit_reflect::{Reflect, reflect_impl};
///     #[derive(Reflect)]
///     #[reflect(crate = qubit_reflect)]
///     pub struct Service;
///     #[reflect_impl(crate = qubit_reflect)]
///     impl Service {
///         fn ping(&self) {}
///     }
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let implementation = TypeDescriptor::of::<example::Service>()
///     .impls_global()?
///     .first()
///     .expect("reflected implementation");
/// assert_eq!(implementation.target_type().query_name(), "Service");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
pub struct ImplDescriptor {
    /// Source declaration represented by this concrete application.
    pub(super) definition: &'static ImplDefinitionDescriptor,
    /// Resolver for the concrete target root.
    pub(super) target_type: TypeDescriptorResolver,
    /// Concrete applied trait namespace, if this is a trait impl.
    pub(super) implemented_trait: Option<&'static TraitDescriptor>,
    /// Methods declared by the impl definition.
    pub(super) methods: &'static [MethodDescriptor],
    /// Effective concrete method instances.
    pub(super) method_instances: Box<[MethodInstanceDescriptor]>,
    /// Concrete associated type bindings.
    pub(super) associated_types: Box<[AssociatedTypeBindingDescriptor]>,
    /// Concrete associated constant bindings.
    pub(super) associated_consts: Box<[AssociatedConstBindingDescriptor]>,
    /// Concrete generic arguments in definition order.
    pub(super) arguments: Box<[GenericArgument]>,
}

impl ImplDescriptor {
    /// Starts a concrete impl builder for `definition` and `target_type`.
    ///
    /// # Parameters
    ///
    /// - `definition`: Source impl declaration to instantiate.
    /// - `target_type`: Resolver for the concrete target root.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized for the requested declaration and target.
    #[must_use]
    pub fn builder(
        definition: &'static ImplDefinitionDescriptor,
        target_type: TypeDescriptorResolver,
    ) -> ImplDescriptorBuilder {
        ImplDescriptorBuilder::new(definition, target_type)
    }

    /// Returns whether two descriptors represent the same concrete impl
    /// application.
    ///
    /// # Parameters
    ///
    /// - `other`: Descriptor to compare with this application.
    ///
    /// # Returns
    ///
    /// Returns `true` when both descriptors identify the same impl application.
    #[must_use]
    pub(crate) fn same_application(&self, other: &Self) -> bool {
        self.kind() == other.kind()
            && self.definition().fragment_identity() == other.definition().fragment_identity()
            && self.arguments() == other.arguments()
            && self.target_type().type_id() == other.target_type().type_id()
    }

    /// Orders implementations by kind, namespace, and source identity.
    ///
    /// # Parameters
    ///
    /// - `other`: Descriptor to compare with this implementation.
    ///
    /// # Returns
    ///
    /// Returns the deterministic ordering between the implementations.
    #[must_use]
    pub(crate) fn registry_cmp(&self, other: &Self) -> Ordering {
        self.kind()
            .registry_rank()
            .cmp(&other.kind().registry_rank())
            .then_with(|| self.namespace_cmp(other))
            .then_with(|| {
                self.definition()
                    .fragment_identity()
                    .cmp(other.definition().fragment_identity())
            })
    }

    /// Orders implementation namespaces deterministically.
    ///
    /// # Parameters
    ///
    /// - `other`: Descriptor whose namespace is compared with this one.
    ///
    /// # Returns
    ///
    /// Returns the ordering between the implementation namespaces.
    #[must_use]
    fn namespace_cmp(&self, other: &Self) -> Ordering {
        match (self.implemented_trait(), other.implemented_trait()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(left), Some(right)) => match (left.definition().trait_id(), right.definition().trait_id()) {
                (TraitId::Reflected(_), TraitId::Reflected(_)) => left.rust_path().cmp(right.rust_path()),
                (TraitId::External(left), TraitId::External(right)) => left.cmp(right),
                (TraitId::Reflected(_), TraitId::External(_)) => Ordering::Less,
                (TraitId::External(_), TraitId::Reflected(_)) => Ordering::Greater,
            },
        }
    }

    /// Returns whether this implementation belongs to a lookup namespace.
    ///
    /// # Parameters
    ///
    /// - `qualifier`: Namespace restriction for the lookup.
    ///
    /// # Returns
    ///
    /// Returns `true` when this implementation matches the qualifier.
    #[must_use]
    pub(crate) fn matches_qualifier(&self, qualifier: MethodQualifier<'_>) -> bool {
        match qualifier {
            MethodQualifier::Any => true,
            MethodQualifier::Inherent => self.kind() == ImplKind::Inherent,
            MethodQualifier::Trait(expected) => self
                .implemented_trait()
                .is_some_and(|actual| actual.same_application(expected)),
        }
    }

    /// Returns the generic or blanket impl definition.
    ///
    /// # Returns
    ///
    /// Returns the source impl definition.
    #[must_use]
    #[inline]
    pub const fn definition(&self) -> &'static ImplDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected root targeted by this concrete impl.
    ///
    /// # Returns
    ///
    /// Returns the resolved target type descriptor.
    #[must_use]
    pub fn target_type(&self) -> &'static TypeDescriptor {
        (self.target_type)()
    }

    /// Returns whether this is an inherent or trait implementation.
    ///
    /// # Returns
    ///
    /// Returns the implementation kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ImplKind {
        self.definition.kind()
    }

    /// Returns the concrete applied trait, or `None` for an inherent impl.
    ///
    /// # Returns
    ///
    /// Returns the concrete trait application, or `None` for an inherent impl.
    #[must_use]
    #[inline]
    pub const fn implemented_trait(&self) -> Option<&'static TraitDescriptor> {
        self.implemented_trait
    }

    /// Returns methods explicitly declared by this impl definition.
    ///
    /// # Returns
    ///
    /// Returns the impl's declared methods.
    #[must_use]
    #[inline]
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Returns methods explicitly declared by this impl definition.
    ///
    /// # Returns
    ///
    /// Returns the impl's declared methods.
    #[must_use]
    #[inline]
    pub const fn implementation_methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method explicitly declared by this concrete impl, by query
    /// name.
    ///
    /// # Parameters
    ///
    /// - `name`: Query name to match.
    ///
    /// # Returns
    ///
    /// Returns the matching declared method, or `None` when absent.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns concrete effective instances, including defaulted methods.
    ///
    /// # Returns
    ///
    /// Returns effective method instances in their stored order.
    #[must_use]
    #[inline]
    pub const fn method_instances(&self) -> &[MethodInstanceDescriptor] {
        &self.method_instances
    }

    /// Returns associated type bindings in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the concrete associated type bindings.
    #[must_use = "the associated type bindings describe this impl application"]
    #[inline]
    pub const fn associated_types(&self) -> &[AssociatedTypeBindingDescriptor] {
        &self.associated_types
    }

    /// Returns associated constant bindings in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the concrete associated constant bindings.
    #[must_use = "the associated constant bindings describe this impl application"]
    #[inline]
    pub const fn associated_consts(&self) -> &[AssociatedConstBindingDescriptor] {
        &self.associated_consts
    }

    /// Returns concrete impl arguments in definition parameter order.
    ///
    /// # Returns
    ///
    /// Returns this application's generic arguments.
    #[must_use]
    #[inline]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Looks up one effective method across impl namespaces.
    ///
    /// # Parameters
    ///
    /// - `implementations`: Concrete implementations searched in the given
    ///   order.
    /// - `qualifier`: Namespace restriction applied to each implementation.
    /// - `name`: Method query name to match.
    ///
    /// # Returns
    ///
    /// Returns `Missing`, `Unique`, or `Ambiguous` according to the matches
    /// found.
    #[must_use]
    pub fn lookup_method<'a>(
        implementations: &'a [&'a ImplDescriptor],
        qualifier: MethodQualifier<'_>,
        name: &str,
    ) -> MethodLookup<'a> {
        let mut found = None;
        for implementation in implementations {
            if !implementation.matches_qualifier(qualifier) {
                continue;
            }
            for instance in implementation.method_instances() {
                if instance.declaration().query_name() != name {
                    continue;
                }
                if found.is_some() {
                    return MethodLookup::Ambiguous;
                }
                found = Some(instance);
            }
        }
        found.map_or(MethodLookup::Missing, MethodLookup::Unique)
    }
}

impl fmt::Debug for ImplDescriptor {
    /// Formats local facts without recursively expanding graph roots.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the local, address-free
    ///   representation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImplDescriptor")
            .field("fragment", self.definition.fragment_identity())
            .field("kind", &self.kind())
            .field("target_type", &"<resolver>")
            .field("method_instance_count", &self.method_instances.len())
            .field("arguments", &self.arguments)
            .finish()
    }
}
