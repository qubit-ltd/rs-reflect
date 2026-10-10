// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Deterministic views of transitive supertraits.

use super::TraitDescriptorRef;
use crate::descriptor::TraitDescriptor;

/// A deterministic, duplicate-free transitive supertrait view.
///
/// # Type Parameters
///
/// - `'a`: The lifetime of the borrowed applied supertrait descriptors.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::{TraitCompleteness, TraitDefinitionDescriptor, TraitDescriptor, TraitId};
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
/// let applied = TraitDescriptor::builder(&DEFINITION).build().expect("valid application");
/// let closure = applied.all_supertraits();
/// assert!(closure.is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SupertraitClosure<'a> {
    /// Sorted, duplicate-free applied supertraits.
    pub(in crate::descriptor::trait_descriptor) descriptors: &'a [TraitDescriptorRef],
}

impl<'a> SupertraitClosure<'a> {
    /// Returns applied supertraits in deterministic path order.
    ///
    /// # Returns
    ///
    /// Returns an exact-size iterator over the transitive closure.
    #[must_use]
    #[inline]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'a TraitDescriptor> {
        self.descriptors.iter().map(|descriptor| descriptor.descriptor())
    }

    /// Returns the number of distinct transitive supertraits.
    ///
    /// # Returns
    ///
    /// Returns the number of applied supertraits in the closure.
    #[must_use]
    #[inline]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether the closure is empty.
    ///
    /// # Returns
    ///
    /// Returns `true` when no supertraits are present.
    #[must_use]
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }
}
