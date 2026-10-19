use std::fmt;
use std::sync::Arc;

use super::identity::LookupCandidate;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextError {
    message: Arc<str>,
}

impl ContextError {
    pub fn new(message: impl Into<Arc<str>>) -> Self {
        Self { message: message.into() }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ContextError {}

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
    GenericArguments(Box<[Box<str>]>),
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
