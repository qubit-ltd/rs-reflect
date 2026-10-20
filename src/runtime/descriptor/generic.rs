// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;

use crate::runtime::capability::Capabilities;
use crate::runtime::capability::Capability;
use crate::runtime::capability::CapabilityError;
use crate::runtime::capability::CapabilitySpec;
use crate::runtime::capability::CapabilityTarget;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::metadata::TypeMetadata;
use crate::runtime::error::ContextError;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::expression::ConstExpression;
use crate::runtime::expression::ExpressionData;
use crate::runtime::expression::GenericArgumentError;
use crate::runtime::expression::GenericArgumentKind;
use crate::runtime::expression::LifetimeExpression;
use crate::runtime::expression::PredicateData;
use crate::runtime::expression::Predicates;
use crate::runtime::expression::TypeExpression;
use crate::runtime::expression::TypeExpressionError;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;

/// Process-local identity of one generic source declaration.
#[derive(Clone, Copy)]
pub struct TypeDefinitionId(fn() -> TypeId);

impl TypeDefinitionId {
    /// Creates an identity from a generated marker type.
    pub(crate) const fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>)
    }

    /// Returns this declaration's process-local marker identity.
    pub(crate) fn type_id(self) -> TypeId {
        (self.0)()
    }
}

impl std::fmt::Debug for TypeDefinitionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.type_id().fmt(formatter)
    }
}

impl PartialEq for TypeDefinitionId {
    fn eq(&self, other: &Self) -> bool {
        self.type_id() == other.type_id()
    }
}

impl Eq for TypeDefinitionId {}

impl Hash for TypeDefinitionId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id().hash(state);
    }
}

/// Source kind of a generic type definition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypeDefinitionKind {
    /// A struct declaration, including its named, tuple, or unit shape.
    Struct(StructKind),
    /// An enum declaration.
    Enum,
    /// A union declaration.
    Union,
    /// A declaration whose source shape is not otherwise retained.
    Opaque,
}

/// Struct shape retained as source metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructKind {
    /// A struct with named fields.
    Named,
    /// A tuple struct with positional fields.
    Tuple,
    /// A unit struct without fields.
    Unit,
}

/// Static source metadata for one generic declaration.
#[derive(Clone, Copy)]
pub struct TypeDefinitionMetadata {
    pub(crate) id: TypeDefinitionId,
    pub(crate) name: &'static str,
    pub(crate) path: &'static str,
    pub(crate) kind: TypeDefinitionKind,
    pub(crate) generics: &'static GenericDefinitionMetadata,
}

/// Generic declaration metadata in source order.
#[derive(Clone, Copy)]
pub struct GenericDefinitionMetadata {
    pub(crate) parameters: &'static [GenericParameterMetadata],
    pub(crate) predicates: &'static [PredicateData],
}

/// One lifetime, type, or const parameter declared by a generic definition.
#[derive(Clone, Copy)]
pub struct GenericParameterMetadata {
    pub(crate) name: &'static str,
    pub(crate) index: usize,
    pub(crate) kind: GenericParameterMetadataKind,
}

/// Static facts specific to the parameter kind.
#[derive(Clone, Copy)]
pub enum GenericParameterMetadataKind {
    /// Metadata for a lifetime parameter.
    Lifetime {
        /// Lifetime bounds declared for this parameter.
        bounds: &'static [LifetimeExpression],
    },
    /// Metadata for a type parameter.
    Type {
        /// Predicates constraining this type parameter.
        bounds: &'static [PredicateData],
        /// The declared default type, when present.
        default: Option<&'static ExpressionData>,
    },
    /// Metadata for a const parameter.
    Const {
        /// The declared type of the const parameter.
        ty: &'static ExpressionData,
        /// The declared default value, when present.
        default: Option<ConstExpression>,
    },
}

impl TypeDefinitionMetadata {
    /// Creates static metadata bound to a marker type chosen by generated code.
    pub const fn new<T: 'static>(
        name: &'static str,
        path: &'static str,
        kind: TypeDefinitionKind,
        generics: &'static GenericDefinitionMetadata,
    ) -> Self {
        Self {
            id: TypeDefinitionId::of::<T>(),
            name,
            path,
            kind,
            generics,
        }
    }

    /// Returns the declaration's generic parameters in source order.
    #[doc(hidden)]
    pub const fn parameters(&self) -> &'static [GenericParameterMetadata] {
        self.generics.parameters
    }
}

impl GenericDefinitionMetadata {
    /// Creates static generic parameters and predicates in source order.
    pub const fn new(parameters: &'static [GenericParameterMetadata], predicates: &'static [PredicateData]) -> Self {
        Self { parameters, predicates }
    }

    /// Returns the declaration's generic parameters in source order.
    #[doc(hidden)]
    pub const fn parameters(&self) -> &'static [GenericParameterMetadata] {
        self.parameters
    }
}

impl GenericParameterMetadata {
    /// Creates a lifetime parameter and its source bounds.
    pub const fn lifetime(name: &'static str, index: usize, bounds: &'static [LifetimeExpression]) -> Self {
        Self {
            name,
            index,
            kind: GenericParameterMetadataKind::Lifetime { bounds },
        }
    }

    /// Creates a type parameter and its source bounds and optional default.
    pub const fn type_parameter(
        name: &'static str,
        index: usize,
        bounds: &'static [PredicateData],
        default: Option<&'static ExpressionData>,
    ) -> Self {
        Self {
            name,
            index,
            kind: GenericParameterMetadataKind::Type { bounds, default },
        }
    }

    /// Creates a const parameter and its source type and optional default.
    pub const fn const_parameter(
        name: &'static str,
        index: usize,
        ty: &'static ExpressionData,
        default: Option<ConstExpression>,
    ) -> Self {
        Self {
            name,
            index,
            kind: GenericParameterMetadataKind::Const { ty, default },
        }
    }
}

/// Context-bound view of a generic type declaration.
#[derive(Clone, Copy)]
pub struct TypeDefinitionDescriptor<'ctx> {
    metadata: &'static TypeDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TypeDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(metadata: &'static TypeDefinitionMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context that owns this definition.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the process-local identity of this declaration.
    pub const fn id(self) -> TypeDefinitionId {
        self.metadata.id
    }
    /// Returns the declaration's short source name.
    pub fn name(self) -> &'static str {
        self.metadata.name
    }
    /// Returns the retained source path of the declaration.
    pub fn path(self) -> &'static str {
        self.metadata.path
    }
    /// Returns the source kind of the declaration.
    pub fn kind(self) -> TypeDefinitionKind {
        self.metadata.kind
    }
    /// Returns the declaration's generic parameters and predicates.
    pub fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
    /// Returns registered concrete applications of this generic declaration.
    pub fn instances(self) -> Result<Box<[TypeDescriptor<'ctx>]>, ContextError> {
        self.context.type_instances(self.metadata.id)
    }

    /// Returns the immutable capability view registered for this definition.
    pub fn capabilities(self) -> Result<Capabilities<'ctx>, CapabilityError> {
        let target = CapabilityTarget::Definition(self);
        self.context
            .capabilities_for_definition(self.id())
            .map(|capabilities| match capabilities {
                Some(capabilities) => Capabilities::new(capabilities, target, self.context()),
                None => Capabilities::empty(target, self.context()),
            })
            .map_err(CapabilityError::Context)
    }

    /// Finds a capability using its statically declared adapter contract.
    pub fn find_capability<C: CapabilitySpec>(self) -> Result<Option<Capability<'ctx, C>>, CapabilityError> {
        self.capabilities()?.find::<C>()
    }

    /// Returns whether this definition has a capability matching the full
    /// adapter contract.
    ///
    /// Returns an error when the capability ID exists with a different adapter
    /// type.
    pub fn has_capability<C: CapabilitySpec>(self) -> Result<bool, CapabilityError> {
        self.find_capability::<C>().map(|capability| capability.is_some())
    }

    /// Requires a capability using its statically declared adapter contract.
    pub fn capability<C: CapabilitySpec>(self) -> Result<Capability<'ctx, C>, CapabilityError> {
        self.capabilities()?.capability::<C>()
    }
}

/// Context-bound view of a declaration's generic parameters and predicates.
#[derive(Clone, Copy)]
pub struct GenericDefinitionDescriptor<'ctx> {
    metadata: &'static GenericDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(metadata: &'static GenericDefinitionMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context that owns this generic definition.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over generic parameters in declaration order.
    pub fn parameters(self) -> impl ExactSizeIterator<Item = GenericParameterDescriptor<'ctx>> + 'ctx {
        self.metadata
            .parameters
            .iter()
            .map(move |parameter| GenericParameterDescriptor::new(parameter, self.context))
    }
    /// Looks up a generic parameter by name.
    pub fn parameter(self, name: &str) -> Result<GenericParameterDescriptor<'ctx>, LookupError> {
        let mut candidates = self
            .metadata
            .parameters
            .iter()
            .filter(|parameter| parameter.name == name);
        let Some(first) = candidates.next() else {
            return Err(LookupError::Missing {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
            });
        };
        let mut matches = vec![GenericParameterDescriptor::new(first, self.context)];
        matches.extend(candidates.map(|candidate| GenericParameterDescriptor::new(candidate, self.context)));
        match matches.as_slice() {
            [only] => Ok(*only),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
                candidates: matches
                    .into_iter()
                    .map(GenericParameterDescriptor::lookup_candidate)
                    .collect(),
            }),
        }
    }
    /// Returns the declared predicates for this generic definition.
    pub fn predicates(self) -> Predicates<'ctx> {
        Predicates::new(self.metadata.predicates, self.context)
    }
    /// Returns whether the declaration has no generic parameters or predicates.
    pub fn is_empty(self) -> bool {
        self.metadata.parameters.is_empty()
    }
}

/// Context-bound view of one declared generic parameter.
#[derive(Clone, Copy)]
pub struct GenericParameterDescriptor<'ctx> {
    metadata: &'static GenericParameterMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericParameterDescriptor<'ctx> {
    pub(crate) const fn new(metadata: &'static GenericParameterMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context that owns this parameter descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the parameter's source name.
    pub fn name(self) -> &'static str {
        self.metadata.name
    }
    /// Returns the parameter's zero-based position among generic parameters.
    pub fn index(self) -> usize {
        self.metadata.index
    }
    /// Returns the facts specific to the parameter kind.
    pub fn kind(self) -> GenericParameterKind<'ctx> {
        match self.metadata.kind {
            GenericParameterMetadataKind::Lifetime { bounds } => GenericParameterKind::Lifetime { bounds },
            GenericParameterMetadataKind::Type { bounds, default } => GenericParameterKind::Type {
                bounds: Predicates::new(bounds, self.context),
                default: default.map(|expression| TypeExpression::new(expression, self.context)),
            },
            GenericParameterMetadataKind::Const { ty, default } => GenericParameterKind::Const {
                ty: TypeExpression::new(ty, self.context),
                default,
            },
        }
    }
    fn lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.metadata.name.into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Parameter,
                stable_id: StableItemId(format!("{}:{}", self.metadata.index, self.metadata.name).into()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

/// Kind-specific facts for a declared generic parameter.
pub enum GenericParameterKind<'ctx> {
    /// Bounds declared for a lifetime parameter.
    Lifetime {
        /// Bounds declared for the lifetime parameter.
        bounds: &'static [LifetimeExpression],
    },
    /// Bounds and optional default declared for a type parameter.
    Type {
        /// Predicates constraining the type parameter.
        bounds: Predicates<'ctx>,
        /// Default type expression, if declared.
        default: Option<TypeExpression<'ctx>>,
    },
    /// Type and optional value default declared for a const parameter.
    Const {
        /// Type expression of the const parameter.
        ty: TypeExpression<'ctx>,
        /// Default const expression, if declared.
        default: Option<ConstExpression>,
    },
}

/// Static concrete type or const argument associated with a parameter.
#[derive(Clone, Copy)]
pub struct GenericArgumentMetadata {
    pub(crate) parameter: Option<&'static GenericParameterMetadata>,
    pub(crate) kind: GenericArgumentMetadataKind,
}

/// Payload of a concrete generic argument.
#[derive(Clone, Copy)]
pub(crate) enum GenericArgumentMetadataKind {
    Type {
        expression: &'static ExpressionData,
        concrete: Option<fn() -> &'static TypeMetadata>,
    },
    Const(ConstExpression),
}

fn reflected_type_metadata<T: super::Reflect + ?Sized>() -> &'static TypeMetadata {
    T::__reflect_registration().metadata()
}

impl GenericArgumentMetadata {
    /// Creates a type argument tied to a reflected concrete Rust type.
    pub const fn type_argument<T: ?Sized + 'static>(
        parameter: &'static GenericParameterMetadata,
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            parameter: Some(parameter),
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: Some(TypeMetadata::of::<T>),
            },
        }
    }

    /// Creates a type argument while retaining source syntax without requiring
    /// reflection.
    pub const fn type_expression(
        parameter: &'static GenericParameterMetadata,
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            parameter: Some(parameter),
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: None,
            },
        }
    }

    pub(crate) const fn concrete_type(
        parameter: &'static GenericParameterMetadata,
        type_metadata: fn() -> &'static TypeMetadata,
    ) -> Self {
        Self {
            parameter: Some(parameter),
            kind: GenericArgumentMetadataKind::Type {
                expression: &OPAQUE_TYPE_EXPRESSION,
                concrete: Some(type_metadata),
            },
        }
    }

    /// Creates a const argument associated with its declared parameter.
    pub const fn const_argument(parameter: &'static GenericParameterMetadata, value: ConstExpression) -> Self {
        Self {
            parameter: Some(parameter),
            kind: GenericArgumentMetadataKind::Const(value),
        }
    }

    /// Creates a positional type argument when the declaration is unknown.
    pub const fn positional_type_argument<T: ?Sized + 'static>(expression: &'static ExpressionData) -> Self {
        Self {
            parameter: None,
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: Some(TypeMetadata::of::<T>),
            },
        }
    }

    pub(crate) const fn reflected_type_argument<T: super::Reflect + ?Sized>(
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            parameter: None,
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: Some(reflected_type_metadata::<T>),
            },
        }
    }

    /// Creates a positional type argument that retains source syntax without
    /// claiming that the type is concrete or reflected.
    pub const fn positional_type_expression(expression: &'static ExpressionData) -> Self {
        Self {
            parameter: None,
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: None,
            },
        }
    }

    /// Creates a positional const argument when the declaration is unknown.
    pub const fn positional_const_argument(value: ConstExpression) -> Self {
        Self {
            parameter: None,
            kind: GenericArgumentMetadataKind::Const(value),
        }
    }
}

static OPAQUE_TYPE_EXPRESSION: ExpressionData =
    ExpressionData::Opaque(&crate::runtime::expression::OpaqueTypeData::new(&[]));

/// Context-bound collection of the type and const arguments of one application.
#[derive(Clone)]
pub struct GenericArguments<'ctx> {
    arguments: GenericArgumentMetadataSlice,
    context: &'ctx ReflectionContext,
}

#[derive(Clone)]
enum GenericArgumentMetadataSlice {
    Static(&'static [GenericArgumentMetadata]),
    Shared(Arc<[GenericArgumentMetadata]>),
}

impl GenericArgumentMetadataSlice {
    fn as_slice(&self) -> &[GenericArgumentMetadata] {
        match self {
            Self::Static(arguments) => arguments,
            Self::Shared(arguments) => arguments,
        }
    }
}

impl<'ctx> GenericArguments<'ctx> {
    pub(crate) const fn new(arguments: &'static [GenericArgumentMetadata], context: &'ctx ReflectionContext) -> Self {
        Self {
            arguments: GenericArgumentMetadataSlice::Static(arguments),
            context,
        }
    }

    pub(crate) fn from_shared(arguments: Arc<[GenericArgumentMetadata]>, context: &'ctx ReflectionContext) -> Self {
        Self {
            arguments: GenericArgumentMetadataSlice::Shared(arguments),
            context,
        }
    }
    /// Returns the context used to resolve argument type expressions.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Iterates over type and const arguments in application order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = GenericArgumentDescriptor<'ctx>> + '_ {
        self.arguments
            .as_slice()
            .iter()
            .copied()
            .map(|argument| GenericArgumentDescriptor::new(argument, self.context))
            .collect::<Vec<_>>()
            .into_iter()
    }
    /// Finds a uniquely named argument; positional arguments have no name.
    pub fn get(&self, name: &str) -> Option<GenericArgumentDescriptor<'ctx>> {
        let mut matches = self
            .arguments
            .as_slice()
            .iter()
            .filter(|argument| argument.parameter.is_some_and(|parameter| parameter.name == name));
        let first = *matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| GenericArgumentDescriptor::new(first, self.context))
    }
    /// Requires a uniquely named argument, returning missing or ambiguous
    /// lookup errors.
    pub fn argument(&self, name: &str) -> Result<GenericArgumentDescriptor<'ctx>, LookupError> {
        let matches = self
            .arguments
            .as_slice()
            .iter()
            .filter(|argument| argument.parameter.is_some_and(|parameter| parameter.name == name))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(LookupError::Missing {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
            }),
            [argument] => Ok(GenericArgumentDescriptor::new(**argument, self.context)),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
                candidates: matches
                    .iter()
                    .filter_map(|argument| argument.parameter)
                    .map(|parameter| GenericParameterDescriptor::new(parameter, self.context).lookup_candidate())
                    .collect(),
            }),
        }
    }
    /// Returns the type or const argument at its position, excluding lifetime
    /// parameters.
    pub fn get_at(&self, index: usize) -> Option<GenericArgumentDescriptor<'ctx>> {
        self.arguments
            .as_slice()
            .get(index)
            .copied()
            .map(|argument| GenericArgumentDescriptor::new(argument, self.context))
    }
    /// Returns the number of type and const arguments in this application.
    pub fn len(&self) -> usize {
        self.arguments.as_slice().len()
    }
    /// Returns whether this application has no type or const arguments.
    pub fn is_empty(&self) -> bool {
        self.arguments.as_slice().is_empty()
    }
}

/// A concrete type or const argument in one already compiled application.
#[derive(Clone, Copy)]
pub struct GenericArgumentDescriptor<'ctx> {
    metadata: GenericArgumentMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericArgumentDescriptor<'ctx> {
    const fn new(metadata: GenericArgumentMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the context used to resolve this argument.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Views this argument as a type argument.
    pub fn type_argument(self) -> Result<TypeArgumentDescriptor<'ctx>, crate::runtime::error::ReflectionError> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Type { .. } => Ok(TypeArgumentDescriptor {
                metadata: self.metadata,
                context: self.context,
            }),
            GenericArgumentMetadataKind::Const(_) => {
                Err(GenericArgumentError::new(GenericArgumentKind::Type, GenericArgumentKind::Const).into())
            }
        }
    }
    /// Views this argument as a const argument.
    pub fn const_argument(self) -> Result<ConstArgumentDescriptor<'ctx>, crate::runtime::error::ReflectionError> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Const(_) => Ok(ConstArgumentDescriptor {
                metadata: self.metadata,
                _context: self.context,
            }),
            GenericArgumentMetadataKind::Type { .. } => {
                Err(GenericArgumentError::new(GenericArgumentKind::Const, GenericArgumentKind::Type).into())
            }
        }
    }
}

/// A type argument, preserving syntax when no reflected descriptor exists.
#[derive(Clone, Copy)]
pub struct TypeArgumentDescriptor<'ctx> {
    metadata: GenericArgumentMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TypeArgumentDescriptor<'ctx> {
    /// Returns the context used to resolve this argument.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the declaration parameter, or `None` for positional external
    /// arguments.
    pub fn parameter(self) -> Option<GenericParameterDescriptor<'ctx>> {
        self.metadata
            .parameter
            .map(|parameter| GenericParameterDescriptor::new(parameter, self.context))
    }
    /// Returns the retained type expression.
    pub fn expression(self) -> TypeExpression<'ctx> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Type { expression, .. } => TypeExpression::new(expression, self.context),
            GenericArgumentMetadataKind::Const(_) => unreachable!("type argument kind was checked"),
        }
    }
    /// Finds a concrete type descriptor when the argument's type is registered.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Type { concrete, .. } => {
                concrete.map(|metadata| TypeDescriptor::new(metadata(), self.context))
            }
            GenericArgumentMetadataKind::Const(_) => None,
        }
    }
    /// Resolves the concrete type or reports why no descriptor is available.
    pub fn concrete_type(self) -> Result<TypeDescriptor<'ctx>, crate::runtime::error::ReflectionError> {
        self.find_concrete_type().ok_or_else(|| {
            TypeExpressionError::ConcreteDescriptorUnavailable {
                parameter: self
                    .metadata
                    .parameter
                    .map_or("<unknown positional parameter>", |parameter| parameter.name),
            }
            .into()
        })
    }
}

/// A const argument retained as a structural expression.
#[derive(Clone, Copy)]
pub struct ConstArgumentDescriptor<'ctx> {
    metadata: GenericArgumentMetadata,
    _context: &'ctx ReflectionContext,
}

impl<'ctx> ConstArgumentDescriptor<'ctx> {
    /// Returns the declaration parameter, or `None` for positional external
    /// arguments.
    pub fn parameter(self) -> Option<GenericParameterDescriptor<'ctx>> {
        self.metadata
            .parameter
            .map(|parameter| GenericParameterDescriptor::new(parameter, self._context))
    }
    /// Returns the structural const expression retained for this argument.
    pub fn expression(self) -> ConstExpression {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Const(expression) => expression,
            GenericArgumentMetadataKind::Type { .. } => {
                unreachable!("const argument kind was checked")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    use std::sync::Arc;

    use super::CapabilityError;
    use super::GenericArgumentDescriptor;
    use super::GenericArgumentMetadata;
    use super::GenericArguments;
    use super::GenericDefinitionDescriptor;
    use super::GenericDefinitionMetadata;
    use super::GenericParameterKind;
    use super::GenericParameterMetadata;
    use super::StructKind;
    use super::TypeDefinitionDescriptor;
    use super::TypeDefinitionId;
    use super::TypeDefinitionKind;
    use super::TypeDefinitionMetadata;
    use super::TypeMetadata;
    use crate::ReflectionContext;
    use crate::runtime::expression::ConstExpression;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::TypeExpressionError;

    struct Marker;
    struct OtherMarker;

    static TYPE_DEFAULT: ExpressionData = ExpressionData::ParameterName("U");
    static CONST_TYPE: ExpressionData = ExpressionData::Named {
        path: "usize",
        definition: None,
        arguments: &[],
    };
    static TYPE_PARAMETER: GenericParameterMetadata =
        GenericParameterMetadata::type_parameter("T", 0, &[], Some(&TYPE_DEFAULT));
    static LIFETIME_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::lifetime("a", 0, &[]);
    static CONST_PARAMETER: GenericParameterMetadata =
        GenericParameterMetadata::const_parameter("N", 1, &CONST_TYPE, Some(ConstExpression::UnsignedInteger(8)));
    static PARAMETERS: [GenericParameterMetadata; 3] = [LIFETIME_PARAMETER, TYPE_PARAMETER, CONST_PARAMETER];
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&PARAMETERS, &[]);
    static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static DEFINITION: TypeDefinitionMetadata = TypeDefinitionMetadata::new::<Marker>(
        "Marker",
        "fixture::Marker",
        TypeDefinitionKind::Struct(StructKind::Unit),
        &GENERICS,
    );
    static DUPLICATE_ARGUMENTS: [GenericArgumentMetadata; 2] = [
        GenericArgumentMetadata::type_expression(&TYPE_PARAMETER, &TYPE_DEFAULT),
        GenericArgumentMetadata::type_expression(&TYPE_PARAMETER, &TYPE_DEFAULT),
    ];

    fn u8_metadata() -> &'static TypeMetadata {
        TypeMetadata::of::<u8>()
    }

    fn hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn definition_identity_uses_marker_type_for_equality_and_hashing() {
        let marker = TypeDefinitionId::of::<Marker>();
        let same_marker = TypeDefinitionId::of::<Marker>();
        let other_marker = TypeDefinitionId::of::<OtherMarker>();

        assert_eq!(marker, same_marker);
        assert_ne!(marker, other_marker);
        assert_eq!(hash(&marker), hash(&same_marker));
        assert_eq!(format!("{marker:?}"), format!("{same_marker:?}"));
    }

    #[test]
    fn definition_and_generic_descriptors_preserve_metadata_and_context() {
        let context = ReflectionContext::empty();
        let definition = TypeDefinitionDescriptor::new(&DEFINITION, &context);
        let generics = definition.generics();

        assert!(std::ptr::eq(definition.context(), &context));
        assert_eq!(definition.id(), TypeDefinitionId::of::<Marker>());
        assert_eq!(definition.name(), "Marker");
        assert_eq!(definition.path(), "fixture::Marker");
        assert_eq!(definition.kind(), TypeDefinitionKind::Struct(StructKind::Unit));
        assert!(std::ptr::eq(DEFINITION.parameters(), &PARAMETERS));
        assert!(std::ptr::eq(generics.context(), &context));
        assert_eq!(generics.parameters().len(), 3);
        assert!(!generics.is_empty());
        assert!(generics.predicates().is_empty());

        let empty = GenericDefinitionDescriptor::new(&EMPTY_GENERICS, &context);
        assert!(empty.is_empty());
        assert_eq!(empty.parameters().len(), 0);
        assert!(
            !definition
                .has_capability::<crate::runtime::capability::SendCapability>()
                .expect("empty definition capability lookup succeeds")
        );
        assert!(matches!(
            definition.capability::<crate::runtime::capability::SendCapability>(),
            Err(CapabilityError::Missing {
                capability_id: "qubit.reflect.send",
                ..
            })
        ));
    }

    #[test]
    fn generic_parameter_kinds_retain_bounds_defaults_and_declared_positions() {
        let context = ReflectionContext::empty();
        let generics = GenericDefinitionDescriptor::new(&GENERICS, &context);
        let lifetime = generics.parameter("a").expect("lifetime parameter exists");
        let type_parameter = generics.parameter("T").expect("type parameter exists");
        let const_parameter = generics.parameter("N").expect("const parameter exists");

        assert_eq!(lifetime.index(), 0);
        assert!(std::ptr::eq(lifetime.context(), &context));
        assert!(matches!(
            lifetime.kind(),
            GenericParameterKind::Lifetime { bounds } if bounds.is_empty()
        ));
        assert_eq!(type_parameter.index(), 0);
        assert!(matches!(
            type_parameter.kind(),
            GenericParameterKind::Type { bounds, default: Some(default) }
                if bounds.is_empty() && default.display().to_string() == "U"
        ));
        assert_eq!(const_parameter.index(), 1);
        assert!(matches!(
            const_parameter.kind(),
            GenericParameterKind::Const { ty, default: Some(ConstExpression::UnsignedInteger(8)) }
                if ty.display().to_string() == "usize"
        ));
    }

    #[test]
    fn positional_and_shared_arguments_preserve_argument_kind_and_source() {
        static POSITIONAL_TYPE: ExpressionData = ExpressionData::ParameterName("External");
        static NAMED_TYPE: ExpressionData = ExpressionData::ParameterName("T");
        let context = ReflectionContext::empty();
        let shared: Arc<[GenericArgumentMetadata]> = vec![
            GenericArgumentMetadata::positional_type_expression(&POSITIONAL_TYPE),
            GenericArgumentMetadata::const_argument(&CONST_PARAMETER, ConstExpression::UnsignedInteger(3)),
            GenericArgumentMetadata::type_expression(&TYPE_PARAMETER, &NAMED_TYPE),
        ]
        .into();
        let arguments = GenericArguments::from_shared(shared, &context);

        assert!(std::ptr::eq(arguments.context(), &context));
        assert_eq!(arguments.len(), 3);
        assert!(!arguments.is_empty());
        assert_eq!(arguments.iter().len(), 3);
        assert!(arguments.get("External").is_none());
        assert!(arguments.get("N").is_some());
        assert!(arguments.get_at(3).is_none());
        assert!(GenericArguments::new(&DUPLICATE_ARGUMENTS, &context).get("T").is_none());

        let positional = arguments
            .get_at(0)
            .expect("first positional type argument exists")
            .type_argument()
            .expect("first argument is a type");
        assert!(positional.parameter().is_none());
        assert!(std::ptr::eq(positional.context(), &context));
        assert_eq!(positional.expression().display().to_string(), "External");
        assert!(matches!(
            positional.concrete_type(),
            Err(crate::runtime::error::ReflectionError::TypeExpression(
                TypeExpressionError::ConcreteDescriptorUnavailable {
                    parameter: "<unknown positional parameter>"
                }
            ))
        ));

        let named = arguments
            .argument("T")
            .expect("named type argument exists")
            .type_argument()
            .expect("T is a type argument");
        assert_eq!(named.parameter().expect("T has a declaration").name(), "T");

        let constant = arguments
            .get_at(1)
            .expect("second const argument exists")
            .const_argument()
            .expect("second argument is a const");
        assert_eq!(constant.parameter().expect("N has a declaration").name(), "N");
        assert_eq!(constant.expression(), ConstExpression::UnsignedInteger(3));
    }

    #[test]
    fn concrete_and_unreflected_type_argument_constructors_keep_their_contracts() {
        static TYPE_EXPRESSION: ExpressionData = ExpressionData::ParameterName("T");
        let context = ReflectionContext::empty();
        let concrete = GenericArgumentMetadata::type_argument::<u8>(&TYPE_PARAMETER, &TYPE_EXPRESSION);
        let unresolved = GenericArgumentMetadata::type_expression(&TYPE_PARAMETER, &TYPE_EXPRESSION);
        let concrete_argument = GenericArgumentDescriptor::new(concrete, &context)
            .type_argument()
            .expect("type argument has type kind");

        assert_eq!(
            concrete_argument.parameter().expect("parameter is attached").name(),
            "T"
        );
        assert_eq!(
            concrete_argument
                .concrete_type()
                .expect("concrete type is known")
                .type_id(),
            TypeId::of::<u8>()
        );
        assert!(
            GenericArgumentDescriptor::new(unresolved, &context)
                .type_argument()
                .expect("unresolved argument is still a type")
                .find_concrete_type()
                .is_none()
        );

        let positional = GenericArgumentMetadata::positional_type_argument::<u16>(&TYPE_EXPRESSION);
        let positional_descriptor = GenericArgumentDescriptor::new(positional, &context)
            .type_argument()
            .expect("positional type constructor preserves type kind");
        assert!(positional_descriptor.parameter().is_none());
        assert_eq!(
            positional_descriptor
                .concrete_type()
                .expect("concrete positional type is known")
                .type_id(),
            TypeId::of::<u16>()
        );

        let positional_const = GenericArgumentMetadata::positional_const_argument(ConstExpression::Boolean(true));
        let const_descriptor = GenericArgumentDescriptor::new(positional_const, &context)
            .const_argument()
            .expect("positional const constructor preserves const kind");
        assert!(const_descriptor.parameter().is_none());
        assert_eq!(const_descriptor.expression(), ConstExpression::Boolean(true));

        let callback_type = GenericArgumentMetadata::concrete_type(&TYPE_PARAMETER, u8_metadata);
        let callback_descriptor = GenericArgumentDescriptor::new(callback_type, &context)
            .type_argument()
            .expect("metadata callback constructor yields a type argument");
        assert_eq!(
            callback_descriptor
                .concrete_type()
                .expect("metadata callback returns the reflected type")
                .type_id(),
            TypeId::of::<u8>()
        );
    }
}
