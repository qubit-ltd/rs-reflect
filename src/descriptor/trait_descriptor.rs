// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Trait definitions, concrete applications, supertraits, and associated items.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::descriptor::AssociatedConstReader;
use crate::descriptor::InvocationAdapter;
use crate::descriptor::InvocationUnavailableReason;
use crate::descriptor::MethodDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::ConstExpression;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::GenericParameterDescriptor;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;
use crate::identity::ExternalTraitId;
use crate::identity::Visibility;

type AppliedTraitCache = HashMap<(TypeId, AppliedTraitId), Arc<OnceLock<TraitImplPayload>>>;
type DynTraitCache = HashMap<TypeId, &'static OnceLock<TraitDescriptor>>;

/// The process-local identity source of a reflected or external trait.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TraitId {
    /// A marker type generated for a reflected trait declaration.
    Reflected(TypeId),
    /// A stable, caller-supplied identity for an unreflected trait.
    External(ExternalTraitId),
}

/// The complete identity of one concrete trait application.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppliedTraitId {
    definition: TraitId,
    arguments: Box<[GenericArgument]>,
    associated_type_arguments: Box<[GenericArgument]>,
}

impl AppliedTraitId {
    /// Returns the reflected marker or external definition identity.
    #[must_use]
    #[inline(always)]
    pub const fn definition(&self) -> &TraitId {
        &self.definition
    }

    /// Returns concrete type and const arguments in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns concrete associated-type equalities in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_type_arguments(&self) -> &[GenericArgument] {
        &self.associated_type_arguments
    }
}

/// How much of a trait declaration is known to reflection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitCompleteness {
    /// The trait declaration, supertraits, and associated items are known.
    Complete,
    /// Only facts proven by an observed external trait impl are known.
    ExternalIncomplete,
}

/// Declaration-level facts shared by every concrete application of a trait.
#[derive(Debug)]
pub struct TraitDefinitionDescriptor {
    trait_id: TraitId,
    rust_name: &'static str,
    rust_path: &'static str,
    query_name: &'static str,
    completeness: TraitCompleteness,
    generic_definition: &'static GenericDefinitionDescriptor,
    visibility: Visibility,
    members: OnceLock<TraitDefinitionMembers>,
}

/// Associated-item facts retained before a concrete trait application exists.
#[derive(Debug)]
struct TraitDefinitionMembers {
    methods: Box<[MethodDescriptor]>,
    associated_types: Box<[AssociatedTypeDescriptor]>,
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

/// Concrete trait facts supplied by a reflected trait's hidden implementation
/// hook.
///
/// The hook carries declaration identity without imposing object-safety
/// requirements on the reflected trait. Implementation expansion enriches this
/// payload with concrete application details before it becomes part of an
/// implementation descriptor.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct TraitImplPayload {
    definition: &'static TraitDefinitionDescriptor,
    applied: &'static TraitDescriptor,
    default_method_adapters: &'static [Option<&'static InvocationAdapter>],
    default_method_unavailable_reasons: &'static [&'static [InvocationUnavailableReason]],
    associated_type_resolvers: &'static [Option<TypeDescriptorResolver>],
    associated_const_readers: &'static [Option<&'static AssociatedConstReader>],
}

impl TraitImplPayload {
    /// Creates a payload for one reflected trait declaration.
    #[doc(hidden)]
    pub const fn new(definition: &'static TraitDefinitionDescriptor, applied: &'static TraitDescriptor) -> Self {
        Self {
            definition,
            applied,
            default_method_adapters: &[],
            default_method_unavailable_reasons: &[],
            associated_type_resolvers: &[],
            associated_const_readers: &[],
        }
    }

    /// Returns the complete trait declaration shared by every implementation.
    #[doc(hidden)]
    #[must_use]
    pub const fn definition(self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the concrete applied trait descriptor for the hook receiver.
    #[doc(hidden)]
    #[must_use]
    pub const fn applied(self) -> &'static TraitDescriptor {
        self.applied
    }

    /// Returns concrete adapters for default methods in declaration order.
    #[doc(hidden)]
    #[must_use]
    pub const fn default_method_adapters(self) -> &'static [Option<&'static InvocationAdapter>] {
        self.default_method_adapters
    }

    /// Returns unavailable-reason sets for default methods in declaration
    /// order.
    #[doc(hidden)]
    #[must_use]
    pub const fn default_method_unavailable_reasons(self) -> &'static [&'static [InvocationUnavailableReason]] {
        self.default_method_unavailable_reasons
    }

    /// Returns proven concrete associated-type resolvers in declaration order.
    #[doc(hidden)]
    #[must_use]
    pub const fn associated_type_resolvers(self) -> &'static [Option<TypeDescriptorResolver>] {
        self.associated_type_resolvers
    }

    /// Returns safe associated-constant readers in declaration order.
    #[doc(hidden)]
    #[must_use]
    pub const fn associated_const_readers(self) -> &'static [Option<&'static AssociatedConstReader>] {
        self.associated_const_readers
    }

    /// Reuses an applied descriptor without constructing a discarded candidate
    /// on cache hits.
    #[doc(hidden)]
    pub fn cached_with_arguments<T: ?Sized + 'static>(
        definition: &'static TraitDefinitionDescriptor,
        arguments: Vec<GenericArgument>,
        build: impl FnOnce(Vec<GenericArgument>) -> Result<TraitDescriptor, TraitDescriptorBuildError>,
        build_default_method_adapters: impl FnOnce() -> Vec<Option<&'static InvocationAdapter>>,
        build_default_method_unavailable_reasons: impl FnOnce() -> Vec<&'static [InvocationUnavailableReason]>,
        build_associated_type_resolvers: impl FnOnce() -> Vec<Option<TypeDescriptorResolver>>,
        build_associated_const_readers: impl FnOnce() -> Vec<Option<&'static AssociatedConstReader>>,
    ) -> Self {
        static CACHE: LazyLock<Mutex<AppliedTraitCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));
        let identity = AppliedTraitId {
            definition: definition.trait_id().clone(),
            arguments: arguments.clone().into_boxed_slice(),
            associated_type_arguments: Box::new([]),
        };
        let key = (TypeId::of::<T>(), identity);
        let mut cache = CACHE.lock().expect("trait payload cache mutex must not be poisoned");
        let cell = cache.entry(key).or_insert_with(|| Arc::new(OnceLock::new())).clone();
        drop(cache);
        *cell.get_or_init(|| {
            let applied = Box::leak(Box::new(
                build(arguments).expect("a reflected trait must build a valid applied descriptor"),
            ));
            let default_method_adapters = Box::leak(build_default_method_adapters().into_boxed_slice());
            let default_method_unavailable_reasons =
                Box::leak(build_default_method_unavailable_reasons().into_boxed_slice());
            let associated_type_resolvers = Box::leak(build_associated_type_resolvers().into_boxed_slice());
            let associated_const_readers = Box::leak(build_associated_const_readers().into_boxed_slice());
            Self {
                definition,
                applied,
                default_method_adapters,
                default_method_unavailable_reasons,
                associated_type_resolvers,
                associated_const_readers,
            }
        })
    }
}

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

impl TraitDefinitionDescriptor {
    /// Returns whether two declarations can be merged for one external trait
    /// ID.
    pub(crate) fn is_compatible_with(&self, other: &Self) -> bool {
        self.completeness() == other.completeness() && self.generic_definition() == other.generic_definition()
    }

    /// Creates immutable trait definition facts.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        trait_id: TraitId,
        rust_name: &'static str,
        rust_path: &'static str,
        query_name: &'static str,
        completeness: TraitCompleteness,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Self {
        Self::new_with_visibility(
            trait_id,
            rust_name,
            rust_path,
            query_name,
            completeness,
            generic_definition,
            Visibility::Private,
        )
    }

    /// Creates immutable trait definition facts with normalized source
    /// visibility.
    #[doc(hidden)]
    #[must_use]
    pub const fn new_with_visibility(
        trait_id: TraitId,
        rust_name: &'static str,
        rust_path: &'static str,
        query_name: &'static str,
        completeness: TraitCompleteness,
        generic_definition: &'static GenericDefinitionDescriptor,
        visibility: Visibility,
    ) -> Self {
        Self {
            trait_id,
            rust_name,
            rust_path,
            query_name,
            completeness,
            generic_definition,
            visibility,
            members: OnceLock::new(),
        }
    }

    /// Returns the trait declaration's normalized source visibility.
    #[must_use]
    #[inline(always)]
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Returns the reflected marker or external trait identity.
    #[must_use]
    #[inline(always)]
    pub const fn trait_id(&self) -> &TraitId {
        &self.trait_id
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the diagnostic fully qualified Rust path.
    #[must_use]
    #[inline(always)]
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the lookup name, which may differ from the Rust name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns whether the complete declaration is known.
    #[must_use]
    #[inline(always)]
    pub const fn completeness(&self) -> TraitCompleteness {
        self.completeness
    }

    /// Returns generic parameters and predicates in source order.
    #[must_use]
    #[inline(always)]
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }

    /// Returns methods declared by this trait in source order.
    #[must_use]
    pub fn methods(&self) -> &[MethodDescriptor] {
        self.members.get().map_or(&[], |members| members.methods.as_ref())
    }

    /// Returns associated types declared by this trait in source order.
    #[must_use]
    #[inline(always)]
    pub fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        self.members
            .get()
            .map_or(&[], |members| members.associated_types.as_ref())
    }

    /// Returns associated constants declared by this trait in source order.
    #[must_use]
    #[inline(always)]
    pub fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        self.members
            .get()
            .map_or(&[], |members| members.associated_consts.as_ref())
    }

    /// Initializes declaration-level associated-item facts exactly once.
    #[doc(hidden)]
    pub fn initialize_members(
        &'static self,
        initialize: impl FnOnce(
            &'static Self,
        ) -> (
            Box<[MethodDescriptor]>,
            Box<[AssociatedTypeDescriptor]>,
            Box<[AssociatedConstDescriptor]>,
        ),
    ) {
        self.members.get_or_init(|| {
            let (methods, associated_types, associated_consts) = initialize(self);
            TraitDefinitionMembers {
                methods,
                associated_types,
                associated_consts,
            }
        });
    }
}

/// One associated type declaration.
#[derive(Clone, Debug)]
pub struct AssociatedTypeDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    bounds: Box<[PredicateDescriptor]>,
    default: Option<TypeExpression>,
    generic_definition: Option<Box<GenericDefinitionDescriptor>>,
}

impl AssociatedTypeDescriptor {
    /// Creates associated type facts in declaration order.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
            generic_definition: None,
        }
    }

    /// Creates associated type facts with GAT parameters and predicates.
    #[doc(hidden)]
    #[must_use]
    pub fn new_with_generic_definition(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
        generic_definition: GenericDefinitionDescriptor,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
            generic_definition: Some(Box::new(generic_definition)),
        }
    }

    /// Returns the source declaration index.
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns declared bounds in source order.
    #[must_use]
    #[inline(always)]
    pub const fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }

    /// Returns GAT parameters and where predicates in declaration order.
    #[must_use]
    #[inline(always)]
    pub fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        static EMPTY: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
            parameters: Box::new([]),
            predicates: Box::new([]),
            diagnostic: crate::expression::DiagnosticText::default(),
        });
        self.generic_definition.as_deref().unwrap_or(&EMPTY)
    }

    /// Returns the symbolic default type.
    ///
    /// `None` means the trait requires implementations to provide the binding.
    #[must_use]
    #[inline(always)]
    pub const fn default(&self) -> Option<&TypeExpression> {
        self.default.as_ref()
    }

    /// Applies one concrete trait application to this declaration.
    fn substituted(self, substitutions: &TraitApplicationSubstitutions) -> Self {
        Self {
            bounds: self
                .bounds
                .iter()
                .map(|predicate| substitutions.predicate(predicate))
                .collect(),
            default: self
                .default
                .as_ref()
                .map(|expression| substitutions.type_expression(expression)),
            generic_definition: self
                .generic_definition
                .as_ref()
                .map(|definition| Box::new(substitutions.generic_definition(definition))),
            ..self
        }
    }
}

/// One associated constant declaration.
#[derive(Clone, Debug)]
pub struct AssociatedConstDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    declared_type: TypeExpression,
    has_default: bool,
}

impl AssociatedConstDescriptor {
    /// Creates associated constant facts in declaration order.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        declared_type: TypeExpression,
        has_default: bool,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            declared_type,
            has_default,
        }
    }

    /// Returns the source declaration index.
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared constant type.
    #[must_use]
    #[inline(always)]
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }

    /// Returns whether the trait declaration provides a default value.
    #[must_use]
    #[inline(always)]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Applies one concrete trait application to this declaration.
    fn substituted(self, substitutions: &TraitApplicationSubstitutions) -> Self {
        Self {
            declared_type: substitutions.type_expression(&self.declared_type),
            ..self
        }
    }
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
    descriptors: &'a [TraitDescriptorRef],
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

/// An applied trait descriptor with concrete generic arguments.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// use qubit_reflect::{Reflect, TypeDescriptor};
/// #[cfg(feature = "derive")]
/// use qubit_reflect::{reflect, reflect_impl};
///
/// #[cfg(feature = "derive")]
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
///
/// #[cfg(feature = "derive")]
/// #[reflect(crate = qubit_reflect)]
/// trait Named {
///     fn name(&self) -> &'static str;
/// }
///
/// #[cfg(feature = "derive")]
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Named for Service {
///     fn name(&self) -> &'static str { "service" }
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let applied = TypeDescriptor::of::<Service>()
///     .impls()?
///     .iter()
///     .find_map(|implementation| implementation.implemented_trait())
///     .expect("reflected trait implementation");
/// assert_eq!(applied.definition().rust_name(), "Named");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
pub struct TraitDescriptor {
    definition: &'static TraitDefinitionDescriptor,
    trait_id: AppliedTraitId,
    arguments: Box<[GenericArgument]>,
    associated_type_arguments: Box<[GenericArgument]>,
    direct_supertraits: Box<[TraitDescriptorRef]>,
    all_supertraits: Box<[TraitDescriptorRef]>,
    methods: &'static [MethodDescriptor],
    associated_types: Box<[AssociatedTypeDescriptor]>,
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

impl TraitDescriptor {
    /// Starts an applied trait builder for `definition`.
    pub fn builder(definition: &'static TraitDefinitionDescriptor) -> TraitDescriptorBuilder {
        TraitDescriptorBuilder::new(definition)
    }

    /// Returns the declaration-level descriptor shared by every application.
    #[must_use]
    #[inline(always)]
    pub const fn definition(&self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected marker or external identity.
    #[must_use]
    #[inline(always)]
    pub const fn trait_id(&self) -> &AppliedTraitId {
        &self.trait_id
    }

    /// Returns concrete generic arguments in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns concrete associated-type equalities required by this applied
    /// trait object or application.
    #[must_use]
    #[inline(always)]
    pub const fn associated_type_arguments(&self) -> &[GenericArgument] {
        &self.associated_type_arguments
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.definition.rust_name()
    }

    /// Returns the diagnostic fully qualified Rust path.
    #[must_use]
    #[inline(always)]
    pub const fn rust_path(&self) -> &'static str {
        self.definition.rust_path()
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.definition.query_name()
    }

    /// Returns whether this descriptor contains a complete declaration.
    #[must_use]
    #[inline(always)]
    pub const fn completeness(&self) -> TraitCompleteness {
        self.definition.completeness()
    }

    /// Returns direct supertraits in source declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn direct_supertraits(&self) -> &[TraitDescriptorRef] {
        &self.direct_supertraits
    }

    /// Returns the sorted, duplicate-free, transitive supertrait closure.
    #[must_use]
    #[inline(always)]
    pub const fn all_supertraits(&self) -> SupertraitClosure<'_> {
        SupertraitClosure {
            descriptors: &self.all_supertraits,
        }
    }

    /// Returns method declarations in source order.
    #[must_use]
    #[inline(always)]
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method by query name.
    ///
    /// `None` means this applied trait has no method with the requested name.
    #[must_use]
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns associated type declarations in source order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        &self.associated_types
    }

    /// Finds an associated type by query name.
    ///
    /// `None` means no associated type has the requested name.
    #[must_use]
    pub fn associated_type(&self, name: &str) -> Option<&AssociatedTypeDescriptor> {
        self.associated_types.iter().find(|item| item.query_name() == name)
    }

    /// Returns associated constant declarations in source order.
    #[must_use]
    #[inline(always)]
    pub const fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        &self.associated_consts
    }

    /// Finds an associated constant by query name.
    ///
    /// `None` means no associated constant has the requested name.
    #[must_use]
    pub fn associated_const(&self, name: &str) -> Option<&AssociatedConstDescriptor> {
        self.associated_consts.iter().find(|item| item.query_name() == name)
    }

    /// Returns whether two descriptors are the same concrete trait application.
    #[must_use]
    pub fn same_application(&self, other: &Self) -> bool {
        self.trait_id == other.trait_id
    }
}

impl fmt::Debug for TraitDescriptor {
    /// Formats local facts without recursively expanding supertraits.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraitDescriptor")
            .field("definition", &self.definition)
            .field("arguments", &self.arguments)
            .field("direct_supertrait_count", &self.direct_supertraits.len())
            .field("all_supertrait_count", &self.all_supertraits.len())
            .field("method_count", &self.methods.len())
            .field("associated_type_count", &self.associated_types.len())
            .field("associated_const_count", &self.associated_consts.len())
            .finish()
    }
}

/// An invalid applied trait graph or incomplete external declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraitDescriptorBuildError {
    /// A supertrait resolves to the application currently being built.
    RecursiveSupertrait {
        /// Diagnostic Rust path of the recursive application.
        rust_path: &'static str,
    },
    /// An external incomplete trait attempted to claim unobservable facts.
    ExternalTraitHasUnprovenFacts,
    /// The number of concrete type/const arguments does not match the
    /// definition.
    GenericArgumentCount {
        /// Number of runtime identity arguments required by the definition.
        expected: usize,
        /// Number of arguments supplied by the applied descriptor.
        actual: usize,
    },
    /// An argument kind does not match its type or const parameter.
    GenericArgumentKind {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An argument still contains a symbolic type or const parameter.
    NonConcreteGenericArgument {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An associated-type argument is unknown, duplicated, or non-concrete.
    InvalidAssociatedTypeArgument,
    /// A method declaration belongs to another trait or an impl.
    ForeignMethod,
}

impl fmt::Display for TraitDescriptorBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecursiveSupertrait { rust_path } => {
                write!(formatter, "recursive supertrait application: {rust_path}")
            }
            Self::ExternalTraitHasUnprovenFacts => {
                formatter.write_str("an external incomplete trait cannot claim supertraits or associated items")
            }
            Self::GenericArgumentCount { expected, actual } => write!(
                formatter,
                "trait application requires {expected} concrete arguments but received {actual}"
            ),
            Self::GenericArgumentKind { index } => {
                write!(formatter, "trait argument {index} has the wrong generic kind")
            }
            Self::NonConcreteGenericArgument { index } => {
                write!(formatter, "trait argument {index} is not concrete")
            }
            Self::InvalidAssociatedTypeArgument => {
                formatter.write_str("an associated-type argument must name one declared item and have a concrete value")
            }
            Self::ForeignMethod => formatter.write_str("applied trait contains a foreign method declaration"),
        }
    }
}

impl std::error::Error for TraitDescriptorBuildError {}

/// Builds one applied trait and validates its supertrait closure.
#[derive(Debug)]
pub struct TraitDescriptorBuilder {
    definition: &'static TraitDefinitionDescriptor,
    arguments: Vec<GenericArgument>,
    associated_type_arguments: Vec<GenericArgument>,
    direct_supertraits: Vec<TraitDescriptorRef>,
    methods: &'static [MethodDescriptor],
    associated_types: Vec<AssociatedTypeDescriptor>,
    associated_consts: Vec<AssociatedConstDescriptor>,
}

impl TraitDescriptorBuilder {
    /// Creates an empty applied view for `definition`.
    fn new(definition: &'static TraitDefinitionDescriptor) -> Self {
        Self {
            definition,
            arguments: Vec::new(),
            associated_type_arguments: Vec::new(),
            direct_supertraits: Vec::new(),
            methods: &[],
            associated_types: Vec::new(),
            associated_consts: Vec::new(),
        }
    }

    /// Sets concrete generic arguments in declaration order.
    #[must_use]
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Sets concrete associated-type equalities in declaration order.
    #[must_use]
    pub fn associated_type_arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.associated_type_arguments = arguments;
        self
    }

    /// Sets direct supertraits in source declaration order.
    pub fn direct_supertraits<const N: usize>(mut self, direct_supertraits: [&'static TraitDescriptor; N]) -> Self {
        self.direct_supertraits = direct_supertraits.into_iter().map(TraitDescriptorRef::new).collect();
        self
    }

    /// Sets applied method declarations in source order.
    #[must_use]
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets applied associated types in source order.
    #[must_use]
    pub fn associated_types(mut self, associated_types: Vec<AssociatedTypeDescriptor>) -> Self {
        self.associated_types = associated_types;
        self
    }

    /// Sets applied associated constants in source order.
    #[must_use]
    pub fn associated_consts(mut self, associated_consts: Vec<AssociatedConstDescriptor>) -> Self {
        self.associated_consts = associated_consts;
        self
    }

    /// Validates and builds the applied trait descriptor.
    ///
    /// Returns [`TraitDescriptorBuildError`] for recursive supertraits or when
    /// an incomplete external trait claims supertraits or associated items.
    pub fn build(self) -> Result<TraitDescriptor, TraitDescriptorBuildError> {
        self.validate_arguments()?;
        self.validate_associated_type_arguments()?;
        if self.methods.iter().any(|method| {
            !method
                .declaring_trait()
                .is_some_and(|owner| std::ptr::eq(owner, self.definition))
        }) {
            return Err(TraitDescriptorBuildError::ForeignMethod);
        }
        if self.definition.completeness() == TraitCompleteness::ExternalIncomplete
            && (!self.direct_supertraits.is_empty()
                || !self.associated_types.is_empty()
                || !self.associated_consts.is_empty())
        {
            return Err(TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts);
        }

        let mut all_supertraits = Vec::new();
        for direct in &self.direct_supertraits {
            self.collect_supertrait(direct.descriptor(), &mut all_supertraits)?;
        }
        all_supertraits.sort_by(|left, right| {
            left.rust_path()
                .cmp(right.rust_path())
                .then_with(|| left.query_name().cmp(right.query_name()))
        });

        let trait_id = AppliedTraitId {
            definition: self.definition.trait_id().clone(),
            arguments: self.arguments.clone().into_boxed_slice(),
            associated_type_arguments: self.associated_type_arguments.clone().into_boxed_slice(),
        };
        let substitutions =
            TraitApplicationSubstitutions::new(self.definition, &self.arguments, &self.associated_type_arguments);
        let methods = if substitutions.is_empty()
            || !self
                .methods
                .iter()
                .any(|method| method.needs_trait_application_substitution(&substitutions))
        {
            self.methods
        } else {
            Box::leak(
                self.methods
                    .iter()
                    .map(|method| method.substituted_for_trait_application(&substitutions))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        };
        let associated_types = self
            .associated_types
            .into_iter()
            .map(|descriptor| descriptor.substituted(&substitutions))
            .collect::<Vec<_>>();
        let associated_consts = self
            .associated_consts
            .into_iter()
            .map(|descriptor| descriptor.substituted(&substitutions))
            .collect::<Vec<_>>();
        Ok(TraitDescriptor {
            definition: self.definition,
            trait_id,
            arguments: self.arguments.into_boxed_slice(),
            associated_type_arguments: self.associated_type_arguments.into_boxed_slice(),
            direct_supertraits: self.direct_supertraits.into_boxed_slice(),
            all_supertraits: all_supertraits.into_boxed_slice(),
            methods,
            associated_types: associated_types.into_boxed_slice(),
            associated_consts: associated_consts.into_boxed_slice(),
        })
    }

    /// Adds `candidate` and its direct ancestors while rejecting recursion and
    /// duplicate applied identities.
    fn collect_supertrait(
        &self,
        candidate: &'static TraitDescriptor,
        closure: &mut Vec<TraitDescriptorRef>,
    ) -> Result<(), TraitDescriptorBuildError> {
        if candidate.trait_id().definition() == self.definition.trait_id()
            && candidate.trait_id().arguments() == self.arguments
        {
            return Err(TraitDescriptorBuildError::RecursiveSupertrait {
                rust_path: self.definition.rust_path(),
            });
        }
        if closure.iter().any(|existing| existing.same_application(candidate)) {
            return Ok(());
        }
        closure.push(TraitDescriptorRef::new(candidate));
        for ancestor in candidate.direct_supertraits() {
            self.collect_supertrait(ancestor.descriptor(), closure)?;
        }
        Ok(())
    }

    /// Verifies every runtime identity parameter has one concrete argument of
    /// the matching generic kind.
    fn validate_arguments(&self) -> Result<(), TraitDescriptorBuildError> {
        if self.definition.completeness() == TraitCompleteness::ExternalIncomplete {
            for (index, argument) in self.arguments.iter().enumerate() {
                if !generic_argument_is_concrete(argument) {
                    return Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index });
                }
            }
            return Ok(());
        }
        let parameters: Vec<_> = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .collect();
        if parameters.len() != self.arguments.len() {
            return Err(TraitDescriptorBuildError::GenericArgumentCount {
                expected: parameters.len(),
                actual: self.arguments.len(),
            });
        }
        for (index, (parameter, argument)) in parameters.into_iter().zip(&self.arguments).enumerate() {
            let kind_matches = matches!(
                (parameter, argument),
                (GenericParameterDescriptor::Type { .. }, GenericArgument::Type(_))
                    | (GenericParameterDescriptor::Const { .. }, GenericArgument::Const(_))
            );
            if !kind_matches {
                return Err(TraitDescriptorBuildError::GenericArgumentKind { index });
            }
            if !generic_argument_is_concrete(argument) {
                return Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index });
            }
        }
        Ok(())
    }

    /// Verifies associated-type equalities are concrete, unique, and declared
    /// by this trait or one of its direct supertraits.
    fn validate_associated_type_arguments(&self) -> Result<(), TraitDescriptorBuildError> {
        let mut names = std::collections::HashSet::new();
        for argument in &self.associated_type_arguments {
            let GenericArgument::AssociatedType { name, value } = argument else {
                return Err(TraitDescriptorBuildError::InvalidAssociatedTypeArgument);
            };
            if !names.insert(name.as_ref())
                || !(self
                    .associated_types
                    .iter()
                    .any(|descriptor| descriptor.rust_name() == name.as_ref())
                    || self.direct_supertraits.iter().any(|supertrait| {
                        supertrait
                            .descriptor()
                            .associated_types()
                            .iter()
                            .any(|descriptor| descriptor.rust_name() == name.as_ref())
                    }))
                || !type_expression_is_concrete(value)
            {
                return Err(TraitDescriptorBuildError::InvalidAssociatedTypeArgument);
            }
        }
        Ok(())
    }
}

/// Concrete substitutions carried by one applied trait descriptor.
#[derive(Clone)]
pub(crate) struct TraitApplicationSubstitutions {
    types: HashMap<crate::expression::ExpressionName, TypeExpression>,
    consts: HashMap<crate::expression::ExpressionName, ConstExpression>,
    lifetimes: std::collections::HashSet<crate::expression::ExpressionName>,
    associated_types: HashMap<crate::expression::ExpressionName, TypeExpression>,
}

impl TraitApplicationSubstitutions {
    /// Builds substitutions from declaration-order runtime identity arguments.
    fn new(
        definition: &TraitDefinitionDescriptor,
        arguments: &[GenericArgument],
        associated_type_arguments: &[GenericArgument],
    ) -> Self {
        let mut types = HashMap::new();
        let mut consts = HashMap::new();
        let mut lifetimes = std::collections::HashSet::new();
        let mut arguments = arguments.iter();
        for parameter in definition.generic_definition().parameters.iter() {
            match parameter {
                GenericParameterDescriptor::Lifetime { name, .. } => {
                    lifetimes.insert(name.clone());
                }
                GenericParameterDescriptor::Type { name, .. } => {
                    if let Some(GenericArgument::Type(value)) = arguments.next() {
                        types.insert(name.clone(), value.clone());
                    }
                }
                GenericParameterDescriptor::Const { name, .. } => {
                    if let Some(GenericArgument::Const(value)) = arguments.next() {
                        consts.insert(name.clone(), value.value.clone());
                    }
                }
            }
        }
        let associated_types = associated_type_arguments
            .iter()
            .filter_map(|argument| match argument {
                GenericArgument::AssociatedType { name, value } => Some((name.clone(), value.as_ref().clone())),
                _ => None,
            })
            .collect();
        Self {
            types,
            consts,
            lifetimes,
            associated_types,
        }
    }

    /// Returns whether this application carries no substitutions.
    fn is_empty(&self) -> bool {
        self.types.is_empty() && self.consts.is_empty() && self.lifetimes.is_empty() && self.associated_types.is_empty()
    }

    /// Applies outer trait arguments inside a nested item generic definition
    /// while preserving names shadowed by the item's own parameters.
    fn generic_definition(&self, definition: &GenericDefinitionDescriptor) -> GenericDefinitionDescriptor {
        let mut scoped = self.clone();
        for parameter in &definition.parameters {
            match parameter {
                GenericParameterDescriptor::Lifetime { name, .. } => {
                    scoped.lifetimes.remove(name.as_str());
                }
                GenericParameterDescriptor::Type { name, .. } => {
                    scoped.types.remove(name.as_str());
                }
                GenericParameterDescriptor::Const { name, .. } => {
                    scoped.consts.remove(name.as_str());
                }
            }
        }
        let parameters = definition
            .parameters
            .iter()
            .map(|parameter| match parameter {
                GenericParameterDescriptor::Lifetime {
                    name,
                    bounds,
                    diagnostic,
                } => GenericParameterDescriptor::Lifetime {
                    name: name.clone(),
                    bounds: bounds.iter().map(|bound| scoped.lifetime(bound)).collect(),
                    diagnostic: diagnostic.clone(),
                },
                GenericParameterDescriptor::Type {
                    name,
                    bounds,
                    default,
                    diagnostic,
                } => GenericParameterDescriptor::Type {
                    name: name.clone(),
                    bounds: bounds.iter().map(|bound| scoped.predicate(bound)).collect(),
                    default: default.as_ref().map(|value| scoped.type_expression(value)),
                    diagnostic: diagnostic.clone(),
                },
                GenericParameterDescriptor::Const {
                    name,
                    ty,
                    default,
                    diagnostic,
                } => GenericParameterDescriptor::Const {
                    name: name.clone(),
                    ty: Box::new(scoped.type_expression(ty)),
                    default: default.as_ref().map(|value| scoped.const_expression(value)),
                    diagnostic: diagnostic.clone(),
                },
            })
            .collect();
        let predicates = definition
            .predicates
            .iter()
            .map(|predicate| scoped.predicate(predicate))
            .collect();
        GenericDefinitionDescriptor {
            parameters,
            predicates,
            diagnostic: definition.diagnostic.clone(),
        }
    }

    /// Substitutes one structural type expression recursively.
    pub(crate) fn type_expression(&self, expression: &TypeExpression) -> TypeExpression {
        match expression {
            TypeExpression::Parameter(name) => self
                .types
                .get(name.as_str())
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            TypeExpression::Concrete(concrete)
                if concrete.path.len() == 1 && self.types.contains_key(concrete.path[0].as_ref()) =>
            {
                self.types
                    .get(concrete.path[0].as_ref())
                    .expect("the guarded type substitution exists")
                    .clone()
            }
            TypeExpression::Concrete(concrete)
                if concrete.path.len() == 2
                    && concrete.path[0].as_ref() == "Self"
                    && self.associated_types.contains_key(concrete.path[1].as_ref()) =>
            {
                self.associated_types
                    .get(concrete.path[1].as_ref())
                    .expect("the guarded associated-type substitution exists")
                    .clone()
            }
            TypeExpression::Associated(associated)
                if matches!(associated.self_type.as_ref(), TypeExpression::SelfType)
                    && self.associated_types.contains_key(associated.item.as_str()) =>
            {
                self.associated_types[associated.item.as_str()].clone()
            }
            _ => {
                let mut result = expression.clone();
                match &mut result {
                    TypeExpression::Concrete(concrete) => {
                        concrete.arguments = concrete
                            .arguments
                            .iter()
                            .map(|argument| self.generic_argument(argument))
                            .collect();
                    }
                    TypeExpression::Associated(associated) => {
                        *associated.self_type = self.type_expression(&associated.self_type);
                        associated.trait_path = associated
                            .trait_path
                            .as_ref()
                            .map(|path| Box::new(self.type_expression(path)));
                        associated.arguments = associated
                            .arguments
                            .iter()
                            .map(|argument| self.generic_argument(argument))
                            .collect();
                    }
                    TypeExpression::Reference(reference) => {
                        reference.lifetime = self.lifetime(&reference.lifetime);
                        *reference.target = self.type_expression(&reference.target);
                    }
                    TypeExpression::RawPointer(pointer) => {
                        *pointer.target = self.type_expression(&pointer.target);
                    }
                    TypeExpression::Slice(element) => {
                        **element = self.type_expression(element);
                    }
                    TypeExpression::Array(array) => {
                        *array.element = self.type_expression(&array.element);
                        array.length = self.const_expression(&array.length);
                    }
                    TypeExpression::Tuple(elements) => {
                        *elements = elements.iter().map(|element| self.type_expression(element)).collect();
                    }
                    TypeExpression::FunctionPointer(function) => {
                        function.parameters = function
                            .parameters
                            .iter()
                            .map(|parameter| self.type_expression(parameter))
                            .collect();
                        *function.return_type = self.type_expression(&function.return_type);
                    }
                    TypeExpression::TraitObject(object) => {
                        object.bounds = object
                            .bounds
                            .iter()
                            .map(|predicate| self.predicate(predicate))
                            .collect();
                    }
                    TypeExpression::Opaque(opaque) => {
                        opaque.bounds = opaque
                            .bounds
                            .iter()
                            .map(|predicate| self.predicate(predicate))
                            .collect();
                    }
                    TypeExpression::Parameter(_) | TypeExpression::SelfType | TypeExpression::Never => {}
                }
                result
            }
        }
    }

    /// Substitutes one generic argument recursively.
    fn generic_argument(&self, argument: &GenericArgument) -> GenericArgument {
        match argument {
            GenericArgument::Type(value) => GenericArgument::Type(self.type_expression(value)),
            GenericArgument::Lifetime(value) => GenericArgument::Lifetime(self.lifetime(value)),
            GenericArgument::Const(value) => {
                let mut value = value.clone();
                value.declared_type = Box::new(self.type_expression(&value.declared_type));
                value.value = self.const_expression(&value.value);
                GenericArgument::Const(value)
            }
            GenericArgument::AssociatedType { name, value } => GenericArgument::AssociatedType {
                name: name.clone(),
                value: Box::new(self.type_expression(value)),
            },
            GenericArgument::AssociatedTypeBound { name, bounds } => GenericArgument::AssociatedTypeBound {
                name: name.clone(),
                bounds: bounds.iter().map(|predicate| self.predicate(predicate)).collect(),
            },
        }
    }

    /// Substitutes one const parameter reference.
    fn const_expression(&self, expression: &ConstExpression) -> ConstExpression {
        match expression {
            ConstExpression::Parameter(name) => self
                .consts
                .get(name.as_str())
                .cloned()
                .unwrap_or_else(|| expression.clone()),
            _ => expression.clone(),
        }
    }

    /// Maps declaration lifetimes to the only lifetime supported by a
    /// `'static` trait-object root.
    fn lifetime(&self, lifetime: &crate::expression::LifetimeExpression) -> crate::expression::LifetimeExpression {
        match lifetime {
            crate::expression::LifetimeExpression::Named(name) if self.lifetimes.contains(name.as_str()) => {
                crate::expression::LifetimeExpression::Static
            }
            _ => lifetime.clone(),
        }
    }

    /// Substitutes types nested in one predicate.
    pub(crate) fn predicate(&self, predicate: &PredicateDescriptor) -> PredicateDescriptor {
        let mut result = predicate.clone();
        match &mut result {
            PredicateDescriptor::TypeBound { subject, bounds, .. } => {
                *subject = self.type_expression(subject);
                *bounds = bounds.iter().map(|bound| self.type_expression(bound)).collect();
            }
            PredicateDescriptor::LifetimeOutlives { lifetime, bounds, .. } => {
                *lifetime = self.lifetime(lifetime);
                *bounds = bounds.iter().map(|bound| self.lifetime(bound)).collect();
            }
            PredicateDescriptor::TypeOutlives { ty, lifetime, .. } => {
                *ty = self.type_expression(ty);
                *lifetime = self.lifetime(lifetime);
            }
            PredicateDescriptor::TypeEquality { left, right, .. } => {
                *left = self.type_expression(left);
                *right = self.type_expression(right);
            }
        }
        result
    }
}

/// Returns whether an argument contains only concrete runtime identity facts.
pub(super) fn generic_argument_is_concrete(argument: &GenericArgument) -> bool {
    match argument {
        GenericArgument::Type(expression) => type_expression_is_concrete(expression),
        GenericArgument::Const(argument) => !matches!(argument.value, ConstExpression::Parameter(_)),
        GenericArgument::Lifetime(_) => true,
        GenericArgument::AssociatedType { value, .. } => type_expression_is_concrete(value),
        GenericArgument::AssociatedTypeBound { .. } => false,
    }
}

/// Returns whether a substituted type expression contains no symbolic type.
fn type_expression_is_concrete(expression: &TypeExpression) -> bool {
    match expression {
        TypeExpression::Concrete(concrete) => concrete.arguments.iter().all(generic_argument_is_concrete),
        TypeExpression::Reference(reference) => type_expression_is_concrete(&reference.target),
        TypeExpression::RawPointer(pointer) => type_expression_is_concrete(&pointer.target),
        TypeExpression::Slice(element) => type_expression_is_concrete(element),
        TypeExpression::Array(array) => {
            type_expression_is_concrete(&array.element) && !matches!(array.length, ConstExpression::Parameter(_))
        }
        TypeExpression::Tuple(elements) => elements.iter().all(type_expression_is_concrete),
        TypeExpression::FunctionPointer(function) => {
            function.parameters.iter().all(type_expression_is_concrete)
                && type_expression_is_concrete(&function.return_type)
        }
        TypeExpression::TraitObject(_) | TypeExpression::Never => true,
        TypeExpression::Parameter(_)
        | TypeExpression::SelfType
        | TypeExpression::Associated(_)
        | TypeExpression::Opaque(_) => false,
    }
}
