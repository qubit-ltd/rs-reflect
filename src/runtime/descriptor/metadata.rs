// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::generic::GenericArgumentMetadata;
use super::generic::TypeDefinitionMetadata;
use crate::runtime::construct::ConstructionShape;

pub(crate) struct TypeMetadata {
    type_id: fn() -> TypeId,
    rust_name: fn() -> &'static str,
    shape: Option<&'static super::type_kind::TypeKindMetadata>,
    construction_shape: Option<ConstructionShape>,
    type_definition: Option<&'static TypeDefinitionMetadata>,
    generic_arguments: &'static [GenericArgumentMetadata],
}

impl TypeMetadata {
    pub(crate) const fn new<T: ?Sized + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            shape: None,
            construction_shape: None,
            type_definition: None,
            generic_arguments: &[],
        }
    }

    pub(crate) const fn with_shape<T: ?Sized + 'static>(
        shape: &'static super::type_kind::TypeKindMetadata,
        construction_shape: Option<ConstructionShape>,
        type_definition: Option<&'static TypeDefinitionMetadata>,
    ) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            shape: Some(shape),
            construction_shape,
            type_definition,
            generic_arguments: &[],
        }
    }

    pub(crate) const fn with_generic_arguments(
        mut self,
        generic_arguments: &'static [GenericArgumentMetadata],
    ) -> Self {
        self.generic_arguments = generic_arguments;
        self
    }

    pub(crate) const fn shape(&self) -> Option<&'static super::type_kind::TypeKindMetadata> {
        self.shape
    }

    pub(crate) const fn construction_shape(&self) -> Option<ConstructionShape> {
        self.construction_shape
    }

    pub(crate) const fn type_definition(&self) -> Option<&'static TypeDefinitionMetadata> {
        self.type_definition
    }

    pub(crate) const fn generic_arguments(&self) -> &'static [GenericArgumentMetadata] {
        self.generic_arguments
    }

    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn rust_name(&self) -> &'static str {
        (self.rust_name)()
    }

    pub(crate) fn of<T: ?Sized + 'static>() -> &'static Self {
        let type_id = TypeId::of::<T>();
        let metadata = metadata_cache();
        let mut metadata = metadata.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        metadata
            .entry(type_id)
            .or_insert_with(|| Box::leak(Box::new(Self::new::<T>())))
    }
}

fn metadata_cache() -> &'static Mutex<HashMap<TypeId, &'static TypeMetadata>> {
    static METADATA: OnceLock<Mutex<HashMap<TypeId, &'static TypeMetadata>>> = OnceLock::new();
    METADATA.get_or_init(|| Mutex::new(HashMap::new()))
}
