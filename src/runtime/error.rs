use std::any::TypeId;
use std::fmt;
use std::sync::Arc;

use super::identity::{
    GenericArgumentIdentity, ItemIdentity, LookupCandidate, RegistrationOrigin, TypeIdentity,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextError {
    data: Arc<ContextErrorData>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ContextErrorData {
    issues: Box<[ContextIssue]>,
}

impl ContextErrorData {
    fn new(issues: Box<[ContextIssue]>) -> Self { Self { issues } }
}

impl ContextError {
    pub fn new(message: impl Into<Arc<str>>) -> Self {
        let issue = ContextIssue::Message(message.into());
        Self { data: Arc::new(ContextErrorData::new(vec![issue].into_boxed_slice())) }
    }

    pub fn from_issues(issues: impl Into<Box<[ContextIssue]>>) -> Option<Self> {
        let issues = issues.into();
        (!issues.is_empty()).then(|| Self { data: Arc::new(ContextErrorData::new(issues)) })
    }

    pub fn issues(&self) -> &[ContextIssue] {
        &self.data.issues
    }

    pub fn primary_issue(&self) -> &ContextIssue {
        &self.data.issues[0]
    }

    pub fn message(&self) -> &str {
        match self.primary_issue() {
            ContextIssue::Message(message) => message,
            _ => "reflection context validation failed",
        }
    }
}

impl fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.primary_issue())
    }
}

impl std::error::Error for ContextError {}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ContextIssue {
    DuplicateTypeIdentity { type_id: TypeId, registrations: Box<[RegistrationOrigin]> },
    ConflictingTypeDefinition { identity: ItemIdentity, registrations: Box<[RegistrationOrigin]> },
    ConflictingTraitDefinition { identity: ItemIdentity, registrations: Box<[RegistrationOrigin]> },
    DuplicateTraitImpl { target: TypeIdentity, trait_: ItemIdentity, registrations: Box<[RegistrationOrigin]> },
    DuplicateMethodSpecialization { method: ItemIdentity, arguments: Box<[GenericArgumentIdentity]>, registrations: Box<[RegistrationOrigin]> },
    ConflictingCapability { target: TypeIdentity, capability: ItemIdentity, registrations: Box<[RegistrationOrigin]> },
    MissingDependency { item: ItemIdentity, dependency: ItemIdentity },
    DependencyCycle { items: Box<[ItemIdentity]> },
    InvalidAdapter { item: ItemIdentity, reason: AdapterValidationError },
    InvalidMetadata { item: ItemIdentity, reason: MetadataValidationError },
    Message(Arc<str>),
}

impl fmt::Display for ContextIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTypeIdentity { type_id, .. } => write!(formatter, "duplicate type identity {type_id:?}"),
            Self::ConflictingTypeDefinition { identity, .. } => write!(formatter, "conflicting type definition {identity:?}"),
            Self::ConflictingTraitDefinition { identity, .. } => write!(formatter, "conflicting trait definition {identity:?}"),
            Self::DuplicateTraitImpl { target, trait_, .. } => write!(formatter, "duplicate trait implementation for {target:?} and {trait_:?}"),
            Self::DuplicateMethodSpecialization { method, arguments, .. } => write!(formatter, "duplicate specialization of {method:?} for {} arguments", arguments.len()),
            Self::ConflictingCapability { target, capability, .. } => write!(formatter, "conflicting capability {capability:?} for {target:?}"),
            Self::MissingDependency { item, dependency } => write!(formatter, "{item:?} requires missing {dependency:?}"),
            Self::DependencyCycle { items } => write!(formatter, "registration dependency cycle among {} items", items.len()),
            Self::InvalidAdapter { item, reason } => write!(formatter, "invalid adapter for {item:?}: {reason}"),
            Self::InvalidMetadata { item, reason } => write!(formatter, "invalid metadata for {item:?}: {reason}"),
            Self::Message(message) => formatter.write_str(message),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterValidationError(Box<str>);

impl AdapterValidationError {
    pub fn new(message: impl Into<Box<str>>) -> Self { Self(message.into()) }
}

impl fmt::Display for AdapterValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.0) }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataValidationError(Box<str>);

impl MetadataValidationError {
    pub fn new(message: impl Into<Box<str>>) -> Self { Self(message.into()) }
}

impl fmt::Display for MetadataValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.0) }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum LookupSubject {
    Type,
    TypeDefinition,
    TraitDefinition,
    TraitApplication,
    Impl,
    Field,
    Variant,
    Method,
    MethodSpecialization,
    Parameter,
    AssociatedType,
    AssociatedConst,
    Capability,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum LookupQuery {
    Name(Box<str>),
    ShortName(Box<str>),
    TypeId(std::any::TypeId),
    ItemId(super::identity::StableItemId),
    Index(usize),
    GenericArguments(Box<[GenericArgumentIdentity]>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LookupError {
    Context(ContextError),
    Missing { subject: LookupSubject, query: LookupQuery },
    Ambiguous {
        subject: LookupSubject,
        query: LookupQuery,
        candidates: Box<[LookupCandidate]>,
    },
}

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context(error) => write!(formatter, "reflection context failed: {error}"),
            Self::Missing { subject, query } => write!(formatter, "missing {subject:?} for {query:?}"),
            Self::Ambiguous { subject, query, candidates } => write!(
                formatter,
                "ambiguous {subject:?} for {query:?} ({} candidates)",
                candidates.len(),
            ),
        }
    }
}

impl std::error::Error for LookupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Context(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum ReflectionError {
    Lookup(LookupError),
    Context(ContextError),
}

impl fmt::Display for ReflectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lookup(error) => error.fmt(formatter),
            Self::Context(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ReflectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lookup(error) => Some(error),
            Self::Context(error) => Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoverableFailureParts<E, R> {
    error: E,
    recovery: R,
}

pub trait RecoverableFailure {
    type Error: std::error::Error + Send + Sync + 'static;
    type Recovery;

    fn into_parts(self) -> (Self::Error, Self::Recovery);
}
