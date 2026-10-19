use std::any::TypeId;
use std::hash::{Hash, Hasher};

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::{GenericArgumentMetadata, GenericDefinitionMetadata};
use crate::runtime::descriptor::metadata::TypeMetadata;
use crate::runtime::descriptor::method::{EffectiveMethod, MethodMetadata};
use crate::runtime::descriptor::trait_descriptor::{
    TraitApplicationMetadata, TraitExpressionMetadata,
};
use crate::runtime::expression::ExpressionData;
use crate::runtime::value::{DynamicOwned, Local, ThreadSafe};

/// Process-local identity of one source `impl` block.
#[derive(Clone, Copy)]
pub(crate) struct ImplDefinitionId(fn() -> TypeId);

impl ImplDefinitionId {
    /// Creates an identity from a generated marker type.
    pub(crate) const fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>)
    }
    pub(crate) fn type_id(self) -> TypeId {
        (self.0)()
    }
}

impl std::fmt::Debug for ImplDefinitionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id().fmt(formatter)
    }
}
impl PartialEq for ImplDefinitionId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}
impl Eq for ImplDefinitionId {}
impl Hash for ImplDefinitionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id().hash(state);
    }
}

/// Whether an implementation block is inherent or implements a trait.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ImplKind {
    Inherent,
    Trait,
}

/// Qualifiers retained from an implementation block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ImplQualifiers {
    pub(crate) is_unsafe: bool,
}

/// Origin of one final associated item binding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AssociatedItemSource {
    Declared,
    Defaulted,
    Overridden,
    RequiredButUnavailable,
}

/// Static metadata for a concrete associated type binding.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedTypeBindingMetadata {
    pub(crate) definition:
        &'static crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata,
    pub(crate) expression: &'static ExpressionData,
    pub(crate) concrete: Option<&'static TypeMetadata>,
    pub(crate) source: AssociatedItemSource,
}

/// Static metadata for a concrete associated constant binding.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedConstBindingMetadata {
    pub(crate) definition:
        &'static crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata,
    pub(crate) expression: crate::runtime::expression::ConstExpression,
    pub(crate) source: AssociatedItemSource,
    pub(crate) value: Option<&'static AssociatedConstValueAdapter>,
}

/// Mode-specific constructor for a statically known associated constant value.
pub(crate) struct AssociatedConstValueAdapter {
    pub(crate) local: Option<fn() -> DynamicOwned<Local>>,
    pub(crate) thread_safe: Option<fn() -> DynamicOwned<ThreadSafe>>,
}

/// Static source facts for an `impl` declaration and context-visible applications.
pub(crate) struct ImplDefinitionMetadata {
    pub(crate) id: ImplDefinitionId,
    pub(crate) kind: ImplKind,
    pub(crate) target: &'static ExpressionData,
    pub(crate) implemented_trait: Option<&'static TraitExpressionMetadata>,
    pub(crate) generics: &'static GenericDefinitionMetadata,
    pub(crate) qualifiers: ImplQualifiers,
    pub(crate) methods: &'static [MethodMetadata],
    pub(crate) associated_types: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata],
    pub(crate) associated_consts: &'static [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata],
    pub(crate) instances: for<'ctx> fn(&'ctx ReflectionContext) -> Result<Box<[&'static ImplApplicationMetadata]>, crate::runtime::error::ContextError>,
}

/// Static metadata for one compiled implementation application.
pub(crate) struct ImplApplicationMetadata {
    pub(crate) definition: &'static ImplDefinitionMetadata,
    pub(crate) target: &'static TypeMetadata,
    pub(crate) implemented_trait: Option<&'static TraitApplicationMetadata>,
    pub(crate) arguments: &'static [GenericArgumentMetadata],
    pub(crate) identity:
        &'static [crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
    pub(crate) methods: &'static [EffectiveMethod],
    pub(crate) associated_types: &'static [AssociatedTypeBindingMetadata],
    pub(crate) associated_consts: &'static [AssociatedConstBindingMetadata],
}
