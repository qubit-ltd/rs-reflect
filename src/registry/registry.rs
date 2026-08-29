//! Frozen registry snapshot and public deterministic lookup views.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::descriptor::TypeDescriptor;
use crate::error::RegistryError;
use crate::registry::builder::{build_inventory_registry, initialize_cached};
use crate::registry::indexes::RegistryIndexes;

/// A borrowed, deterministic set of type-descriptor name matches.
#[derive(Clone, Copy, Debug)]
pub struct TypeCandidates<'registry> {
    descriptors: &'registry [&'static TypeDescriptor],
}

impl<'registry> TypeCandidates<'registry> {
    /// Creates a borrowed candidate view over a registry-owned slice.
    pub(crate) const fn new(descriptors: &'registry [&'static TypeDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TypeDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching descriptors.
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no descriptor matched the requested name.
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }
}

impl<'registry> IntoIterator for TypeCandidates<'registry> {
    type Item = &'static TypeDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TypeDescriptor>>;

    /// Iterates over candidates in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}

/// The immutable process-wide snapshot of linked reflection fragments.
#[derive(Debug)]
pub struct ReflectRegistry {
    pub(super) types: Box<[&'static TypeDescriptor]>,
    pub(super) indexes: RegistryIndexes,
}

impl ReflectRegistry {
    /// Initializes and returns the process-wide immutable registry snapshot.
    ///
    /// All linked fragments are sorted, materialized, and validated before a
    /// snapshot is published. Both success and [`RegistryError`] are cached;
    /// concurrent callers therefore observe the same immutable result.
    pub fn initialize() -> Result<&'static Self, RegistryError> {
        static REGISTRY: OnceLock<Result<ReflectRegistry, RegistryError>> = OnceLock::new();
        initialize_cached(&REGISTRY, build_inventory_registry)
    }

    /// Looks up one exact concrete registered type.
    ///
    /// `None` means no linked static fragment registered `type_id`.
    pub fn get(&self, type_id: TypeId) -> Option<&'static TypeDescriptor> {
        self.indexes.types_by_id.get(&type_id).copied()
    }

    /// Finds every descriptor with the diagnostic Rust type name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    pub fn find_by_type_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(
            self.indexes
                .types_by_type_name
                .get(name)
                .map_or(&[], Box::as_ref),
        )
    }

    /// Finds every descriptor with the reflection query name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    pub fn find_by_query_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(
            self.indexes
                .types_by_query_name
                .get(name)
                .map_or(&[], Box::as_ref),
        )
    }

    /// Enumerates all statically registered roots in stable fragment order.
    pub fn types(&self) -> &[&'static TypeDescriptor] {
        &self.types
    }
}
