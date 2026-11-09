// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::collections::HashMap;

use super::registration::TypeRecord;
use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionId;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
use crate::runtime::short_name::index_short_name;

/// Lookup maps built from the descriptors registered in a context.
#[derive(Default)]
pub(crate) struct ContextIndexes {
    /// Maps concrete Rust type IDs to descriptor positions.
    pub(crate) types_by_id: HashMap<TypeId, usize>,
    /// Maps fully qualified Rust names to matching descriptor positions.
    pub(crate) types_by_name: HashMap<Box<str>, Box<[usize]>>,
    /// Maps short Rust names to matching descriptor positions.
    pub(crate) types_by_short_name: HashMap<Box<str>, Box<[usize]>>,
    /// Maps source type paths to their definition IDs.
    pub(crate) type_definitions_by_path: HashMap<Box<str>, Box<[TypeDefinitionId]>>,
    /// Maps short source type names to their definition IDs.
    pub(crate) type_definitions_by_short_name: HashMap<Box<str>, Box<[TypeDefinitionId]>>,
    /// Maps source trait paths to their definition IDs.
    pub(crate) trait_definitions_by_path: HashMap<Box<str>, Box<[TraitDefinitionId]>>,
    /// Maps short source trait names to their definition IDs.
    pub(crate) trait_definitions_by_short_name: HashMap<Box<str>, Box<[TraitDefinitionId]>>,
}

impl ContextIndexes {
    /// Builds context lookup maps from concrete and source definition metadata.
    ///
    /// # Parameters
    /// - `types`: concrete type records in descriptor order.
    /// - `type_definitions`: source type definitions to index.
    /// - `trait_definitions`: source trait definitions to index.
    pub(crate) fn build(
        types: &[TypeRecord],
        type_definitions: impl IntoIterator<Item = &'static TypeDefinitionMetadata>,
        trait_definitions: impl IntoIterator<Item = &'static TraitDefinitionMetadata>,
    ) -> Self {
        let mut types_by_id = HashMap::with_capacity(types.len());
        let mut types_by_name: HashMap<Box<str>, Vec<usize>> = HashMap::new();
        let mut types_by_short_name: HashMap<Box<str>, Vec<usize>> = HashMap::new();

        for (index, metadata) in types.iter().enumerate() {
            types_by_id.insert(metadata.type_id(), index);
            let name = metadata.rust_name();
            types_by_name.entry(name.into()).or_default().push(index);
            types_by_short_name
                .entry(index_short_name(name).into())
                .or_default()
                .push(index);
        }

        let mut type_definitions = type_definitions.into_iter().collect::<Vec<_>>();
        type_definitions.sort_by(|left, right| left.path.cmp(right.path));
        let mut type_definitions_by_path = HashMap::<Box<str>, Vec<TypeDefinitionId>>::new();
        let mut type_definitions_by_short_name = HashMap::<Box<str>, Vec<TypeDefinitionId>>::new();
        for definition in type_definitions {
            type_definitions_by_path
                .entry(definition.path.into())
                .or_default()
                .push(definition.id);
            type_definitions_by_short_name
                .entry(index_short_name(definition.path).into())
                .or_default()
                .push(definition.id);
        }

        let mut trait_definitions = trait_definitions.into_iter().collect::<Vec<_>>();
        trait_definitions.sort_by(|left, right| left.path.cmp(right.path));
        let mut trait_definitions_by_path = HashMap::<Box<str>, Vec<TraitDefinitionId>>::new();
        let mut trait_definitions_by_short_name = HashMap::<Box<str>, Vec<TraitDefinitionId>>::new();
        for definition in trait_definitions {
            trait_definitions_by_path
                .entry(definition.path.into())
                .or_default()
                .push(definition.id);
            trait_definitions_by_short_name
                .entry(index_short_name(definition.path).into())
                .or_default()
                .push(definition.id);
        }

        Self {
            types_by_id,
            types_by_name: freeze_candidates(types_by_name),
            types_by_short_name: freeze_candidates(types_by_short_name),
            type_definitions_by_path: freeze_candidates(type_definitions_by_path),
            type_definitions_by_short_name: freeze_candidates(type_definitions_by_short_name),
            trait_definitions_by_path: freeze_candidates(trait_definitions_by_path),
            trait_definitions_by_short_name: freeze_candidates(trait_definitions_by_short_name),
        }
    }
}

/// Converts mutable candidate vectors into immutable slices for lookup.
///
/// # Type Parameters
/// - `T`: the candidate value stored in each index.
///
/// # Parameters
/// - `index`: mutable candidate vectors grouped by lookup name.
fn freeze_candidates<T>(index: HashMap<Box<str>, Vec<T>>) -> HashMap<Box<str>, Box<[T]>> {
    index
        .into_iter()
        .map(|(name, candidates)| (name, candidates.into_boxed_slice()))
        .collect()
}
