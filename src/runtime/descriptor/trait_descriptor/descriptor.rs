// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::metadata::AssociatedConstDefinitionMetadata;
use super::metadata::AssociatedTypeDefinitionMetadata;
use super::metadata::TraitApplicationMetadata;
use super::metadata::TraitDefinitionId;
use super::metadata::TraitDefinitionMetadata;
use super::metadata::TraitExpressionMetadata;
use super::metadata::Visibility;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::GenericArguments;
use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::method::MethodQualifiers;
use crate::runtime::descriptor::method::ParameterDescriptor;
use crate::runtime::descriptor::method::ReceiverDescriptor;
use crate::runtime::descriptor::method::ReceiverKind;
use crate::runtime::descriptor::method::ReturnDescriptor;
use crate::runtime::error::ContextError;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::expression::ConstExpression;
use crate::runtime::expression::Predicates;
use crate::runtime::expression::TypeExpression;
use crate::runtime::identity::IntoLookupCandidate;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;

/// Context-bound view of one source trait declaration.
#[derive(Clone, Copy)]
pub struct TraitDefinitionDescriptor<'ctx> {
    metadata: &'static TraitDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitDefinitionDescriptor<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(metadata: &'static TraitDefinitionMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context that owns this trait declaration.
    pub const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the process-local identity of this source trait declaration.
    pub const fn id(self) -> TraitDefinitionId {
        self.metadata.id
    }
    /// Returns the source name of this declaration.
    pub const fn name(self) -> &'static str {
        self.metadata.name
    }
    /// Returns the retained source path of this declaration.
    pub const fn path(self) -> &'static str {
        self.metadata.path
    }
    /// Returns source visibility retained for this declaration.
    pub const fn visibility(self) -> Visibility {
        self.metadata.visibility
    }
    /// Returns the completeness status of retained declaration metadata.
    pub const fn completeness(self) -> super::TraitCompleteness {
        self.metadata.completeness
    }
    /// Returns this declaration's generic parameters and predicates.
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
    /// Returns the declared supertrait expressions.
    pub fn supertraits(self) -> TraitExpressions<'ctx> {
        TraitExpressions::new(self.metadata.supertraits, self.context)
    }
    /// Returns the method declarations in source order.
    pub fn methods(self) -> MethodDefinitions<'ctx> {
        MethodDefinitions::new(self.metadata.methods, MethodDefinitionOwner::Trait(self), self.context)
    }
    /// Finds a uniquely named method declaration.
    pub fn method(self, name: &str) -> Result<MethodDefinitionDescriptor<'ctx>, LookupError> {
        unique_named(
            self.metadata
                .methods
                .iter()
                .filter(|method| method.name == name)
                .map(|method| {
                    MethodDefinitionDescriptor::new(method, MethodDefinitionOwner::Trait(self), self.context)
                }),
            LookupSubject::Method,
            name,
        )
    }
    /// Returns associated type declarations in source order.
    pub fn associated_types(self) -> AssociatedTypeDefinitions<'ctx> {
        AssociatedTypeDefinitions::new(self.metadata.associated_types, self.context)
    }
    /// Finds an associated type declaration by name.
    pub fn associated_type(self, name: &str) -> Result<AssociatedTypeDefinitionDescriptor<'ctx>, LookupError> {
        unique_named(
            self.metadata
                .associated_types
                .iter()
                .filter(|item| item.name == name)
                .map(|item| AssociatedTypeDefinitionDescriptor::new(item, self.context)),
            LookupSubject::AssociatedType,
            name,
        )
    }
    /// Returns associated constant declarations in source order.
    pub fn associated_consts(self) -> AssociatedConstDefinitions<'ctx> {
        AssociatedConstDefinitions::new(self.metadata.associated_consts, self.context)
    }
    /// Finds an associated constant declaration by name.
    pub fn associated_const(self, name: &str) -> Result<AssociatedConstDefinitionDescriptor<'ctx>, LookupError> {
        unique_named(
            self.metadata
                .associated_consts
                .iter()
                .filter(|item| item.name == name)
                .map(|item| AssociatedConstDefinitionDescriptor::new(item, self.context)),
            LookupSubject::AssociatedConst,
            name,
        )
    }
    /// Returns concrete applications linked to this declaration.
    pub fn applications(self) -> Result<Traits<'ctx>, ContextError> {
        self.context
            .trait_applications_for_definition(self.metadata.id)
            .map(|applications| Traits::new(applications, self.context))
    }
}

/// Context-bound view of a compiled, target-independent trait application.
#[derive(Clone, Copy)]
pub struct TraitDescriptor<'ctx> {
    metadata: &'static TraitApplicationMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitDescriptor<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(metadata: &'static TraitApplicationMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the reflection context associated with this descriptor.
    pub const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the linked trait definition, if known.
    pub fn definition(self) -> Option<TraitDefinitionDescriptor<'ctx>> {
        self.metadata
            .definition
            .map(|metadata| TraitDefinitionDescriptor::new(metadata, self.context))
    }
    /// Returns generic arguments retained for this trait application.
    pub fn arguments(self) -> GenericArguments<'ctx> {
        GenericArguments::new(self.metadata.arguments, self.context)
    }
    /// Returns the source name of this declaration.
    pub fn name(self) -> &'static str {
        self.find_definition().map_or_else(
            || self.path().rsplit("::").next().unwrap_or(self.path()).trim(),
            TraitDefinitionDescriptor::name,
        )
    }
    /// Returns the retained source path of this declaration.
    pub fn path(self) -> &'static str {
        self.metadata.expression.map_or_else(
            || self.metadata.definition.expect("linked application").path,
            |expr| expr.path,
        )
    }
    /// Returns the completeness status of retained declaration metadata.
    pub fn completeness(self) -> super::TraitCompleteness {
        self.find_definition().map_or(
            super::TraitCompleteness::ExternalIncomplete,
            TraitDefinitionDescriptor::completeness,
        )
    }
    /// Finds the linked trait definition, if known.
    pub fn find_definition(self) -> Option<TraitDefinitionDescriptor<'ctx>> {
        self.metadata
            .definition
            .map(|metadata| TraitDefinitionDescriptor::new(metadata, self.context))
    }
    /// Returns directly declared supertrait applications.
    pub fn direct_supertraits(self) -> Traits<'ctx> {
        Traits::from_metadata(self.metadata.direct_supertraits, self.context)
    }
    /// Returns all retained reachable supertrait applications.
    pub fn all_supertraits(self) -> Traits<'ctx> {
        Traits::from_metadata(self.metadata.all_supertraits, self.context)
    }
    /// Returns whether the retained supertrait closure is complete.
    pub const fn supertrait_completeness(self) -> super::SupertraitCompleteness {
        self.metadata.supertrait_completeness
    }
    /// Returns the `same_application` metadata retained for this item.
    pub(crate) fn same_application(self, other: Self) -> bool {
        if self.context.id() != other.context.id() {
            return false;
        }
        match (self.find_definition(), other.find_definition()) {
            (Some(left), Some(right)) => left.id() == right.id() && self.metadata.identity == other.metadata.identity,
            (None, None) => std::ptr::eq(self.metadata, other.metadata),
            _ => false,
        }
    }
}

/// A trait expression retained in source syntax.
#[derive(Clone, Copy)]
pub struct TraitExpressionDescriptor<'ctx> {
    metadata: &'static TraitExpressionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpressionDescriptor<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(metadata: &'static TraitExpressionMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the reflection context associated with this descriptor.
    pub const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Finds the linked trait definition, if known.
    pub fn find_definition(self) -> Option<TraitDefinitionDescriptor<'ctx>> {
        self.metadata
            .definition
            .map(|definition| TraitDefinitionDescriptor::new(definition, self.context))
    }
    /// Returns the completeness status of retained declaration metadata.
    pub fn completeness(self) -> super::TraitCompleteness {
        self.find_definition().map_or(
            super::TraitCompleteness::ExternalIncomplete,
            TraitDefinitionDescriptor::completeness,
        )
    }
    /// Returns the retained source path of this declaration.
    pub const fn path(self) -> &'static str {
        self.metadata.expression.path
    }
    /// Returns generic arguments retained for this trait application.
    pub fn arguments(self) -> crate::runtime::expression::GenericExpressionArguments<'ctx> {
        crate::runtime::expression::TraitExpression::new(self.metadata.expression, self.context).arguments()
    }
}

/// Context-bound method declaration and its source owner.
#[derive(Clone, Copy)]
pub struct MethodDefinitionDescriptor<'ctx> {
    metadata: &'static MethodMetadata,
    owner: MethodDefinitionOwner<'ctx>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> MethodDefinitionDescriptor<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(
        metadata: &'static MethodMetadata,
        owner: MethodDefinitionOwner<'ctx>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self {
            metadata,
            owner,
            context,
        }
    }
    /// Returns the reflection context associated with this descriptor.
    pub const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the source name of this declaration.
    pub const fn name(self) -> &'static str {
        self.metadata.name
    }
    /// Returns source visibility retained for this declaration.
    pub const fn visibility(self) -> Visibility {
        match self.metadata.visibility {
            crate::runtime::descriptor::method::MethodVisibility::Public => Visibility::Public,
            crate::runtime::descriptor::method::MethodVisibility::Restricted => Visibility::RestrictedUnknown,
            crate::runtime::descriptor::method::MethodVisibility::Private => Visibility::Private,
        }
    }
    /// Returns the trait or impl declaration that owns this method.
    pub const fn owner(self) -> MethodDefinitionOwner<'ctx> {
        self.owner
    }
    /// Returns receiver metadata, or `None` for an associated function.
    pub fn receiver(self) -> Option<ReceiverDefinitionDescriptor<'ctx>> {
        self.metadata.receiver.map(|receiver| ReceiverDefinitionDescriptor {
            receiver,
            context: self.context,
        })
    }
    /// Returns method parameters in declaration order.
    pub fn parameters(self) -> impl ExactSizeIterator<Item = ParameterDescriptor<'ctx>> + 'ctx {
        self.metadata
            .parameters
            .iter()
            .map(move |parameter| ParameterDescriptor::new(parameter, self.context))
    }
    /// Returns the declared output type metadata.
    pub fn output(self) -> ReturnDescriptor<'ctx> {
        ReturnDescriptor::new(self.metadata.output, self.context)
    }
    /// Returns qualifiers retained for this method.
    pub const fn qualifiers(self) -> MethodQualifiers {
        self.metadata.qualifiers
    }
    /// Returns whether the trait method declares a default body.
    pub const fn has_default_body(self) -> bool {
        self.metadata.default_body
    }
    /// Returns this declaration's generic parameters and predicates.
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
}

/// Receiver syntax retained for a trait method declaration.
///
/// Unlike a method implementation descriptor, this view describes the receiver
/// written in the trait definition. Its type is available only when the
/// declaration's receiver type can be resolved in the reflection context.
#[derive(Clone, Copy)]
pub struct ReceiverDefinitionDescriptor<'ctx> {
    receiver: ReceiverDescriptor,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ReceiverDefinitionDescriptor<'ctx> {
    /// Returns the receiver form declared by the method.
    pub const fn kind(self) -> ReceiverKind {
        self.receiver.kind
    }
    /// Returns the receiver type descriptor when it is resolvable.
    pub fn concrete_type(self) -> Option<crate::runtime::descriptor::TypeDescriptor<'ctx>> {
        self.receiver
            .type_metadata
            .map(|metadata| crate::runtime::descriptor::TypeDescriptor::new(metadata(), self.context))
    }
}

/// Source declaration that owns a reflected method definition.
#[derive(Clone, Copy)]
pub enum MethodDefinitionOwner<'ctx> {
    /// The method is declared by a trait definition.
    Trait(TraitDefinitionDescriptor<'ctx>),
    /// The method is declared by an inherent or trait impl block.
    Impl(crate::runtime::descriptor::impl_descriptor::ImplDefinitionDescriptor<'ctx>),
}

/// Ordered source method declarations for one trait or impl block.
pub struct MethodDefinitions<'ctx> {
    values: &'static [MethodMetadata],
    owner: MethodDefinitionOwner<'ctx>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> MethodDefinitions<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(
        values: &'static [MethodMetadata],
        owner: MethodDefinitionOwner<'ctx>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, owner, context }
    }
    /// Returns the reflection context associated with this descriptor.
    pub const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over the items in source or collection order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = MethodDefinitionDescriptor<'ctx>> + '_ {
        self.values
            .iter()
            .map(|method| MethodDefinitionDescriptor::new(method, self.owner, self.context))
    }
    /// Returns the number of items in this collection.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns whether this collection contains no items.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Finds an item by name or collection position.
    pub fn get(&self, name: &str) -> Option<MethodDefinitionDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|method| method.name == name)
            .map(|method| MethodDefinitionDescriptor::new(method, self.owner, self.context))
    }
}

/// A source-level direct supertrait expression list.
#[derive(Clone, Copy)]
pub struct TraitExpressions<'ctx> {
    expressions: &'static [TraitExpressionMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpressions<'ctx> {
    const fn new(expressions: &'static [TraitExpressionMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { expressions, context }
    }
    /// Iterates over the items in source or collection order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = TraitExpressionDescriptor<'ctx>> + 'ctx {
        self.expressions
            .iter()
            .map(move |metadata| TraitExpressionDescriptor::new(metadata, self.context))
    }
    /// Returns the number of items in this collection.
    pub fn len(self) -> usize {
        self.expressions.len()
    }
    /// Returns whether this collection contains no items.
    pub fn is_empty(self) -> bool {
        self.expressions.is_empty()
    }
}

/// An ordered set of concrete trait applications.
#[derive(Clone)]
pub struct Traits<'ctx> {
    values: Box<[TraitDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> Traits<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) fn new(values: Box<[TraitDescriptor<'ctx>]>, context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    fn from_metadata(values: &'static [&'static TraitApplicationMetadata], context: &'ctx ReflectionContext) -> Self {
        Self::new(
            values
                .iter()
                .map(|metadata| TraitDescriptor::new(metadata, context))
                .collect(),
            context,
        )
    }
    /// Returns the context that owns these trait applications.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over the items in source or collection order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = TraitDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    /// Returns the number of items in this collection.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns whether this collection contains no items.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Finds an item by name or collection position.
    pub fn get(&self, index: usize) -> Option<TraitDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
}

/// A definition of one associated type in a trait.
#[derive(Clone, Copy)]
pub struct AssociatedTypeDefinitionDescriptor<'ctx> {
    metadata: &'static AssociatedTypeDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedTypeDefinitionDescriptor<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(
        metadata: &'static AssociatedTypeDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    /// Returns the source name of this declaration.
    pub const fn name(self) -> &'static str {
        self.metadata.name
    }
    /// Returns source visibility retained for this declaration.
    pub const fn visibility(self) -> Visibility {
        self.metadata.visibility
    }
    /// Returns the declared bounds of this associated type.
    pub fn bounds(self) -> Predicates<'ctx> {
        Predicates::new(self.metadata.bounds, self.context)
    }
    /// Returns the default expression, if one was declared.
    pub fn default(self) -> Option<TypeExpression<'ctx>> {
        self.metadata
            .default
            .map(|data| TypeExpression::new(data, self.context))
    }
}

/// Ordered collection of associated type declarations from one trait.
///
/// Lookups return `None` when no declaration has the requested name or when
/// more than one declaration matches it.
pub struct AssociatedTypeDefinitions<'ctx> {
    values: &'static [AssociatedTypeDefinitionMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedTypeDefinitions<'ctx> {
    const fn new(values: &'static [AssociatedTypeDefinitionMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    /// Iterates over the items in source or collection order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = AssociatedTypeDefinitionDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedTypeDefinitionDescriptor::new(value, self.context))
    }
    /// Returns the number of items in this collection.
    pub fn len(self) -> usize {
        self.values.len()
    }
    /// Returns whether this collection contains no items.
    pub fn is_empty(self) -> bool {
        self.values.is_empty()
    }
    /// Finds an item by name or collection position.
    pub fn get(self, name: &str) -> Option<AssociatedTypeDefinitionDescriptor<'ctx>> {
        unique_named(
            self.values
                .iter()
                .filter(|value| value.name == name)
                .map(|value| AssociatedTypeDefinitionDescriptor::new(value, self.context)),
            LookupSubject::AssociatedType,
            name,
        )
        .ok()
    }
}

/// A definition of one associated constant in a trait.
#[derive(Clone, Copy)]
pub struct AssociatedConstDefinitionDescriptor<'ctx> {
    metadata: &'static AssociatedConstDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedConstDefinitionDescriptor<'ctx> {
    /// Returns the `new` metadata retained for this item.
    pub(crate) const fn new(
        metadata: &'static AssociatedConstDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    /// Returns the source name of this declaration.
    pub const fn name(self) -> &'static str {
        self.metadata.name
    }
    /// Returns source visibility retained for this declaration.
    pub const fn visibility(self) -> Visibility {
        self.metadata.visibility
    }
    /// Returns the declared type expression.
    pub fn ty(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.ty, self.context)
    }
    /// Returns the default expression, if one was declared.
    pub const fn default(self) -> Option<ConstExpression> {
        self.metadata.default
    }
}

/// Ordered collection of associated constant declarations from one trait.
///
/// Lookups return `None` when no declaration has the requested name or when
/// more than one declaration matches it.
pub struct AssociatedConstDefinitions<'ctx> {
    values: &'static [AssociatedConstDefinitionMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedConstDefinitions<'ctx> {
    const fn new(values: &'static [AssociatedConstDefinitionMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    /// Iterates over the items in source or collection order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = AssociatedConstDefinitionDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedConstDefinitionDescriptor::new(value, self.context))
    }
    /// Returns the number of items in this collection.
    pub fn len(self) -> usize {
        self.values.len()
    }
    /// Returns whether this collection contains no items.
    pub fn is_empty(self) -> bool {
        self.values.is_empty()
    }
    /// Finds an item by name or collection position.
    pub fn get(self, name: &str) -> Option<AssociatedConstDefinitionDescriptor<'ctx>> {
        unique_named(
            self.values
                .iter()
                .filter(|value| value.name == name)
                .map(|value| AssociatedConstDefinitionDescriptor::new(value, self.context)),
            LookupSubject::AssociatedConst,
            name,
        )
        .ok()
    }
}

fn unique_named<T>(mut matches: impl Iterator<Item = T>, subject: LookupSubject, name: &str) -> Result<T, LookupError>
where
    T: Copy + IntoLookupCandidate,
{
    let Some(first) = matches.next() else {
        return Err(LookupError::Missing {
            subject,
            query: LookupQuery::Name(name.into()),
        });
    };
    let Some(second) = matches.next() else {
        return Ok(first);
    };
    let candidates = std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(matches)
        .map(IntoLookupCandidate::into_lookup_candidate)
        .collect();
    Err(LookupError::Ambiguous {
        subject,
        query: LookupQuery::Name(name.into()),
        candidates,
    })
}

impl IntoLookupCandidate for MethodDefinitionDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.name().into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Method,
                stable_id: StableItemId(display_name.clone()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

impl IntoLookupCandidate for AssociatedTypeDefinitionDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.name().into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::AssociatedType,
                stable_id: StableItemId(display_name.clone()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

impl IntoLookupCandidate for AssociatedConstDefinitionDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.name().into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::AssociatedConst,
                stable_id: StableItemId(display_name.clone()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

#[cfg(test)]
mod external_identity_tests {
    use super::TraitDescriptor;
    use super::TraitExpressionDescriptor;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::context::handle::data::ContextData;
    use crate::runtime::context::handle::registration::Registration;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::GenericExpressionArgument;
    use crate::runtime::expression::TraitExpressionData;

    struct ExternalDisplay;

    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<ExternalDisplay>(
        "Display",
        "std :: fmt :: Display",
        Visibility::Public,
        TraitCompleteness::ExternalIncomplete,
        &GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static EXPRESSION: TraitExpressionData = TraitExpressionData::new("std :: fmt :: Display", &[], &[]);
    static UNRESOLVED: TraitExpressionMetadata = TraitExpressionMetadata::unresolved(&EXPRESSION);
    static UNREGISTERED_EXPRESSION: TraitExpressionData = TraitExpressionData::new("std :: fmt :: Debug", &[], &[]);
    static UNREGISTERED_APPLICATION: TraitApplicationMetadata =
        TraitApplicationMetadata::unresolved(&UNREGISTERED_EXPRESSION, &[], &[]);
    static U8: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    static GENERIC_ARGUMENTS: [GenericExpressionArgument; 1] = [GenericExpressionArgument::Type(&U8)];
    static GENERIC_EXPRESSION: TraitExpressionData =
        TraitExpressionData::new("vendor::api::External", &GENERIC_ARGUMENTS, &[]);
    static GENERIC_EXPRESSION_METADATA: TraitExpressionMetadata =
        TraitExpressionMetadata::unresolved(&GENERIC_EXPRESSION);

    #[test]
    fn unresolved_impl_trait_expression_does_not_resolve_from_matching_path() {
        let registration = Registration::metadata_only().with_trait_definition(&DEFINITION);
        let context = ReflectionContext::from_data(
            ContextData::build(vec![registration]).expect("valid external trait registration"),
        );
        let expression = TraitExpressionDescriptor::new(&UNRESOLVED, &context);

        assert!(expression.find_definition().is_none());
        assert_eq!(expression.path(), "std :: fmt :: Display");

        let raw_expression = crate::runtime::expression::TraitExpression::new(&EXPRESSION, &context);
        assert!(raw_expression.find_definition().is_none());
    }

    #[test]
    fn unregistered_external_application_remains_incomplete_with_its_source_path() {
        let context = ReflectionContext::from_data(ContextData::empty());
        let application = TraitDescriptor::new(&UNREGISTERED_APPLICATION, &context);

        assert_eq!(application.path(), "std :: fmt :: Debug");
        assert_eq!(application.name(), "Debug");
        assert_eq!(application.completeness(), TraitCompleteness::ExternalIncomplete);
        assert!(application.find_definition().is_none());
    }

    #[test]
    fn the_same_unresolved_application_record_has_stable_identity() {
        let context = ReflectionContext::from_data(ContextData::empty());
        let application = TraitDescriptor::new(&UNREGISTERED_APPLICATION, &context);
        let same_record = TraitDescriptor::new(&UNREGISTERED_APPLICATION, &context);

        assert!(application.same_application(same_record));
    }

    #[test]
    fn unresolved_trait_expression_exposes_its_source_arguments() {
        let context = ReflectionContext::from_data(ContextData::empty());
        let expression = TraitExpressionDescriptor::new(&GENERIC_EXPRESSION_METADATA, &context);

        assert_eq!(expression.path(), "vendor::api::External");
        assert!(expression.find_definition().is_none());
        assert_eq!(expression.arguments().len(), 1);
        assert!(!expression.arguments().is_empty());
        assert!(expression.arguments().get(1).is_none());
        assert_eq!(expression.arguments().iter().count(), 1);
    }
}

#[cfg(test)]
mod metadata_boundary_tests {
    use super::TraitDefinitionDescriptor;
    use super::TraitDescriptor;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::context::handle::data::ContextData;
    use crate::runtime::context::handle::registration::Registration;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::expression::TraitExpressionData;

    struct ParentMarker;
    struct ChildMarker;

    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static PARENT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<ParentMarker>(
        "Parent",
        "crate :: api :: Parent",
        Visibility::Public,
        TraitCompleteness::Complete,
        &GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static CHILD_PARENT_EXPRESSION: TraitExpressionData = TraitExpressionData::new("crate :: api :: Parent", &[], &[]);
    static CHILD_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<ChildMarker>(
        "Child",
        "crate :: api :: Child",
        Visibility::Restricted("crate :: api"),
        TraitCompleteness::Complete,
        &GENERICS,
        &[TraitExpressionMetadata::for_definition(
            &CHILD_PARENT_EXPRESSION,
            &PARENT_DEFINITION,
        )],
        &[],
        &[],
        &[],
    );
    static CHILD_APPLICATION_EXPRESSION: TraitExpressionData = TraitExpressionData::new("api_alias :: Child", &[], &[]);
    static ALTERNATE_CHILD_APPLICATION_EXPRESSION: TraitExpressionData =
        TraitExpressionData::new("crate :: api :: Child", &[], &[]);
    static PARENT_APPLICATION: TraitApplicationMetadata =
        TraitApplicationMetadata::new(&PARENT_DEFINITION, &[], &[], &[], &[], SupertraitCompleteness::Complete);
    static CHILD_APPLICATION: TraitApplicationMetadata = TraitApplicationMetadata::linked(
        &CHILD_DEFINITION,
        &CHILD_APPLICATION_EXPRESSION,
        &[],
        &[],
        &[&PARENT_APPLICATION],
        &[&PARENT_APPLICATION],
        SupertraitCompleteness::Incomplete,
    );
    static ALTERNATE_CHILD_APPLICATION: TraitApplicationMetadata = TraitApplicationMetadata::linked(
        &CHILD_DEFINITION,
        &ALTERNATE_CHILD_APPLICATION_EXPRESSION,
        &[],
        &[],
        &[&PARENT_APPLICATION],
        &[&PARENT_APPLICATION],
        SupertraitCompleteness::Incomplete,
    );
    static EXTERNAL_EXPRESSION: TraitExpressionData = TraitExpressionData::new("vendor::api::External", &[], &[]);
    static EXTERNAL_APPLICATION: TraitApplicationMetadata =
        TraitApplicationMetadata::unresolved(&EXTERNAL_EXPRESSION, &[], &[]);

    fn context_with_trait_metadata() -> ReflectionContext {
        let registration = Registration::metadata_only()
            .with_trait_definition(&PARENT_DEFINITION)
            .with_trait_definition(&CHILD_DEFINITION)
            .with_trait_application(&CHILD_APPLICATION)
            .with_trait_application(&EXTERNAL_APPLICATION);
        ReflectionContext::from_data(
            ContextData::build(vec![registration]).expect("trait metadata registration is valid"),
        )
    }

    #[test]
    fn trait_definition_retains_source_metadata_and_declared_supertrait() {
        let context = context_with_trait_metadata();
        let definition = TraitDefinitionDescriptor::new(&CHILD_DEFINITION, &context);

        assert_eq!(definition.name(), "Child");
        assert_eq!(definition.path(), "crate :: api :: Child");
        assert_eq!(definition.visibility(), Visibility::Restricted("crate :: api"));
        assert_eq!(definition.completeness(), TraitCompleteness::Complete);
        let supertraits = definition.supertraits();
        assert_eq!(supertraits.len(), 1);
        let parent = supertraits.iter().next().expect("the declared parent is retained");
        assert_eq!(parent.path(), "crate :: api :: Parent");
        assert_eq!(parent.completeness(), TraitCompleteness::Complete);
        assert_eq!(parent.find_definition().expect("parent is linked").name(), "Parent");
        assert_eq!(supertraits.iter().count(), 1);
    }

    #[test]
    fn trait_definition_applications_query_is_scoped_by_definition_identity() {
        let context = context_with_trait_metadata();
        let child = TraitDefinitionDescriptor::new(&CHILD_DEFINITION, &context);
        let parent = TraitDefinitionDescriptor::new(&PARENT_DEFINITION, &context);

        let applications = child.applications().expect("child application query succeeds");
        assert_eq!(applications.len(), 1);
        assert_eq!(
            applications.get(0).expect("one child application").path(),
            "api_alias :: Child"
        );
        assert!(applications.get(1).is_none());
        assert!(
            parent
                .applications()
                .expect("parent application query succeeds")
                .is_empty()
        );
    }

    #[test]
    fn linked_application_preserves_source_path_and_supertrait_completeness() {
        let context = context_with_trait_metadata();
        let application = TraitDescriptor::new(&CHILD_APPLICATION, &context);

        assert_eq!(application.name(), "Child");
        assert_eq!(application.path(), "api_alias :: Child");
        assert_eq!(application.completeness(), TraitCompleteness::Complete);
        assert!(application.find_definition().is_some());
        assert!(application.arguments().is_empty());
        assert_eq!(application.direct_supertraits().len(), 1);
        assert_eq!(application.all_supertraits().len(), 1);
        assert_eq!(
            application.supertrait_completeness(),
            SupertraitCompleteness::Incomplete,
        );
        assert_eq!(
            application
                .direct_supertraits()
                .get(0)
                .expect("direct parent exists")
                .name(),
            "Parent",
        );
    }

    #[test]
    fn linked_application_identity_uses_definition_and_arguments_within_context() {
        let context = context_with_trait_metadata();
        let application = TraitDescriptor::new(&CHILD_APPLICATION, &context);
        let same_application_with_another_source_path = TraitDescriptor::new(&ALTERNATE_CHILD_APPLICATION, &context);
        let different_definition = TraitDescriptor::new(&PARENT_APPLICATION, &context);
        let other_context = ReflectionContext::from_data(ContextData::empty());
        let application_in_other_context = TraitDescriptor::new(&CHILD_APPLICATION, &other_context);

        assert!(application.same_application(application));
        assert!(application.same_application(same_application_with_another_source_path));
        assert!(!application.same_application(different_definition));
        assert!(!application.same_application(application_in_other_context));
    }

    #[test]
    fn unresolved_application_stays_external_incomplete_at_lookup_boundaries() {
        let context = context_with_trait_metadata();
        let application = TraitDescriptor::new(&EXTERNAL_APPLICATION, &context);

        assert_eq!(application.name(), "External");
        assert_eq!(application.path(), "vendor::api::External");
        assert_eq!(application.completeness(), TraitCompleteness::ExternalIncomplete);
        assert!(application.find_definition().is_none());
        assert_eq!(application.arguments().len(), 0);
        assert!(application.direct_supertraits().is_empty());
        assert!(application.all_supertraits().is_empty());
        assert_eq!(
            application.supertrait_completeness(),
            SupertraitCompleteness::Incomplete,
        );
    }
}

#[cfg(test)]
mod declaration_lookup_tests {
    use super::MethodDefinitionOwner;
    use super::TraitDefinitionDescriptor;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::context::handle::data::ContextData;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::method::FunctionAbi;
    use crate::runtime::descriptor::method::MethodMetadata;
    use crate::runtime::descriptor::method::MethodMetadataSpec;
    use crate::runtime::descriptor::method::MethodQualifiers;
    use crate::runtime::descriptor::method::MethodVisibility;
    use crate::runtime::descriptor::method::ParameterMetadata;
    use crate::runtime::descriptor::method::ParameterPattern;
    use crate::runtime::descriptor::method::PassingMode;
    use crate::runtime::descriptor::method::ReceiverDescriptor;
    use crate::runtime::descriptor::method::ReceiverKind;
    use crate::runtime::descriptor::method::ReturnKind;
    use crate::runtime::descriptor::method::ReturnMetadata;
    use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::error::LookupError;
    use crate::runtime::error::LookupQuery;
    use crate::runtime::error::LookupSubject;
    use crate::runtime::expression::ConstExpression;
    use crate::runtime::expression::ExpressionData;

    struct Declaration;

    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static METHODS: [MethodMetadata; 3] = [
        MethodMetadata::inherent("first", "Trait"),
        MethodMetadata::inherent("duplicate", "Trait"),
        MethodMetadata::inherent("duplicate", "Trait"),
    ];
    static U8: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    static USIZE: ExpressionData = ExpressionData::Named {
        path: "usize",
        definition: None,
        arguments: &[],
    };
    static DECLARED_RECEIVER: ExpressionData = ExpressionData::SelfType;
    static DECLARED_PARAMETERS: [ParameterMetadata; 2] = [
        ParameterMetadata::new(0, ParameterPattern::Identifier("value"), PassingMode::Shared, &U8),
        ParameterMetadata::new(1, ParameterPattern::Wildcard, PassingMode::Owned, &USIZE),
    ];
    static DIAGNOSTIC_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "inspect",
        crate::runtime::descriptor::method::MethodNamespaceMetadata::Trait {
            trait_: "crate::Example",
            implementation: "trait declaration",
        },
        MethodVisibility::Restricted,
        Some(ReceiverDescriptor::trait_self(ReceiverKind::Shared, &DECLARED_RECEIVER)),
        &DECLARED_PARAMETERS,
        &ReturnMetadata::new(ReturnKind::Shared, &U8),
        MethodQualifiers::new(false, true, true, true).with_abi(Some(FunctionAbi::C)),
        &GENERICS,
        false,
        true,
    ));
    static ASSOCIATED_TYPES: [AssociatedTypeDefinitionMetadata; 3] = [
        AssociatedTypeDefinitionMetadata::new("Item", Visibility::Public, &[], Some(&U8)),
        AssociatedTypeDefinitionMetadata::new("Hidden", Visibility::Private, &[], None),
        AssociatedTypeDefinitionMetadata::new("Item", Visibility::Public, &[], None),
    ];
    static ASSOCIATED_CONSTS: [AssociatedConstDefinitionMetadata; 1] = [AssociatedConstDefinitionMetadata::new(
        "COUNT",
        Visibility::Public,
        &USIZE,
        Some(ConstExpression::UnsignedInteger(4)),
    )];
    static DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<Declaration>(
        "Example",
        "crate::Example",
        Visibility::Public,
        TraitCompleteness::Complete,
        &GENERICS,
        &[],
        &METHODS,
        &ASSOCIATED_TYPES,
        &ASSOCIATED_CONSTS,
    );

    #[test]
    fn method_lookup_reports_missing_and_ambiguous_declarations() {
        let context = ReflectionContext::from_data(ContextData::empty());
        let declaration = TraitDefinitionDescriptor::new(&DEFINITION, &context);

        assert!(matches!(
            declaration.method("absent"),
            Err(LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name),
            }) if name.as_ref() == "absent"
        ));
        let error = match declaration.method("duplicate") {
            Err(error) => error,
            Ok(_) => panic!("duplicate source declarations should be ambiguous"),
        };
        match error {
            LookupError::Ambiguous {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name),
                candidates,
            } => {
                assert_eq!(name.as_ref(), "duplicate");
                assert_eq!(candidates.len(), 2);
                assert!(
                    candidates
                        .iter()
                        .all(|candidate| candidate.display_name.as_ref() == "duplicate")
                );
            }
            other => panic!("unexpected lookup error: {other:?}"),
        }
    }

    #[test]
    fn declaration_collections_expose_source_metadata_and_duplicate_policy() {
        let context = ReflectionContext::from_data(ContextData::empty());
        let declaration = TraitDefinitionDescriptor::new(&DEFINITION, &context);
        let methods = declaration.methods();
        assert_eq!(methods.len(), 3);
        assert!(!methods.is_empty());
        assert_eq!(
            methods.iter().map(|method| method.name()).collect::<Vec<_>>(),
            ["first", "duplicate", "duplicate"]
        );
        let first = methods.get("first").expect("named method is present");
        assert!(matches!(first.owner(), MethodDefinitionOwner::Trait(owner) if owner.id() == declaration.id()));
        assert_eq!(first.visibility(), Visibility::Public);
        assert!(first.receiver().is_none());
        assert!(!first.has_default_body());
        assert!(methods.get("absent").is_none());

        assert_eq!(declaration.associated_types().len(), 3);
        assert_eq!(
            declaration
                .associated_types()
                .iter()
                .map(|item| item.name())
                .collect::<Vec<_>>(),
            ["Item", "Hidden", "Item"]
        );
        assert!(!declaration.associated_types().is_empty());
        assert!(
            declaration.associated_types().get("Item").is_none(),
            "ambiguous associated type lookup yields None"
        );
        let hidden = declaration
            .associated_types()
            .get("Hidden")
            .expect("unique associated type is present");
        assert_eq!(hidden.visibility(), Visibility::Private);
        assert!(hidden.default().is_none());
        let item = declaration.associated_type("Item");
        assert!(matches!(
            item,
            Err(LookupError::Ambiguous {
                subject: LookupSubject::AssociatedType,
                query: LookupQuery::Name(name),
                candidates,
            }) if name.as_ref() == "Item" && candidates.len() == 2
        ));

        assert_eq!(declaration.associated_consts().len(), 1);
        assert!(!declaration.associated_consts().is_empty());
        let count = declaration
            .associated_const("COUNT")
            .expect("associated constant lookup succeeds");
        assert_eq!(count.visibility(), Visibility::Public);
        assert_eq!(count.default(), Some(ConstExpression::UnsignedInteger(4)));
        assert_eq!(count.ty().display().to_string(), "usize");
        assert!(declaration.associated_const("MISSING").is_err());

        let defaulted_item = declaration.associated_types().get("Item");
        assert!(
            defaulted_item.is_none(),
            "duplicate associated type names are ambiguous"
        );
        let default_expression = declaration
            .associated_types()
            .iter()
            .find(|item| item.name() == "Item")
            .and_then(|item| item.default())
            .expect("the first Item declaration retains its default type");
        assert_eq!(default_expression.display().to_string(), "u8");
    }

    #[test]
    fn method_declaration_exposes_receiver_parameters_qualifiers_and_output() {
        let context = ReflectionContext::from_data(ContextData::empty());
        let method = super::MethodDefinitionDescriptor::new(
            &DIAGNOSTIC_METHOD,
            MethodDefinitionOwner::Trait(TraitDefinitionDescriptor::new(&DEFINITION, &context)),
            &context,
        );

        assert_eq!(method.visibility(), Visibility::RestrictedUnknown);
        assert!(method.has_default_body());
        assert!(method.generics().is_empty());
        let receiver = method.receiver().expect("receiver metadata is present");
        assert_eq!(receiver.kind(), ReceiverKind::Shared);
        assert!(receiver.concrete_type().is_none());

        let parameters = method.parameters().collect::<Vec<_>>();
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].index(), 0);
        assert_eq!(parameters[0].name(), Some("value"));
        assert_eq!(parameters[0].pattern(), ParameterPattern::Identifier("value"));
        assert_eq!(parameters[0].passing(), PassingMode::Shared);
        assert_eq!(parameters[0].type_expression().display().to_string(), "u8");
        assert!(parameters[0].find_concrete_type().is_none());
        assert_eq!(parameters[1].index(), 1);
        assert_eq!(parameters[1].name(), None);
        assert_eq!(parameters[1].pattern(), ParameterPattern::Wildcard);
        assert_eq!(parameters[1].passing(), PassingMode::Owned);

        let qualifiers = method.qualifiers();
        assert!(!qualifiers.is_const);
        assert!(qualifiers.is_async);
        assert!(qualifiers.is_unsafe);
        assert!(qualifiers.is_variadic);
        assert_eq!(qualifiers.abi, Some(FunctionAbi::C));
        let output = method.output();
        assert_eq!(output.kind(), ReturnKind::Shared);
        assert_eq!(
            output
                .type_expression()
                .expect("non-unit return retains its source type")
                .display()
                .to_string(),
            "u8"
        );
        assert!(output.find_concrete_type().is_none());
    }
}
