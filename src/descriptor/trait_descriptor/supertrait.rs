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
pub struct TraitDescriptorRef(&'static TraitDescriptor);

impl TraitDescriptorRef {
    /// Creates a supertrait reference.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(descriptor: &'static TraitDescriptor) -> Self {
        Self(descriptor)
    }

    /// Returns the referenced applied trait descriptor.
    #[must_use]
    #[inline(always)]
    pub const fn descriptor(self) -> &'static TraitDescriptor {
        let Self(descriptor) = self;
        descriptor
    }
}

impl Deref for TraitDescriptorRef {
    type Target = TraitDescriptor;

    /// Dereferences to the applied trait descriptor.
    fn deref(&self) -> &Self::Target {
        let Self(descriptor) = self;
        descriptor
    }
}

/// A deterministic, duplicate-free transitive supertrait view.
#[derive(Clone, Copy, Debug)]
pub struct SupertraitClosure<'a> {
    pub(super) descriptors: &'a [TraitDescriptorRef],
}

impl<'a> SupertraitClosure<'a> {
    /// Returns applied supertraits in deterministic path order.
    #[must_use]
    #[inline(always)]
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'a TraitDescriptor> {
        self.descriptors.iter().map(|descriptor| descriptor.descriptor())
    }

    /// Returns the number of distinct transitive supertraits.
    #[must_use]
    #[inline(always)]
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether the closure is empty.
    #[must_use]
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }
}
