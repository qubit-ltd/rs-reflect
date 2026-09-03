// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Reflected inherent and trait implementation descriptors.

use std::cmp::Ordering;
use std::fmt;
use std::sync::OnceLock;

use super::trait_descriptor::generic_argument_is_concrete;
use crate::descriptor::AssociatedConstDescriptor;
use crate::descriptor::AssociatedTypeDescriptor;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::MethodInstanceDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::GenericParameterDescriptor;
use crate::expression::TypeExpression;
use crate::identity::FragmentIdentity;
use crate::value::ReflectedOwned;

/// Whether an implementation is inherent or implements a trait.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplKind {
    /// An inherent implementation block.
    Inherent,
    /// A trait implementation block.
    Trait,
}

impl ImplKind {
    /// Returns the deterministic inherent-before-trait registry rank.
    pub(crate) const fn registry_rank(self) -> u8 {
        match self {
            Self::Inherent => 0,
            Self::Trait => 1,
        }
    }
}

/// Declaration facts for a generic, blanket, or concrete impl block.
#[derive(Debug)]
pub struct ImplDefinitionDescriptor {
    fragment_identity: FragmentIdentity,
    target_type: TypeExpression,
    kind: ImplKind,
    implemented_trait: OnceLock<&'static TraitDefinitionDescriptor>,
    implemented_trait_id: Option<TraitId>,
    implemented_trait_path: Option<Box<str>>,
    generic_definition: &'static GenericDefinitionDescriptor,
    methods: OnceLock<Box<[MethodDescriptor]>>,
    associated_items: OnceLock<ImplAssociatedItems>,
}

#[derive(Debug)]
struct ImplAssociatedItems {
    types: Box<[ImplAssociatedTypeDescriptor]>,
    consts: Box<[ImplAssociatedConstDescriptor]>,
}

/// One associated type explicitly bound by an impl definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplAssociatedTypeDescriptor {
    rust_name: &'static str,
}

impl ImplAssociatedTypeDescriptor {
    /// Creates declaration-level associated type binding facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(rust_name: &'static str) -> Self {
        Self { rust_name }
    }

    /// Returns the Rust associated type name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }
}

/// One associated constant explicitly bound by an impl definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplAssociatedConstDescriptor {
    rust_name: &'static str,
    declared_type: TypeExpression,
}

impl ImplAssociatedConstDescriptor {
    /// Creates declaration-level associated constant binding facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(rust_name: &'static str, declared_type: TypeExpression) -> Self {
        Self {
            rust_name,
            declared_type,
        }
    }

    /// Returns the Rust associated constant name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the declared constant type.
    #[must_use]
    #[inline(always)]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }
}

impl ImplDefinitionDescriptor {
    /// Returns whether complete declaration facts identify the same trait.
    pub(crate) fn matches_trait_definition(&self, candidate: &TraitDefinitionDescriptor) -> bool {
        if candidate.completeness() != crate::descriptor::TraitCompleteness::Complete {
            return false;
        }
        let has_facts =
            !self.methods().is_empty() || !self.associated_types().is_empty() || !self.associated_consts().is_empty();
        has_facts
            && MethodDescriptor::signatures_match(self.methods(), candidate.methods())
            && self
                .associated_types()
                .iter()
                .map(|item| item.rust_name())
                .eq(candidate.associated_types().iter().map(|item| item.rust_name()))
            && self
                .associated_consts()
                .iter()
                .map(|item| (item.rust_name(), item.declared_type()))
                .eq(candidate
                    .associated_consts()
                    .iter()
                    .map(|item| (item.rust_name(), item.declared_type())))
    }

    /// Creates an impl definition without claiming a concrete target instance.
    ///
    /// Returns [`ImplDescriptorBuildError`] when `kind` and
    /// `implemented_trait` disagree.
    #[doc(hidden)]
    pub fn new(
        fragment_identity: FragmentIdentity,
        target_type: TypeExpression,
        kind: ImplKind,
        implemented_trait: Option<&'static TraitDefinitionDescriptor>,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Result<Self, ImplDescriptorBuildError> {
        validate_kind(kind, implemented_trait.is_some())?;
        let implemented_trait_cell = OnceLock::new();
        if let Some(descriptor) = implemented_trait {
            implemented_trait_cell
                .set(descriptor)
                .expect("a newly-created impl definition has no trait link");
        }
        Ok(Self {
            fragment_identity,
            target_type,
            kind,
            implemented_trait: implemented_trait_cell,
            implemented_trait_id: implemented_trait.map(|descriptor| descriptor.trait_id().clone()),
            implemented_trait_path: implemented_trait.map(|descriptor| descriptor.rust_path().into()),
            generic_definition,
            methods: OnceLock::new(),
            associated_items: OnceLock::new(),
        })
    }

    /// Creates a trait impl definition whose declaration link is resolved by
    /// the immutable registry after all trait fragments have been collected.
    #[doc(hidden)]
    pub fn new_unresolved_trait(
        fragment_identity: FragmentIdentity,
        target_type: TypeExpression,
        implemented_trait_path: impl Into<Box<str>>,
        implemented_trait_id: Option<TraitId>,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Self {
        Self {
            fragment_identity,
            target_type,
            kind: ImplKind::Trait,
            implemented_trait: OnceLock::new(),
            implemented_trait_id,
            implemented_trait_path: Some(implemented_trait_path.into()),
            generic_definition,
            methods: OnceLock::new(),
            associated_items: OnceLock::new(),
        }
    }

    /// Returns the source/content identity of this impl fragment.
    #[must_use]
    #[inline(always)]
    pub const fn fragment_identity(&self) -> &FragmentIdentity {
        &self.fragment_identity
    }

    /// Returns the possibly symbolic target type expression.
    #[must_use]
    #[inline(always)]
    pub const fn target_type(&self) -> &TypeExpression {
        &self.target_type
    }

    /// Returns whether this definition is inherent or implements a trait.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> ImplKind {
        self.kind
    }

    /// Returns the implemented trait definition.
    ///
    /// `None` identifies an inherent impl definition.
    #[must_use]
    #[inline(always)]
    pub fn implemented_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        self.implemented_trait.get().copied()
    }

    /// Returns the diagnostic trait path recorded by the impl declaration.
    #[must_use]
    #[inline(always)]
    pub fn implemented_trait_path(&self) -> Option<&str> {
        self.implemented_trait_path.as_deref()
    }

    /// Returns an exact trait identity supplied by the impl declaration when
    /// one is available before registry linking.
    #[must_use]
    #[inline(always)]
    pub fn implemented_trait_id(&self) -> Option<&TraitId> {
        self.implemented_trait_id.as_ref()
    }

    /// Resolves a symbolic trait link exactly once while freezing the global
    /// registry.
    pub(crate) fn resolve_implemented_trait(&'static self, descriptor: &'static TraitDefinitionDescriptor) -> bool {
        match self.implemented_trait.get() {
            Some(existing) => std::ptr::eq(*existing, descriptor),
            None => self.implemented_trait.set(descriptor).is_ok(),
        }
    }

    /// Returns generic parameters and predicates in source order.
    #[must_use]
    #[inline(always)]
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }

    /// Returns methods declared by this impl definition in source order.
    #[must_use]
    #[inline(always)]
    pub fn methods(&self) -> &[MethodDescriptor] {
        self.methods.get().map_or(&[], Box::as_ref)
    }

    /// Returns associated types explicitly bound by this impl in source order.
    #[must_use]
    #[inline(always)]
    pub fn associated_types(&self) -> &[ImplAssociatedTypeDescriptor] {
        self.associated_items.get().map_or(&[], |items| items.types.as_ref())
    }

    /// Returns associated constants explicitly bound by this impl in source
    /// order.
    #[must_use]
    #[inline(always)]
    pub fn associated_consts(&self) -> &[ImplAssociatedConstDescriptor] {
        self.associated_items.get().map_or(&[], |items| items.consts.as_ref())
    }

    /// Initializes declaration-level methods exactly once.
    #[doc(hidden)]
    pub fn initialize_methods(&'static self, initialize: impl FnOnce(&'static Self) -> Box<[MethodDescriptor]>) {
        self.methods.get_or_init(|| initialize(self));
    }

    /// Initializes declaration-level associated-item facts exactly once.
    #[doc(hidden)]
    pub fn initialize_associated_items(
        &'static self,
        initialize: impl FnOnce(
            &'static Self,
        ) -> (
            Box<[ImplAssociatedTypeDescriptor]>,
            Box<[ImplAssociatedConstDescriptor]>,
        ),
    ) {
        self.associated_items.get_or_init(|| {
            let (types, consts) = initialize(self);
            ImplAssociatedItems { types, consts }
        });
    }
}

/// The effective source of an associated constant value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstImplementationSource {
    /// The implementation uses the trait declaration's default value.
    Defaulted,
    /// The implementation explicitly overrides the constant.
    Overridden,
}

/// Why an associated constant has no safe owned-value reader.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstReadUnavailableReason {
    /// The generated code cannot prove that the declared value type is sized
    /// and `'static`, as required by the local owned dynamic boundary.
    UnprovenOwnedValue,
}

/// A safe reader for one concrete associated constant value.
pub struct AssociatedConstReader {
    read: AssociatedConstReadAdapter,
}

enum AssociatedConstReadAdapter {
    Function(fn() -> ReflectedOwned),
    Closure(&'static (dyn Fn() -> ReflectedOwned + Send + Sync)),
}

impl AssociatedConstReader {
    /// Creates a reader from generated safe adapter code.
    #[doc(hidden)]
    pub const fn new(read: fn() -> ReflectedOwned) -> Self {
        Self {
            read: AssociatedConstReadAdapter::Function(read),
        }
    }

    /// Creates a reader from a compiler-proven sized `'static` value getter.
    #[doc(hidden)]
    pub fn from_getter<T: 'static>(getter: fn() -> T) -> Self {
        let read = Box::leak(Box::new(move || ReflectedOwned::new(getter())));
        Self {
            read: AssociatedConstReadAdapter::Closure(read),
        }
    }

    /// Reads a fresh owned reflected value.
    #[must_use]
    #[inline(always)]
    pub fn read(&self) -> ReflectedOwned {
        match self.read {
            AssociatedConstReadAdapter::Function(read) => read(),
            AssociatedConstReadAdapter::Closure(read) => read(),
        }
    }
}

impl fmt::Debug for AssociatedConstReader {
    /// Formats adapter availability without exposing a process address.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AssociatedConstReader(..)")
    }
}

/// One associated type binding contributed by a concrete impl.
#[derive(Clone, Debug)]
pub struct AssociatedTypeBindingDescriptor {
    declaration: &'static AssociatedTypeDescriptor,
    value: TypeExpression,
    concrete_type: Option<TypeDescriptorResolver>,
}

impl AssociatedTypeBindingDescriptor {
    /// Creates an associated type binding.
    ///
    /// `concrete_type` is present only when `value` resolves to an exact root.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        declaration: &'static AssociatedTypeDescriptor,
        value: TypeExpression,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            declaration,
            value,
            concrete_type,
        }
    }

    /// Returns the trait declaration being bound.
    #[must_use]
    #[inline(always)]
    pub const fn declaration(&self) -> &'static AssociatedTypeDescriptor {
        self.declaration
    }

    /// Returns the concrete or still-symbolic binding expression.
    #[must_use]
    #[inline(always)]
    pub const fn value(&self) -> &TypeExpression {
        &self.value
    }

    /// Returns the exact reflected binding when it is known.
    ///
    /// `None` means the expression remains symbolic or unresolved.
    #[must_use]
    #[inline(always)]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// One associated constant binding contributed by a concrete impl.
#[derive(Clone, Debug)]
pub struct AssociatedConstBindingDescriptor {
    declaration: &'static AssociatedConstDescriptor,
    implementation_source: AssociatedConstImplementationSource,
    reader: Option<&'static AssociatedConstReader>,
    read_unavailable_reason: Option<AssociatedConstReadUnavailableReason>,
}

impl AssociatedConstBindingDescriptor {
    /// Creates associated constant binding facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        declaration: &'static AssociatedConstDescriptor,
        implementation_source: AssociatedConstImplementationSource,
        reader: Option<&'static AssociatedConstReader>,
    ) -> Self {
        let read_unavailable_reason = match reader {
            Some(_) => None,
            None => Some(AssociatedConstReadUnavailableReason::UnprovenOwnedValue),
        };
        Self {
            declaration,
            implementation_source,
            reader,
            read_unavailable_reason,
        }
    }

    /// Returns the trait declaration being implemented.
    #[must_use]
    #[inline(always)]
    pub const fn declaration(&self) -> &'static AssociatedConstDescriptor {
        self.declaration
    }

    /// Returns whether the value is defaulted or explicitly overridden.
    #[must_use]
    #[inline(always)]
    pub const fn implementation_source(&self) -> AssociatedConstImplementationSource {
        self.implementation_source
    }

    /// Returns whether a safe owned-value reader is available.
    #[must_use]
    #[inline(always)]
    pub const fn is_readable(&self) -> bool {
        self.reader.is_some()
    }

    /// Returns the structured reason why no safe reader is available.
    ///
    /// `None` means [`Self::read`] can produce a fresh owned value.
    #[must_use]
    #[inline(always)]
    pub const fn read_unavailable_reason(&self) -> Option<AssociatedConstReadUnavailableReason> {
        self.read_unavailable_reason
    }

    /// Reads the associated constant through its safe adapter.
    ///
    /// `None` means the declared type cannot cross the owned dynamic boundary.
    #[must_use]
    #[inline(always)]
    pub fn read(&self) -> Option<ReflectedOwned> {
        self.reader.map(AssociatedConstReader::read)
    }
}

/// An invalid impl definition or concrete application.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplDescriptorBuildError {
    /// An inherent impl attempted to name an implemented trait.
    InherentImplHasTrait,
    /// A trait impl omitted its trait link.
    TraitImplMissingTrait,
    /// Concrete arguments do not match the impl definition.
    GenericArgumentsDoNotMatchDefinition,
    /// The applied trait does not originate from the definition's trait.
    ImplementedTraitDefinitionMismatch,
    /// A method or associated binding belongs to another descriptor graph.
    ForeignMember,
}

impl fmt::Display for ImplDescriptorBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InherentImplHasTrait => formatter.write_str("an inherent impl cannot name a trait"),
            Self::TraitImplMissingTrait => formatter.write_str("a trait impl must name a trait"),
            Self::GenericArgumentsDoNotMatchDefinition => {
                formatter.write_str("concrete impl arguments do not match the definition")
            }
            Self::ImplementedTraitDefinitionMismatch => {
                formatter.write_str("applied trait does not match the impl definition")
            }
            Self::ForeignMember => formatter.write_str("impl descriptor contains a foreign member"),
        }
    }
}

impl std::error::Error for ImplDescriptorBuildError {}

/// A qualifier used to resolve methods across implementation namespaces.
#[derive(Clone, Copy, Debug)]
pub enum MethodQualifier<'a> {
    /// Search inherent and every trait namespace.
    Any,
    /// Search only inherent implementations.
    Inherent,
    /// Search one concrete applied trait namespace.
    Trait(&'a TraitDescriptor),
}

/// The result of a method lookup across implementation namespaces.
#[derive(Clone, Copy, Debug)]
pub enum MethodLookup<'a> {
    /// No matching concrete method instance exists.
    Missing,
    /// Exactly one concrete method instance matches.
    Unique(&'a MethodInstanceDescriptor),
    /// Multiple namespaces or fragments match the query.
    Ambiguous,
}

/// One explicitly registered concrete instance of an impl definition.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// use qubit_reflect::{Reflect, TypeDescriptor};
/// #[cfg(feature = "derive")]
/// use qubit_reflect::reflect_impl;
///
/// #[cfg(feature = "derive")]
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
///
/// #[cfg(feature = "derive")]
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Service {
///     fn ping(&self) {}
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let implementation = TypeDescriptor::of::<Service>()
///     .impls()?
///     .first()
///     .expect("reflected implementation");
/// assert_eq!(implementation.target_type().query_name(), "Service");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
pub struct ImplDescriptor {
    definition: &'static ImplDefinitionDescriptor,
    target_type: TypeDescriptorResolver,
    implemented_trait: Option<&'static TraitDescriptor>,
    methods: &'static [MethodDescriptor],
    method_instances: Box<[MethodInstanceDescriptor]>,
    associated_types: Box<[AssociatedTypeBindingDescriptor]>,
    associated_consts: Box<[AssociatedConstBindingDescriptor]>,
    arguments: Box<[GenericArgument]>,
}

impl ImplDescriptor {
    /// Returns whether two descriptors represent the same concrete impl
    /// application.
    pub(crate) fn same_application(&self, other: &Self) -> bool {
        self.kind() == other.kind()
            && self.definition().fragment_identity() == other.definition().fragment_identity()
            && self.arguments() == other.arguments()
            && self.target_type().type_id() == other.target_type().type_id()
    }

    /// Orders implementations by kind, namespace, and source identity.
    pub(crate) fn registry_cmp(&self, other: &Self) -> Ordering {
        self.kind()
            .registry_rank()
            .cmp(&other.kind().registry_rank())
            .then_with(|| self.namespace_cmp(other))
            .then_with(|| {
                self.definition()
                    .fragment_identity()
                    .cmp(other.definition().fragment_identity())
            })
    }

    /// Orders implementation namespaces deterministically.
    fn namespace_cmp(&self, other: &Self) -> Ordering {
        match (self.implemented_trait(), other.implemented_trait()) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(left), Some(right)) => match (left.definition().trait_id(), right.definition().trait_id()) {
                (TraitId::Reflected(_), TraitId::Reflected(_)) => left.rust_path().cmp(right.rust_path()),
                (TraitId::External(left), TraitId::External(right)) => left.cmp(right),
                (TraitId::Reflected(_), TraitId::External(_)) => Ordering::Less,
                (TraitId::External(_), TraitId::Reflected(_)) => Ordering::Greater,
            },
        }
    }

    /// Returns whether this implementation belongs to a lookup namespace.
    pub(crate) fn matches_qualifier(&self, qualifier: MethodQualifier<'_>) -> bool {
        match qualifier {
            MethodQualifier::Any => true,
            MethodQualifier::Inherent => self.kind() == ImplKind::Inherent,
            MethodQualifier::Trait(expected) => self
                .implemented_trait()
                .is_some_and(|actual| actual.same_application(expected)),
        }
    }

    /// Starts a concrete impl builder for `definition` and `target_type`.
    pub fn builder(
        definition: &'static ImplDefinitionDescriptor,
        target_type: TypeDescriptorResolver,
    ) -> ImplDescriptorBuilder {
        ImplDescriptorBuilder::new(definition, target_type)
    }

    /// Returns the generic or blanket impl definition.
    #[must_use]
    #[inline(always)]
    pub const fn definition(&self) -> &'static ImplDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected root targeted by this concrete impl.
    #[must_use]
    #[inline(always)]
    pub fn target_type(&self) -> &'static TypeDescriptor {
        (self.target_type)()
    }

    /// Returns whether this is an inherent or trait implementation.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> ImplKind {
        self.definition.kind()
    }

    /// Returns the concrete applied trait, or `None` for an inherent impl.
    #[must_use]
    #[inline(always)]
    pub const fn implemented_trait(&self) -> Option<&'static TraitDescriptor> {
        self.implemented_trait
    }

    /// Returns methods explicitly declared by this impl definition.
    #[must_use]
    #[inline(always)]
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Returns methods explicitly declared by this impl definition.
    #[must_use]
    #[inline(always)]
    pub const fn implementation_methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method explicitly declared by this concrete impl, by query
    /// name.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns concrete effective instances, including defaulted methods.
    #[must_use]
    #[inline(always)]
    pub const fn method_instances(&self) -> &[MethodInstanceDescriptor] {
        &self.method_instances
    }

    /// Returns associated type bindings in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_types(&self) -> &[AssociatedTypeBindingDescriptor] {
        &self.associated_types
    }

    /// Returns associated constant bindings in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_consts(&self) -> &[AssociatedConstBindingDescriptor] {
        &self.associated_consts
    }

    /// Returns concrete impl arguments in definition parameter order.
    #[must_use]
    #[inline(always)]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Looks up one effective method across impl namespaces.
    pub fn lookup_method<'a>(
        implementations: &'a [&'a ImplDescriptor],
        qualifier: MethodQualifier<'_>,
        name: &str,
    ) -> MethodLookup<'a> {
        let mut found = None;
        for implementation in implementations {
            if !implementation.matches_qualifier(qualifier) {
                continue;
            }
            for instance in implementation.method_instances() {
                if instance.declaration().query_name() != name {
                    continue;
                }
                if found.is_some() {
                    return MethodLookup::Ambiguous;
                }
                found = Some(instance);
            }
        }
        found.map_or(MethodLookup::Missing, MethodLookup::Unique)
    }
}

impl fmt::Debug for ImplDescriptor {
    /// Formats local facts without recursively expanding graph roots.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImplDescriptor")
            .field("fragment", self.definition.fragment_identity())
            .field("kind", &self.kind())
            .field("target_type", &"<resolver>")
            .field("method_instance_count", &self.method_instances.len())
            .field("arguments", &self.arguments)
            .finish()
    }
}

/// Builds a concrete impl while preserving source order.
#[derive(Debug)]
pub struct ImplDescriptorBuilder {
    definition: &'static ImplDefinitionDescriptor,
    target_type: TypeDescriptorResolver,
    implemented_trait: Option<&'static TraitDescriptor>,
    methods: &'static [MethodDescriptor],
    method_instances: Vec<MethodInstanceDescriptor>,
    associated_types: Vec<AssociatedTypeBindingDescriptor>,
    associated_consts: Vec<AssociatedConstBindingDescriptor>,
    arguments: Vec<GenericArgument>,
}

impl ImplDescriptorBuilder {
    /// Creates an empty concrete instance builder.
    fn new(definition: &'static ImplDefinitionDescriptor, target_type: TypeDescriptorResolver) -> Self {
        Self {
            definition,
            target_type,
            implemented_trait: None,
            methods: &[],
            method_instances: Vec::new(),
            associated_types: Vec::new(),
            associated_consts: Vec::new(),
            arguments: Vec::new(),
        }
    }

    /// Sets the applied trait implemented by this instance.
    pub fn implemented_trait(mut self, implemented_trait: &'static TraitDescriptor) -> Self {
        self.implemented_trait = Some(implemented_trait);
        self
    }

    /// Sets methods explicitly declared by the impl definition.
    #[must_use]
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets concrete effective method instances.
    #[must_use]
    pub fn method_instances(mut self, instances: Vec<MethodInstanceDescriptor>) -> Self {
        self.method_instances = instances;
        self
    }

    /// Sets associated type bindings in declaration order.
    #[must_use]
    pub fn associated_types(mut self, bindings: Vec<AssociatedTypeBindingDescriptor>) -> Self {
        self.associated_types = bindings;
        self
    }

    /// Sets associated constant bindings in declaration order.
    #[must_use]
    pub fn associated_consts(mut self, bindings: Vec<AssociatedConstBindingDescriptor>) -> Self {
        self.associated_consts = bindings;
        self
    }

    /// Sets concrete impl arguments in definition parameter order.
    #[must_use]
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Validates and builds the concrete impl descriptor.
    ///
    /// Returns [`ImplDescriptorBuildError`] for inconsistent trait, generic,
    /// method, or associated-item relationships.
    pub fn build(self) -> Result<ImplDescriptor, ImplDescriptorBuildError> {
        validate_kind(self.definition.kind(), self.implemented_trait.is_some())?;
        let expected_arguments = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .count();
        if expected_arguments != self.arguments.len()
            || self
                .arguments
                .iter()
                .any(|argument| !generic_argument_is_concrete(argument))
        {
            return Err(ImplDescriptorBuildError::GenericArgumentsDoNotMatchDefinition);
        }
        let kinds_match = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .zip(&self.arguments)
            .all(|(parameter, argument)| {
                matches!(
                    (parameter, argument),
                    (GenericParameterDescriptor::Type { .. }, GenericArgument::Type(_))
                        | (GenericParameterDescriptor::Const { .. }, GenericArgument::Const(_))
                )
            });
        if !kinds_match {
            return Err(ImplDescriptorBuildError::GenericArgumentsDoNotMatchDefinition);
        }
        if let (Some(expected), Some(actual)) = (self.definition.implemented_trait(), self.implemented_trait)
            && actual.definition().trait_id() != expected.trait_id()
        {
            return Err(ImplDescriptorBuildError::ImplementedTraitDefinitionMismatch);
        }
        if self.methods.iter().any(|method| {
            !method
                .declaring_impl()
                .is_some_and(|owner| std::ptr::eq(owner, self.definition))
        }) {
            return Err(ImplDescriptorBuildError::ForeignMember);
        }
        if let Some(applied_trait) = self.implemented_trait {
            let foreign_method = self.method_instances.iter().any(|instance| {
                !applied_trait
                    .methods()
                    .iter()
                    .any(|method| std::ptr::eq(method, instance.declaration()))
                    || instance
                        .implementation_method()
                        .is_some_and(|method| !self.methods.iter().any(|candidate| std::ptr::eq(candidate, method)))
            });
            let foreign_type = self.associated_types.iter().any(|binding| {
                !applied_trait
                    .associated_types()
                    .iter()
                    .any(|item| std::ptr::eq(item, binding.declaration()))
            });
            let foreign_const = self.associated_consts.iter().any(|binding| {
                !applied_trait
                    .associated_consts()
                    .iter()
                    .any(|item| std::ptr::eq(item, binding.declaration()))
            });
            if foreign_method || foreign_type || foreign_const {
                return Err(ImplDescriptorBuildError::ForeignMember);
            }
        } else if self.method_instances.iter().any(|instance| {
            instance.implementation_source() != crate::descriptor::MethodImplementationSource::Declared
                || !self
                    .methods
                    .iter()
                    .any(|method| std::ptr::eq(method, instance.declaration()))
                || instance.implementation_method().is_some()
        }) {
            return Err(ImplDescriptorBuildError::ForeignMember);
        }
        Ok(ImplDescriptor {
            definition: self.definition,
            target_type: self.target_type,
            implemented_trait: self.implemented_trait,
            methods: self.methods,
            method_instances: self.method_instances.into_boxed_slice(),
            associated_types: self.associated_types.into_boxed_slice(),
            associated_consts: self.associated_consts.into_boxed_slice(),
            arguments: self.arguments.into_boxed_slice(),
        })
    }
}

/// Validates the invariant shared by impl definitions and instances.
fn validate_kind(kind: ImplKind, has_trait: bool) -> Result<(), ImplDescriptorBuildError> {
    match (kind, has_trait) {
        (ImplKind::Inherent, true) => Err(ImplDescriptorBuildError::InherentImplHasTrait),
        (ImplKind::Trait, false) => Err(ImplDescriptorBuildError::TraitImplMissingTrait),
        _ => Ok(()),
    }
}
