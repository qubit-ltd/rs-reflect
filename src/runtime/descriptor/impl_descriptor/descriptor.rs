// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::metadata::AssociatedConstBindingMetadata;
use super::metadata::AssociatedItemSource;
use super::metadata::AssociatedTypeBindingMetadata;
use super::metadata::ImplApplicationMetadata;
use super::metadata::ImplDefinitionId;
use super::metadata::ImplDefinitionMetadata;
use super::metadata::ImplKind;
use super::metadata::ImplQualifiers;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::GenericArguments;
use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
use crate::runtime::descriptor::method::MethodDescriptor;
use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::MethodDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::MethodDefinitionOwner;
use crate::runtime::descriptor::trait_descriptor::MethodDefinitions;
use crate::runtime::descriptor::trait_descriptor::TraitDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitExpressionDescriptor;
use crate::runtime::error::ContextError;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::error::ReflectionError;
use crate::runtime::expression::ConstExpression;
use crate::runtime::expression::TypeExpression;
use crate::runtime::identity::IntoLookupCandidate;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;
use crate::runtime::lookup::Lookup;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;
use crate::runtime::value::ValueMode;

/// Context-bound view of an `impl` source declaration.
#[derive(Clone, Copy)]
pub struct ImplDefinitionDescriptor<'ctx> {
    metadata: &'static ImplDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ImplDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(metadata: &'static ImplDefinitionMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context that owns this source declaration.
    pub const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the process-local identity of this source `impl` block.
    pub const fn id(self) -> ImplDefinitionId {
        self.metadata.id
    }
    /// Returns whether this is an inherent or trait implementation.
    pub const fn kind(self) -> ImplKind {
        self.metadata.kind
    }
    /// Returns the retained source expression for the implementation target.
    pub fn target(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.target, self.context)
    }
    /// Returns the implemented trait expression, or `None` for an inherent
    /// impl.
    pub fn implemented_trait(self) -> Option<TraitExpressionDescriptor<'ctx>> {
        self.metadata
            .implemented_trait
            .map(|metadata| TraitExpressionDescriptor::new(metadata, self.context))
    }
    /// Returns the source generic parameters and predicates of this impl.
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
    /// Returns qualifiers such as `unsafe` retained for this impl.
    pub const fn qualifiers(self) -> ImplQualifiers {
        self.metadata.qualifiers
    }
    /// Returns the source method declarations in this impl.
    pub fn methods(self) -> MethodDefinitions<'ctx> {
        MethodDefinitions::new(self.metadata.methods, MethodDefinitionOwner::Impl(self), self.context)
    }
    /// Finds a uniquely named method declaration in this impl.
    pub fn method(self, name: &str) -> Result<MethodDefinitionDescriptor<'ctx>, LookupError> {
        unique_named(
            self.metadata
                .methods
                .iter()
                .filter(|method| method.name == name)
                .map(|method| MethodDefinitionDescriptor::new(method, MethodDefinitionOwner::Impl(self), self.context)),
            LookupSubject::Method,
            name,
        )
    }
    /// Returns associated type declarations made by this impl.
    pub fn associated_types(self) -> ImplAssociatedTypeDefinitions<'ctx> {
        ImplAssociatedTypeDefinitions::new(self.metadata.associated_types, self.context)
    }
    /// Finds an associated type declaration by name.
    pub fn associated_type(self, name: &str) -> Result<AssociatedTypeDefinitionDescriptor<'ctx>, LookupError> {
        self.associated_types().get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::AssociatedType,
            query: LookupQuery::Name(name.into()),
        })
    }
    /// Returns associated constant declarations made by this impl.
    pub fn associated_consts(self) -> ImplAssociatedConstDefinitions<'ctx> {
        ImplAssociatedConstDefinitions::new(self.metadata.associated_consts, self.context)
    }
    /// Finds an associated constant declaration by name.
    pub fn associated_const(self, name: &str) -> Result<AssociatedConstDefinitionDescriptor<'ctx>, LookupError> {
        self.associated_consts().get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::AssociatedConst,
            query: LookupQuery::Name(name.into()),
        })
    }
    /// Returns registered concrete applications of this source impl.
    pub fn instances(self) -> Result<Impls<'ctx>, ContextError> {
        self.context
            .impl_applications(self.metadata.id)
            .map(|applications| Impls::new(applications, self.context))
    }
}

/// Context-bound view of one compiled `impl` application.
#[derive(Clone, Copy)]
pub struct ImplDescriptor<'ctx> {
    metadata: &'static ImplApplicationMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ImplDescriptor<'ctx> {
    pub(crate) const fn new(metadata: &'static ImplApplicationMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context that owns this concrete application.
    pub const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the source declaration that produced this application.
    pub fn definition(self) -> ImplDefinitionDescriptor<'ctx> {
        ImplDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    /// Returns whether this application is inherent or implements a trait.
    pub const fn kind(self) -> ImplKind {
        self.metadata.definition.kind
    }
    /// Returns the concrete target type of this implementation.
    pub fn target(self) -> TypeDescriptor<'ctx> {
        TypeDescriptor::new((self.metadata.target)(), self.context)
    }
    /// Returns the implemented trait application, or `None` for an inherent
    /// impl.
    pub fn implemented_trait(self) -> Option<TraitDescriptor<'ctx>> {
        self.metadata
            .implemented_trait
            .map(|metadata| TraitDescriptor::new(metadata, self.context))
    }
    /// Returns type and const arguments for this impl application.
    pub fn arguments(self) -> GenericArguments<'ctx> {
        GenericArguments::new(self.metadata.arguments, self.context)
    }
    /// Returns methods available through this concrete implementation.
    pub fn methods(self) -> Methods<'ctx> {
        Methods::new(self.metadata.methods, self.metadata, self.context)
    }
    /// Finds a method by its source name.
    pub fn method(self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.methods().get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::Method,
            query: LookupQuery::Name(name.into()),
        })
    }
    /// Returns associated type bindings for this concrete implementation.
    pub fn associated_types(self) -> AssociatedTypeBindings<'ctx> {
        AssociatedTypeBindings::new(self.metadata.associated_types, self.context)
    }
    /// Finds a concrete associated type binding by name.
    pub fn associated_type(self, name: &str) -> Result<AssociatedTypeBindingDescriptor<'ctx>, LookupError> {
        self.associated_types().get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::AssociatedType,
            query: LookupQuery::Name(name.into()),
        })
    }
    /// Returns associated constant bindings for this concrete implementation.
    pub fn associated_consts(self) -> AssociatedConstBindings<'ctx> {
        AssociatedConstBindings::new(self.metadata.associated_consts, self.context)
    }
    /// Finds a concrete associated constant binding by name.
    pub fn associated_const(self, name: &str) -> Result<AssociatedConstBindingDescriptor<'ctx>, LookupError> {
        self.associated_consts().get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::AssociatedConst,
            query: LookupQuery::Name(name.into()),
        })
    }
    /// Returns whether both descriptors identify the same context-bound impl.
    pub fn same_application(self, other: Self) -> bool {
        self.context.id() == other.context.id()
            && self.definition().id() == other.definition().id()
            && self.target().type_id() == other.target().type_id()
            && self.metadata.identity == other.metadata.identity
    }
}

/// Ordered methods available on one concrete implementation.
pub struct Methods<'ctx> {
    values: &'static [crate::runtime::descriptor::method::EffectiveMethod],
    implementation: &'static ImplApplicationMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> Methods<'ctx> {
    const fn new(
        values: &'static [crate::runtime::descriptor::method::EffectiveMethod],
        implementation: &'static ImplApplicationMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self {
            values,
            implementation,
            context,
        }
    }
    /// Returns the context that owns these methods.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over methods in declaration and effective-availability order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = MethodDescriptor<'ctx>> + '_ {
        self.values
            .iter()
            .map(|method| MethodDescriptor::with_implementation(*method, self.implementation, self.context))
    }
    /// Returns the number of methods in this implementation.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns whether this implementation provides no methods.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Finds a method by its source name.
    pub fn get(&self, name: &str) -> Option<MethodDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|method| method.metadata.name == name)
            .map(|method| MethodDescriptor::with_implementation(*method, self.implementation, self.context))
    }
}

/// Ordered source-level implementation declarations.
pub struct ImplDefinitions<'ctx> {
    values: Box<[ImplDefinitionDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}
impl<'ctx> ImplDefinitions<'ctx> {
    pub(crate) fn new(values: Box<[ImplDefinitionDescriptor<'ctx>]>, context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    /// Returns the context that owns these implementation definitions.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over source impl declarations in context order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ImplDefinitionDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    /// Returns the number of source impl declarations.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns whether the context contains no source impl declarations.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Returns the declaration at its collection position, if present.
    pub fn get(&self, index: usize) -> Option<ImplDefinitionDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
}

/// Ordered concrete implementation applications.
pub struct Impls<'ctx> {
    values: Box<[ImplDescriptor<'ctx>]>,
    context: &'ctx ReflectionContext,
}
impl<'ctx> Impls<'ctx> {
    pub(crate) fn new(values: Box<[ImplDescriptor<'ctx>]>, context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    /// Returns the context that owns these implementation applications.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over concrete impl applications in context order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ImplDescriptor<'ctx>> + '_ {
        self.values.iter().copied()
    }
    /// Returns the number of concrete impl applications.
    pub fn len(&self) -> usize {
        self.values.len()
    }
    /// Returns whether there are no concrete impl applications.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    /// Returns the application at its collection position, if present.
    pub fn get(&self, index: usize) -> Option<ImplDescriptor<'ctx>> {
        self.values.get(index).copied()
    }
    /// Looks up implementations linked to one trait declaration and preserves
    /// ambiguity.
    pub fn lookup_trait_definition(
        &self,
        definition: crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor<'ctx>,
    ) -> Result<Lookup<ImplDescriptor<'ctx>>, ContextError> {
        if self.context.id() != definition.context().id() {
            return Err(ContextError::new(
                "trait definition belongs to a different reflection context",
            ));
        }
        let mut matches = self.values.iter().copied().filter(|implementation| {
            implementation.implemented_trait().is_some_and(|trait_| {
                trait_
                    .find_definition()
                    .is_some_and(|linked| linked.id() == definition.id())
            })
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
    /// Finds the implementation for one exact trait application.
    pub fn find_trait_application(
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
pub struct AssociatedTypeBindingDescriptor<'ctx> {
    metadata: &'static AssociatedTypeBindingMetadata,
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedTypeBindingDescriptor<'ctx> {
    const fn new(metadata: &'static AssociatedTypeBindingMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the trait or impl declaration of this associated type.
    pub fn definition(self) -> AssociatedTypeDefinitionDescriptor<'ctx> {
        AssociatedTypeDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    /// Returns the retained source expression for the binding.
    pub fn expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.expression, self.context)
    }
    /// Finds the concrete type descriptor if the bound type is registered.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
    /// Returns whether the binding was declared, defaulted, or overridden.
    pub const fn source(self) -> AssociatedItemSource {
        self.metadata.source
    }
}

/// Collection of associated type bindings for one implementation.
pub struct AssociatedTypeBindings<'ctx> {
    values: &'static [AssociatedTypeBindingMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedTypeBindings<'ctx> {
    const fn new(values: &'static [AssociatedTypeBindingMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    /// Iterates over associated type bindings in declaration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = AssociatedTypeBindingDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedTypeBindingDescriptor::new(value, self.context))
    }
    /// Returns the number of associated type bindings.
    pub fn len(self) -> usize {
        self.values.len()
    }
    /// Returns whether there are no associated type bindings.
    pub fn is_empty(self) -> bool {
        self.values.is_empty()
    }
    /// Finds an associated type binding by name.
    pub fn get(self, name: &str) -> Option<AssociatedTypeBindingDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|value| value.definition.name == name)
            .map(|value| AssociatedTypeBindingDescriptor::new(value, self.context))
    }
}

/// Final binding of one associated constant for a concrete implementation.
#[derive(Clone, Copy)]
pub struct AssociatedConstBindingDescriptor<'ctx> {
    metadata: &'static AssociatedConstBindingMetadata,
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedConstBindingDescriptor<'ctx> {
    const fn new(metadata: &'static AssociatedConstBindingMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the trait or impl declaration of this associated constant.
    pub fn definition(self) -> AssociatedConstDefinitionDescriptor<'ctx> {
        AssociatedConstDefinitionDescriptor::new(self.metadata.definition, self.context)
    }
    /// Returns the retained structural constant expression.
    pub const fn expression(self) -> ConstExpression {
        self.metadata.expression
    }
    // Keep mode dispatch sealed to the built-in Local and ThreadSafe modes;
    // exposing the adapter trait would make downstream implementations depend
    // on the private associated-constant adapter representation.
    #[allow(private_bounds)]
    /// Evaluates the constant when a value adapter exists for `M`.
    pub fn value<M: AssociatedConstValueMode>(self) -> Result<Option<DynamicOwned<M>>, ReflectionError> {
        Ok(self.metadata.value.and_then(M::make))
    }
    /// Returns whether the binding was declared, defaulted, or overridden.
    pub const fn source(self) -> AssociatedItemSource {
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

/// Collection of associated constant bindings for one implementation.
pub struct AssociatedConstBindings<'ctx> {
    values: &'static [AssociatedConstBindingMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> AssociatedConstBindings<'ctx> {
    const fn new(values: &'static [AssociatedConstBindingMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { values, context }
    }
    /// Iterates over associated constant bindings in declaration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = AssociatedConstBindingDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedConstBindingDescriptor::new(value, self.context))
    }
    /// Returns the number of associated constant bindings.
    pub fn len(self) -> usize {
        self.values.len()
    }
    /// Returns whether there are no associated constant bindings.
    pub fn is_empty(self) -> bool {
        self.values.is_empty()
    }
    /// Finds an associated constant binding by name.
    pub fn get(self, name: &str) -> Option<AssociatedConstBindingDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|value| value.definition.name == name)
            .map(|value| AssociatedConstBindingDescriptor::new(value, self.context))
    }
}

/// Collection of source-level associated type declarations in an impl.
pub struct ImplAssociatedTypeDefinitions<'ctx> {
    values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> ImplAssociatedTypeDefinitions<'ctx> {
    const fn new(
        values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    /// Iterates over associated type declarations in source order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = AssociatedTypeDefinitionDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedTypeDefinitionDescriptor::new(value, self.context))
    }
    /// Returns the number of associated type declarations.
    pub fn len(self) -> usize {
        self.values.len()
    }
    /// Returns whether there are no associated type declarations.
    pub fn is_empty(self) -> bool {
        self.values.is_empty()
    }
    /// Finds an associated type declaration by name.
    pub fn get(self, name: &str) -> Option<AssociatedTypeDefinitionDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|value| value.name == name)
            .map(|value| AssociatedTypeDefinitionDescriptor::new(value, self.context))
    }
}

/// Collection of source-level associated constant declarations in an impl.
pub struct ImplAssociatedConstDefinitions<'ctx> {
    values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
    context: &'ctx ReflectionContext,
}
impl<'ctx> ImplAssociatedConstDefinitions<'ctx> {
    const fn new(
        values: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { values, context }
    }
    /// Iterates over associated constant declarations in source order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = AssociatedConstDefinitionDescriptor<'ctx>> + 'ctx {
        self.values
            .iter()
            .map(move |value| AssociatedConstDefinitionDescriptor::new(value, self.context))
    }
    /// Returns the number of associated constant declarations.
    pub fn len(self) -> usize {
        self.values.len()
    }
    /// Returns whether there are no associated constant declarations.
    pub fn is_empty(self) -> bool {
        self.values.is_empty()
    }
    /// Finds an associated constant declaration by name.
    pub fn get(self, name: &str) -> Option<AssociatedConstDefinitionDescriptor<'ctx>> {
        self.values
            .iter()
            .find(|value| value.name == name)
            .map(|value| AssociatedConstDefinitionDescriptor::new(value, self.context))
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

impl IntoLookupCandidate for ImplDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.target().type_identity().rust_name.into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Impl,
                stable_id: StableItemId(format!("{}:{:?}", display_name, self.definition().id()).into()),
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

#[cfg(test)]
mod tests {
    use super::ImplDefinitionDescriptor;
    use super::ImplDefinitions;
    use super::ImplDescriptor;
    use super::Impls;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata;
    use crate::runtime::descriptor::impl_descriptor::AssociatedItemSource;
    use crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplApplicationMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplDefinitionMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplKind;
    use crate::runtime::descriptor::impl_descriptor::ImplQualifiers;
    use crate::runtime::descriptor::metadata::TypeMetadata;
    use crate::runtime::descriptor::method::EffectiveMethod;
    use crate::runtime::descriptor::method::MethodImplementationSource;
    use crate::runtime::descriptor::method::MethodMetadata;
    use crate::runtime::descriptor::method::MethodNamespaceMetadata;
    use crate::runtime::descriptor::method::MethodQualifiers;
    use crate::runtime::descriptor::method::MethodVisibility;
    use crate::runtime::descriptor::method::ReturnKind;
    use crate::runtime::descriptor::method::ReturnMetadata;
    use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitDescriptor;
    use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::error::LookupError;
    use crate::runtime::error::LookupQuery;
    use crate::runtime::error::LookupSubject;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::TraitExpressionData;
    use crate::runtime::lookup::Lookup;
    use crate::runtime::value::Local;
    use crate::runtime::value::ThreadSafe;

    struct InherentImpl;
    struct DuplicateMethodsImpl;
    struct ReflectedTrait;
    struct TraitImpl;
    struct AssociatedItemsImpl;

    static NO_PREDICATES: &[crate::runtime::expression::PredicateData] = &[];
    static NO_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &[],
        predicates: NO_PREDICATES,
    };
    static TARGET: ExpressionData = ExpressionData::Named {
        path: "fixture::Target",
        definition: None,
        arguments: &[],
    };
    static TYPE_EXPRESSION: ExpressionData = ExpressionData::Named {
        path: "alloc::string::String",
        definition: None,
        arguments: &[],
    };
    static TRAIT_EXPRESSION: TraitExpressionData = TraitExpressionData::new("fixture::Contract", &[], &[]);
    static TRAIT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<ReflectedTrait>(
        "Contract",
        "fixture::Contract",
        Visibility::Public,
        TraitCompleteness::Complete,
        &NO_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static TRAIT_APPLICATION: TraitApplicationMetadata = TraitApplicationMetadata::new(
        &TRAIT_DEFINITION,
        &[],
        &[],
        &[],
        &[],
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
    );
    static TRAIT_EXPRESSION_METADATA: TraitExpressionMetadata =
        TraitExpressionMetadata::for_definition(&TRAIT_EXPRESSION, &TRAIT_DEFINITION);
    static NO_ASSOCIATED_TYPES: &[AssociatedTypeDefinitionMetadata] = &[];
    static NO_ASSOCIATED_CONSTS: &[AssociatedConstDefinitionMetadata] = &[];
    static ITEM_TYPE: AssociatedTypeDefinitionMetadata = AssociatedTypeDefinitionMetadata {
        name: "Item",
        visibility: Visibility::Public,
        bounds: NO_PREDICATES,
        default: None,
    };
    static ITEM_CONST: AssociatedConstDefinitionMetadata = AssociatedConstDefinitionMetadata {
        name: "COUNT",
        visibility: Visibility::Public,
        ty: &TYPE_EXPRESSION,
        default: None,
    };
    static RETURN: ReturnMetadata = ReturnMetadata {
        kind: ReturnKind::Unit,
        expression: None,
        concrete_type: None,
    };

    const fn method(name: &'static str) -> MethodMetadata {
        MethodMetadata {
            name,
            namespace: MethodNamespaceMetadata::Inherent {
                implementation: "impl fixture::Target",
            },
            visibility: MethodVisibility::Public,
            receiver: None,
            parameters: &[],
            output: &RETURN,
            qualifiers: MethodQualifiers {
                is_const: false,
                is_async: false,
                is_unsafe: false,
                abi: None,
                is_variadic: false,
            },
            generics: &NO_GENERICS,
            is_generic: false,
            default_body: false,
        }
    }

    static METHODS_ARRAY: [MethodMetadata; 1] = [method("run")];
    static SAME_NAME_METHODS: [MethodMetadata; 2] = [method("run"), method("run")];
    static METHODS: &[MethodMetadata] = &METHODS_ARRAY;
    static DUPLICATE_METHODS: &[MethodMetadata] = &SAME_NAME_METHODS;
    static EFFECTIVE_METHOD: EffectiveMethod = EffectiveMethod {
        target: TypeMetadata::of::<String>,
        metadata: &METHODS_ARRAY[0],
        source: MethodImplementationSource::Declared,
        availability: crate::runtime::descriptor::method::MethodAvailability::Callable,
        specializations: &[],
        specialization_specs: &[],
        adapters: None,
    };
    static EFFECTIVE_METHODS: &[EffectiveMethod] = &[EFFECTIVE_METHOD];
    static DECLARED_TYPE: AssociatedTypeBindingMetadata = AssociatedTypeBindingMetadata {
        definition: &ITEM_TYPE,
        expression: &TYPE_EXPRESSION,
        concrete: Some(TypeMetadata::of::<String>),
        source: AssociatedItemSource::Declared,
    };
    static UNAVAILABLE_CONST: AssociatedConstBindingMetadata = AssociatedConstBindingMetadata {
        definition: &ITEM_CONST,
        expression: crate::runtime::expression::ConstExpression::UnsignedInteger(5),
        source: AssociatedItemSource::RequiredButUnavailable,
        value: None,
    };
    static TYPE_BINDINGS: &[AssociatedTypeBindingMetadata] = &[DECLARED_TYPE];
    static CONST_BINDINGS: &[AssociatedConstBindingMetadata] = &[UNAVAILABLE_CONST];
    static INHERENT_DEFINITION: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<InherentImpl>(
        ImplKind::Inherent,
        &TARGET,
        None,
        &NO_GENERICS,
        ImplQualifiers::new(false),
        METHODS,
        NO_ASSOCIATED_TYPES,
        NO_ASSOCIATED_CONSTS,
    );
    static DUPLICATE_DEFINITION: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<DuplicateMethodsImpl>(
        ImplKind::Inherent,
        &TARGET,
        None,
        &NO_GENERICS,
        ImplQualifiers::new(false),
        DUPLICATE_METHODS,
        NO_ASSOCIATED_TYPES,
        NO_ASSOCIATED_CONSTS,
    );
    static TRAIT_DEFINITION_IMPL: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<TraitImpl>(
        ImplKind::Trait,
        &TARGET,
        Some(&TRAIT_EXPRESSION_METADATA),
        &NO_GENERICS,
        ImplQualifiers::new(false),
        METHODS,
        NO_ASSOCIATED_TYPES,
        NO_ASSOCIATED_CONSTS,
    );
    static ASSOCIATED_ITEMS_DEFINITION: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<AssociatedItemsImpl>(
        ImplKind::Inherent,
        &TARGET,
        None,
        &NO_GENERICS,
        ImplQualifiers::new(false),
        &[],
        &[ITEM_TYPE],
        &[ITEM_CONST],
    );
    static APPLICATION: ImplApplicationMetadata = ImplApplicationMetadata {
        definition: &INHERENT_DEFINITION,
        target: TypeMetadata::of::<String>,
        implemented_trait: None,
        arguments: &[],
        identity: &[],
        methods: EFFECTIVE_METHODS,
        associated_types: TYPE_BINDINGS,
        associated_consts: CONST_BINDINGS,
    };
    static TRAIT_APPLICATION_IMPL: ImplApplicationMetadata = ImplApplicationMetadata {
        definition: &TRAIT_DEFINITION_IMPL,
        target: TypeMetadata::of::<String>,
        implemented_trait: Some(&TRAIT_APPLICATION),
        arguments: &[],
        identity: &[],
        methods: EFFECTIVE_METHODS,
        associated_types: TYPE_BINDINGS,
        associated_consts: CONST_BINDINGS,
    };
    static EMPTY_BINDINGS_APPLICATION: ImplApplicationMetadata = ImplApplicationMetadata {
        associated_types: &[],
        associated_consts: &[],
        ..APPLICATION
    };
    static EMPTY_METHODS_APPLICATION: ImplApplicationMetadata = ImplApplicationMetadata {
        methods: &[],
        ..APPLICATION
    };

    fn context() -> ReflectionContext {
        ReflectionContext::builder().build().expect("empty context builds")
    }

    #[test]
    fn impl_descriptors_expose_kind_source_and_method_namespace() {
        let context = context();
        let definition = ImplDefinitionDescriptor::new(&INHERENT_DEFINITION, &context);
        assert_eq!(definition.kind(), ImplKind::Inherent);
        assert_eq!(definition.context().id(), context.id());
        assert_eq!(definition.id(), INHERENT_DEFINITION.id);
        assert!(definition.target().display().to_string().contains("fixture::Target"));
        assert!(definition.implemented_trait().is_none());
        assert!(!definition.qualifiers().is_unsafe);
        assert_eq!(definition.methods().len(), 1);
        assert_eq!(definition.methods().get("run").expect("method").name(), "run");

        let application = ImplDescriptor::new(&APPLICATION, &context);
        assert_eq!(application.kind(), ImplKind::Inherent);
        assert_eq!(application.context().id(), context.id());
        assert_eq!(application.definition().id(), definition.id());
        assert!(application.arguments().is_empty());
        assert_eq!(application.target().type_id(), std::any::TypeId::of::<String>());
        assert!(application.implemented_trait().is_none());
        let method = application.method("run").expect("method is available");
        assert!(matches!(
            method.namespace(),
            crate::runtime::descriptor::method::MethodNamespace::Inherent { .. }
        ));
    }

    #[test]
    fn associated_bindings_preserve_source_expression_and_resolution() {
        let context = context();
        let application = ImplDescriptor::new(&APPLICATION, &context);
        let type_binding = application.associated_type("Item").expect("type binding");
        assert_eq!(type_binding.source(), AssociatedItemSource::Declared);
        assert_eq!(type_binding.definition().name(), "Item");
        assert!(
            type_binding
                .expression()
                .display()
                .to_string()
                .contains("alloc::string::String")
        );
        assert_eq!(
            type_binding
                .find_concrete_type()
                .expect("registered concrete type")
                .type_id(),
            std::any::TypeId::of::<String>()
        );

        let const_binding = application.associated_const("COUNT").expect("const binding");
        assert_eq!(const_binding.source(), AssociatedItemSource::RequiredButUnavailable);
        assert_eq!(
            const_binding.expression(),
            crate::runtime::expression::ConstExpression::UnsignedInteger(5)
        );
        assert!(const_binding.value::<Local>().expect("local adapter lookup").is_none());
        assert!(
            const_binding
                .value::<ThreadSafe>()
                .expect("thread-safe adapter lookup")
                .is_none()
        );

        assert_eq!(application.associated_types().len(), 1);
        assert!(!application.associated_types().is_empty());
        assert_eq!(application.associated_types().iter().count(), 1);
        assert!(application.associated_types().get("Missing").is_none());

        assert_eq!(application.associated_consts().len(), 1);
        assert!(!application.associated_consts().is_empty());
        assert_eq!(application.associated_consts().iter().count(), 1);
        assert!(application.associated_consts().get("MISSING").is_none());

        let no_bindings = ImplDescriptor::new(&EMPTY_BINDINGS_APPLICATION, &context);
        assert!(no_bindings.associated_types().is_empty());
        assert!(no_bindings.associated_consts().is_empty());
    }

    #[test]
    fn impl_collections_preserve_order_and_bounds() {
        let context = context();
        let definition = ImplDefinitionDescriptor::new(&INHERENT_DEFINITION, &context);
        let definitions = ImplDefinitions::new(vec![definition].into_boxed_slice(), &context);
        assert_eq!(definitions.context().id(), context.id());
        assert_eq!(definitions.len(), 1);
        assert!(!definitions.is_empty());
        assert_eq!(definitions.iter().count(), 1);
        assert_eq!(definitions.get(0).expect("first definition").id(), definition.id());
        assert!(definitions.get(1).is_none());

        let applications = Impls::new(
            vec![ImplDescriptor::new(&APPLICATION, &context)].into_boxed_slice(),
            &context,
        );
        assert_eq!(applications.context().id(), context.id());
        assert_eq!(applications.len(), 1);
        assert!(!applications.is_empty());
        assert_eq!(applications.iter().count(), 1);
        assert!(applications.get(0).is_some());
        assert!(applications.get(1).is_none());

        let empty_definitions = ImplDefinitions::new(Box::new([]), &context);
        assert!(empty_definitions.is_empty());
        assert_eq!(empty_definitions.len(), 0);
        let empty_applications = Impls::new(Box::new([]), &context);
        assert!(empty_applications.is_empty());
        assert_eq!(empty_applications.len(), 0);
    }

    #[test]
    fn impl_associated_declaration_collections_support_iteration_and_lookup() {
        let context = context();
        let definition = ImplDefinitionDescriptor::new(&ASSOCIATED_ITEMS_DEFINITION, &context);
        assert_eq!(definition.associated_types().len(), 1);
        assert!(!definition.associated_types().is_empty());
        assert_eq!(definition.associated_types().iter().count(), 1);
        assert_eq!(
            definition
                .associated_types()
                .get("Item")
                .expect("associated type")
                .name(),
            "Item"
        );
        assert!(definition.associated_types().get("Missing").is_none());

        assert_eq!(definition.associated_consts().len(), 1);
        assert!(!definition.associated_consts().is_empty());
        assert_eq!(definition.associated_consts().iter().count(), 1);
        assert_eq!(
            definition
                .associated_consts()
                .get("COUNT")
                .expect("associated constant")
                .name(),
            "COUNT"
        );
        assert!(definition.associated_consts().get("Missing").is_none());

        let empty = ImplDefinitionDescriptor::new(&INHERENT_DEFINITION, &context);
        assert!(empty.associated_types().is_empty());
        assert!(empty.associated_consts().is_empty());
    }

    #[test]
    fn method_collection_exposes_lookup_and_empty_state() {
        let context = context();
        let implementation = ImplDescriptor::new(&APPLICATION, &context);
        let methods = implementation.methods();
        assert_eq!(methods.context().id(), context.id());
        assert_eq!(methods.len(), 1);
        assert!(!methods.is_empty());
        assert_eq!(methods.iter().count(), 1);
        assert_eq!(methods.get("run").expect("method exists").name(), "run");
        assert!(methods.get("missing").is_none());

        let empty_implementation = ImplDescriptor::new(&EMPTY_METHODS_APPLICATION, &context);
        let empty_methods = empty_implementation.methods();
        assert!(empty_methods.is_empty());
        assert_eq!(empty_methods.len(), 0);
        assert_eq!(empty_methods.iter().count(), 0);
    }

    #[test]
    fn trait_impl_collection_preserves_lookup_results_and_context_errors() {
        let ctx = context();
        let trait_definition =
            crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor::new(&TRAIT_DEFINITION, &ctx);
        let trait_application = TraitDescriptor::new(&TRAIT_APPLICATION, &ctx);
        let implementation = ImplDescriptor::new(&TRAIT_APPLICATION_IMPL, &ctx);
        let implementations = Impls::new(vec![implementation].into_boxed_slice(), &ctx);

        assert!(matches!(
            implementations.lookup_trait_definition(trait_definition),
            Ok(Lookup::Unique(found)) if found.same_application(implementation)
        ));
        assert!(matches!(
            implementations.find_trait_application(trait_application),
            Ok(Some(found)) if found.same_application(implementation)
        ));
        let empty = Impls::new(Box::new([]), &ctx);
        assert!(matches!(
            empty.lookup_trait_definition(trait_definition),
            Ok(Lookup::Missing)
        ));
        assert!(matches!(empty.find_trait_application(trait_application), Ok(None)));

        let duplicated = Impls::new(vec![implementation, implementation].into_boxed_slice(), &ctx);
        assert!(matches!(
            duplicated.lookup_trait_definition(trait_definition),
            Ok(Lookup::Ambiguous(candidates)) if candidates.len() == 2
        ));
        assert!(duplicated.find_trait_application(trait_application).is_err());

        let foreign_context = context();
        let foreign_definition = crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor::new(
            &TRAIT_DEFINITION,
            &foreign_context,
        );
        let foreign_application = TraitDescriptor::new(&TRAIT_APPLICATION, &foreign_context);
        assert!(implementations.lookup_trait_definition(foreign_definition).is_err());
        assert!(implementations.find_trait_application(foreign_application).is_err());
    }

    #[test]
    fn impl_lookup_reports_missing_and_ambiguous_names() {
        let context = context();
        let missing_definition = ImplDefinitionDescriptor::new(&INHERENT_DEFINITION, &context);
        assert!(matches!(
            missing_definition.method("missing"),
            Err(LookupError::Missing { subject: LookupSubject::Method, query: LookupQuery::Name(name) })
                if name.as_ref() == "missing"
        ));
        assert!(matches!(
            missing_definition.associated_type("missing"),
            Err(LookupError::Missing {
                subject: LookupSubject::AssociatedType,
                ..
            })
        ));
        assert!(matches!(
            missing_definition.associated_const("missing"),
            Err(LookupError::Missing {
                subject: LookupSubject::AssociatedConst,
                ..
            })
        ));
        assert!(matches!(
            ImplDescriptor::new(&APPLICATION, &context).method("missing"),
            Err(LookupError::Missing {
                subject: LookupSubject::Method,
                ..
            })
        ));

        let duplicate = ImplDefinitionDescriptor::new(&DUPLICATE_DEFINITION, &context);
        assert!(matches!(
            duplicate.method("run"),
            Err(LookupError::Ambiguous { subject: LookupSubject::Method, candidates, .. })
                if candidates.len() == 2
        ));
    }
}
