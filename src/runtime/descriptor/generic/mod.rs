use std::any::TypeId;
use std::hash::Hash;
use std::hash::Hasher;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::metadata::TypeMetadata;
use crate::runtime::error::{ContextError, LookupError, LookupQuery, LookupSubject};
use crate::runtime::expression::{
    ConstExpression, ExpressionData, GenericArgumentError, GenericArgumentKind, LifetimeExpression,
    PredicateData, Predicates, TypeExpression, TypeExpressionError,
};
use crate::runtime::identity::{ItemIdentity, ItemKind, LookupCandidate, StableItemId};

/// Process-local identity of one generic source declaration.
#[derive(Clone, Copy)]
pub(crate) struct TypeDefinitionId(fn() -> TypeId);

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
    Struct(StructKind),
    Enum,
    Union,
    Opaque,
}

/// Struct shape retained as source metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructKind {
    Named,
    Tuple,
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
    Lifetime {
        bounds: &'static [LifetimeExpression],
    },
    Type {
        bounds: &'static [PredicateData],
        default: Option<&'static ExpressionData>,
    },
    Const {
        ty: &'static ExpressionData,
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
}

impl GenericDefinitionMetadata {
    /// Creates static generic parameters and predicates in source order.
    pub const fn new(
        parameters: &'static [GenericParameterMetadata],
        predicates: &'static [PredicateData],
    ) -> Self {
        Self {
            parameters,
            predicates,
        }
    }
}

impl GenericParameterMetadata {
    /// Creates a lifetime parameter and its source bounds.
    pub const fn lifetime(
        name: &'static str,
        index: usize,
        bounds: &'static [LifetimeExpression],
    ) -> Self {
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
pub(crate) struct TypeDefinitionDescriptor<'ctx> {
    metadata: &'static TypeDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TypeDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static TypeDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) const fn id(self) -> TypeDefinitionId {
        self.metadata.id
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.name
    }
    pub(crate) const fn path(self) -> &'static str {
        self.metadata.path
    }
    pub(crate) const fn kind(self) -> TypeDefinitionKind {
        self.metadata.kind
    }
    pub(crate) fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.metadata.generics, self.context)
    }
    pub(crate) fn instances(self) -> Result<Box<[TypeDescriptor<'ctx>]>, ContextError> {
        self.context.type_instances(self.metadata.id)
    }
}

/// Context-bound view of a declaration's generic parameters and predicates.
#[derive(Clone, Copy)]
pub(crate) struct GenericDefinitionDescriptor<'ctx> {
    metadata: &'static GenericDefinitionMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericDefinitionDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static GenericDefinitionMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn parameters(
        self,
    ) -> impl ExactSizeIterator<Item = GenericParameterDescriptor<'ctx>> + 'ctx {
        self.metadata
            .parameters
            .iter()
            .map(move |parameter| GenericParameterDescriptor::new(parameter, self.context))
    }
    pub(crate) fn parameter(
        self,
        name: &str,
    ) -> Result<GenericParameterDescriptor<'ctx>, LookupError> {
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
        matches.extend(
            candidates.map(|candidate| GenericParameterDescriptor::new(candidate, self.context)),
        );
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
    pub(crate) fn predicates(self) -> Predicates<'ctx> {
        Predicates::new(self.metadata.predicates, self.context)
    }
    pub(crate) fn is_empty(self) -> bool {
        self.metadata.parameters.is_empty()
    }
}

/// Context-bound view of one declared generic parameter.
#[derive(Clone, Copy)]
pub(crate) struct GenericParameterDescriptor<'ctx> {
    metadata: &'static GenericParameterMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericParameterDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static GenericParameterMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) const fn name(self) -> &'static str {
        self.metadata.name
    }
    pub(crate) const fn index(self) -> usize {
        self.metadata.index
    }
    pub(crate) fn kind(self) -> GenericParameterKind<'ctx> {
        match self.metadata.kind {
            GenericParameterMetadataKind::Lifetime { bounds } => {
                GenericParameterKind::Lifetime { bounds }
            }
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
                stable_id: StableItemId(
                    format!("{}:{}", self.metadata.index, self.metadata.name).into(),
                ),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

/// Kind-specific facts for a declared generic parameter.
pub(crate) enum GenericParameterKind<'ctx> {
    Lifetime {
        bounds: &'static [LifetimeExpression],
    },
    Type {
        bounds: Predicates<'ctx>,
        default: Option<TypeExpression<'ctx>>,
    },
    Const {
        ty: TypeExpression<'ctx>,
        default: Option<ConstExpression>,
    },
}

/// Static concrete type or const argument associated with a parameter.
#[derive(Clone, Copy)]
pub struct GenericArgumentMetadata {
    pub(crate) parameter: &'static GenericParameterMetadata,
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

impl GenericArgumentMetadata {
    /// Creates a type argument tied to a reflected concrete Rust type.
    pub const fn type_argument<T: crate::runtime::descriptor::Reflect>(
        parameter: &'static GenericParameterMetadata,
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            parameter,
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: Some(TypeMetadata::of::<T>),
            },
        }
    }

    /// Creates a type argument while retaining source syntax without requiring reflection.
    pub const fn type_expression(
        parameter: &'static GenericParameterMetadata,
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            parameter,
            kind: GenericArgumentMetadataKind::Type {
                expression,
                concrete: None,
            },
        }
    }

    /// Creates a const argument associated with its declared parameter.
    pub const fn const_argument(
        parameter: &'static GenericParameterMetadata,
        value: ConstExpression,
    ) -> Self {
        Self {
            parameter,
            kind: GenericArgumentMetadataKind::Const(value),
        }
    }
}

/// Context-bound collection of the type and const arguments of one application.
#[derive(Clone, Copy)]
pub(crate) struct GenericArguments<'ctx> {
    arguments: &'static [GenericArgumentMetadata],
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericArguments<'ctx> {
    pub(crate) const fn new(
        arguments: &'static [GenericArgumentMetadata],
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { arguments, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = GenericArgumentDescriptor<'ctx>> + 'ctx {
        self.arguments
            .iter()
            .map(move |argument| GenericArgumentDescriptor::new(argument, self.context))
    }
    pub(crate) fn get(self, name: &str) -> Option<GenericArgumentDescriptor<'ctx>> {
        let mut matches = self
            .arguments
            .iter()
            .filter(|argument| argument.parameter.name == name);
        let first = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| GenericArgumentDescriptor::new(first, self.context))
    }
    pub(crate) fn argument(
        self,
        name: &str,
    ) -> Result<GenericArgumentDescriptor<'ctx>, LookupError> {
        let matches = self
            .arguments
            .iter()
            .filter(|argument| argument.parameter.name == name)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(LookupError::Missing {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
            }),
            [argument] => Ok(GenericArgumentDescriptor::new(argument, self.context)),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
                candidates: matches
                    .iter()
                    .map(|argument| {
                        GenericParameterDescriptor::new(argument.parameter, self.context)
                            .lookup_candidate()
                    })
                    .collect(),
            }),
        }
    }
    pub(crate) fn is_empty(self) -> bool {
        self.arguments.is_empty()
    }
}

/// A concrete type or const argument in one already compiled application.
#[derive(Clone, Copy)]
pub(crate) struct GenericArgumentDescriptor<'ctx> {
    metadata: &'static GenericArgumentMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericArgumentDescriptor<'ctx> {
    const fn new(
        metadata: &'static GenericArgumentMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn type_argument(
        self,
    ) -> Result<TypeArgumentDescriptor<'ctx>, GenericArgumentError> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Type { .. } => Ok(TypeArgumentDescriptor {
                metadata: self.metadata,
                context: self.context,
            }),
            GenericArgumentMetadataKind::Const(_) => Err(GenericArgumentError::new(
                GenericArgumentKind::Type,
                GenericArgumentKind::Const,
            )),
        }
    }
    pub(crate) fn const_argument(
        self,
    ) -> Result<ConstArgumentDescriptor<'ctx>, GenericArgumentError> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Const(_) => Ok(ConstArgumentDescriptor {
                metadata: self.metadata,
                _context: self.context,
            }),
            GenericArgumentMetadataKind::Type { .. } => Err(GenericArgumentError::new(
                GenericArgumentKind::Const,
                GenericArgumentKind::Type,
            )),
        }
    }
}

/// A type argument, preserving syntax when no reflected descriptor exists.
#[derive(Clone, Copy)]
pub(crate) struct TypeArgumentDescriptor<'ctx> {
    metadata: &'static GenericArgumentMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TypeArgumentDescriptor<'ctx> {
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn parameter(self) -> GenericParameterDescriptor<'ctx> {
        GenericParameterDescriptor::new(self.metadata.parameter, self.context)
    }
    pub(crate) fn expression(self) -> TypeExpression<'ctx> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Type { expression, .. } => {
                TypeExpression::new(expression, self.context)
            }
            GenericArgumentMetadataKind::Const(_) => unreachable!("type argument kind was checked"),
        }
    }
    pub(crate) fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Type { concrete, .. } => {
                concrete.map(|metadata| TypeDescriptor::new(metadata(), self.context))
            }
            GenericArgumentMetadataKind::Const(_) => None,
        }
    }
    pub(crate) fn concrete_type(self) -> Result<TypeDescriptor<'ctx>, TypeExpressionError> {
        self.find_concrete_type().ok_or_else(|| {
            TypeExpressionError::ConcreteDescriptorUnavailable {
                parameter: self.metadata.parameter.name,
            }
        })
    }
}

/// A const argument retained as a structural expression.
#[derive(Clone, Copy)]
pub(crate) struct ConstArgumentDescriptor<'ctx> {
    metadata: &'static GenericArgumentMetadata,
    _context: &'ctx ReflectionContext,
}

impl<'ctx> ConstArgumentDescriptor<'ctx> {
    pub(crate) fn parameter(self) -> GenericParameterDescriptor<'ctx> {
        GenericParameterDescriptor::new(self.metadata.parameter, self._context)
    }
    pub(crate) fn expression(self) -> ConstExpression {
        match self.metadata.kind {
            GenericArgumentMetadataKind::Const(expression) => expression,
            GenericArgumentMetadataKind::Type { .. } => {
                unreachable!("const argument kind was checked")
            }
        }
    }
}
