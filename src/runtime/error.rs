// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::fmt;
use std::sync::Arc;

use super::capability::CapabilityError;
use super::capability::CapabilityTargetId;
use super::construct::ConstructionError;
use super::construct::UpdateError;
use super::descriptor::field::FieldAccessError;
use super::descriptor::field::FieldSetError;
use super::expression::GenericArgumentError;
use super::expression::TypeExpressionError;
use super::identity::GenericArgumentIdentity;
use super::identity::ItemIdentity;
use super::identity::LookupCandidate;
use super::identity::RegistrationOrigin;
use super::identity::TypeIdentity;
use super::invoke::InvocationError;
pub use super::value::TypeMismatch;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Validation errors produced while building a reflection context.
pub struct ContextError {
    data: Arc<ContextErrorData>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ContextErrorData {
    issues: Box<[ContextIssue]>,
}

impl ContextErrorData {
    fn new(issues: Box<[ContextIssue]>) -> Self {
        Self { issues }
    }
}

impl ContextError {
    /// Creates a context error with one free-form issue message.
    pub fn new(message: impl Into<Arc<str>>) -> Self {
        let issue = ContextIssue::Message(message.into());
        Self {
            data: Arc::new(ContextErrorData::new(vec![issue].into_boxed_slice())),
        }
    }

    /// Creates an error from validation issues, or returns `None` when empty.
    pub fn from_issues(issues: impl Into<Box<[ContextIssue]>>) -> Option<Self> {
        let issues = issues.into();
        (!issues.is_empty()).then(|| Self {
            data: Arc::new(ContextErrorData::new(issues)),
        })
    }

    /// Returns every validation issue in the order it was reported.
    pub fn issues(&self) -> &[ContextIssue] {
        &self.data.issues
    }

    /// Returns the first issue, which is used by the error's display message.
    pub fn primary_issue(&self) -> &ContextIssue {
        &self.data.issues[0]
    }

    /// Returns the primary issue's message, or a generic validation message.
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
/// A specific registration or metadata validation problem.
pub enum ContextIssue {
    /// Multiple registrations claim the same concrete Rust type.
    DuplicateTypeIdentity {
        /// The Rust `TypeId` claimed by the registrations.
        type_id: TypeId,
        /// Registration sites that contributed the duplicate type.
        registrations: Box<[RegistrationOrigin]>,
    },
    /// Multiple incompatible definitions claim the same type identity.
    ConflictingTypeDefinition {
        /// Identity shared by the conflicting definitions.
        identity: ItemIdentity,
        /// Registration sites that supplied those definitions.
        registrations: Box<[RegistrationOrigin]>,
    },
    /// Multiple incompatible definitions claim the same trait identity.
    ConflictingTraitDefinition {
        /// Identity shared by the conflicting definitions.
        identity: ItemIdentity,
        /// Registration sites that supplied those definitions.
        registrations: Box<[RegistrationOrigin]>,
    },
    /// The same trait application was registered more than once.
    DuplicateTraitImpl {
        /// Concrete type receiving the implementation.
        target: TypeIdentity,
        /// Reflected trait implemented by the target.
        trait_: ItemIdentity,
        /// Registration sites that supplied the duplicate applications.
        registrations: Box<[RegistrationOrigin]>,
    },
    /// Two methods occupy the same target and namespace slot.
    ConflictingMethod {
        /// Concrete type on which the methods are registered.
        target: TypeIdentity,
        /// Method namespace in which the collision occurred.
        namespace: Box<str>,
        /// Name shared by the conflicting methods.
        method: &'static str,
    },
    /// The same method specialization was registered more than once.
    DuplicateMethodSpecialization {
        /// Identity of the specialized method.
        method: ItemIdentity,
        /// Concrete generic arguments identifying the specialization.
        arguments: Box<[GenericArgumentIdentity]>,
        /// Registration sites that supplied the duplicate specialization.
        registrations: Box<[RegistrationOrigin]>,
    },
    /// Conflicting registrations provide the same capability for a target.
    ConflictingCapability {
        /// Type or definition to which the capability applies.
        target: CapabilityTargetId,
        /// Identity of the conflicting capability.
        capability: ItemIdentity,
        /// Registration sites that supplied the conflicting capability.
        registrations: Box<[RegistrationOrigin]>,
    },
    /// A registered item refers to an item absent from the context.
    MissingDependency {
        /// Item whose registration has an unresolved dependency.
        item: ItemIdentity,
        /// Required item that was not registered or included.
        dependency: ItemIdentity,
    },
    /// Registration dependencies form a cycle.
    DependencyCycle {
        /// Items participating in the detected cycle.
        items: Box<[ItemIdentity]>,
    },
    /// A generated runtime adapter failed validation.
    InvalidAdapter {
        /// Item whose adapter could not be validated.
        item: ItemIdentity,
        /// Reason the adapter was rejected.
        reason: AdapterValidationError,
    },
    /// Registered metadata failed structural validation.
    InvalidMetadata {
        /// Item whose metadata could not be validated.
        item: ItemIdentity,
        /// Reason the metadata was rejected.
        reason: MetadataValidationError,
    },
    /// A caller supplied a free-form validation issue.
    Message(Arc<str>),
}

impl fmt::Display for ContextIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTypeIdentity { type_id, .. } => {
                write!(formatter, "duplicate type identity {type_id:?}")
            }
            Self::ConflictingTypeDefinition { identity, .. } => {
                write!(formatter, "conflicting type definition {identity:?}")
            }
            Self::ConflictingTraitDefinition { identity, .. } => {
                write!(formatter, "conflicting trait definition {identity:?}")
            }
            Self::DuplicateTraitImpl { target, trait_, .. } => write!(
                formatter,
                "duplicate trait implementation for {target:?} and {trait_:?}"
            ),
            Self::ConflictingMethod {
                target,
                namespace,
                method,
            } => write!(
                formatter,
                "duplicate method {method} in namespace {namespace} for {target:?}"
            ),
            Self::DuplicateMethodSpecialization { method, arguments, .. } => write!(
                formatter,
                "duplicate specialization of {method:?} for {} arguments",
                arguments.len()
            ),
            Self::ConflictingCapability { target, capability, .. } => {
                write!(formatter, "conflicting capability {capability:?} for {target:?}")
            }
            Self::MissingDependency { item, dependency } => {
                write!(formatter, "{item:?} requires missing {dependency:?}")
            }
            Self::DependencyCycle { items } => {
                write!(formatter, "registration dependency cycle among {} items", items.len())
            }
            Self::InvalidAdapter { item, reason } => {
                write!(formatter, "invalid adapter for {item:?}: {reason}")
            }
            Self::InvalidMetadata { item, reason } => {
                write!(formatter, "invalid metadata for {item:?}: {reason}")
            }
            Self::Message(message) => formatter.write_str(message),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Explains why a generated runtime adapter cannot be used.
pub struct AdapterValidationError(Box<str>);

impl AdapterValidationError {
    /// Creates an adapter validation error with the supplied explanation.
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for AdapterValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Explains why reflection metadata failed validation.
pub struct MetadataValidationError(Box<str>);

impl MetadataValidationError {
    /// Creates a metadata validation error with the supplied explanation.
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for MetadataValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
/// Kind of reflected item requested by a lookup.
pub enum LookupSubject {
    /// A concrete Rust type.
    Type,
    /// A generic type declaration.
    TypeDefinition,
    /// A trait declaration.
    TraitDefinition,
    /// A trait application to a concrete type.
    TraitApplication,
    /// An implementation item.
    Impl,
    /// A field.
    Field,
    /// An enum variant.
    Variant,
    /// A method.
    Method,
    /// A concrete generic method specialization.
    MethodSpecialization,
    /// A function or method parameter.
    Parameter,
    /// An associated type.
    AssociatedType,
    /// An associated constant.
    AssociatedConst,
    /// A capability registration.
    Capability,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
/// Criterion used to select a reflected item during lookup.
pub enum LookupQuery {
    /// Match an item's fully qualified name.
    Name(Box<str>),
    /// Match an item's unqualified name.
    ShortName(Box<str>),
    /// Match a concrete Rust type identity.
    TypeId(std::any::TypeId),
    /// Match a stable reflected item identifier.
    ItemId(super::identity::StableItemId),
    /// Match a zero-based collection index.
    Index(usize),
    /// Match an exact set of generic arguments.
    GenericArguments(Box<[GenericArgumentIdentity]>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
/// Failure to resolve a unique reflected item or to construct its context.
pub enum LookupError {
    /// The reflection context could not be built or validated.
    Context(ContextError),
    /// No item matched the requested subject and query.
    Missing {
        /// Kind of item requested.
        subject: LookupSubject,
        /// Criterion used to find the item.
        query: LookupQuery,
    },
    /// More than one item matched the requested subject and query.
    Ambiguous {
        /// Kind of item requested.
        subject: LookupSubject,
        /// Criterion used to find the items.
        query: LookupQuery,
        /// Matching candidates, including their available origins.
        candidates: Box<[LookupCandidate]>,
    },
}

impl fmt::Display for LookupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context(error) => write!(formatter, "reflection context failed: {error}"),
            Self::Missing { subject, query } => {
                write!(formatter, "missing {subject:?} for {query:?}")
            }
            Self::Ambiguous {
                subject,
                query,
                candidates,
            } => write!(
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
#[non_exhaustive]
/// Top-level error returned by reflective operations.
// Keep the typed construction causes intact when this aggregate error wraps them.
#[allow(private_interfaces)]
pub enum ReflectionError {
    /// Lookup failed because the item was missing, ambiguous, or
    /// context-invalid.
    Lookup(LookupError),
    /// Reflection context validation failed.
    Context(ContextError),
    /// A runtime value had an unexpected concrete type.
    TypeMismatch(TypeMismatch),
    /// A field could not be read.
    FieldAccess(FieldAccessError),
    /// A field could not be set.
    FieldSet(FieldSetError),
    /// A reflected method call failed.
    Invocation(InvocationError),
    /// A value could not be constructed.
    Construction(ConstructionError),
    /// An existing value could not be updated.
    Update(UpdateError),
    /// A requested capability was unavailable or invalid.
    Capability(CapabilityError),
    /// A generic argument was used through the wrong kind-specific API.
    GenericArgument(GenericArgumentError),
    /// A type expression could not provide the requested descriptor.
    TypeExpression(TypeExpressionError),
    /// The requested reflective operation is unsupported.
    Unsupported(UnsupportedOperationError),
}

impl fmt::Display for ReflectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lookup(error) => error.fmt(formatter),
            Self::Context(error) => error.fmt(formatter),
            Self::TypeMismatch(error) => error.fmt(formatter),
            Self::FieldAccess(error) => error.fmt(formatter),
            Self::FieldSet(error) => error.fmt(formatter),
            Self::Invocation(error) => error.fmt(formatter),
            Self::Construction(error) => error.fmt(formatter),
            Self::Update(error) => error.fmt(formatter),
            Self::Capability(error) => error.fmt(formatter),
            Self::GenericArgument(error) => error.fmt(formatter),
            Self::TypeExpression(error) => error.fmt(formatter),
            Self::Unsupported(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ReflectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Lookup(error) => Some(error),
            Self::Context(error) => Some(error),
            Self::TypeMismatch(error) => Some(error),
            Self::FieldAccess(error) => Some(error),
            Self::FieldSet(error) => Some(error),
            Self::Invocation(error) => Some(error),
            Self::Construction(error) => Some(error),
            Self::Update(error) => Some(error),
            Self::Capability(error) => Some(error),
            Self::GenericArgument(error) => Some(error),
            Self::TypeExpression(error) => Some(error),
            Self::Unsupported(error) => Some(error),
        }
    }
}

macro_rules! reflection_from {
    ($source:ty, $variant:ident) => {
        impl From<$source> for ReflectionError {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}

reflection_from!(ContextError, Context);
reflection_from!(LookupError, Lookup);
reflection_from!(TypeMismatch, TypeMismatch);
reflection_from!(FieldAccessError, FieldAccess);
reflection_from!(FieldSetError, FieldSet);
reflection_from!(InvocationError, Invocation);
reflection_from!(ConstructionError, Construction);
reflection_from!(UpdateError, Update);
reflection_from!(CapabilityError, Capability);
reflection_from!(GenericArgumentError, GenericArgument);
reflection_from!(TypeExpressionError, TypeExpression);
reflection_from!(UnsupportedOperationError, Unsupported);

#[derive(Clone, Debug, Eq, PartialEq)]
/// Explains why a requested reflective operation is unsupported.
pub struct UnsupportedOperationError(Box<str>);

impl UnsupportedOperationError {
    /// Creates an unsupported-operation error with the supplied explanation.
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for UnsupportedOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for UnsupportedOperationError {}

/// The error and retained input extracted from a recoverable operation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoverableFailureParts<E, R> {
    /// The error explaining why the operation was rejected.
    pub error: E,
    /// The original input retained for inspection or retry.
    pub recovery: R,
}

impl<E, R> RecoverableFailureParts<E, R> {
    /// Creates a pair containing an operation error and its retained input.
    pub const fn new(error: E, recovery: R) -> Self {
        Self { error, recovery }
    }

    /// Returns the error and retained input as separate owned values.
    pub fn into_parts(self) -> (E, R) {
        (self.error, self.recovery)
    }
}

/// Provides shared access to a failure and transfers its retained input on
/// demand.
pub trait RecoverableFailure {
    /// The error that describes why the operation was rejected.
    type Error: std::error::Error + Send + Sync + 'static;
    /// The original input or state retained so the caller can inspect or retry
    /// it.
    type Recovery;

    /// Returns the error that explains why the operation was rejected.
    fn error(&self) -> &Self::Error;

    /// Returns the original input retained for inspection or retry.
    fn recovery(&self) -> &Self::Recovery;

    /// Separates the error from the retained input.
    fn into_parts(self) -> (Self::Error, Self::Recovery);
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::AdapterValidationError;
    use super::CapabilityError;
    use super::CapabilityTargetId;
    use super::ConstructionError;
    use super::ContextError;
    use super::ContextIssue;
    use super::FieldAccessError;
    use super::FieldSetError;
    use super::GenericArgumentError;
    use super::InvocationError;
    use super::ItemIdentity;
    use super::LookupCandidate;
    use super::LookupError;
    use super::LookupQuery;
    use super::LookupSubject;
    use super::MetadataValidationError;
    use super::RecoverableFailureParts;
    use super::ReflectionError;
    use super::TypeExpressionError;
    use super::UnsupportedOperationError;
    use super::UpdateError;
    use crate::runtime::capability::capability_error::CapabilityInvocationError;
    use crate::runtime::capability::capability_error::CapabilityInvocationMode;
    use crate::runtime::descriptor::generic::TypeDefinitionId;
    use crate::runtime::expression::GenericArgumentKind;
    use crate::runtime::identity::GenericArgumentIdentity;
    use crate::runtime::identity::ItemKind;
    use crate::runtime::identity::RegistrationOrigin;
    use crate::runtime::identity::TypeIdentity;
    use crate::runtime::invoke::InvocationErrorKind;
    use crate::runtime::value::TypeMismatch;

    fn item(kind: ItemKind, stable_id: &str, display_name: &str) -> ItemIdentity {
        ItemIdentity {
            kind,
            stable_id: super::super::identity::StableItemId(stable_id.into()),
            display_name: display_name.into(),
        }
    }

    fn origin(module_path: &'static str) -> RegistrationOrigin {
        RegistrationOrigin {
            crate_name: "test-crate",
            crate_version: Some("1.2.3"),
            module_path,
            file: Some("src/lib.rs"),
            line: Some(17),
            column: Some(5),
        }
    }

    fn type_identity<T: 'static>(rust_name: &'static str) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<T>(),
            rust_name,
        }
    }

    fn mismatch() -> TypeMismatch {
        TypeMismatch::new(type_identity::<u8>("u8"), type_identity::<u16>("u16"))
    }

    #[test]
    fn context_error_constructors_keep_order_and_use_primary_issue() {
        let free_form = ContextError::new("registration rejected");
        assert_eq!(free_form.message(), "registration rejected");
        assert_eq!(
            free_form.issues(),
            &[ContextIssue::Message("registration rejected".into())]
        );
        assert_eq!(free_form.to_string(), "registration rejected");
        assert!(std::error::Error::source(&free_form).is_none());

        assert!(ContextError::from_issues(Vec::<ContextIssue>::new()).is_none());
        let issues = vec![
            ContextIssue::Message("primary".into()),
            ContextIssue::Message("secondary".into()),
        ];
        let aggregate = ContextError::from_issues(issues.clone()).expect("non-empty issues");
        assert_eq!(aggregate.issues(), issues);
        assert_eq!(aggregate.primary_issue(), &issues[0]);
        assert_eq!(aggregate.message(), "primary");
        assert_eq!(aggregate.to_string(), "primary");
        assert_eq!(aggregate.clone(), aggregate);
    }

    #[test]
    fn context_issue_displays_structured_validation_causes() {
        let ty = type_identity::<u8>("u8");
        let type_item = item(ItemKind::TypeDefinition, "type:u8", "Number");
        let trait_item = item(ItemKind::TraitDefinition, "trait:Display", "Display");
        let method_item = item(ItemKind::MethodSpecialization, "method:fmt<u8>", "fmt");
        let capability_item = item(ItemKind::Capability, "cap:serde", "Serde");
        let issues = [
            ContextIssue::DuplicateTypeIdentity {
                type_id: ty.type_id,
                registrations: vec![origin("first"), origin("second")].into_boxed_slice(),
            },
            ContextIssue::ConflictingTypeDefinition {
                identity: type_item.clone(),
                registrations: vec![origin("types")].into_boxed_slice(),
            },
            ContextIssue::ConflictingTraitDefinition {
                identity: trait_item.clone(),
                registrations: vec![origin("traits")].into_boxed_slice(),
            },
            ContextIssue::DuplicateTraitImpl {
                target: ty,
                trait_: trait_item.clone(),
                registrations: vec![origin("impls")].into_boxed_slice(),
            },
            ContextIssue::ConflictingMethod {
                target: ty,
                namespace: "instance".into(),
                method: "render",
            },
            ContextIssue::DuplicateMethodSpecialization {
                method: method_item.clone(),
                arguments: vec![GenericArgumentIdentity::Type("u8".into())].into_boxed_slice(),
                registrations: vec![origin("methods")].into_boxed_slice(),
            },
            ContextIssue::ConflictingCapability {
                target: CapabilityTargetId::Type(ty),
                capability: capability_item,
                registrations: vec![origin("capabilities")].into_boxed_slice(),
            },
            ContextIssue::MissingDependency {
                item: method_item.clone(),
                dependency: type_item,
            },
            ContextIssue::DependencyCycle {
                items: vec![method_item.clone(), trait_item.clone()].into_boxed_slice(),
            },
            ContextIssue::InvalidAdapter {
                item: method_item.clone(),
                reason: AdapterValidationError::new("wrong adapter type"),
            },
            ContextIssue::InvalidMetadata {
                item: method_item,
                reason: MetadataValidationError::new("missing method metadata"),
            },
        ];
        let displays = issues.map(|issue| issue.to_string());
        assert!(displays[0].contains("duplicate type identity"));
        assert!(displays[1].contains("conflicting type definition"));
        assert!(displays[2].contains("conflicting trait definition"));
        assert!(displays[3].contains("duplicate trait implementation"));
        assert!(displays[4].contains("duplicate method render in namespace instance"));
        assert!(displays[5].contains("duplicate specialization"));
        assert!(displays[6].contains("conflicting capability"));
        assert!(displays[7].contains("requires missing"));
        assert!(displays[8].contains("dependency cycle among 2 items"));
        assert!(displays[9].contains("invalid adapter") && displays[9].contains("wrong adapter type"));
        assert!(displays[10].contains("invalid metadata") && displays[10].contains("missing method metadata"));
        assert_eq!(AdapterValidationError::new("adapter").to_string(), "adapter");
        assert_eq!(MetadataValidationError::new("metadata").to_string(), "metadata");
        assert_eq!(ContextIssue::Message("custom".into()).to_string(), "custom");
    }

    #[test]
    fn lookup_error_displays_variants_and_exposes_only_context_source() {
        let query = LookupQuery::ShortName("Thing".into());
        let missing = LookupError::Missing {
            subject: LookupSubject::Type,
            query: query.clone(),
        };
        assert!(missing.to_string().contains("missing Type"));
        assert!(std::error::Error::source(&missing).is_none());

        let candidate = LookupCandidate {
            identity: item(ItemKind::Type, "type:thing", "Thing"),
            display_name: "Thing".into(),
            origin: Some(origin("fixture")),
        };
        let ambiguous = LookupError::Ambiguous {
            subject: LookupSubject::Type,
            query,
            candidates: vec![candidate].into_boxed_slice(),
        };
        assert!(ambiguous.to_string().contains("ambiguous Type"));
        assert!(ambiguous.to_string().contains("1 candidates"));
        assert!(std::error::Error::source(&ambiguous).is_none());

        let context = LookupError::Context(ContextError::new("invalid context"));
        assert!(
            context
                .to_string()
                .contains("reflection context failed: invalid context")
        );
        assert_eq!(
            std::error::Error::source(&context).unwrap().to_string(),
            "invalid context"
        );
    }

    #[test]
    fn reflection_error_conversions_preserve_types_and_sources() {
        let generic = GenericArgumentError::new(GenericArgumentKind::Type, GenericArgumentKind::Const);
        assert_eq!(generic.expected(), GenericArgumentKind::Type);
        assert_eq!(generic.actual(), GenericArgumentKind::Const);
        assert!(generic.to_string().contains("expected Type generic argument"));

        let expression = TypeExpressionError::ConcreteDescriptorUnavailable { parameter: "T" };
        assert!(expression.to_string().contains("parameter T"));
        let unsupported = UnsupportedOperationError::new("not supported");
        assert_eq!(unsupported.to_string(), "not supported");
        assert!(std::error::Error::source(&unsupported).is_none());

        let lookup = LookupError::Missing {
            subject: LookupSubject::Method,
            query: LookupQuery::Index(3),
        };
        let type_error = mismatch();
        let errors = vec![
            ReflectionError::from(ContextError::new("context")),
            ReflectionError::from(lookup),
            ReflectionError::from(type_error),
            ReflectionError::from(FieldAccessError::NotReadable),
            ReflectionError::from(FieldSetError::NotReplaceable),
            ReflectionError::from(InvocationError::new(InvocationErrorKind::ReceiverMissing)),
            ReflectionError::from(ConstructionError::MissingProvider),
            ReflectionError::from(UpdateError::MissingProvider),
            ReflectionError::from(CapabilityError::Missing {
                target: CapabilityTargetId::Type(type_identity::<u8>("u8")),
                capability_id: "fixture.capability",
            }),
            ReflectionError::from(generic),
            ReflectionError::from(expression),
            ReflectionError::from(unsupported),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
            assert!(std::error::Error::source(&error).is_some());
        }

        let nested = ReflectionError::from(LookupError::Context(ContextError::new("nested")));
        let source = std::error::Error::source(&nested).expect("lookup source");
        assert_eq!(source.to_string(), "reflection context failed: nested");
        assert_eq!(source.source().unwrap().to_string(), "nested");

        let capability = CapabilityError::Missing {
            target: CapabilityTargetId::Definition(TypeDefinitionId::of::<u8>()),
            capability_id: "missing",
        };
        assert!(capability.to_string().contains("missing capability missing"));
        assert!(std::error::Error::source(&capability).is_none());
        let invocation = CapabilityInvocationError::UnsupportedMode {
            expected_type: TypeId::of::<u8>(),
            mode: CapabilityInvocationMode::Local,
        };
        assert!(invocation.to_string().contains("does not support Local values"));
    }

    #[test]
    fn recoverable_failure_parts_round_trip_error_and_input() {
        let parts = RecoverableFailureParts::new("rejected", vec![1, 2, 3]);
        assert_eq!(parts.error, "rejected");
        assert_eq!(parts.recovery, [1, 2, 3]);
        assert_eq!(parts.into_parts(), ("rejected", vec![1, 2, 3]));
    }
}
