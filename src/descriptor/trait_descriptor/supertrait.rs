// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Supertrait closure views and process-local descriptor caches.

use std::any::TypeId;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::TraitCompleteness;
use super::TraitDefinitionDescriptor;
use super::TraitDescriptor;
use super::TraitId;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::ExternalTraitId;

type DynTraitCache = HashMap<TypeId, &'static OnceLock<TraitDescriptor>>;

/// Returns a cached incomplete descriptor for an explicitly mapped external
/// supertrait.
///
/// # Type Parameters
///
/// - `T`: Concrete dyn trait-object root linked to the external supertrait.
///
/// # Parameters
///
/// - `id`: Stable external trait identifier.
/// - `rust_path`: Fully qualified source path retained for diagnostics.
/// - `arguments`: Concrete generic arguments for the application.
///
/// # Returns
///
/// Returns the process-lifetime applied trait descriptor.
///
/// # Panics
///
/// Panics if the supplied external ID is invalid or generated descriptor facts
/// violate the incomplete-trait contract.
#[doc(hidden)]
#[must_use]
pub fn external_supertrait<T: ?Sized + 'static>(
    id: &'static str,
    rust_path: &'static str,
    arguments: Vec<GenericArgument>,
) -> &'static TraitDescriptor {
    let external_id =
        ExternalTraitId::new(id).expect("the macro validator must only emit valid external trait identifiers");
    let key = (
        TypeId::of::<T>(),
        external_id.clone(),
        arguments.clone().into_boxed_slice(),
    );
    let cell = crate::descriptor::internal::trait_cache::external_supertrait_cell(key);
    cell.get_or_init(|| {
        let definition = Box::leak(Box::new(TraitDefinitionDescriptor::new(
            TraitId::External(external_id),
            rust_path.rsplit("::").next().unwrap_or(rust_path).trim(),
            rust_path.trim(),
            rust_path.trim(),
            TraitCompleteness::ExternalIncomplete,
            Box::leak(Box::new(GenericDefinitionDescriptor {
                parameters: Box::new([]),
                predicates: Box::new([]),
                diagnostic: crate::expression::DiagnosticText::default(),
            })),
        )));
        Box::leak(Box::new(
            TraitDescriptor::builder(definition)
                .arguments(arguments)
                .build()
                .expect("an external supertrait descriptor must be valid"),
        ))
    })
}

/// Returns the unique applied trait descriptor linked from one concrete dyn
/// trait-object root.
///
/// # Type Parameters
///
/// - `T`: Concrete trait-object type used as the cache key.
///
/// # Parameters
///
/// - `build`: One-time factory for the applied trait descriptor.
///
/// # Returns
///
/// Returns the unique process-lifetime applied trait descriptor for `T`.
///
/// # Panics
///
/// Propagates a panic from `build`; the cache cell remains uninitialized.
#[doc(hidden)]
pub fn cached_trait_object_descriptor<T: ?Sized + 'static>(
    build: impl FnOnce() -> TraitDescriptor,
) -> &'static TraitDescriptor {
    static CACHE: LazyLock<Mutex<DynTraitCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let cell = *cache
        .entry(TypeId::of::<T>())
        .or_insert_with(|| Box::leak(Box::new(OnceLock::new())));
    drop(cache);
    cell.get_or_init(build)
}

/// A static reference used by direct and transitive supertrait views.
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

/// A deterministic, duplicate-free transitive supertrait view.
#[derive(Clone, Copy, Debug)]
pub struct SupertraitClosure<'a> {
    /// Sorted, duplicate-free applied supertraits.
    pub(super) descriptors: &'a [TraitDescriptorRef],
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
