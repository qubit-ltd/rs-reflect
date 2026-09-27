// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Reflection descriptors for common dyn-compatible trait objects.

use std::any::type_name;
use std::fmt::Debug;
use std::sync::LazyLock;

use crate::builtin::interner;
use crate::descriptor::Reflect;
use crate::descriptor::TraitCompleteness;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::expression::GenericDefinitionDescriptor;
use crate::identity::ExternalTraitId;

/// Returns the process-lifetime declaration link for the built-in `dyn Debug`
/// descriptor.
///
/// # Returns
///
/// The shared descriptor for the external `Debug` trait declaration.
///
/// # Panics
///
/// Panics if the built-in trait ID or its descriptor violates the crate's
/// fixed initialization invariants.
#[must_use]
fn debug_trait_descriptor() -> &'static TraitDescriptor {
    static GENERICS: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
        parameters: Box::new([]),
        predicates: Box::new([]),
        diagnostic: crate::expression::DiagnosticText::default(),
    });
    static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
        TraitDefinitionDescriptor::new(
            TraitId::External(ExternalTraitId::new("core.fmt.Debug").expect("the built-in Debug trait ID is valid")),
            "Debug",
            "std::fmt::Debug",
            "Debug",
            TraitCompleteness::ExternalIncomplete,
            &GENERICS,
        )
    });
    static APPLIED: LazyLock<TraitDescriptor> = LazyLock::new(|| {
        TraitDescriptor::builder(&DEFINITION)
            .build()
            .expect("the built-in Debug trait descriptor is valid")
    });
    &APPLIED
}

impl Reflect for dyn Debug {
    /// Returns the interned descriptor for `dyn Debug`.
    ///
    /// # Returns
    ///
    /// The shared descriptor linked to the external `Debug` trait metadata.
    fn type_descriptor() -> &'static TypeDescriptor {
        interner::intern::<Self>(|| {
            TypeDescriptor::new_trait_object::<Self>(type_name::<Self>(), debug_trait_descriptor)
        })
    }
}
