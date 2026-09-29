// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Trait definitions, concrete applications, supertraits, and associated items.

use std::fmt;

use crate::descriptor::MethodDescriptor;
use crate::expression::GenericArgument;

// Owns declaration metadata and associated items.
mod definition;
// Owns concrete applications and substitutions.
mod application;
// Owns validated construction.
mod trait_descriptor_build_error;
mod trait_descriptor_builder;
// Owns supertrait closure and external caching.
mod supertrait;

pub use self::application::AppliedTraitId;
pub(crate) use self::application::TraitApplicationSubstitutions;
pub use self::application::TraitId;
pub use self::application::TraitImplPayload;
pub use self::definition::AssociatedConstDescriptor;
pub use self::definition::AssociatedTypeDescriptor;
pub use self::definition::TraitCompleteness;
pub use self::definition::TraitDefinitionDescriptor;
pub use self::supertrait::SupertraitClosure;
pub use self::supertrait::TraitDescriptorRef;
pub use self::supertrait::cached_trait_object_descriptor;
pub use self::supertrait::external_supertrait;
pub use self::trait_descriptor_build_error::TraitDescriptorBuildError;
pub use self::trait_descriptor_builder::TraitDescriptorBuilder;
pub(super) use self::trait_descriptor_builder::generic_argument_is_concrete;

/// An applied trait descriptor with concrete generic arguments.
///
/// # Examples
///
/// ```standalone_crate
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// # #[cfg(feature = "derive")]
/// use qubit_reflect::TypeDescriptor;
/// #[cfg(feature = "derive")]
/// mod example {
///     use qubit_reflect::{reflect, reflect_impl, Reflect};
///     #[derive(Reflect)]
///     #[reflect(crate = qubit_reflect)]
///     pub struct Service;
///     #[reflect(crate = qubit_reflect)]
///     pub trait Named {
///         fn name(&self) -> &'static str;
///     }
///     #[reflect_impl(crate = qubit_reflect)]
///     impl Named for Service {
///         fn name(&self) -> &'static str { "service" }
///     }
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let applied = TypeDescriptor::of::<example::Service>()
///     .impls()?
///     .iter()
///     .find_map(|implementation| implementation.implemented_trait())
///     .expect("reflected trait implementation");
/// assert_eq!(applied.definition().rust_name(), "Named");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
pub struct TraitDescriptor {
    /// Declaration shared by every concrete application.
    definition: &'static TraitDefinitionDescriptor,
    /// Complete identity of this concrete trait application.
    trait_id: AppliedTraitId,
    /// Concrete generic arguments in declaration order.
    arguments: Box<[GenericArgument]>,
    /// Concrete associated-type equalities for this application.
    associated_type_arguments: Box<[GenericArgument]>,
    /// Direct applied supertraits in declaration order.
    direct_supertraits: Box<[TraitDescriptorRef]>,
    /// Sorted, duplicate-free transitive supertrait closure.
    all_supertraits: Box<[TraitDescriptorRef]>,
    /// Applied method declarations in source order.
    methods: &'static [MethodDescriptor],
    /// Applied associated-type declarations in source order.
    associated_types: Box<[AssociatedTypeDescriptor]>,
    /// Applied associated-constant declarations in source order.
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

impl TraitDescriptor {
    /// Starts an applied trait builder for `definition`.
    ///
    /// # Parameters
    ///
    /// - `definition`: Declaration whose concrete application is being built.
    ///
    /// # Returns
    ///
    /// Returns an empty builder for that trait declaration.
    #[must_use]
    #[inline]
    pub fn builder(definition: &'static TraitDefinitionDescriptor) -> TraitDescriptorBuilder {
        TraitDescriptorBuilder::new(definition)
    }

    /// Returns the declaration-level descriptor shared by every application.
    ///
    /// # Returns
    ///
    /// Returns the source declaration descriptor.
    #[must_use]
    #[inline]
    pub const fn definition(&self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected marker or external identity.
    ///
    /// # Returns
    ///
    /// Returns the concrete applied trait identity.
    #[must_use]
    #[inline]
    pub const fn trait_id(&self) -> &AppliedTraitId {
        &self.trait_id
    }

    /// Returns concrete generic arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the ordered concrete type and const arguments.
    #[must_use]
    #[inline]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns concrete associated-type equalities required by this applied
    /// trait object or application.
    ///
    /// # Returns
    ///
    /// Returns the concrete associated-type equalities.
    #[must_use]
    #[inline]
    pub const fn associated_type_arguments(&self) -> &[GenericArgument] {
        &self.associated_type_arguments
    }

    /// Returns the Rust declaration name.
    ///
    /// # Returns
    ///
    /// Returns the source trait name.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.definition.rust_name()
    }

    /// Returns the diagnostic fully qualified Rust path.
    ///
    /// # Returns
    ///
    /// Returns the source trait path.
    #[must_use]
    #[inline]
    pub const fn rust_path(&self) -> &'static str {
        self.definition.rust_path()
    }

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.definition.query_name()
    }

    /// Returns whether this descriptor contains a complete declaration.
    ///
    /// # Returns
    ///
    /// Returns the completeness classification.
    #[must_use]
    #[inline]
    pub const fn completeness(&self) -> TraitCompleteness {
        self.definition.completeness()
    }

    /// Returns direct supertraits in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns the direct applied supertrait references.
    #[must_use]
    #[inline]
    pub const fn direct_supertraits(&self) -> &[TraitDescriptorRef] {
        &self.direct_supertraits
    }

    /// Returns the sorted, duplicate-free, transitive supertrait closure.
    ///
    /// # Returns
    ///
    /// Returns a deterministic view of every transitive supertrait.
    #[must_use]
    #[inline]
    pub const fn all_supertraits(&self) -> SupertraitClosure<'_> {
        SupertraitClosure {
            descriptors: &self.all_supertraits,
        }
    }

    /// Returns method declarations in source order.
    ///
    /// # Returns
    ///
    /// Returns the applied method declarations.
    #[must_use]
    #[inline]
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method by query name.
    ///
    /// `None` means this applied trait has no method with the requested name.
    ///
    /// # Parameters
    ///
    /// - `name`: Method query name to match.
    ///
    /// # Returns
    ///
    /// Returns the method declaration, or `None` when no method matches.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns associated type declarations in source order.
    ///
    /// # Returns
    ///
    /// Returns the applied associated type declarations.
    #[must_use]
    #[inline]
    pub const fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        &self.associated_types
    }

    /// Finds an associated type by query name.
    ///
    /// `None` means no associated type has the requested name.
    ///
    /// # Parameters
    ///
    /// - `name`: Associated-type query name to match.
    ///
    /// # Returns
    ///
    /// Returns the associated type, or `None` when no item matches.
    #[must_use]
    pub fn associated_type(&self, name: &str) -> Option<&AssociatedTypeDescriptor> {
        self.associated_types.iter().find(|item| item.query_name() == name)
    }

    /// Returns associated constant declarations in source order.
    ///
    /// # Returns
    ///
    /// Returns the applied associated constant declarations.
    #[must_use]
    #[inline]
    pub const fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        &self.associated_consts
    }

    /// Finds an associated constant by query name.
    ///
    /// `None` means no associated constant has the requested name.
    ///
    /// # Parameters
    ///
    /// - `name`: Associated-constant query name to match.
    ///
    /// # Returns
    ///
    /// Returns the associated constant, or `None` when no item matches.
    #[must_use]
    pub fn associated_const(&self, name: &str) -> Option<&AssociatedConstDescriptor> {
        self.associated_consts.iter().find(|item| item.query_name() == name)
    }

    /// Returns whether two descriptors are the same concrete trait application.
    ///
    /// # Parameters
    ///
    /// - `other`: Applied trait to compare with this descriptor.
    ///
    /// # Returns
    ///
    /// Returns `true` when both descriptors have the same complete identity.
    #[must_use]
    pub fn same_application(&self, other: &Self) -> bool {
        self.trait_id == other.trait_id
    }
}

impl fmt::Debug for TraitDescriptor {
    /// Formats local facts without recursively expanding supertraits.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the local trait application facts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the representation.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraitDescriptor")
            .field("definition", &self.definition)
            .field("arguments", &self.arguments)
            .field("direct_supertrait_count", &self.direct_supertraits.len())
            .field("all_supertrait_count", &self.all_supertraits.len())
            .field("method_count", &self.methods.len())
            .field("associated_type_count", &self.associated_types.len())
            .field("associated_const_count", &self.associated_consts.len())
            .finish()
    }
}
