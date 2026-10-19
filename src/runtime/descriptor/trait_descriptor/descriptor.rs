use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::{GenericArguments, GenericDefinitionDescriptor};
use crate::runtime::descriptor::method::{
    MethodMetadata, MethodQualifiers, ParameterDescriptor, ReceiverDescriptor, ReceiverKind,
    ReturnDescriptor,
};
use crate::runtime::error::{ContextError, LookupError, LookupQuery, LookupSubject};
use crate::runtime::expression::{ConstExpression, Predicates, TypeExpression};
use crate::runtime::identity::{
    IntoLookupCandidate, ItemIdentity, ItemKind, LookupCandidate, StableItemId,
};

use super::metadata::{
    AssociatedConstDefinitionMetadata, AssociatedTypeDefinitionMetadata, TraitApplicationMetadata,
    TraitDefinitionId, TraitDefinitionMetadata, TraitExpressionMetadata, Visibility,
};

/// Context-bound view of one source trait declaration.
#[derive(Clone, Copy)]
pub(crate) struct TraitDefinitionDescriptor<'ctx> {
    metadata: &'static TraitDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static TraitDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) const fn id(self) -> TraitDefinitionId {
        self.metadata.id
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.name
    }
    pub(crate) const fn path(self) -> &'static str {
        self.metadata.path
    }
    pub(crate) const fn visibility(self) -> Visibility {
        self.metadata.visibility
    }
    pub(crate) const fn completeness(self) -> super::TraitCompleteness {
        self.metadata.completeness
    }
    pub(crate) fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
    pub(crate) fn supertraits(self) -> TraitExpressions<'ctx> {
        TraitExpressions::new(self.metadata.supertraits, self.context)
    }
    pub(crate) fn methods(self) -> MethodDefinitions<'ctx> {
        MethodDefinitions::new(
            self.metadata.methods,
            MethodDefinitionOwner::Trait(self),
            self.context,
        )
    }
    pub(crate) fn method(
        self,
        name: &str,
    ) -> Result<MethodDefinitionDescriptor<'ctx>, LookupError> {
        unique_named(
            self.metadata
                .methods
                .iter()
                .filter(|method| method.name == name)
                .map(|method| {
                    MethodDefinitionDescriptor::new(
                        method,
                        MethodDefinitionOwner::Trait(self),
                        self.context,
                    )
                }),
            LookupSubject::Method,
            name,
        )
    }
    pub(crate) fn associated_types(self) -> AssociatedTypeDefinitions<'ctx> {
        AssociatedTypeDefinitions::new(self.metadata.associated_types, self.context)
    }
    pub(crate) fn associated_type(
        self,
        name: &str,
    ) -> Result<AssociatedTypeDefinitionDescriptor<'ctx>, LookupError> {
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
    pub(crate) fn associated_consts(self) -> AssociatedConstDefinitions<'ctx> {
        AssociatedConstDefinitions::new(self.metadata.associated_consts, self.context)
    }
    pub(crate) fn associated_const(
        self,
        name: &str,
    ) -> Result<AssociatedConstDefinitionDescriptor<'ctx>, LookupError> {
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
    pub(crate) fn applications(self) -> Result<Traits<'ctx>, ContextError> {
        self.context
            .trait_applications(self.metadata.id)
            .map(|applications| Traits::new(applications, self.context))
    }
}

/// Context-bound view of a compiled, target-independent trait application.
#[derive(Clone, Copy)]
pub(crate) struct TraitDescriptor<'ctx> {
    metadata: &'static TraitApplicationMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static TraitApplicationMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn definition(self) -> TraitDefinitionDescriptor<'ctx> {
        TraitDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    pub(crate) fn arguments(self) -> GenericArguments<'ctx> {
        GenericArguments::new(self.metadata.arguments, self.context)
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.definition.name
    }
    pub(crate) const fn path(self) -> &'static str {
        self.metadata.definition.path
    }
    pub(crate) fn direct_supertraits(self) -> Traits<'ctx> {
        Traits::from_metadata(self.metadata.direct_supertraits, self.context)
    }
    pub(crate) fn all_supertraits(self) -> Traits<'ctx> {
        Traits::from_metadata(self.metadata.all_supertraits, self.context)
    }
    pub(crate) const fn supertrait_completeness(self) -> super::SupertraitCompleteness {
        self.metadata.supertrait_completeness
    }
    pub(crate) fn same_application(self, other: Self) -> bool {
        self.context.id() == other.context.id()
            && self.definition().id() == other.definition().id()
            && self.metadata.identity == other.metadata.identity
    }
}

/// A trait expression retained in source syntax.
#[derive(Clone, Copy)]
pub(crate) struct TraitExpressionDescriptor<'ctx> {
    metadata: &'static TraitExpressionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpressionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static TraitExpressionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn find_definition(self) -> Option<TraitDefinitionDescriptor<'ctx>> {
        self.metadata
            .definition
            .map(|definition| TraitDefinitionDescriptor::new(definition, self.context))
    }
    pub(crate) const fn path(self) -> &'static str {
        self.metadata.expression.path
    }
    pub(crate) fn arguments(
        self,
    ) -> Box<[crate::runtime::expression::GenericExpressionArgumentView<'ctx>]> {
        crate::runtime::expression::TraitExpression::new(self.metadata.expression, self.context)
            .arguments()
    }
}

/// Context-bound method declaration and its source owner.
#[derive(Clone, Copy)]
pub(crate) struct MethodDefinitionDescriptor<'ctx> {
    metadata: &'static MethodMetadata,
    owner: MethodDefinitionOwner<'ctx>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> MethodDefinitionDescriptor<'ctx> {
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
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.name
    }
    pub(crate) const fn visibility(self) -> Visibility {
        match self.metadata.visibility {
            crate::runtime::descriptor::method::MethodVisibility::Public => Visibility::Public,
            crate::runtime::descriptor::method::MethodVisibility::Restricted => {
                Visibility::RestrictedUnknown
            }
            crate::runtime::descriptor::method::MethodVisibility::Private => Visibility::Private,
        }
    }
    pub(crate) const fn owner(self) -> MethodDefinitionOwner<'ctx> {
        self.owner
    }
    pub(crate) fn receiver(self) -> Option<ReceiverDefinitionDescriptor<'ctx>> {
        self.metadata
            .receiver
            .map(|receiver| ReceiverDefinitionDescriptor {
                receiver,
                context: self.context,
            })
    }
    pub(crate) fn parameters(
        self,
    ) -> impl ExactSizeIterator<Item = ParameterDescriptor<'ctx>> + 'ctx {
        self.metadata
            .parameters
            .iter()
            .map(move |parameter| ParameterDescriptor::new(parameter, self.context))
    }
    pub(crate) fn output(self) -> ReturnDescriptor<'ctx> {
        ReturnDescriptor::new(self.metadata.output, self.context)
    }
    pub(crate) const fn qualifiers(self) -> MethodQualifiers {
        self.metadata.qualifiers
    }
    pub(crate) const fn has_default_body(self) -> bool {
        self.metadata.default_body
    }
    pub(crate) fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ReceiverDefinitionDescriptor<'ctx> {
    receiver: ReceiverDescriptor,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ReceiverDefinitionDescriptor<'ctx> {
    pub(crate) const fn kind(self) -> ReceiverKind {
        self.receiver.kind
    }
    pub(crate) fn concrete_type(self) -> Option<crate::runtime::descriptor::TypeDescriptor<'ctx>> {
        self.receiver.type_metadata.map(|metadata| {
            crate::runtime::descriptor::TypeDescriptor::new(metadata(), self.context)
        })
    }
    pub(crate) const fn expression(
        self,
    ) -> Option<&'static crate::runtime::expression::ExpressionData> {
        self.receiver.expression
    }
}

#[derive(Clone, Copy)]
pub(crate) enum MethodDefinitionOwner<'ctx> {
    Trait(TraitDefinitionDescriptor<'ctx>),
    Impl(crate::runtime::descriptor::impl_descriptor::ImplDefinitionDescriptor<'ctx>),
}

/// Ordered source method declarations for one trait or impl block.
pub(crate) struct MethodDefinitions<'ctx> {
    values: &'static [MethodMetadata],
    owner: MethodDefinitionOwner<'ctx>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> MethodDefinitions<'ctx> {
    pub(crate) const fn new(
        values: &'static [MethodMetadata],
        owner: MethodDefinitionOwner<'ctx>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self {
            values,
            owner,
            context,
        }
    }
    pub(crate) const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = MethodDefinitionDescriptor<'ctx>> + '_ {
        self.values
            .iter()
            .map(|method| MethodDefinitionDescriptor::new(method, self.owner, self.context))
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn get(&self, name: &str) -> Option<MethodDefinitionDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|method| method.name == name)
            .map(|method| MethodDefinitionDescriptor::new(method, self.owner, self.context))
    }
}

/// A source-level direct supertrait expression list.
#[derive(Clone, Copy)]
pub(crate) struct TraitExpressions<'ctx> {
    expressions: &'static [TraitExpressionMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpressions<'ctx> {
    const fn new(
        expressions: &'static [TraitExpressionMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self {
            expressions,
            context,
        }
    }
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = TraitExpressionDescriptor<'ctx>> + 'ctx {
        self.expressions
            .iter()
            .map(move |metadata| TraitExpressionDescriptor::new(metadata, self.context))
    }
    pub(crate) fn len(self) -> usize {
        self.expressions.len()
    }
}

/// An ordered set of concrete trait applications.
#[derive(Clone)]
pub(crate) struct Traits<'ctx> {
    values: Box<[TraitDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> Traits<'ctx> {
    pub(crate) fn new(
        values: Box<[TraitDescriptor<'ctx>]>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    fn from_metadata(
        values: &'static [&'static TraitApplicationMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self::new(
            values
                .iter()
                .map(|metadata| TraitDescriptor::new(metadata, context))
                .collect(),
            context,
        )
    }
    pub(crate) const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = TraitDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn get(&self, index: usize) -> Option<TraitDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
}

/// An ordered set of trait declaration descriptors.
pub(crate) struct TraitDefinitions<'ctx> {
    values: Box<[TraitDefinitionDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitDefinitions<'ctx> {
    pub(crate) fn new(
        values: Box<[TraitDefinitionDescriptor<'ctx>]>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = TraitDefinitionDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn get(&self, index: usize) -> Option<TraitDefinitionDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
}

/// A definition of one associated type in a trait.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedTypeDefinitionDescriptor<'ctx> {
    metadata: &'static AssociatedTypeDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedTypeDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static AssociatedTypeDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.name
    }
    pub(crate) const fn visibility(self) -> Visibility {
        self.metadata.visibility
    }
    pub(crate) fn bounds(self) -> Predicates<'ctx> {
        Predicates::new(self.metadata.bounds, self.context)
    }
    pub(crate) fn default(self) -> Option<TypeExpression<'ctx>> {
        self.metadata
            .default
            .map(|data| TypeExpression::new(data, self.context))
    }
}

pub(crate) struct AssociatedTypeDefinitions<'ctx> {
    values: &'static [AssociatedTypeDefinitionMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedTypeDefinitions<'ctx> {
    const fn new(
        values: &'static [AssociatedTypeDefinitionMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = AssociatedTypeDefinitionDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedTypeDefinitionDescriptor::new(value, self.context))
    }
    pub(crate) fn len(self) -> usize {
        self.values.len()
    }
    pub(crate) fn get(self, name: &str) -> Option<AssociatedTypeDefinitionDescriptor<'ctx>> {
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
pub(crate) struct AssociatedConstDefinitionDescriptor<'ctx> {
    metadata: &'static AssociatedConstDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedConstDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static AssociatedConstDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.name
    }
    pub(crate) const fn visibility(self) -> Visibility {
        self.metadata.visibility
    }
    pub(crate) fn ty(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.ty, self.context)
    }
    pub(crate) const fn default(self) -> Option<ConstExpression> {
        self.metadata.default
    }
}

pub(crate) struct AssociatedConstDefinitions<'ctx> {
    values: &'static [AssociatedConstDefinitionMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedConstDefinitions<'ctx> {
    const fn new(
        values: &'static [AssociatedConstDefinitionMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = AssociatedConstDefinitionDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedConstDefinitionDescriptor::new(value, self.context))
    }
    pub(crate) fn len(self) -> usize {
        self.values.len()
    }
    pub(crate) fn get(self, name: &str) -> Option<AssociatedConstDefinitionDescriptor<'ctx>> {
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

fn unique_named<T>(
    mut matches: impl Iterator<Item = T>,
    subject: LookupSubject,
    name: &str,
) -> Result<T, LookupError>
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
