use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::{GenericArguments, GenericDefinitionDescriptor};
use crate::runtime::descriptor::method::{MethodCatalog, MethodDescriptor};
use crate::runtime::descriptor::trait_descriptor::{
    AssociatedConstDefinitionDescriptor, AssociatedTypeDefinitionDescriptor,
    MethodDefinitionDescriptor, MethodDefinitionOwner, MethodDefinitions, TraitDescriptor,
    TraitExpressionDescriptor,
};
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::error::{
    ContextError, LookupError, LookupQuery, LookupSubject, ReflectionError,
};
use crate::runtime::expression::{ConstExpression, TypeExpression};
use crate::runtime::identity::{
    IntoLookupCandidate, ItemIdentity, ItemKind, LookupCandidate, StableItemId,
};
use crate::runtime::lookup::Lookup;
use crate::runtime::value::{DynamicOwned, Local, ThreadSafe, ValueMode};

use super::metadata::{
    AssociatedConstBindingMetadata, AssociatedItemSource, AssociatedTypeBindingMetadata,
    ImplApplicationMetadata, ImplDefinitionId, ImplDefinitionMetadata, ImplKind, ImplQualifiers,
};

/// Context-bound view of an `impl` source declaration.
#[derive(Clone, Copy)]
pub(crate) struct ImplDefinitionDescriptor<'ctx> {
    metadata: &'static ImplDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ImplDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static ImplDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) const fn id(self) -> ImplDefinitionId {
        self.metadata.id
    }
    pub(crate) const fn kind(self) -> ImplKind {
        self.metadata.kind
    }
    pub(crate) fn target(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.target, self.context)
    }
    pub(crate) fn implemented_trait(self) -> Option<TraitExpressionDescriptor<'ctx>> {
        self.metadata
            .implemented_trait
            .map(|metadata| TraitExpressionDescriptor::new(metadata, self.context))
    }
    pub(crate) fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
    pub(crate) const fn qualifiers(self) -> ImplQualifiers {
        self.metadata.qualifiers
    }
    pub(crate) fn methods(self) -> MethodDefinitions<'ctx> {
        MethodDefinitions::new(
            self.metadata.methods,
            MethodDefinitionOwner::Impl(self),
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
                        MethodDefinitionOwner::Impl(self),
                        self.context,
                    )
                }),
            LookupSubject::Method,
            name,
        )
    }
    pub(crate) fn associated_types(self) -> ImplAssociatedTypeDefinitions<'ctx> {
        ImplAssociatedTypeDefinitions::new(self.metadata.associated_types, self.context)
    }
    pub(crate) fn associated_type(
        self,
        name: &str,
    ) -> Result<AssociatedTypeDefinitionDescriptor<'ctx>, LookupError> {
        self.associated_types()
            .get(name)
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::AssociatedType,
                query: LookupQuery::Name(name.into()),
            })
    }
    pub(crate) fn associated_consts(self) -> ImplAssociatedConstDefinitions<'ctx> {
        ImplAssociatedConstDefinitions::new(self.metadata.associated_consts, self.context)
    }
    pub(crate) fn associated_const(
        self,
        name: &str,
    ) -> Result<AssociatedConstDefinitionDescriptor<'ctx>, LookupError> {
        self.associated_consts()
            .get(name)
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::AssociatedConst,
                query: LookupQuery::Name(name.into()),
            })
    }
    pub(crate) fn instances(self) -> Result<Impls<'ctx>, ContextError> {
        self.context
            .impl_applications(self.metadata.id)
            .map(|applications| Impls::new(applications, self.context))
    }
}

/// Context-bound view of one compiled `impl` application.
#[derive(Clone, Copy)]
pub(crate) struct ImplDescriptor<'ctx> {
    metadata: &'static ImplApplicationMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ImplDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static ImplApplicationMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn definition(self) -> ImplDefinitionDescriptor<'ctx> {
        ImplDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    pub(crate) const fn kind(self) -> ImplKind {
        self.metadata.definition.kind
    }
    pub(crate) fn target(self) -> TypeDescriptor<'ctx> {
        TypeDescriptor::new((self.metadata.target)(), self.context)
    }
    pub(crate) fn implemented_trait(self) -> Option<TraitDescriptor<'ctx>> {
        self.metadata
            .implemented_trait
            .map(|metadata| TraitDescriptor::new(metadata, self.context))
    }
    pub(crate) fn arguments(self) -> GenericArguments<'ctx> {
        GenericArguments::new(self.metadata.arguments, self.context)
    }
    pub(crate) fn methods(self) -> Methods<'ctx> {
        Methods::new(self.metadata.methods, self.context)
    }
    pub(crate) fn method(self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        MethodCatalog::new(self.target(), self.metadata.methods).method(name)
    }
    pub(crate) fn associated_types(self) -> AssociatedTypeBindings<'ctx> {
        AssociatedTypeBindings::new(self.metadata.associated_types, self.context)
    }
    pub(crate) fn associated_type(
        self,
        name: &str,
    ) -> Result<AssociatedTypeBindingDescriptor<'ctx>, LookupError> {
        self.associated_types()
            .get(name)
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::AssociatedType,
                query: LookupQuery::Name(name.into()),
            })
    }
    pub(crate) fn associated_consts(self) -> AssociatedConstBindings<'ctx> {
        AssociatedConstBindings::new(self.metadata.associated_consts, self.context)
    }
    pub(crate) fn associated_const(
        self,
        name: &str,
    ) -> Result<AssociatedConstBindingDescriptor<'ctx>, LookupError> {
        self.associated_consts()
            .get(name)
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::AssociatedConst,
                query: LookupQuery::Name(name.into()),
            })
    }
    pub(crate) fn same_application(self, other: Self) -> bool {
        self.context.id() == other.context.id()
            && self.definition().id() == other.definition().id()
            && self.target().type_id() == other.target().type_id()
            && self.metadata.identity == other.metadata.identity
    }
}

/// Ordered methods available on one concrete implementation.
pub(crate) struct Methods<'ctx> {
    values: &'static [crate::runtime::descriptor::method::EffectiveMethod],
    context: &'ctx ReflectionContext,
}

impl<'ctx> Methods<'ctx> {
    const fn new(
        values: &'static [crate::runtime::descriptor::method::EffectiveMethod],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = MethodDescriptor<'ctx>> + '_ {
        self.values
            .iter()
            .map(|method| MethodDescriptor::new(*method, self.context))
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn get(&self, name: &str) -> Option<MethodDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|method| method.metadata.name == name)
            .map(|method| MethodDescriptor::new(*method, self.context))
    }
}

/// Ordered source-level implementation declarations.
pub(crate) struct ImplDefinitions<'ctx> {
    values: Box<[ImplDefinitionDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}
impl<'ctx> ImplDefinitions<'ctx> {
    pub(crate) fn new(
        values: Box<[ImplDefinitionDescriptor<'ctx>]>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = ImplDefinitionDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn get(&self, index: usize) -> Option<ImplDefinitionDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
}

/// Ordered concrete implementation applications.
pub(crate) struct Impls<'ctx> {
    values: Box<[ImplDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}
impl<'ctx> Impls<'ctx> {
    pub(crate) fn new(
        values: Box<[ImplDescriptor<'ctx>]>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) const fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = ImplDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    pub(crate) fn len(&self) -> usize {
        self.values.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(crate) fn get(&self, index: usize) -> Option<ImplDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
    pub(crate) fn lookup_trait_definition(
        &self,
        definition: crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor<'ctx>,
    ) -> Result<Lookup<ImplDescriptor<'ctx>>, ContextError> {
        if self.context.id() != definition.context().id() {
            return Err(ContextError::new(
                "trait definition belongs to a different reflection context",
            ));
        }
        let mut matches = self.values.iter().copied().filter(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.definition().id() == definition.id())
        });
        let Some(first) = matches.next() else {
            return Ok(Lookup::Missing);
        };
        match matches.next() {
            None => Ok(Lookup::Unique(first)),
            Some(second) => Ok(Lookup::Ambiguous(
                std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(matches)
                    .collect(),
            )),
        }
    }
    pub(crate) fn find_trait_application(
        &self,
        trait_: TraitDescriptor<'ctx>,
    ) -> Result<Option<ImplDescriptor<'ctx>>, ContextError> {
        if self.context.id() != trait_.context().id() {
            return Err(ContextError::new(
                "trait application belongs to a different reflection context",
            ));
        }
        let mut matches = self.values.iter().copied().filter(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|candidate| candidate.same_application(trait_))
        });
        let Some(first) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(ContextError::new(
                "duplicate implementation for one concrete trait application",
            ));
        }
        Ok(Some(first))
    }
}

/// Final binding of one associated type for a concrete implementation.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedTypeBindingDescriptor<'ctx> {
    metadata: &'static AssociatedTypeBindingMetadata,
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedTypeBindingDescriptor<'ctx> {
    const fn new(
        metadata: &'static AssociatedTypeBindingMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) fn definition(self) -> AssociatedTypeDefinitionDescriptor<'ctx> {
        AssociatedTypeDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    pub(crate) fn expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.expression, self.context)
    }
    pub(crate) fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
    pub(crate) const fn source(self) -> AssociatedItemSource {
        self.metadata.source
    }
}

pub(crate) struct AssociatedTypeBindings<'ctx> {
    values: &'static [AssociatedTypeBindingMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedTypeBindings<'ctx> {
    const fn new(
        values: &'static [AssociatedTypeBindingMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = AssociatedTypeBindingDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedTypeBindingDescriptor::new(value, self.context))
    }
    pub(crate) fn len(self) -> usize {
        self.values.len()
    }
    pub(crate) fn get(self, name: &str) -> Option<AssociatedTypeBindingDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|value| value.definition.name == name)
            .map(|value| AssociatedTypeBindingDescriptor::new(value, self.context))
    }
}

/// Final binding of one associated constant for a concrete implementation.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedConstBindingDescriptor<'ctx> {
    metadata: &'static AssociatedConstBindingMetadata,
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedConstBindingDescriptor<'ctx> {
    const fn new(
        metadata: &'static AssociatedConstBindingMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) fn definition(self) -> AssociatedConstDefinitionDescriptor<'ctx> {
        AssociatedConstDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    pub(crate) const fn expression(self) -> ConstExpression {
        self.metadata.expression
    }
    pub(crate) fn value<M: AssociatedConstValueMode>(
        self,
    ) -> Result<Option<DynamicOwned<M>>, ReflectionError> {
        Ok(self.metadata.value.and_then(M::make))
    }
    pub(crate) const fn source(self) -> AssociatedItemSource {
        self.metadata.source
    }
}

pub(crate) trait AssociatedConstValueMode: ValueMode + Sized {
    fn make(adapter: &super::metadata::AssociatedConstValueAdapter) -> Option<DynamicOwned<Self>>;
}

impl AssociatedConstValueMode for Local {
    fn make(adapter: &super::metadata::AssociatedConstValueAdapter) -> Option<DynamicOwned<Self>> {
        adapter.local.map(|make| make())
    }
}

impl AssociatedConstValueMode for ThreadSafe {
    fn make(adapter: &super::metadata::AssociatedConstValueAdapter) -> Option<DynamicOwned<Self>> {
        adapter.thread_safe.map(|make| make())
    }
}

pub(crate) struct AssociatedConstBindings<'ctx> {
    values: &'static [AssociatedConstBindingMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedConstBindings<'ctx> {
    const fn new(
        values: &'static [AssociatedConstBindingMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = AssociatedConstBindingDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedConstBindingDescriptor::new(value, self.context))
    }
    pub(crate) fn len(self) -> usize {
        self.values.len()
    }
    pub(crate) fn get(self, name: &str) -> Option<AssociatedConstBindingDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|value| value.definition.name == name)
            .map(|value| AssociatedConstBindingDescriptor::new(value, self.context))
    }
}

pub(crate) struct ImplAssociatedTypeDefinitions<'ctx> {
    values:
        &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> ImplAssociatedTypeDefinitions<'ctx> {
    const fn new(
        values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
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
        self.values
            .iter()
            .find(|value| value.name == name)
            .map(|value| AssociatedTypeDefinitionDescriptor::new(value, self.context))
    }
}

pub(crate) struct ImplAssociatedConstDefinitions<'ctx> { values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata], context: &'ctx ReflectionContext }
impl<'ctx> ImplAssociatedConstDefinitions<'ctx> {
    const fn new(
        values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
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
        self.values
            .iter()
            .find(|value| value.name == name)
            .map(|value| AssociatedConstDefinitionDescriptor::new(value, self.context))
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

impl IntoLookupCandidate for ImplDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.target().type_identity().rust_name.into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Impl,
                stable_id: StableItemId(
                    format!("{}:{:?}", display_name, self.definition().id()).into(),
                ),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

impl IntoLookupCandidate for ImplDefinitionDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = format!("{:?}", self.id()).into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Impl,
                stable_id: StableItemId(display_name.clone()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}
