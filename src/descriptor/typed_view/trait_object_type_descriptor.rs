// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::descriptor::TraitDescriptor;

/// The typed view of a dyn-compatible trait object.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let object = TypeDescriptor::of::<dyn std::fmt::Debug>()
///     .as_trait_object()
///     .expect("dyn Debug type");
/// assert!(object.trait_descriptor().rust_path().ends_with("Debug"));
/// ```
#[derive(Clone, Copy)]
pub struct TraitObjectTypeDescriptor {
    /// Resolver for the applied trait descriptor represented by the object.
    trait_descriptor: fn() -> &'static TraitDescriptor,
}

impl TraitObjectTypeDescriptor {
    /// Creates a trait-object view backed by a lazy applied-trait resolver.
    ///
    /// # Parameters
    ///
    /// - `trait_descriptor`: Resolver for the applied trait descriptor.
    ///
    /// # Returns
    ///
    /// Returns a trait-object view backed by that resolver.
    pub(crate) const fn new(trait_descriptor: fn() -> &'static TraitDescriptor) -> Self {
        Self { trait_descriptor }
    }

    /// Returns the applied trait declaration represented by this object type.
    ///
    /// # Returns
    ///
    /// Returns the process-lifetime applied trait descriptor, initializing it
    /// on first access.
    #[must_use]
    pub fn trait_descriptor(&self) -> &'static TraitDescriptor {
        (self.trait_descriptor)()
    }
}

impl std::fmt::Debug for TraitObjectTypeDescriptor {
    /// Formats the linked trait identity without expanding its full graph.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the trait object's local identity.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the identity, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by `formatter`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TraitObjectTypeDescriptor")
            .field("trait", &self.trait_descriptor().rust_path())
            .finish()
    }
}
