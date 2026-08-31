// qubit-style: allow public-type-layout
//! Frozen registry snapshot and public deterministic lookup views.

use std::any::TypeId;
use std::sync::OnceLock;

use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::error::RegistryError;
use crate::expression::TypeExpression;
use crate::registry::EffectiveTypeView;
use crate::registry::builder::build_inventory_registry;
use crate::registry::builder::initialize_cached;
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

/// An ordered borrowed view over trait declarations sharing one diagnostic
/// path.
#[derive(Clone, Copy, Debug)]
pub struct TraitCandidates<'registry> {
    descriptors: &'registry [&'static TraitDefinitionDescriptor],
}

impl<'registry> TraitCandidates<'registry> {
    /// Creates a borrowed candidate view over a registry-owned slice.
    pub(crate) const fn new(descriptors: &'registry [&'static TraitDefinitionDescriptor]) -> Self {
        Self { descriptors }
    }

    /// Returns candidates in stable fragment order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'static TraitDefinitionDescriptor> + 'registry {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching declarations.
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no declarations match.
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }

    /// Returns the sole matching declaration, rejecting ambiguous path lookups.
    pub fn only(self) -> Option<&'static TraitDefinitionDescriptor> {
        (self.descriptors.len() == 1).then(|| self.descriptors[0])
    }
}

impl<'registry> IntoIterator for TraitCandidates<'registry> {
    type Item = &'static TraitDefinitionDescriptor;
    type IntoIter = std::iter::Copied<std::slice::Iter<'registry, &'static TraitDefinitionDescriptor>>;

    /// Iterates over candidates in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.iter().copied()
    }
}

/// A deterministic borrowed set of impl definitions matching one symbolic
/// target expression.
#[derive(Clone, Debug)]
pub struct ImplDefinitionCandidates {
    descriptors: Box<[&'static ImplDefinitionDescriptor]>,
}

impl ImplDefinitionCandidates {
    /// Iterates over matching definitions in stable fragment order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'static ImplDefinitionDescriptor> + '_ {
        self.descriptors.iter().copied()
    }

    /// Returns the number of matching definitions.
    pub const fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether no impl definition has the requested target.
    pub const fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

impl IntoIterator for ImplDefinitionCandidates {
    type Item = &'static ImplDefinitionDescriptor;
    type IntoIter = std::vec::IntoIter<&'static ImplDefinitionDescriptor>;

    /// Iterates over matching definitions in stable fragment order.
    fn into_iter(self) -> Self::IntoIter {
        self.descriptors.into_vec().into_iter()
    }
}

/// The immutable process-wide snapshot of linked reflection fragments.
#[derive(Debug)]
pub struct ReflectRegistry {
    pub(super) types: Box<[&'static TypeDescriptor]>,
    pub(super) impl_definitions: Box<[&'static ImplDefinitionDescriptor]>,
    pub(super) indexes: RegistryIndexes,
    pub(super) empty_effective_view: EffectiveTypeView,
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
        TypeCandidates::new(self.indexes.types_by_type_name.get(name).map_or(&[], Box::as_ref))
    }

    /// Finds every descriptor with the reflection query name `name`.
    ///
    /// The returned view is empty when no descriptor matches and preserves the
    /// registry's stable fragment order when the name is ambiguous.
    pub fn find_by_query_name(&self, name: &str) -> TypeCandidates<'_> {
        TypeCandidates::new(self.indexes.types_by_query_name.get(name).map_or(&[], Box::as_ref))
    }

    /// Enumerates all statically registered roots in stable fragment order.
    pub fn types(&self) -> &[&'static TypeDescriptor] {
        &self.types
    }

    /// Returns every reflected implementation targeting `type_id`.
    ///
    /// The slice is empty when no linked implementation fragment targets the
    /// exact root. Its order is the registry's stable fragment order, so a
    /// caller can pass it directly to [`ImplDescriptor::lookup_method`].
    pub fn implementations(&self, type_id: TypeId) -> &[&'static ImplDescriptor] {
        self.indexes.impls_by_target.get(&type_id).map_or(&[], Box::as_ref)
    }

    /// Enumerates statically registered generic, blanket, and constrained impl
    /// declarations in stable source-fragment order.
    ///
    /// Generic and blanket definitions appear here even when they have no
    /// explicitly registered concrete specialization.
    pub fn impl_definitions(&self) -> &[&'static ImplDefinitionDescriptor] {
        &self.impl_definitions
    }

    /// Finds impl declarations whose symbolic target exactly equals `target`.
    ///
    /// Diagnostic-only text does not participate because
    /// [`TypeExpression`] equality is structural.
    pub fn find_impl_definitions_by_target(&self, target: &TypeExpression) -> ImplDefinitionCandidates {
        ImplDefinitionCandidates {
            descriptors: self
                .impl_definitions
                .iter()
                .copied()
                .filter(|definition| definition.target_type() == target)
                .collect(),
        }
    }

    /// Returns the target's frozen deterministic effective method view.
    ///
    /// Repeated calls borrow the same registry-owned view without allocation.
    /// An unregistered target borrows one stable empty view.
    pub fn effective_view(&self, type_id: TypeId) -> &EffectiveTypeView {
        self.indexes
            .effective_views_by_target
            .get(&type_id)
            .unwrap_or(&self.empty_effective_view)
    }

    /// Finds a reflected or external trait declaration by its process-local
    /// identity.
    ///
    /// `None` means no linked registration fragment declared the requested
    /// trait.
    pub fn trait_definition(&self, trait_id: &TraitId) -> Option<&'static TraitDefinitionDescriptor> {
        self.indexes.traits_by_id.get(trait_id).copied()
    }

    /// Finds the sole reflected trait declaration with a diagnostic Rust path.
    ///
    /// `None` means no linked reflected trait declaration has the exact path,
    /// or the path is ambiguous across linked fragments.
    pub fn trait_definition_by_path(&self, rust_path: &str) -> Option<&'static TraitDefinitionDescriptor> {
        self.find_trait_definitions_by_path(rust_path).only()
    }

    /// Finds every trait declaration with a diagnostic Rust path in stable
    /// order.
    pub fn find_trait_definitions_by_path(&self, rust_path: &str) -> TraitCandidates<'_> {
        TraitCandidates::new(self.indexes.traits_by_rust_path.get(rust_path).map_or(&[], Box::as_ref))
    }
}
