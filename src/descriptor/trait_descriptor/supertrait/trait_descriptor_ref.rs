// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Static references to applied trait descriptors.

use std::ops::Deref;

use super::super::TraitDescriptor;

/// A static reference used by direct and transitive supertrait views.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{TraitCompleteness, TraitDefinitionDescriptor, TraitDescriptor, TraitDescriptorRef, TraitId};
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
///
/// struct Marker;
/// static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///     LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
/// static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///     TraitDefinitionDescriptor::new(
///         TraitId::Reflected(TypeId::of::<Marker>()), "Example", "example::Example", "Example",
///         TraitCompleteness::Complete, &GENERICS,
///     )
/// });
/// static APPLIED: LazyLock<TraitDescriptor> = LazyLock::new(|| {
///     TraitDescriptor::builder(&DEFINITION).build().expect("valid application")
/// });
/// let reference = TraitDescriptorRef::new(&APPLIED);
/// assert_eq!(reference.rust_name(), "Example");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TraitDescriptorRef(
    /// Process-lifetime applied trait descriptor retained by this reference.
    &'static TraitDescriptor,
);

impl TraitDescriptorRef {
    /// Creates a supertrait reference.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Applied trait descriptor to retain.
    ///
    /// # Returns
    ///
    /// Returns a lightweight static reference to the descriptor.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(descriptor: &'static TraitDescriptor) -> Self {
        Self(descriptor)
    }

    /// Returns the referenced applied trait descriptor.
    ///
    /// # Returns
    ///
    /// Returns the process-lifetime applied trait descriptor.
    #[must_use]
    #[inline]
    pub const fn descriptor(self) -> &'static TraitDescriptor {
        let Self(descriptor) = self;
        descriptor
    }
}

impl Deref for TraitDescriptorRef {
    type Target = TraitDescriptor;

    /// Dereferences to the applied trait descriptor.
    ///
    /// # Returns
    ///
    /// Returns a shared reference to the retained descriptor.
    fn deref(&self) -> &Self::Target {
        let Self(descriptor) = self;
        descriptor
    }
}
