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
mod builder;
// Owns supertrait closure and external caching.
mod supertrait;

pub use self::application::AppliedTraitId;
pub(crate) use self::application::TraitApplicationSubstitutions;
pub use self::application::TraitId;
pub use self::application::TraitImplPayload;
pub use self::builder::TraitDescriptorBuildError;
pub use self::builder::TraitDescriptorBuilder;
pub(super) use self::builder::generic_argument_is_concrete;
pub use self::definition::AssociatedConstDescriptor;
pub use self::definition::AssociatedTypeDescriptor;
pub use self::definition::TraitCompleteness;
pub use self::definition::TraitDefinitionDescriptor;
pub use self::supertrait::SupertraitClosure;
pub use self::supertrait::TraitDescriptorRef;
pub use self::supertrait::cached_trait_object_descriptor;
pub use self::supertrait::external_supertrait;

/// An applied trait descriptor with concrete generic arguments.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// use qubit_reflect::{Reflect, TypeDescriptor};
/// #[cfg(feature = "derive")]
/// use qubit_reflect::{reflect, reflect_impl};
///
/// #[cfg(feature = "derive")]
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
///
/// #[cfg(feature = "derive")]
/// #[reflect(crate = qubit_reflect)]
/// trait Named {
///     fn name(&self) -> &'static str;
/// }
///
/// #[cfg(feature = "derive")]
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Named for Service {
///     fn name(&self) -> &'static str { "service" }
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let applied = TypeDescriptor::of::<Service>()
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
    definition: &'static TraitDefinitionDescriptor,
    trait_id: AppliedTraitId,
    arguments: Box<[GenericArgument]>,
    associated_type_arguments: Box<[GenericArgument]>,
    direct_supertraits: Box<[TraitDescriptorRef]>,
    all_supertraits: Box<[TraitDescriptorRef]>,
    methods: &'static [MethodDescriptor],
    associated_types: Box<[AssociatedTypeDescriptor]>,
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

impl TraitDescriptor {
    /// Starts an applied trait builder for `definition`.
    pub fn builder(definition: &'static TraitDefinitionDescriptor) -> TraitDescriptorBuilder {
        TraitDescriptorBuilder::new(definition)
    }

    /// Returns the declaration-level descriptor shared by every application.
    #[must_use]
    #[inline(always)]
    pub const fn definition(&self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected marker or external identity.
    #[must_use]
    #[inline(always)]
    pub const fn trait_id(&self) -> &AppliedTraitId {
        &self.trait_id
    }

    /// Returns concrete generic arguments in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns concrete associated-type equalities required by this applied
    /// trait object or application.
    #[must_use]
    #[inline(always)]
    pub const fn associated_type_arguments(&self) -> &[GenericArgument] {
        &self.associated_type_arguments
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.definition.rust_name()
    }

    /// Returns the diagnostic fully qualified Rust path.
    #[must_use]
    #[inline(always)]
    pub const fn rust_path(&self) -> &'static str {
        self.definition.rust_path()
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.definition.query_name()
    }

    /// Returns whether this descriptor contains a complete declaration.
    #[must_use]
    #[inline(always)]
    pub const fn completeness(&self) -> TraitCompleteness {
        self.definition.completeness()
    }

    /// Returns direct supertraits in source declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn direct_supertraits(&self) -> &[TraitDescriptorRef] {
        &self.direct_supertraits
    }

    /// Returns the sorted, duplicate-free, transitive supertrait closure.
    #[must_use]
    #[inline(always)]
    pub const fn all_supertraits(&self) -> SupertraitClosure<'_> {
        SupertraitClosure {
            descriptors: &self.all_supertraits,
        }
    }

    /// Returns method declarations in source order.
    #[must_use]
    #[inline(always)]
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method by query name.
    ///
    /// `None` means this applied trait has no method with the requested name.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns associated type declarations in source order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        &self.associated_types
    }

    /// Finds an associated type by query name.
    ///
    /// `None` means no associated type has the requested name.
    #[must_use]
    pub fn associated_type(&self, name: &str) -> Option<&AssociatedTypeDescriptor> {
        self.associated_types.iter().find(|item| item.query_name() == name)
    }

    /// Returns associated constant declarations in source order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        &self.associated_consts
    }

    /// Finds an associated constant by query name.
    ///
    /// `None` means no associated constant has the requested name.
    #[must_use]
    pub fn associated_const(&self, name: &str) -> Option<&AssociatedConstDescriptor> {
        self.associated_consts.iter().find(|item| item.query_name() == name)
    }

    /// Returns whether two descriptors are the same concrete trait application.
    #[must_use]
    pub fn same_application(&self, other: &Self) -> bool {
        self.trait_id == other.trait_id
    }
}

impl fmt::Debug for TraitDescriptor {
    /// Formats local facts without recursively expanding supertraits.
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
