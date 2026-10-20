// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::metadata::TypeMetadata;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::construct::ConstructionTargetKind;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::descriptor::variant::VariantMetadata;

/// Supplies the static registration used to reflect a Rust type.
pub trait Reflect: 'static {
    /// Returns the shared registration for this concrete type.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(TypeRegistration::new)
    }
}

/// Static type metadata retained by the reflection runtime.
pub struct TypeRegistration<T: ?Sized + 'static> {
    metadata: TypeMetadata,
    shape: Option<&'static TypeKindMetadata>,
    type_definition: Option<&'static TypeDefinitionMetadata>,
    construction_plans: &'static [ConstructionPlan],
    _type: PhantomData<fn() -> T>,
}

impl<T: ?Sized + 'static> TypeRegistration<T> {
    /// Creates a registration containing only the concrete type identity.
    pub const fn new() -> Self {
        Self {
            metadata: TypeMetadata::new::<T>(),
            shape: None,
            type_definition: None,
            construction_plans: &[],
            _type: PhantomData,
        }
    }

    /// Creates a registration with static source shape and construction plans.
    pub fn with_shape(
        shape: &'static TypeKindMetadata,
        type_definition: Option<&'static TypeDefinitionMetadata>,
        construction_plans: &'static [ConstructionPlan],
    ) -> Self {
        let construction_shape = construction_plans
            .iter()
            .find(|plan| plan.target.kind == ConstructionTargetKind::Struct)
            .map(|plan| plan.shape);
        Self {
            metadata: TypeMetadata::with_shape::<T>(shape, construction_shape, type_definition),
            shape: Some(shape),
            type_definition,
            construction_plans,
            _type: PhantomData,
        }
    }

    /// Returns the stable registration for this concrete type, initializing it
    /// once.
    pub fn get_or_init_with(initialize: impl FnOnce() -> Self) -> &'static Self {
        let type_id = TypeId::of::<T>();
        let cell = {
            let mut registrations = registration_cache()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *registrations
                .entry(type_id)
                .or_insert_with(|| Box::leak(Box::new(OnceLock::new())))
        };
        cell.get_or_init(|| Box::new(initialize()) as Box<dyn Any + Send + Sync>)
            .downcast_ref::<Self>()
            .expect("registration cache is keyed by exact TypeId")
    }

    pub(crate) fn metadata(&'static self) -> &'static TypeMetadata {
        &self.metadata
    }

    /// Adds concrete arguments retained for this compiled type application.
    #[doc(hidden)]
    pub fn with_generic_arguments(
        mut self,
        arguments: &'static [crate::runtime::descriptor::generic::GenericArgumentMetadata],
    ) -> Self {
        self.metadata = self.metadata.with_generic_arguments(arguments);
        self
    }

    pub(crate) fn shape(&'static self) -> Option<&'static TypeKindMetadata> {
        self.shape
    }

    pub(crate) fn type_definition(&'static self) -> Option<&'static TypeDefinitionMetadata> {
        self.type_definition
    }

    pub(crate) fn construction_plans(&'static self) -> &'static [ConstructionPlan] {
        self.construction_plans
    }

    /// Returns a source variant by index when this registration has enum shape.
    pub fn variant(&'static self, index: usize) -> Option<&'static VariantMetadata> {
        match self.shape? {
            TypeKindMetadata::Enum(variants) => variants.get(index),
            _ => None,
        }
    }
}

impl<T: ?Sized + 'static> Default for TypeRegistration<T> {
    fn default() -> Self {
        Self::new()
    }
}

type RegistrationCache = HashMap<TypeId, &'static OnceLock<Box<dyn Any + Send + Sync>>>;

fn registration_cache() -> &'static Mutex<RegistrationCache> {
    static REGISTRATIONS: OnceLock<Mutex<RegistrationCache>> = OnceLock::new();
    REGISTRATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
