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
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ImplKind;
/// let kind = ImplKind::Inherent;
/// assert_eq!(kind, ImplKind::Inherent);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplKind {
    /// An inherent implementation block.
    Inherent,
    /// A trait implementation block.
    Trait,
}

impl ImplKind {
    /// Returns the deterministic inherent-before-trait registry rank.
    ///
    /// # Returns
    ///
    /// Returns `0` for inherent impls and `1` for trait impls.
    pub(crate) const fn registry_rank(self) -> u8 {
        match self {
            Self::Inherent => 0,
            Self::Trait => 1,
        }
    }
}

/// Declaration facts for a generic, blanket, or concrete impl block.
///
/// This descriptor is constructed by generated registration code.
#[derive(Debug)]
pub struct ImplDefinitionDescriptor {
    /// Stable identity of the source impl fragment.
    fragment_identity: FragmentIdentity,
    /// Target expression, which may contain generic parameters.
    target_type: TypeExpression,
    /// Whether the source impl is inherent or implements a trait.
    kind: ImplKind,
    /// Resolved trait declaration, when it is already available.
    implemented_trait: Option<&'static TraitDefinitionDescriptor>,
    /// Exact trait identity retained for registry linking.
    implemented_trait_id: Option<TraitId>,
    /// Diagnostic path retained even when the trait declaration is unresolved.
    implemented_trait_path: Option<Box<str>>,
    /// Generic parameters and predicates declared by this impl.
    generic_definition: &'static GenericDefinitionDescriptor,
    /// Lazily initialized methods in source order.
    methods: OnceLock<Box<[MethodDescriptor]>>,
    /// Lazily initialized associated-item facts.
    associated_items: OnceLock<ImplAssociatedItems>,
}

#[derive(Debug)]
struct ImplAssociatedItems {
    /// Explicit associated type bindings.
    types: Box<[ImplAssociatedTypeDescriptor]>,
    /// Explicit associated constant bindings.
    consts: Box<[ImplAssociatedConstDescriptor]>,
}

/// One associated type explicitly bound by an impl definition.
///
/// This descriptor is constructed by generated registration code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplAssociatedTypeDescriptor {
    /// Name used in the Rust declaration.
    rust_name: &'static str,
}

impl ImplAssociatedTypeDescriptor {
    /// Creates declaration-level associated type binding facts.
    ///
    /// # Parameters
    ///
    /// - `rust_name`: Name used by the associated type declaration.
    ///
    /// # Returns
    ///
    /// Returns the declaration-level binding facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(rust_name: &'static str) -> Self {
        Self { rust_name }
    }

    /// Returns the Rust associated type name.
    ///
    /// # Returns
    ///
    /// Returns the associated type's Rust name.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }
}

/// One associated constant explicitly bound by an impl definition.
///
/// This descriptor is constructed by generated registration code.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImplAssociatedConstDescriptor {
    /// Name used in the Rust declaration.
    rust_name: &'static str,
    /// Declared type expression for the constant.
    declared_type: TypeExpression,
}

impl ImplAssociatedConstDescriptor {
    /// Creates declaration-level associated constant binding facts.
    ///
    /// # Parameters
    ///
    /// - `rust_name`: Name used by the associated constant declaration.
    /// - `declared_type`: Type expression declared for the constant.
    ///
    /// # Returns
    ///
    /// Returns the declaration-level binding facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(rust_name: &'static str, declared_type: TypeExpression) -> Self {
        Self {
            rust_name,
            declared_type,
        }
    }

    /// Returns the Rust associated constant name.
    ///
    /// # Returns
    ///
    /// Returns the associated constant's Rust name.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the declared constant type.
    ///
    /// # Returns
    ///
    /// Returns the declared type expression.
    #[must_use]
    #[inline]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }
}

impl ImplDefinitionDescriptor {
    /// Creates an impl definition without claiming a concrete target instance.
    ///
    /// Returns [`ImplDescriptorBuildError`] when `kind` and
    /// `implemented_trait` disagree.
    ///
    /// # Parameters
    ///
    /// - `fragment_identity`: Stable source identity for this impl fragment.
    /// - `target_type`: Target type expression, possibly containing parameters.
    /// - `kind`: Whether the impl is inherent or a trait impl.
    /// - `implemented_trait`: Trait declaration for a trait impl, or `None` for
    ///   an inherent impl.
    /// - `generic_definition`: Generic parameters and predicates declared by
    ///   the impl.
    ///
    /// # Returns
    ///
    /// Returns the initialized declaration descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`ImplDescriptorBuildError::InherentImplHasTrait`] when an
    /// inherent impl names a trait,
    /// or [`ImplDescriptorBuildError::TraitImplMissingTrait`] when a trait impl
    /// has no trait declaration.
    #[doc(hidden)]
    pub fn new(
        fragment_identity: FragmentIdentity,
        target_type: TypeExpression,
        kind: ImplKind,
        implemented_trait: Option<&'static TraitDefinitionDescriptor>,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Result<Self, ImplDescriptorBuildError> {
        validate_kind(kind, implemented_trait.is_some())?;
        Ok(Self {
            fragment_identity,
            target_type,
            kind,
            implemented_trait,
            implemented_trait_id: implemented_trait.map(|descriptor| descriptor.trait_id().clone()),
            implemented_trait_path: implemented_trait.map(|descriptor| descriptor.rust_path().into()),
            generic_definition,
            methods: OnceLock::new(),
            associated_items: OnceLock::new(),
        })
    }

    /// Creates a trait impl definition whose declaration link is resolved by
    /// the immutable registry after all trait fragments have been collected.
    ///
    /// # Parameters
    ///
    /// - `fragment_identity`: Stable source identity for this impl fragment.
    /// - `target_type`: Target type expression, possibly containing parameters.
    /// - `implemented_trait_path`: Diagnostic Rust path for the trait
    ///   declaration.
    /// - `implemented_trait_id`: Exact trait identity when available.
    /// - `generic_definition`: Generic parameters and predicates declared by
    ///   the impl.
    ///
    /// # Returns
    ///
    /// Returns an unresolved trait impl declaration for later registry linking.
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
            implemented_trait: None,
            implemented_trait_id,
            implemented_trait_path: Some(implemented_trait_path.into()),
            generic_definition,
            methods: OnceLock::new(),
            associated_items: OnceLock::new(),
        }
    }

    /// Returns the source/content identity of this impl fragment.
    ///
    /// # Returns
    ///
    /// Returns the stable source identity.
    #[must_use]
    #[inline]
    pub const fn fragment_identity(&self) -> &FragmentIdentity {
        &self.fragment_identity
    }

    /// Returns the possibly symbolic target type expression.
    ///
    /// # Returns
    ///
    /// Returns the target type expression.
    #[must_use]
    #[inline]
    pub const fn target_type(&self) -> &TypeExpression {
        &self.target_type
    }

    /// Returns whether this definition is inherent or implements a trait.
    ///
    /// # Returns
    ///
    /// Returns the implementation kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ImplKind {
        self.kind
    }

    /// Returns the implemented trait definition.
    ///
    /// `None` identifies an inherent impl or an unresolved trait declaration.
    /// Use [`Self::implemented_trait_in`] for snapshot-resolved links.
    ///
    /// # Returns
    ///
    /// Returns the linked trait declaration, or `None` when this definition is
    /// inherent or unresolved.
    #[must_use]
    #[inline]
    pub fn implemented_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        self.implemented_trait
    }

    /// Returns the diagnostic trait path recorded by the impl declaration.
    ///
    /// # Returns
    ///
    /// Returns the recorded trait path, or `None` for an inherent impl.
    #[must_use]
    #[inline]
    pub fn implemented_trait_path(&self) -> Option<&str> {
        self.implemented_trait_path.as_deref()
    }

    /// Returns an exact trait identity supplied by the impl declaration when
    /// one is available before registry linking.
    ///
    /// # Returns
    ///
    /// Returns the exact trait identity, or `None` when it was not supplied.
    #[must_use]
    #[inline]
    pub fn implemented_trait_id(&self) -> Option<&TraitId> {
        self.implemented_trait_id.as_ref()
    }

    /// Returns the trait link resolved in `registry`, or `None` when this
    /// definition has no trait link in that snapshot. Never initializes a
    /// global registry or changes this declaration.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable registry snapshot used to resolve the trait
    ///   link.
    ///
    /// # Returns
    ///
    /// Returns the snapshot-resolved trait declaration, or `None` when
    /// unavailable.
    #[must_use]
    pub fn implemented_trait_in(
        &self,
        registry: &crate::registry::ReflectRegistry,
    ) -> Option<&'static TraitDefinitionDescriptor> {
        registry.impl_definition_trait(self)
    }

    /// Returns generic parameters and predicates in source order.
    ///
    /// # Returns
    ///
    /// Returns this impl's generic definition.
    #[must_use]
    #[inline]
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }

    /// Returns methods declared by this impl definition in source order.
    ///
    /// # Returns
    ///
    /// Returns the initialized methods, or an empty slice before
    /// initialization.
    #[must_use]
    #[inline]
    pub fn methods(&self) -> &[MethodDescriptor] {
        self.methods.get().map_or(&[], Box::as_ref)
    }

    /// Returns associated types explicitly bound by this impl in source order.
    ///
    /// # Returns
    ///
    /// Returns the initialized associated type bindings, or an empty slice
    /// before initialization.
    #[must_use]
    #[inline]
    pub fn associated_types(&self) -> &[ImplAssociatedTypeDescriptor] {
        self.associated_items.get().map_or(&[], |items| items.types.as_ref())
    }

    /// Returns associated constants explicitly bound by this impl in source
    /// order.
    ///
    /// # Returns
    ///
    /// Returns the initialized associated constant bindings, or an empty slice
    /// before initialization.
    #[must_use]
    #[inline]
    pub fn associated_consts(&self) -> &[ImplAssociatedConstDescriptor] {
        self.associated_items.get().map_or(&[], |items| items.consts.as_ref())
    }

    /// Initializes declaration-level methods exactly once.
    ///
    /// # Parameters
    ///
    /// - `initialize`: Callback that builds methods from this static
    ///   definition.
    ///
    /// # Returns
    ///
    /// Returns `()`; subsequent calls leave the initialized value unchanged.
    #[doc(hidden)]
    pub fn initialize_methods(&'static self, initialize: impl FnOnce(&'static Self) -> Box<[MethodDescriptor]>) {
        self.methods.get_or_init(|| initialize(self));
    }

    /// Initializes declaration-level associated-item facts exactly once.
    ///
    /// # Parameters
    ///
    /// - `initialize`: Callback that builds associated types and constants from
    ///   this static definition.
    ///
    /// # Returns
    ///
    /// Returns `()`; subsequent calls leave the initialized value unchanged.
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
///
/// This value distinguishes a trait-provided default from an explicit impl
/// override.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstImplementationSource;
/// assert_eq!(AssociatedConstImplementationSource::Defaulted, AssociatedConstImplementationSource::Defaulted);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstImplementationSource {
    /// The implementation uses the trait declaration's default value.
    Defaulted,
    /// The implementation explicitly overrides the constant.
    Overridden,
}

/// Why an associated constant has no safe owned-value reader.
///
/// This reason is reported when generated code cannot prove the value can cross
/// the owned dynamic boundary.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedConstReadUnavailableReason;
/// let reason = AssociatedConstReadUnavailableReason::UnprovenOwnedValue;
/// assert_eq!(reason, AssociatedConstReadUnavailableReason::UnprovenOwnedValue);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstReadUnavailableReason {
    /// The generated code cannot prove that the declared value type is sized
    /// and `'static`, as required by the local owned dynamic boundary.
    UnprovenOwnedValue,
}

/// A safe reader for one concrete associated constant value.
///
/// A reader invokes its generated adapter each time, producing a fresh owned
/// value without exposing a reference to static storage.
pub struct AssociatedConstReader {
    /// Safe function or closure adapter that reads a fresh owned value.
    read: AssociatedConstReadAdapter,
}

/// Internal storage forms for generated associated constant readers.
enum AssociatedConstReadAdapter {
    /// Non-capturing generated reader.
    Function(fn() -> ReflectedOwned),
    /// Static closure used to adapt a concrete value getter.
    Closure(&'static (dyn Fn() -> ReflectedOwned + Send + Sync)),
}

impl AssociatedConstReader {
    /// Creates a reader from generated safe adapter code.
    ///
    /// # Parameters
    ///
    /// - `read`: Generated function returning an owned reflected value.
    ///
    /// # Returns
    ///
    /// Returns a reader that invokes `read` on each call.
    #[doc(hidden)]
    pub const fn new(read: fn() -> ReflectedOwned) -> Self {
        Self {
            read: AssociatedConstReadAdapter::Function(read),
        }
    }

    /// Creates a reader from a compiler-proven sized `'static` value getter.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete associated constant value type.
    ///
    /// # Parameters
    ///
    /// - `getter`: Function that returns the associated constant value.
    ///
    /// # Returns
    ///
    /// Returns a reader that owns each value produced by `getter`.
    /// The generated closure is retained for the process lifetime.
    #[doc(hidden)]
    pub fn from_getter<T: 'static>(getter: fn() -> T) -> Self {
        let read = Box::leak(Box::new(move || ReflectedOwned::new(getter())));
        Self {
            read: AssociatedConstReadAdapter::Closure(read),
        }
    }

    /// Reads a fresh owned reflected value.
    ///
    /// This invokes the generated reader adapter on every call.
    ///
    /// # Returns
    ///
    /// Returns a newly owned reflected value.
    #[must_use]
    pub fn read(&self) -> ReflectedOwned {
        match self.read {
            AssociatedConstReadAdapter::Function(read) => read(),
            AssociatedConstReadAdapter::Closure(read) => read(),
        }
    }
}

impl fmt::Debug for AssociatedConstReader {
    /// Formats adapter availability without exposing a process address.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the stable, address-free
    ///   representation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the representation, or the formatter
    /// error.
    ///
    /// # Errors
    ///
    /// Returns the error reported by `formatter`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AssociatedConstReader(..)")
    }
}

/// One associated type binding contributed by a concrete impl.
///
/// This descriptor is constructed by generated registration code.
#[derive(Clone, Debug)]
pub struct AssociatedTypeBindingDescriptor {
    /// Associated type declaration being implemented.
    declaration: &'static AssociatedTypeDescriptor,
    /// Concrete or symbolic assigned type expression.
    value: TypeExpression,
    /// Resolver for the exact type, when it can be resolved.
    concrete_type: Option<TypeDescriptorResolver>,
}

impl AssociatedTypeBindingDescriptor {
    /// Creates an associated type binding.
    ///
    /// `concrete_type` is present only when `value` resolves to an exact root.
    ///
    /// # Parameters
    ///
    /// - `declaration`: Associated type declaration being bound.
    /// - `value`: Concrete or symbolic assigned type expression.
    /// - `concrete_type`: Exact reflected type resolver, when known.
    ///
    /// # Returns
    ///
    /// Returns the associated type binding facts.
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
    ///
    /// # Returns
    ///
    /// Returns the associated type declaration.
    #[must_use]
    #[inline]
    pub const fn declaration(&self) -> &'static AssociatedTypeDescriptor {
        self.declaration
    }

    /// Returns the concrete or still-symbolic binding expression.
    ///
    /// # Returns
    ///
    /// Returns the assigned type expression.
    #[must_use]
    #[inline]
    pub const fn value(&self) -> &TypeExpression {
        &self.value
    }

    /// Returns the exact reflected binding when it is known.
    ///
    /// `None` means the expression remains symbolic or unresolved.
    ///
    /// # Returns
    ///
    /// Returns the resolved root descriptor, or `None` when unresolved.
    #[must_use]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// One associated constant binding contributed by a concrete impl.
///
/// This descriptor is constructed by generated registration code.
#[derive(Clone, Debug)]
pub struct AssociatedConstBindingDescriptor {
    /// Associated constant declaration being implemented.
    declaration: &'static AssociatedConstDescriptor,
    /// Whether the value comes from the default or an override.
    implementation_source: AssociatedConstImplementationSource,
    /// Safe owned-value reader, when available.
    reader: Option<&'static AssociatedConstReader>,
    /// Reason no reader can be provided.
    read_unavailable_reason: Option<AssociatedConstReadUnavailableReason>,
}

impl AssociatedConstBindingDescriptor {
    /// Creates associated constant binding facts.
    ///
    /// # Parameters
    ///
    /// - `declaration`: Associated constant declaration being bound.
    /// - `implementation_source`: Whether the implementation is defaulted or
    ///   overridden.
    /// - `reader`: Safe owned-value reader, when the value type permits one.
    ///
    /// # Returns
    ///
    /// Returns the associated constant binding facts.
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
    ///
    /// # Returns
    ///
    /// Returns the associated constant declaration.
    #[must_use]
    #[inline]
    pub const fn declaration(&self) -> &'static AssociatedConstDescriptor {
        self.declaration
    }

    /// Returns whether the value is defaulted or explicitly overridden.
    ///
    /// # Returns
    ///
    /// Returns the implementation source.
    #[must_use]
    #[inline]
    pub const fn implementation_source(&self) -> AssociatedConstImplementationSource {
        self.implementation_source
    }

    /// Returns whether a safe owned-value reader is available.
    ///
    /// # Returns
    ///
    /// Returns `true` when [`Self::read`] can return a value.
    #[must_use]
    #[inline]
    pub const fn is_readable(&self) -> bool {
        self.reader.is_some()
    }

    /// Returns the structured reason why no safe reader is available.
    ///
    /// `None` means [`Self::read`] can produce a fresh owned value.
    ///
    /// # Returns
    ///
    /// Returns the reason reading is unavailable, or `None` when it is
    /// available.
    #[must_use]
    #[inline]
    pub const fn read_unavailable_reason(&self) -> Option<AssociatedConstReadUnavailableReason> {
        self.read_unavailable_reason
    }

    /// Reads the associated constant through its safe adapter.
    ///
    /// `None` means the declared type cannot cross the owned dynamic boundary.
    ///
    /// # Returns
    ///
    /// Returns a fresh owned reflected value, or `None` when no safe reader
    /// exists.
    #[must_use]
    pub fn read(&self) -> Option<ReflectedOwned> {
        self.reader.map(AssociatedConstReader::read)
    }
}

/// An invalid impl definition or concrete application.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::ImplDescriptorBuildError;
/// let error = ImplDescriptorBuildError::TraitImplMissingTrait;
/// assert_eq!(error, ImplDescriptorBuildError::TraitImplMissingTrait);
/// ```
#[must_use]
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the diagnostic message.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the message, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns the error reported by `formatter`.
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodQualifier;
/// let qualifier = MethodQualifier::Any;
/// assert!(matches!(qualifier, MethodQualifier::Any));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum MethodQualifier<'a> {
    /// Search inherent and every trait namespace.
    Any,
    /// Search only inherent implementations.
    Inherent,
    /// Search one concrete applied trait namespace, using the contained trait.
    Trait(&'a TraitDescriptor),
}

/// The result of a method lookup across implementation namespaces.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodLookup;
/// let result = MethodLookup::Missing;
/// assert!(matches!(result, MethodLookup::Missing));
/// ```
#[derive(Clone, Copy, Debug)]
pub enum MethodLookup<'a> {
    /// No matching concrete method instance exists.
    Missing,
    /// Exactly one concrete method instance matches; the variant contains it.
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
/// #[cfg(feature = "derive")]
/// {
/// use qubit_reflect::TypeDescriptor;
/// mod example {
///     use qubit_reflect::{Reflect, reflect_impl};
///     #[derive(Reflect)]
///     #[reflect(crate = qubit_reflect)]
///     pub struct Service;
///     #[reflect_impl(crate = qubit_reflect)]
///     impl Service {
///         fn ping(&self) {}
///     }
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let implementation = TypeDescriptor::of::<example::Service>()
///     .impls()?
///     .first()
///     .expect("reflected implementation");
/// assert_eq!(implementation.target_type().query_name(), "Service");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// }
/// ```
pub struct ImplDescriptor {
    /// Source declaration represented by this concrete application.
    definition: &'static ImplDefinitionDescriptor,
    /// Resolver for the concrete target root.
    target_type: TypeDescriptorResolver,
    /// Concrete applied trait namespace, if this is a trait impl.
    implemented_trait: Option<&'static TraitDescriptor>,
    /// Methods declared by the impl definition.
    methods: &'static [MethodDescriptor],
    /// Effective concrete method instances.
    method_instances: Box<[MethodInstanceDescriptor]>,
    /// Concrete associated type bindings.
    associated_types: Box<[AssociatedTypeBindingDescriptor]>,
    /// Concrete associated constant bindings.
    associated_consts: Box<[AssociatedConstBindingDescriptor]>,
    /// Concrete generic arguments in definition order.
    arguments: Box<[GenericArgument]>,
}

impl ImplDescriptor {
    /// Returns whether two descriptors represent the same concrete impl
    /// application.
    ///
    /// # Parameters
    ///
    /// - `other`: Descriptor to compare with this application.
    ///
    /// # Returns
    ///
    /// Returns `true` when both descriptors identify the same impl application.
    pub(crate) fn same_application(&self, other: &Self) -> bool {
        self.kind() == other.kind()
            && self.definition().fragment_identity() == other.definition().fragment_identity()
            && self.arguments() == other.arguments()
            && self.target_type().type_id() == other.target_type().type_id()
    }

    /// Orders implementations by kind, namespace, and source identity.
    ///
    /// # Parameters
    ///
    /// - `other`: Descriptor to compare with this implementation.
    ///
    /// # Returns
    ///
    /// Returns the deterministic ordering between the implementations.
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
    ///
    /// # Parameters
    ///
    /// - `other`: Descriptor whose namespace is compared with this one.
    ///
    /// # Returns
    ///
    /// Returns the ordering between the implementation namespaces.
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
    ///
    /// # Parameters
    ///
    /// - `qualifier`: Namespace restriction for the lookup.
    ///
    /// # Returns
    ///
    /// Returns `true` when this implementation matches the qualifier.
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
    ///
    /// # Parameters
    ///
    /// - `definition`: Source impl declaration to instantiate.
    /// - `target_type`: Resolver for the concrete target root.
    ///
    /// # Returns
    ///
    /// Returns a builder initialized for the requested declaration and target.
    pub fn builder(
        definition: &'static ImplDefinitionDescriptor,
        target_type: TypeDescriptorResolver,
    ) -> ImplDescriptorBuilder {
        ImplDescriptorBuilder::new(definition, target_type)
    }

    /// Returns the generic or blanket impl definition.
    ///
    /// # Returns
    ///
    /// Returns the source impl definition.
    #[must_use]
    #[inline]
    pub const fn definition(&self) -> &'static ImplDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected root targeted by this concrete impl.
    ///
    /// # Returns
    ///
    /// Returns the resolved target type descriptor.
    #[must_use]
    pub fn target_type(&self) -> &'static TypeDescriptor {
        (self.target_type)()
    }

    /// Returns whether this is an inherent or trait implementation.
    ///
    /// # Returns
    ///
    /// Returns the implementation kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ImplKind {
        self.definition.kind()
    }

    /// Returns the concrete applied trait, or `None` for an inherent impl.
    ///
    /// # Returns
    ///
    /// Returns the concrete trait application, or `None` for an inherent impl.
    #[must_use]
    #[inline]
    pub const fn implemented_trait(&self) -> Option<&'static TraitDescriptor> {
        self.implemented_trait
    }

    /// Returns methods explicitly declared by this impl definition.
    ///
    /// # Returns
    ///
    /// Returns the impl's declared methods.
    #[must_use]
    #[inline]
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Returns methods explicitly declared by this impl definition.
    ///
    /// # Returns
    ///
    /// Returns the impl's declared methods.
    #[must_use]
    #[inline]
    pub const fn implementation_methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method explicitly declared by this concrete impl, by query
    /// name.
    ///
    /// # Parameters
    ///
    /// - `name`: Query name to match.
    ///
    /// # Returns
    ///
    /// Returns the matching declared method, or `None` when absent.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns concrete effective instances, including defaulted methods.
    ///
    /// # Returns
    ///
    /// Returns effective method instances in their stored order.
    #[must_use]
    #[inline]
    pub const fn method_instances(&self) -> &[MethodInstanceDescriptor] {
        &self.method_instances
    }

    /// Returns associated type bindings in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the concrete associated type bindings.
    #[must_use]
    #[inline]
    pub const fn associated_types(&self) -> &[AssociatedTypeBindingDescriptor] {
        &self.associated_types
    }

    /// Returns associated constant bindings in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the concrete associated constant bindings.
    #[must_use]
    #[inline]
    pub const fn associated_consts(&self) -> &[AssociatedConstBindingDescriptor] {
        &self.associated_consts
    }

    /// Returns concrete impl arguments in definition parameter order.
    ///
    /// # Returns
    ///
    /// Returns this application's generic arguments.
    #[must_use]
    #[inline]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Looks up one effective method across impl namespaces.
    ///
    /// # Parameters
    ///
    /// - `implementations`: Concrete implementations searched in the given
    ///   order.
    /// - `qualifier`: Namespace restriction applied to each implementation.
    /// - `name`: Method query name to match.
    ///
    /// # Returns
    ///
    /// Returns `Missing`, `Unique`, or `Ambiguous` according to the matches
    /// found.
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the local, address-free
    ///   representation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the formatter.
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
///
/// The builder validates generic arguments and member ownership before
/// producing an [`ImplDescriptor`].
#[derive(Debug)]
pub struct ImplDescriptorBuilder {
    /// Impl declaration being instantiated.
    definition: &'static ImplDefinitionDescriptor,
    /// Resolver for the concrete target type.
    target_type: TypeDescriptorResolver,
    /// Concrete trait namespace, when this is a trait impl.
    implemented_trait: Option<&'static TraitDescriptor>,
    /// Methods declared by the impl definition.
    methods: &'static [MethodDescriptor],
    /// Effective concrete method instances.
    method_instances: Vec<MethodInstanceDescriptor>,
    /// Concrete associated type bindings.
    associated_types: Vec<AssociatedTypeBindingDescriptor>,
    /// Concrete associated constant bindings.
    associated_consts: Vec<AssociatedConstBindingDescriptor>,
    /// Concrete generic arguments in definition order.
    arguments: Vec<GenericArgument>,
}

impl ImplDescriptorBuilder {
    /// Creates an empty concrete instance builder.
    ///
    /// # Parameters
    ///
    /// - `definition`: Impl declaration to instantiate.
    /// - `target_type`: Resolver for its concrete target type.
    ///
    /// # Returns
    ///
    /// Returns an empty builder for the impl application.
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
    ///
    /// # Parameters
    ///
    /// - `implemented_trait`: Concrete applied trait namespace.
    ///
    /// # Returns
    ///
    /// Returns the builder with the trait namespace set.
    #[must_use]
    pub fn implemented_trait(mut self, implemented_trait: &'static TraitDescriptor) -> Self {
        self.implemented_trait = Some(implemented_trait);
        self
    }

    /// Sets methods explicitly declared by the impl definition.
    ///
    /// # Parameters
    ///
    /// - `methods`: Methods declared by the source impl.
    ///
    /// # Returns
    ///
    /// Returns the builder with its declared methods set.
    #[must_use]
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets concrete effective method instances.
    ///
    /// # Parameters
    ///
    /// - `instances`: Effective method instances for this application.
    ///
    /// # Returns
    ///
    /// Returns the builder with its effective method instances set.
    #[must_use]
    pub fn method_instances(mut self, instances: Vec<MethodInstanceDescriptor>) -> Self {
        self.method_instances = instances;
        self
    }

    /// Sets associated type bindings in declaration order.
    ///
    /// # Parameters
    ///
    /// - `bindings`: Concrete associated type bindings.
    ///
    /// # Returns
    ///
    /// Returns the builder with its associated type bindings set.
    #[must_use]
    pub fn associated_types(mut self, bindings: Vec<AssociatedTypeBindingDescriptor>) -> Self {
        self.associated_types = bindings;
        self
    }

    /// Sets associated constant bindings in declaration order.
    ///
    /// # Parameters
    ///
    /// - `bindings`: Concrete associated constant bindings.
    ///
    /// # Returns
    ///
    /// Returns the builder with its associated constant bindings set.
    #[must_use]
    pub fn associated_consts(mut self, bindings: Vec<AssociatedConstBindingDescriptor>) -> Self {
        self.associated_consts = bindings;
        self
    }

    /// Sets concrete impl arguments in definition parameter order.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Concrete type and const arguments in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the builder with its generic arguments set.
    #[must_use]
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Validates and builds the concrete impl descriptor.
    ///
    /// Returns [`ImplDescriptorBuildError`] for inconsistent trait, generic,
    /// method, or associated-item relationships.
    ///
    /// # Returns
    ///
    /// Returns the validated concrete implementation descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`ImplDescriptorBuildError`] when implementation facts do not
    /// match their source definition or descriptor graph.
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
