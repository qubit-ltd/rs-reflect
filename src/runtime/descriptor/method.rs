// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Method declarations, effective implementations, and callable
//! specializations.

use std::any::Any;
use std::any::TypeId;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::GenericArgumentMetadata;
use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
pub(crate) use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadataKind;
use crate::runtime::error::ContextError;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::expression::ConstExpression;
use crate::runtime::expression::ExpressionData;
pub use crate::runtime::expression::TypeExpression;
use crate::runtime::identity::GenericArgumentIdentity;
use crate::runtime::identity::GenericArgumentSelector;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;
use crate::runtime::invoke::CaughtInvocationFailure;
use crate::runtime::invoke::Invocation;
use crate::runtime::invoke::InvocationAdapter;
use crate::runtime::invoke::InvocationAttempt;
use crate::runtime::invoke::InvocationAvailability;
use crate::runtime::invoke::InvocationError;
use crate::runtime::invoke::InvocationErrorKind;
use crate::runtime::invoke::InvocationFailure;
use crate::runtime::invoke::InvocationOutput;
use crate::runtime::invoke::InvocationPanic;
use crate::runtime::invoke::InvocationUnavailableReason;
use crate::runtime::invoke::Local;
use crate::runtime::invoke::ModeAdapter;
use crate::runtime::invoke::PinnedInvocation;
use crate::runtime::invoke::PinnedInvocationAdapter;
use crate::runtime::invoke::PinnedInvocationFailure;
use crate::runtime::invoke::ThreadSafe;
use crate::runtime::invoke::ValueMode;
use crate::runtime::lookup::Lookup;

#[cfg(test)]
mod specialization_integer_tests {
    use super::ExpressionData;
    use super::GenericArgumentIdentity;
    use super::GenericParameterMetadata;
    use super::MethodSpecializationArgument;
    use super::MethodSpecializationSpec;
    use super::NO_ADAPTERS;

    static U8_EXPRESSION: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    static U8_PARAMETER: GenericParameterMetadata =
        GenericParameterMetadata::const_parameter("N", 0, &U8_EXPRESSION, None);
    static PARAMETERS: [GenericParameterMetadata; 1] = [U8_PARAMETER];
    static MAX_ARGUMENT: [MethodSpecializationArgument; 1] =
        [MethodSpecializationArgument::const_integer_literal(false, 255)];
    static OVERFLOW_ARGUMENT: [MethodSpecializationArgument; 1] =
        [MethodSpecializationArgument::const_integer_literal(false, 256)];

    #[test]
    fn test_normalizes_untyped_integer_literal_using_declared_const_type() {
        let resolved = MethodSpecializationSpec::new(&MAX_ARGUMENT, NO_ADAPTERS)
            .resolve(&PARAMETERS)
            .expect("255 fits in u8");
        assert_eq!(
            resolved.arguments.as_slice(),
            &[GenericArgumentIdentity::Const("u8:255".into())]
        );
    }

    #[test]
    fn test_rejects_untyped_integer_literal_outside_declared_const_type() {
        assert!(
            MethodSpecializationSpec::new(&OVERFLOW_ARGUMENT, NO_ADAPTERS)
                .resolve(&PARAMETERS)
                .is_err()
        );
    }
}

/// Visibility recorded for a method declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodVisibility {
    /// The method is public.
    Public,
    /// The method has restricted visibility.
    Restricted,
    /// The method is private.
    Private,
}

/// The receiver form declared by a method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiverKind {
    /// The method takes ownership of the receiver.
    Owned,
    /// The method borrows the receiver immutably.
    Shared,
    /// The method borrows the receiver mutably.
    Mutable,
    /// The method takes a pinned shared receiver.
    PinnedShared,
    /// The method takes a pinned mutable receiver.
    PinnedMutable,
}

/// Metadata for the receiver parameter.
#[derive(Clone, Copy)]
pub struct ReceiverDescriptor {
    /// Returns the declared return-value category.
    pub(crate) kind: ReceiverKind,
    /// Provides the concrete receiver type metadata when the declaration is
    /// statically typed.
    pub(crate) type_metadata: Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
    /// Retains the source type expression used to describe this receiver.
    pub(crate) expression: Option<&'static ExpressionData>,
}

impl ReceiverDescriptor {
    /// Returns the receiver form declared by the method.
    pub const fn kind(self) -> ReceiverKind {
        self.kind
    }

    /// Creates receiver metadata for one concrete reflected target type.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    pub const fn for_type<T: super::Reflect>(kind: ReceiverKind) -> Self {
        Self {
            kind,
            type_metadata: Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
            expression: None,
        }
    }

    /// Creates receiver metadata whose concrete type is supplied by an
    /// effective method.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn trait_self(kind: ReceiverKind, expression: &'static ExpressionData) -> Self {
        Self {
            kind,
            type_metadata: None,
            expression: Some(expression),
        }
    }

    /// Binds an unbound receiver declaration to its effective target type.
    ///
    /// # Parameters
    ///
    /// - `target`: Target type whose effective method metadata is being
    ///   resolved.
    pub(crate) const fn for_target(
        mut self,
        target: fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    ) -> Self {
        if self.type_metadata.is_none() {
            self.type_metadata = Some(target);
        }
        self
    }

    /// Retains the source type expression for an explicitly typed receiver.
    ///
    /// # Parameters
    ///
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn with_expression(mut self, expression: &'static ExpressionData) -> Self {
        self.expression = Some(expression);
        self
    }
}

/// Source pattern retained for a non-receiver parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterPattern {
    /// A parameter bound to the given identifier.
    Identifier(
        /// Identifier text retained from the source pattern.
        &'static str,
    ),
    /// A wildcard parameter with no binding name.
    Wildcard,
    /// A destructured parameter whose nested pattern is not retained.
    Destructure,
    /// A destructured parameter with its source pattern text.
    DestructurePattern(
        /// Original pattern text, retained for diagnostics and inspection.
        &'static str,
    ),
}

/// How a method parameter receives its value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassingMode {
    /// The argument is passed by value.
    Owned,
    /// The argument is passed through a shared reference.
    Shared,
    /// The argument is passed through a mutable reference.
    Mutable,
}

/// Static facts about one declared parameter.
#[derive(Clone, Copy)]
pub struct ParameterMetadata {
    /// Returns the parameter’s zero-based position, excluding the receiver.
    pub(crate) index: usize,
    /// Returns the source binding pattern retained during registration.
    pub(crate) pattern: ParameterPattern,
    /// Returns the argument passing mode declared by the method.
    pub(crate) passing: PassingMode,
    /// Retains the source type expression used to describe this receiver.
    pub(crate) expression: &'static ExpressionData,
    /// Optional metadata for the exact Rust type known at registration time.
    pub(crate) concrete_type: Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
}

impl ParameterMetadata {
    /// Creates static metadata for one method parameter.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field position to inspect.
    /// - `pattern`: Source binding pattern associated with the parameter.
    /// - `passing`: Argument passing mode declared by the method.
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn new(
        index: usize,
        pattern: ParameterPattern,
        passing: PassingMode,
        expression: &'static ExpressionData,
    ) -> Self {
        Self {
            index,
            pattern,
            passing,
            expression,
            concrete_type: None,
        }
    }

    /// Records the exact concrete Rust type when the parameter has one.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub const fn with_concrete_type<T: ?Sized + 'static>(mut self) -> Self {
        self.concrete_type = Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>);
        self
    }
}

/// A context-bound view of one declared method parameter.
#[derive(Clone, Copy)]
pub struct ParameterDescriptor<'ctx> {
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    metadata: &'static ParameterMetadata,
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> ParameterDescriptor<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(crate) const fn new(metadata: &'static ParameterMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the reflection context that owns this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the parameter's zero-based position, excluding the receiver.
    pub const fn index(self) -> usize {
        self.metadata.index
    }
    /// Returns the source identifier, or `None` for wildcard and destructured
    /// patterns.
    pub const fn name(self) -> Option<&'static str> {
        match self.metadata.pattern {
            ParameterPattern::Identifier(name) => Some(name),
            _ => None,
        }
    }
    /// Returns the source binding pattern.
    pub const fn pattern(self) -> ParameterPattern {
        self.metadata.pattern
    }
    /// Returns how the parameter receives its value.
    pub const fn passing(self) -> PassingMode {
        self.metadata.passing
    }
    /// Returns the source-level type expression for this parameter.
    pub fn type_expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.expression, self.context)
    }
    /// Returns the concrete Rust type descriptor when registration retained
    /// one.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete_type
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
}

/// A read-only, source-ordered view of a method's non-receiver parameters.
#[derive(Clone, Copy)]
pub struct Parameters<'ctx> {
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    metadata: &'static [ParameterMetadata],
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> Parameters<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `metadata`: Static method declaration facts resolved in the reflection
    ///   context.
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `metadata`: Static method declaration facts resolved for this
    ///   reflection context.
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    const fn new(metadata: &'static [ParameterMetadata], context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }

    /// Returns the reflection context that owns this parameter collection.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the number of declared parameters.
    pub const fn len(self) -> usize {
        self.metadata.len()
    }

    /// Returns whether the method declares no non-receiver parameters.
    pub const fn is_empty(self) -> bool {
        self.metadata.is_empty()
    }

    /// Iterates over parameters in source declaration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = ParameterDescriptor<'ctx>> + 'ctx {
        self.metadata
            .iter()
            .map(move |metadata| ParameterDescriptor::new(metadata, self.context))
    }

    /// Returns the parameter at its zero-based source position, if present.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field position to inspect.
    pub fn get_at(self, index: usize) -> Option<ParameterDescriptor<'ctx>> {
        self.metadata
            .get(index)
            .map(|metadata| ParameterDescriptor::new(metadata, self.context))
    }

    /// Returns a uniquely named identifier parameter, if present.
    ///
    /// Parameters written with wildcard or destructuring patterns have no name.
    /// A duplicate name in malformed metadata is treated as ambiguous and
    /// returns `None`.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    pub fn get(self, name: &str) -> Option<ParameterDescriptor<'ctx>> {
        let mut matches = self.metadata.iter().filter(
            |metadata| matches!(metadata.pattern, ParameterPattern::Identifier(candidate) if candidate == name),
        );
        let first = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| ParameterDescriptor::new(first, self.context))
    }
}

/// The declared kind and optional type expression for a return value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnKind {
    /// The method does not return normally.
    Never,
    /// The method returns unit.
    Unit,
    /// The method returns an owned value.
    Owned,
    /// The method returns a shared reference.
    Shared,
    /// The method returns an optional shared reference.
    OptionalShared,
    /// The method returns a shared slice.
    SharedSlice,
    /// The method returns a mutable reference.
    Mutable,
    /// The method returns a future.
    Future,
    /// The return type is retained only as opaque metadata.
    Opaque,
}

/// Static return metadata for a method declaration.
#[derive(Clone, Copy)]
pub struct ReturnMetadata {
    /// Returns the declared return-value category.
    pub(crate) kind: ReturnKind,
    /// Retains the source type expression used to describe this receiver.
    pub(crate) expression: Option<&'static ExpressionData>,
    /// Optional metadata for the exact Rust type known at registration time.
    pub(crate) concrete_type: Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
}

impl ReturnMetadata {
    /// Creates unit return metadata.
    pub const fn unit() -> Self {
        Self {
            kind: ReturnKind::Unit,
            expression: None,
            concrete_type: None,
        }
    }

    /// Creates metadata for a non-unit return value.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `expression`: Source type expression retained for context-bound
    ///   resolution.
    pub const fn new(kind: ReturnKind, expression: &'static ExpressionData) -> Self {
        Self {
            kind,
            expression: Some(expression),
            concrete_type: None,
        }
    }

    /// Records the exact concrete Rust type when the return value has one.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub const fn with_concrete_type<T: super::Reflect>(mut self) -> Self {
        self.concrete_type = Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>);
        self
    }
}

/// A context-bound view of a method's declared return value.
#[derive(Clone, Copy)]
pub struct ReturnDescriptor<'ctx> {
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    metadata: &'static ReturnMetadata,
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> ReturnDescriptor<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(crate) const fn new(metadata: &'static ReturnMetadata, context: &'ctx ReflectionContext) -> Self {
        Self { metadata, context }
    }
    /// Returns the reflection context that owns this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the declared return-value category.
    pub const fn kind(self) -> ReturnKind {
        self.metadata.kind
    }
    /// Returns the source-level return type expression when one was declared.
    pub fn type_expression(self) -> Option<TypeExpression<'ctx>> {
        self.metadata
            .expression
            .map(|expression| TypeExpression::new(expression, self.context))
    }
    /// Returns the concrete Rust return type when registration retained one.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete_type
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
}

/// Source-level method qualifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodQualifiers {
    /// Whether the source declaration marks the method `const`.
    pub(crate) is_const: bool,
    /// Whether the source declaration marks the method `async`.
    pub(crate) is_async: bool,
    /// Whether calling the source method requires an unsafe operation.
    pub(crate) is_unsafe: bool,
    /// Optional calling convention declared for the method.
    pub(crate) abi: Option<FunctionAbi>,
    /// Whether the method accepts a C-style variable argument list.
    pub(crate) is_variadic: bool,
}

impl MethodQualifiers {
    /// Creates source qualifiers without a custom ABI.
    ///
    /// # Parameters
    ///
    /// - `is_const`: Whether the source method is declared `const`.
    /// - `is_async`: Whether the source method is declared `async`.
    /// - `is_unsafe`: Whether calling the source method requires an unsafe
    ///   operation.
    /// - `is_variadic`: Whether the declaration accepts variadic arguments.
    pub const fn new(is_const: bool, is_async: bool, is_unsafe: bool, is_variadic: bool) -> Self {
        Self {
            is_const,
            is_async,
            is_unsafe,
            abi: None,
            is_variadic,
        }
    }

    /// Records a non-default ABI without accepting runtime identity data.
    ///
    /// # Parameters
    ///
    /// - `abi`: Optional calling convention recorded from the declaration.
    pub const fn with_abi(mut self, abi: Option<FunctionAbi>) -> Self {
        self.abi = abi;
        self
    }
}

/// Function ABI facts retained from the source declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionAbi {
    /// Rust's native ABI.
    Rust,
    /// The C ABI.
    C,
    /// The system ABI for the target platform.
    System,
    /// A named ABI other than the standard variants.
    Other(
        /// ABI name written in the source declaration.
        &'static str,
    ),
}

/// Context-bound namespace of an effective method implementation.
#[derive(Clone, Copy)]
pub enum MethodNamespace<'ctx> {
    /// An inherent method provided by an implementation on the target type.
    Inherent {
        /// The inherent `impl` block that provides this method.
        implementation: crate::runtime::descriptor::impl_descriptor::ImplDescriptor<'ctx>,
    },
    /// A trait method provided by an implementation on the target type.
    Trait {
        /// The trait application that provides this method.
        trait_: crate::runtime::descriptor::trait_descriptor::TraitDescriptor<'ctx>,
        /// The `impl` block that provides this method.
        implementation: crate::runtime::descriptor::impl_descriptor::ImplDescriptor<'ctx>,
    },
}

/// Structured, context-bound qualified method name.
#[derive(Clone, Copy)]
pub enum MethodQualifiedName<'ctx> {
    /// An inherent method qualified by its target type.
    Inherent {
        /// The type that owns the method.
        target: TypeDescriptor<'ctx>,
        /// The method's source name.
        name: &'static str,
    },
    /// A trait method qualified by its target type and trait application.
    Trait {
        /// The type that owns the method.
        target: TypeDescriptor<'ctx>,
        /// The trait that contributes the method.
        trait_: crate::runtime::descriptor::trait_descriptor::TraitDescriptor<'ctx>,
        /// The method's source name.
        name: &'static str,
    },
}

impl std::fmt::Display for MethodQualifiedName<'_> {
    /// Formats the method metadata using its structured Debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inherent { target, name } => {
                write!(formatter, "{}::{name}", target.rust_type_name())
            }
            Self::Trait { target, trait_, name } => {
                write!(formatter, "<{} as {}>::{name}", target.rust_type_name(), trait_.path())
            }
        }
    }
}

/// Why the effective implementation came from its current source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodImplementationSource {
    /// The implementation defines the method body directly.
    Declared,
    /// The method is required by the trait and has no default body.
    Required,
    /// The implementation uses the trait's default body.
    Defaulted,
    /// The implementation overrides a trait default body.
    Overridden,
}

/// Callability reasons recorded during registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodAvailability {
    /// The method has a usable invocation adapter.
    Callable,
    /// A generic specialization must be selected before invocation.
    RequiresSpecialization,
    /// Multiple matching specializations are available, so one must be
    /// selected.
    RequiresSpecializationSelection {
        /// Number of matching specializations that require disambiguation.
        count: usize,
    },
    /// The method has no usable invocation adapter for the recorded
    /// implementation.
    Unavailable {
        /// Reasons the method cannot be invoked dynamically.
        reasons: &'static [InvocationUnavailableReason],
    },
}

/// Source declaration metadata, shared by all effective implementations.
pub struct MethodMetadata {
    /// Returns the unique source identifier when the parameter pattern binds a
    /// name.
    pub(crate) name: &'static str,
    /// Inherent or trait implementation that supplies this effective method.
    pub(crate) namespace: MethodNamespaceMetadata,
    /// Visibility recorded for this method declaration.
    pub(crate) visibility: MethodVisibility,
    /// Receiver kind and type metadata used to validate invocation binding.
    pub(crate) receiver: Option<ReceiverDescriptor>,
    /// Source-ordered metadata for the method’s non-receiver arguments.
    pub(crate) parameters: &'static [ParameterMetadata],
    /// Return category and type metadata for values produced by the method.
    pub(crate) output: &'static ReturnMetadata,
    /// Const, async, unsafe, ABI, and variadic facts copied from the
    /// declaration.
    pub(crate) qualifiers: MethodQualifiers,
    /// Generic parameter metadata required to select a method specialization.
    pub(crate) generics: &'static GenericDefinitionMetadata,
    /// Whether the declaration requires specialization before invocation.
    pub(crate) is_generic: bool,
    /// Whether a trait method uses its default body rather than an
    /// implementation override.
    pub(crate) default_body: bool,
}

/// Static namespace identity used before it is bound to a reflection context.
#[derive(Clone, Copy)]
pub enum MethodNamespaceMetadata {
    /// An inherent implementation namespace.
    Inherent {
        /// Stable source description of the providing inherent impl.
        implementation: &'static str,
    },
    /// A trait namespace identified by source paths.
    Trait {
        /// Source path of the trait providing this method.
        trait_: &'static str,
        /// Source description of the providing impl.
        implementation: &'static str,
    },
    /// A trait namespace with the exact generic arguments of its application.
    TraitApplication {
        /// Source path of the trait providing this method.
        trait_: &'static str,
        /// Source description of the providing impl.
        implementation: &'static str,
        /// Exact generic arguments identifying the trait application.
        identity: &'static [crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
        /// Reflected trait declaration when one is known.
        definition: Option<&'static crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata>,
    },
}

static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: &[],
};
static NO_METHOD_ADAPTER: [InvocationUnavailableReason; 1] = [InvocationUnavailableReason::AdapterUnavailable];
static UNIT_OUTPUT: ReturnMetadata = ReturnMetadata {
    kind: ReturnKind::Unit,
    expression: None,
    concrete_type: None,
};

impl MethodMetadata {
    /// Creates method metadata from a complete typed specification.
    ///
    /// # Parameters
    ///
    /// - `spec`: Static specialization selector being registered.
    pub const fn from_spec(spec: MethodMetadataSpec) -> Self {
        Self {
            name: spec.name,
            namespace: spec.namespace,
            visibility: spec.visibility,
            receiver: spec.receiver,
            parameters: spec.parameters,
            output: spec.output,
            qualifiers: spec.qualifiers,
            generics: spec.generics,
            is_generic: spec.is_generic,
            default_body: spec.default_body,
        }
    }

    /// Creates a zero-argument inherent method declaration with unit output.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `implementation`: Implementation block or function that supplies the
    ///   method.
    pub const fn inherent(name: &'static str, implementation: &'static str) -> Self {
        Self {
            name,
            namespace: MethodNamespaceMetadata::Inherent { implementation },
            visibility: MethodVisibility::Public,
            receiver: None,
            parameters: &[],
            output: &UNIT_OUTPUT,
            qualifiers: MethodQualifiers {
                is_const: false,
                is_async: false,
                is_unsafe: false,
                abi: None,
                is_variadic: false,
            },
            generics: &EMPTY_GENERICS,
            is_generic: false,
            default_body: false,
        }
    }
}

/// Static source facts used to create a method declaration.
pub struct MethodMetadataSpec {
    /// Returns the unique source identifier when the parameter pattern binds a
    /// name.
    name: &'static str,
    /// Inherent or trait implementation that supplies this effective method.
    namespace: MethodNamespaceMetadata,
    /// Visibility recorded for this method declaration.
    visibility: MethodVisibility,
    /// Receiver kind and type metadata used to validate invocation binding.
    receiver: Option<ReceiverDescriptor>,
    /// Source-ordered metadata for the method’s non-receiver arguments.
    parameters: &'static [ParameterMetadata],
    /// Return category and type metadata for values produced by the method.
    output: &'static ReturnMetadata,
    /// Const, async, unsafe, ABI, and variadic facts copied from the
    /// declaration.
    qualifiers: MethodQualifiers,
    /// Generic parameter metadata required to select a method specialization.
    generics: &'static GenericDefinitionMetadata,
    /// Whether the declaration requires specialization before invocation.
    is_generic: bool,
    /// Whether a trait method uses its default body rather than an
    /// implementation override.
    default_body: bool,
}

impl MethodMetadataSpec {
    /// Creates an inherent method specification with no receiver or parameters.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `implementation`: Implementation block or function that supplies the
    ///   method.
    pub const fn inherent(name: &'static str, implementation: &'static str) -> Self {
        Self {
            name,
            namespace: MethodNamespaceMetadata::Inherent { implementation },
            visibility: MethodVisibility::Public,
            receiver: None,
            parameters: &[],
            output: &UNIT_OUTPUT,
            qualifiers: MethodQualifiers::new(false, false, false, false),
            generics: &EMPTY_GENERICS,
            is_generic: false,
            default_body: false,
        }
    }

    /// Creates a complete method specification from typed static metadata.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `namespace`: Implementation namespace that supplies the method.
    /// - `visibility`: Declared visibility used when checking invocation
    ///   access.
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    /// - `parameters`: Source-ordered metadata for non-receiver arguments.
    /// - `output`: Return category and type metadata for the method.
    /// - `qualifiers`: Const, async, unsafe, ABI, and variadic declaration
    ///   facts.
    /// - `generics`: Generic parameter metadata used to resolve
    ///   specializations.
    /// - `is_generic`: Whether invocation requires concrete generic arguments.
    /// - `default_body`: Whether a trait method uses its default implementation
    ///   body.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: &'static str,
        namespace: MethodNamespaceMetadata,
        visibility: MethodVisibility,
        receiver: Option<ReceiverDescriptor>,
        parameters: &'static [ParameterMetadata],
        output: &'static ReturnMetadata,
        qualifiers: MethodQualifiers,
        generics: &'static GenericDefinitionMetadata,
        is_generic: bool,
        default_body: bool,
    ) -> Self {
        Self {
            name,
            namespace,
            visibility,
            receiver,
            parameters,
            output,
            qualifiers,
            generics,
            is_generic,
            default_body,
        }
    }
}

impl EffectiveMethod {
    /// Binds a method declaration to its exact reflected target type.
    ///
    /// # Parameters
    ///
    /// - `metadata`: Static method declaration facts resolved in the reflection
    ///   context.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `metadata`: Static method declaration facts resolved in the reflection
    ///   context.
    pub const fn for_type<T: super::Reflect>(metadata: &'static MethodMetadata) -> Self {
        Self {
            target: crate::runtime::descriptor::metadata::TypeMetadata::of::<T>,
            metadata,
            source: MethodImplementationSource::Declared,
            availability: MethodAvailability::Unavailable {
                reasons: &NO_METHOD_ADAPTER,
            },
            specializations: &[],
            specialization_specs: &[],
            adapters: None,
        }
    }

    /// Binds generated adapters to the method's exact target type.
    ///
    /// # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    pub const fn with_adapters(mut self, adapters: InvocationAdapters) -> Self {
        self.availability = MethodAvailability::Callable;
        self.specializations = &[];
        self.specialization_specs = &[];
        self.adapters = Some(adapters);
        self
    }

    /// Adds statically typed generic method bindings for this target.
    ///
    /// # Parameters
    ///
    /// - `specializations`: Concrete generic instantiations registered for the
    ///   method.
    pub const fn with_specializations(mut self, specializations: &'static [MethodSpecializationSpec]) -> Self {
        self.availability = MethodAvailability::RequiresSpecialization;
        self.specializations = &[];
        self.specialization_specs = specializations;
        self.adapters = None;
        self
    }

    /// Records why this implementation cannot be called through dynamic
    /// invocation.
    ///
    /// # Parameters
    ///
    /// - `reasons`: Structured reasons that prevent this operation.
    pub const fn with_unavailable(mut self, reasons: &'static [InvocationUnavailableReason]) -> Self {
        self.availability = MethodAvailability::Unavailable { reasons };
        self.specializations = &[];
        self.specialization_specs = &[];
        self.adapters = None;
        self
    }

    /// Records whether the callable implementation is declared, defaulted, or
    /// overridden.
    ///
    /// # Parameters
    ///
    /// - `source`: Source declaration or expression from which this metadata
    ///   was derived.
    pub const fn with_source(mut self, source: MethodImplementationSource) -> Self {
        self.source = source;
        self
    }

    /// Checks whether two registration records describe the same effective
    /// method. # Parameters
    ///
    /// - `other`: ABI name or metadata value that is not one of the standard
    ///   forms.
    /// # Parameters
    ///
    /// - `other`: Second metadata value used in the equivalence comparison.
    pub(crate) fn same_registration(&self, other: &Self) -> bool {
        self.target_type_id() == other.target_type_id()
            && std::ptr::eq(self.metadata, other.metadata)
            && self.source == other.source
            && self.availability == other.availability
            && std::ptr::eq(self.specializations, other.specializations)
            && std::ptr::eq(self.specialization_specs, other.specialization_specs)
            && same_adapters(self.adapters, other.adapters)
    }

    /// Returns the concrete target type identity used to resolve registered
    /// methods.
    pub(crate) fn target_type_id(&self) -> std::any::TypeId {
        (self.target)().type_id()
    }

    /// Resolves declared generic arguments against the method’s specialization
    /// metadata.
    pub(crate) fn resolve_specializations(&self) -> Result<Box<[ResolvedMethodSpecialization]>, &'static str> {
        if self.specialization_specs.is_empty() {
            Ok(self.specializations.to_vec().into_boxed_slice())
        } else {
            self.specialization_specs
                .iter()
                .copied()
                .map(|spec| spec.resolve(self.metadata.generics.parameters))
                .collect::<Result<Vec<_>, _>>()
                .map(Vec::into_boxed_slice)
        }
    }
}

/// Checks whether adapter slots refer to the same generated invocation
/// functions. # Parameters
///
/// - `left`: First metadata value in the equality comparison.
/// - `right`: Second metadata value in the equality comparison.
/// # Parameters
///
/// - `left`: First metadata value used in the equivalence comparison.
/// - `right`: Second metadata value used in the equivalence comparison.
fn same_adapters(left: Option<InvocationAdapters>, right: Option<InvocationAdapters>) -> bool {
    let (Some(left), Some(right)) = (left, right) else {
        return left.is_none() && right.is_none();
    };
    same_adapter(left.local, right.local)
        && same_adapter(left.thread_safe, right.thread_safe)
        && same_adapter(left.catching_local, right.catching_local)
        && same_adapter(left.catching_thread_safe, right.catching_thread_safe)
        && same_adapter(left.pinned_local, right.pinned_local)
        && same_adapter(left.pinned_thread_safe, right.pinned_thread_safe)
}

/// Compares two optional adapter references by their registered function
/// address. # Parameters
///
/// - `left`: First metadata value in the equality comparison.
/// - `right`: Second metadata value in the equality comparison.
///
/// # Type Parameters
///
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
///
/// # Parameters
///
/// - `left`: First metadata value in the equality comparison.
/// - `right`: Second metadata value in the equality comparison.
fn same_adapter<T: ?Sized>(left: Option<&'static T>, right: Option<&'static T>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => std::ptr::eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

/// The callable implementation selected for a target type.
#[derive(Clone, Copy)]
pub struct EffectiveMethod {
    /// Concrete target type whose method metadata is being registered.
    pub(crate) target: fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    /// Static declaration facts shared by descriptors created in a reflection
    /// context.
    pub(crate) metadata: &'static MethodMetadata,
    /// Source method declaration used to build the effective method metadata.
    pub(crate) source: MethodImplementationSource,
    /// Invocation availability after generic arguments are checked.
    pub(crate) availability: MethodAvailability,
    /// Registered concrete generic instantiations available for this method.
    pub(crate) specializations: &'static [ResolvedMethodSpecialization],
    /// Static specifications used to resolve generic arguments at invocation
    /// time.
    pub(crate) specialization_specs: &'static [MethodSpecializationSpec],
    /// Generated adapters for supported value modes and invocation variants.
    pub(crate) adapters: Option<InvocationAdapters>,
}

/// A typed generic argument used by a statically registered method
/// specialization.
#[derive(Clone, Copy)]
pub struct MethodSpecializationArgument {
    /// Concrete signed or unsigned constant used in a specialization selector.
    value: MethodSpecializationArgumentValue,
}

/// One concrete generic argument used to select a method specialization.
#[derive(Clone, Copy)]
enum MethodSpecializationArgumentValue {
    /// Concrete Rust type argument recorded by its static type metadata.
    Type(
        /// Function that resolves the type metadata without runtime allocation.
        fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    ),
    /// Lifetime argument represented by its source spelling.
    Lifetime(
        /// Lifetime name, including the leading apostrophe.
        &'static str,
    ),
    /// Concrete signed or unsigned constant used in a specialization selector.
    /// Returns the declared return-value category.
    /// Signed integer constant argument.
    ConstSigned {
        /// Integer type used to interpret the constant.
        kind: ConstIntegerKind,
        /// Signed value supplied as the generic argument.
        value: i128,
    },
    /// Concrete signed or unsigned constant used in a specialization selector.
    /// Returns the declared return-value category.
    /// Unsigned integer constant argument.
    ConstUnsigned {
        /// Integer type used to interpret the constant.
        kind: ConstIntegerKind,
        /// Unsigned value supplied as the generic argument.
        value: u128,
    },
    /// Unsigned magnitude retained for an integer literal selector.
    /// Whether an integer literal selector has a leading minus sign.
    /// Integer literal selector before its inferred integer type is applied.
    ConstIntegerLiteral {
        /// Whether the source literal has a leading minus sign.
        negative: bool,
        /// Absolute value of the literal, stored without signed overflow.
        magnitude: u128,
    },
    /// Boolean constant argument.
    ConstBoolean(
        /// Boolean value supplied by the specialization.
        bool,
    ),
    /// Character constant argument.
    ConstCharacter(
        /// Unicode scalar value supplied by the specialization.
        char,
    ),
}

/// Provides the constintegerkind operation used to resolve or dispatch
/// reflected methods.
#[derive(Clone, Copy)]
enum ConstIntegerKind {
    /// The selector expects an `i8` constant.
    I8,
    /// The selector expects an `i16` constant.
    I16,
    /// The selector expects an `i32` constant.
    I32,
    /// The selector expects an `i64` constant.
    I64,
    /// The selector expects an `i128` constant.
    I128,
    /// Provides the isize operation used to resolve or dispatch reflected
    /// methods.
    Isize,
    /// The selector expects a `u8` constant.
    U8,
    /// The selector expects a `u16` constant.
    U16,
    /// The selector expects a `u32` constant.
    U32,
    /// The selector expects a `u64` constant.
    U64,
    /// The selector expects a `u128` constant.
    U128,
    /// Provides the usize operation used to resolve or dispatch reflected
    /// methods.
    Usize,
}

impl MethodSpecializationArgument {
    /// Records a concrete type argument without exposing runtime identity data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub const fn type_argument<T: 'static>() -> Self {
        Self {
            value: MethodSpecializationArgumentValue::Type(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
        }
    }

    /// Records a concrete lifetime argument for exact specialization selection.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    pub const fn lifetime(name: &'static str) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::Lifetime(name),
        }
    }

    /// Records a boolean const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn const_bool(value: bool) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstBoolean(value),
        }
    }

    /// Records a character const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn const_char(value: char) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstCharacter(value),
        }
    }

    /// Records an unsuffixed integer literal for validation against its
    /// declaration.
    ///
    /// # Parameters
    ///
    /// - `negative`: Whether the integer literal has a leading minus sign.
    /// - `magnitude`: Absolute integer literal magnitude before applying its
    ///   sign.
    pub const fn const_integer_literal(negative: bool, magnitude: u128) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstIntegerLiteral { negative, magnitude },
        }
    }

    /// Records a signed integer const argument.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `value`: Owned value retained for validation and construction.
    const fn const_signed(kind: ConstIntegerKind, value: i128) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstSigned { kind, value },
        }
    }

    /// Records an unsigned integer const argument.
    ///
    /// # Parameters
    ///
    /// - `kind`: Receiver form, return category, or constant type being
    ///   recorded.
    /// - `value`: Owned value retained for validation and construction.
    const fn const_unsigned(kind: ConstIntegerKind, value: u128) -> Self {
        Self {
            value: MethodSpecializationArgumentValue::ConstUnsigned { kind, value },
        }
    }

    /// Records an `i8` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i8(value: i8) -> Self {
        Self::const_signed(ConstIntegerKind::I8, value as i128)
    }

    /// Records an `i16` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i16(value: i16) -> Self {
        Self::const_signed(ConstIntegerKind::I16, value as i128)
    }

    /// Records an `i32` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i32(value: i32) -> Self {
        Self::const_signed(ConstIntegerKind::I32, value as i128)
    }

    /// Records an `i64` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i64(value: i64) -> Self {
        Self::const_signed(ConstIntegerKind::I64, value as i128)
    }

    /// Records an `i128` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn i128(value: i128) -> Self {
        Self::const_signed(ConstIntegerKind::I128, value)
    }

    /// Records an `isize` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn isize(value: isize) -> Self {
        Self::const_signed(ConstIntegerKind::Isize, value as i128)
    }

    /// Records a `u8` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u8(value: u8) -> Self {
        Self::const_unsigned(ConstIntegerKind::U8, value as u128)
    }

    /// Records a `u16` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u16(value: u16) -> Self {
        Self::const_unsigned(ConstIntegerKind::U16, value as u128)
    }

    /// Records a `u32` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u32(value: u32) -> Self {
        Self::const_unsigned(ConstIntegerKind::U32, value as u128)
    }

    /// Records a `u64` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u64(value: u64) -> Self {
        Self::const_unsigned(ConstIntegerKind::U64, value as u128)
    }

    /// Records a `u128` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn u128(value: u128) -> Self {
        Self::const_unsigned(ConstIntegerKind::U128, value)
    }

    /// Records a `usize` const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub const fn usize(value: usize) -> Self {
        Self::const_unsigned(ConstIntegerKind::Usize, value as u128)
    }
}

/// A concrete specialization declaration with typed arguments and adapters.
#[derive(Clone, Copy)]
pub struct MethodSpecializationSpec {
    /// Generic arguments supplied when selecting a method specialization.
    arguments: &'static [MethodSpecializationArgument],
    /// Invocation availability after generic arguments are checked.
    availability: InvocationAvailability,
    /// Generated adapters for supported value modes and invocation variants.
    adapters: InvocationAdapters,
}

impl MethodSpecializationSpec {
    /// Creates a callable specialization declaration.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Generic arguments or invocation arguments supplied by the
    ///   caller.
    /// - `adapters`: Generated invocation adapters registered for the method.
    pub const fn new(arguments: &'static [MethodSpecializationArgument], adapters: InvocationAdapters) -> Self {
        Self {
            arguments,
            availability: InvocationAvailability::Callable,
            adapters,
        }
    }

    /// Marks this specialization unavailable with typed reasons.
    ///
    /// # Parameters
    ///
    /// - `reasons`: Structured reasons that prevent this operation.
    pub const fn with_unavailable(mut self, reasons: &'static [InvocationUnavailableReason]) -> Self {
        self.availability = InvocationAvailability::Unavailable { reasons };
        self.adapters = NO_ADAPTERS;
        self
    }

    /// Matches requested specialization arguments against the registered
    /// candidates. # Parameters
    ///
    /// - `parameters`: Source-ordered metadata for non-receiver arguments.
    /// # Parameters
    ///
    /// - `parameters`: Source-ordered parameter metadata used to validate
    ///   invocation arguments.
    fn resolve(
        self,
        parameters: &'static [GenericParameterMetadata],
    ) -> Result<ResolvedMethodSpecialization, &'static str> {
        if self.arguments.len() != parameters.len() {
            return Err("method specialization argument count does not match its generic parameters");
        }
        let mut identities = Vec::with_capacity(self.arguments.len());
        let mut selectors = Vec::with_capacity(self.arguments.len());
        let mut metadata = Vec::with_capacity(self.arguments.len());
        for (argument, parameter) in self.arguments.iter().zip(parameters) {
            let (identity, selector, metadata_value) = match argument.value {
                MethodSpecializationArgumentValue::Type(type_metadata) => {
                    let GenericParameterMetadataKind::Type { .. } = parameter.kind else {
                        return Err("type specialization argument does not target a type parameter");
                    };
                    let concrete_type = type_metadata();
                    (
                        GenericArgumentIdentity::Type(concrete_type.rust_name().into()),
                        GenericArgumentSelector::Type(concrete_type.type_id()),
                        Some(GenericArgumentMetadata::concrete_type(parameter, type_metadata)),
                    )
                }
                MethodSpecializationArgumentValue::Lifetime(name) => (
                    GenericArgumentIdentity::Lifetime(name.into()),
                    GenericArgumentSelector::Lifetime(name.into()),
                    None,
                ),
                MethodSpecializationArgumentValue::ConstSigned { kind, value } => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("{}:{value}", kind.type_name()).into()),
                        integer_selector(kind, value.to_string()),
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::SignedInteger(value),
                        )),
                    )
                }
                MethodSpecializationArgumentValue::ConstUnsigned { kind, value } => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("{}:{value}", kind.type_name()).into()),
                        integer_selector(kind, value.to_string()),
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::UnsignedInteger(value),
                        )),
                    )
                }
                MethodSpecializationArgumentValue::ConstIntegerLiteral { negative, magnitude } => {
                    let GenericParameterMetadataKind::Const { ty, .. } = parameter.kind else {
                        return Err("integer specialization argument does not target a const parameter");
                    };
                    let kind = integer_kind_for_expression(ty)
                        .ok_or("unsuffixed integer specialization requires a primitive integer const type")?;
                    let value = kind
                        .normalize(negative, magnitude)
                        .ok_or("integer specialization literal is out of range for its const parameter")?;
                    let expression = match kind {
                        ConstIntegerKind::I8
                        | ConstIntegerKind::I16
                        | ConstIntegerKind::I32
                        | ConstIntegerKind::I64
                        | ConstIntegerKind::I128
                        | ConstIntegerKind::Isize => ConstExpression::SignedInteger(
                            value.parse().map_err(|_| "invalid normalized signed integer")?,
                        ),
                        _ => ConstExpression::UnsignedInteger(
                            value.parse().map_err(|_| "invalid normalized unsigned integer")?,
                        ),
                    };
                    (
                        GenericArgumentIdentity::Const(format!("{}:{value}", kind.type_name()).into()),
                        integer_selector(kind, value.clone()),
                        Some(GenericArgumentMetadata::const_argument(parameter, expression)),
                    )
                }
                MethodSpecializationArgumentValue::ConstBoolean(value) => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("bool:{value}").into()),
                        GenericArgumentSelector::Const {
                            type_id: TypeId::of::<bool>(),
                            value: value.to_string().into(),
                        },
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::Boolean(value),
                        )),
                    )
                }
                MethodSpecializationArgumentValue::ConstCharacter(value) => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("char:{value:?}").into()),
                        GenericArgumentSelector::Const {
                            type_id: TypeId::of::<char>(),
                            value: u32::from(value).to_string().into(),
                        },
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::Character(value),
                        )),
                    )
                }
            };
            identities.push(identity);
            selectors.push(selector);
            if let Some(value) = metadata_value {
                metadata.push(value);
            }
        }
        Ok(ResolvedMethodSpecialization {
            arguments: ResolvedMethodArguments::Owned {
                display: identities.into_boxed_slice(),
                selectors: selectors.into_boxed_slice(),
            },
            argument_metadata: Some(metadata.into()),
            availability: self.availability,
            adapters: self.adapters,
        })
    }
}

/// Infers the integer representation from a source type expression.
/// # Parameters
///
/// - `expression`: Source type expression retained for context-bound
///   resolution.
fn integer_kind_for_expression(expression: &'static ExpressionData) -> Option<ConstIntegerKind> {
    let ExpressionData::Named {
        path,
        definition,
        arguments,
    } = expression
    else {
        return None;
    };
    if definition.is_some() || !arguments.is_empty() {
        return None;
    }
    match *path {
        "i8" | "core::primitive::i8" | "std::primitive::i8" => Some(ConstIntegerKind::I8),
        "i16" | "core::primitive::i16" | "std::primitive::i16" => Some(ConstIntegerKind::I16),
        "i32" | "core::primitive::i32" | "std::primitive::i32" => Some(ConstIntegerKind::I32),
        "i64" | "core::primitive::i64" | "std::primitive::i64" => Some(ConstIntegerKind::I64),
        "i128" | "core::primitive::i128" | "std::primitive::i128" => Some(ConstIntegerKind::I128),
        "isize" | "core::primitive::isize" | "std::primitive::isize" => Some(ConstIntegerKind::Isize),
        "u8" | "core::primitive::u8" | "std::primitive::u8" => Some(ConstIntegerKind::U8),
        "u16" | "core::primitive::u16" | "std::primitive::u16" => Some(ConstIntegerKind::U16),
        "u32" | "core::primitive::u32" | "std::primitive::u32" => Some(ConstIntegerKind::U32),
        "u64" | "core::primitive::u64" | "std::primitive::u64" => Some(ConstIntegerKind::U64),
        "u128" | "core::primitive::u128" | "std::primitive::u128" => Some(ConstIntegerKind::U128),
        "usize" | "core::primitive::usize" | "std::primitive::usize" => Some(ConstIntegerKind::Usize),
        _ => None,
    }
}

impl ConstIntegerKind {
    /// Returns the source-level type name used for a specialization selector.
    fn type_name(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
        }
    }

    /// Normalizes a source expression before comparing specialization
    /// selectors. # Parameters
    ///
    /// - `negative`: Whether the integer literal has a leading minus sign.
    /// - `magnitude`: Absolute integer literal magnitude before applying its
    ///   sign.
    /// # Parameters
    ///
    /// - `negative`: Whether the integer selector has a leading minus sign.
    /// - `magnitude`: Absolute magnitude of the integer selector before
    ///   applying its sign.
    fn normalize(self, negative: bool, magnitude: u128) -> Option<String> {
        let (signed, bits) = match self {
            Self::I8 => (true, 8),
            Self::I16 => (true, 16),
            Self::I32 => (true, 32),
            Self::I64 => (true, 64),
            Self::I128 => (true, 128),
            Self::Isize => (true, usize::BITS),
            Self::U8 => (false, 8),
            Self::U16 => (false, 16),
            Self::U32 => (false, 32),
            Self::U64 => (false, 64),
            Self::U128 => (false, 128),
            Self::Usize => (false, usize::BITS),
        };
        if signed {
            let negative_limit = 1_u128 << (bits - 1);
            let positive_limit = negative_limit - 1;
            if (negative && magnitude > negative_limit) || (!negative && magnitude > positive_limit) {
                return None;
            }
            let value = if negative {
                if magnitude == (1_u128 << 127) {
                    i128::MIN
                } else {
                    -(magnitude as i128)
                }
            } else {
                magnitude as i128
            };
            Some(value.to_string())
        } else if negative || (bits < 128 && magnitude >= (1_u128 << bits)) {
            None
        } else {
            Some(magnitude.to_string())
        }
    }
}

/// Extracts an integer specialization selector from its source expression.
/// # Parameters
///
/// - `kind`: Receiver form, return category, or constant type being recorded.
/// - `value`: Owned value retained for validation and construction.
fn integer_selector(kind: ConstIntegerKind, value: String) -> GenericArgumentSelector {
    let type_id = match kind {
        ConstIntegerKind::I8 => TypeId::of::<i8>(),
        ConstIntegerKind::I16 => TypeId::of::<i16>(),
        ConstIntegerKind::I32 => TypeId::of::<i32>(),
        ConstIntegerKind::I64 => TypeId::of::<i64>(),
        ConstIntegerKind::I128 => TypeId::of::<i128>(),
        ConstIntegerKind::Isize => TypeId::of::<isize>(),
        ConstIntegerKind::U8 => TypeId::of::<u8>(),
        ConstIntegerKind::U16 => TypeId::of::<u16>(),
        ConstIntegerKind::U32 => TypeId::of::<u32>(),
        ConstIntegerKind::U64 => TypeId::of::<u64>(),
        ConstIntegerKind::U128 => TypeId::of::<u128>(),
        ConstIntegerKind::Usize => TypeId::of::<usize>(),
    };
    GenericArgumentSelector::Const {
        type_id,
        value: value.into_boxed_str(),
    }
}

/// A compile-time-bound implementation of one generic method.
#[derive(Clone)]
pub(crate) struct ResolvedMethodSpecialization {
    /// Generic arguments supplied when selecting a method specialization.
    pub(crate) arguments: ResolvedMethodArguments,
    /// Metadata describing each generic parameter accepted by the method.
    pub(crate) argument_metadata: Option<std::sync::Arc<[GenericArgumentMetadata]>>,
    /// Invocation availability after generic arguments are checked.
    pub(crate) availability: InvocationAvailability,
    /// Generated adapters for supported value modes and invocation variants.
    pub(crate) adapters: InvocationAdapters,
}

/// Resolved generic arguments retained for invocation and display.
#[derive(Clone)]
pub(crate) enum ResolvedMethodArguments {
    #[cfg(test)]
    Static(
        /// Static identities used by hand-written method catalog fixtures.
        &'static [GenericArgumentIdentity],
    ),
    /// Owns resolved argument identities and their matching selectors.
    Owned {
        /// Provides the display operation used to resolve or dispatch reflected
        /// methods.
        display: Box<[GenericArgumentIdentity]>,
        /// Returns the selector list used to compare generic arguments.
        selectors: Box<[GenericArgumentSelector]>,
    },
}

impl ResolvedMethodArguments {
    /// Borrows the resolved generic argument identities as a slice.
    pub(crate) fn as_slice(&self) -> &[GenericArgumentIdentity] {
        match self {
            #[cfg(test)]
            Self::Static(arguments) => arguments,
            Self::Owned { display, .. } => display,
        }
    }

    /// Compares resolved selectors to determine whether two registrations
    /// select the same specialization. # Parameters
    ///
    /// - `other`: ABI name or metadata value that is not one of the standard
    ///   forms.
    /// # Parameters
    ///
    /// - `other`: Second metadata value used in the equivalence comparison.
    pub(crate) fn same_selection(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Owned { selectors: left, .. }, Self::Owned { selectors: right, .. }) => {
                selector_slices_match(left, right)
            }
            #[cfg(test)]
            (Self::Static(left), Self::Static(right)) => left.is_empty() && right.is_empty(),
            #[cfg(test)]
            _ => false,
        }
    }

    /// Checks whether the supplied generic arguments satisfy this
    /// specialization. # Parameters
    ///
    /// - `selectors`: Resolved generic argument selectors used to compare
    ///   specializations.
    /// # Parameters
    ///
    /// - `selectors`: Resolved generic argument selectors compared during
    ///   specialization lookup.
    fn matches(&self, selectors: &[GenericArgumentSelector]) -> bool {
        match self {
            Self::Owned {
                selectors: own_selectors,
                ..
            } => selector_slices_match(own_selectors, selectors),
            #[cfg(test)]
            Self::Static(arguments) => arguments.is_empty() && selectors.is_empty(),
        }
    }
}

/// Compares two selector lists element by element.
/// # Parameters
///
/// - `left`: First metadata value in the equality comparison.
/// - `right`: Second metadata value in the equality comparison.
fn selector_slices_match(left: &[GenericArgumentSelector], right: &[GenericArgumentSelector]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(left, right)| left.matches(right))
}

/// Provides the contexteffectivemethod operation used to resolve or dispatch
/// reflected methods.
#[derive(Clone)]
pub(crate) struct ContextEffectiveMethod {
    /// Returns the static effective method record shared by the catalog view.
    pub(crate) effective: EffectiveMethod,
    /// Registered concrete generic instantiations available for this method.
    pub(crate) specializations: std::sync::Arc<[ResolvedMethodSpecialization]>,
    /// Provides the implementation operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) implementation:
        Option<&'static crate::runtime::descriptor::impl_descriptor::metadata::ImplApplicationMetadata>,
}

/// Mode-specific adapters generated for a concrete specialization.
#[derive(Clone, Copy)]
pub struct InvocationAdapters {
    /// Adapter for calls using local value storage.
    pub(crate) local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Adapter for calls using thread-safe value storage.
    pub(crate) thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the catching local operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) catching_local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Provides the catching thread safe operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) catching_thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the pinned local operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) pinned_local: Option<&'static dyn PinnedInvocationAdapter<Local>>,
    /// Provides the pinned thread safe operation used to resolve or dispatch
    /// reflected methods.
    pub(crate) pinned_thread_safe: Option<&'static dyn PinnedInvocationAdapter<ThreadSafe>>,
}

static NO_ADAPTERS: InvocationAdapters = InvocationAdapters {
    local: None,
    thread_safe: None,
    catching_local: None,
    catching_thread_safe: None,
    pinned_local: None,
    pinned_thread_safe: None,
};

/// Builder for an immutable set of generated invocation adapters.
pub struct InvocationAdaptersBuilder {
    /// Adapter for calls using local value storage.
    local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Adapter for calls using thread-safe value storage.
    thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the catching local operation used to resolve or dispatch
    /// reflected methods.
    catching_local: Option<&'static dyn InvocationAdapter<Local>>,
    /// Provides the catching thread safe operation used to resolve or dispatch
    /// reflected methods.
    catching_thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    /// Provides the pinned local operation used to resolve or dispatch
    /// reflected methods.
    pinned_local: Option<&'static dyn PinnedInvocationAdapter<Local>>,
    /// Provides the pinned thread safe operation used to resolve or dispatch
    /// reflected methods.
    pinned_thread_safe: Option<&'static dyn PinnedInvocationAdapter<ThreadSafe>>,
}

impl InvocationAdaptersBuilder {
    /// Sets the local adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn local(mut self, adapter: &'static dyn InvocationAdapter<Local>) -> Self {
        self.local = Some(adapter);
        self
    }

    /// Sets the thread-safe adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn thread_safe(mut self, adapter: &'static dyn InvocationAdapter<ThreadSafe>) -> Self {
        self.thread_safe = Some(adapter);
        self
    }

    /// Sets the local adapter used by explicit panic-catching invocation.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn catching_local(mut self, adapter: &'static dyn InvocationAdapter<Local>) -> Self {
        self.catching_local = Some(adapter);
        self
    }

    /// Sets the thread-safe adapter used by explicit panic-catching invocation.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn catching_thread_safe(mut self, adapter: &'static dyn InvocationAdapter<ThreadSafe>) -> Self {
        self.catching_thread_safe = Some(adapter);
        self
    }

    /// Sets the local pinned receiver adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn pinned_local<T: ?Sized + 'static>(
        mut self,
        adapter: &'static crate::runtime::invoke::PinnedFunctionInvocationAdapter<Local, T>,
    ) -> Self {
        self.pinned_local = Some(adapter);
        self
    }

    /// Sets the thread-safe pinned receiver adapter.
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `adapter`: Generated adapter function registered for this invocation
    ///   mode.
    pub const fn pinned_thread_safe<T: ?Sized + 'static>(
        mut self,
        adapter: &'static crate::runtime::invoke::PinnedFunctionInvocationAdapter<ThreadSafe, T>,
    ) -> Self {
        self.pinned_thread_safe = Some(adapter);
        self
    }

    /// Finishes the immutable adapter set.
    pub const fn build(self) -> InvocationAdapters {
        InvocationAdapters {
            local: self.local,
            thread_safe: self.thread_safe,
            catching_local: self.catching_local,
            catching_thread_safe: self.catching_thread_safe,
            pinned_local: self.pinned_local,
            pinned_thread_safe: self.pinned_thread_safe,
        }
    }
}

/// A method declaration combined with its effective implementation in one
/// context.
#[derive(Clone)]
pub struct MethodDescriptor<'ctx> {
    /// Returns the static effective method record shared by the catalog view.
    effective: EffectiveMethod,
    /// Registered concrete generic instantiations available for this method.
    specializations: std::sync::Arc<[ResolvedMethodSpecialization]>,
    /// Provides the implementation operation used to resolve or dispatch
    /// reflected methods.
    implementation: Option<&'static crate::runtime::descriptor::impl_descriptor::metadata::ImplApplicationMetadata>,
    /// Returns the reflection context required to resolve this descriptor’s
    /// referenced types.
    context: &'ctx ReflectionContext,
}

impl<'ctx> MethodDescriptor<'ctx> {
    /// Creates static metadata from the declaration facts supplied by
    /// registration. # Parameters
    ///
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(crate) fn new(effective: EffectiveMethod, context: &'ctx ReflectionContext) -> Self {
        Self {
            effective,
            specializations: std::sync::Arc::from(effective.specializations),
            implementation: None,
            context,
        }
    }
    /// Provides the with resolved specializations operation used to resolve or
    /// dispatch reflected methods. # Parameters
    ///
    /// - `specializations`: Concrete generic instantiations registered for the
    ///   method.
    /// - `implementation`: Implementation block or function that supplies the
    ///   method.
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
    /// # Parameters
    ///
    /// - `specializations`: Concrete generic method implementations available
    ///   for this registration.
    /// - `implementation`: Method implementation associated with this effective
    ///   registration.
    /// - `context`: Reflection context used to resolve this descriptor’s source
    ///   expressions.
    pub(crate) fn with_resolved_specializations(
        effective: EffectiveMethod,
        specializations: std::sync::Arc<[ResolvedMethodSpecialization]>,
        implementation: Option<&'static crate::runtime::descriptor::impl_descriptor::metadata::ImplApplicationMetadata>,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self {
            effective,
            specializations,
            implementation,
            context,
        }
    }
    /// Returns the context that owns this method descriptor.
    pub fn context(&self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns the concrete type on which this method is available.
    pub fn target(&self) -> TypeDescriptor<'ctx> {
        TypeDescriptor::new((self.effective.target)(), self.context)
    }
    /// Returns the method's source name.
    pub fn name(&self) -> &'static str {
        self.effective.metadata.name
    }
    /// Returns the context-bound implementation namespace of this method.
    pub fn namespace(&self) -> MethodNamespace<'ctx> {
        let implementation = self.implementation_descriptor();
        match &self.effective.metadata.namespace {
            MethodNamespaceMetadata::Inherent { .. } => MethodNamespace::Inherent { implementation },
            MethodNamespaceMetadata::Trait { .. } | MethodNamespaceMetadata::TraitApplication { .. } => {
                MethodNamespace::Trait {
                    trait_: implementation
                        .implemented_trait()
                        .expect("a trait method has a trait implementation"),
                    implementation,
                }
            }
        }
    }
    /// Returns the method's structured qualified name.
    pub fn qualified_name(&self) -> MethodQualifiedName<'ctx> {
        match self.namespace() {
            MethodNamespace::Inherent { .. } => MethodQualifiedName::Inherent {
                target: self.target(),
                name: self.name(),
            },
            MethodNamespace::Trait { trait_, .. } => MethodQualifiedName::Trait {
                target: self.target(),
                trait_,
                name: self.name(),
            },
        }
    }
    /// Provides the implementation descriptor operation used to resolve or
    /// dispatch reflected methods.
    fn implementation_descriptor(&self) -> crate::runtime::descriptor::impl_descriptor::ImplDescriptor<'ctx> {
        if let Some(implementation) = self.implementation {
            return crate::runtime::descriptor::impl_descriptor::ImplDescriptor::new(implementation, self.context);
        }
        self.target()
            .impls()
            .expect("a method descriptor has registered impl applications")
            .iter()
            .find(|implementation| {
                implementation
                    .methods()
                    .iter()
                    .any(|method| std::ptr::eq(method.effective.metadata, self.effective.metadata))
            })
            .expect("a method descriptor belongs to a registered impl application")
    }
    /// Returns the source declaration that contributes this effective method.
    pub fn definition(&self) -> crate::runtime::descriptor::trait_descriptor::MethodDefinitionDescriptor<'ctx> {
        match self.effective.source {
            MethodImplementationSource::Defaulted | MethodImplementationSource::Required => {
                let trait_definition = self
                    .implementation_descriptor()
                    .implemented_trait()
                    .and_then(|trait_| trait_.find_definition())
                    .expect("a linked trait method has a reflected trait definition");
                trait_definition
                    .method(self.name())
                    .expect("the effective method is declared by its reflected trait")
            }
            MethodImplementationSource::Declared | MethodImplementationSource::Overridden => self
                .implementation_descriptor()
                .definition()
                .method(self.name())
                .expect("the effective method is declared by its implementation"),
        }
    }
    /// Returns the visibility retained from the method declaration.
    pub fn visibility(&self) -> crate::runtime::descriptor::trait_descriptor::Visibility {
        match self.effective.metadata.visibility {
            MethodVisibility::Public => crate::runtime::descriptor::trait_descriptor::Visibility::Public,
            MethodVisibility::Restricted => crate::runtime::descriptor::trait_descriptor::Visibility::RestrictedUnknown,
            MethodVisibility::Private => crate::runtime::descriptor::trait_descriptor::Visibility::Private,
        }
    }
    /// Returns metadata for the method receiver, when the declaration has one.
    pub fn receiver(&self) -> Option<ReceiverDescriptor> {
        self.effective
            .metadata
            .receiver
            .map(|receiver| receiver.for_target(self.effective.target))
    }
    /// Returns the method's non-receiver parameters in source order.
    pub fn parameters(&self) -> Parameters<'ctx> {
        Parameters::new(self.effective.metadata.parameters, self.context)
    }
    /// Looks up a named identifier parameter.
    ///
    /// Returns `LookupError::Missing` if no parameter has that identifier, or
    /// `LookupError::Ambiguous` if metadata contains multiple matches.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    pub fn parameter(&self, name: &str) -> Result<ParameterDescriptor<'ctx>, LookupError> {
        let mut matches = self.effective.metadata.parameters.iter().filter(
            |parameter| matches!(parameter.pattern, ParameterPattern::Identifier(candidate) if candidate == name),
        );
        let Some(first) = matches.next() else {
            return Err(LookupError::Missing {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
            });
        };
        let mut candidates = vec![ParameterDescriptor::new(first, self.context)];
        candidates.extend(matches.map(|parameter| ParameterDescriptor::new(parameter, self.context)));
        match candidates.as_slice() {
            [only] => Ok(*only),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
                candidates: candidates.into_iter().map(parameter_candidate).collect(),
            }),
        }
    }
    /// Returns the declared return-value descriptor.
    pub fn output(&self) -> ReturnDescriptor<'ctx> {
        ReturnDescriptor::new(self.effective.metadata.output, self.context)
    }
    /// Returns the method's receiver and async qualifiers.
    pub fn qualifiers(&self) -> MethodQualifiers {
        self.effective.metadata.qualifiers
    }
    /// Returns the method's generic parameter definitions.
    pub fn generics(&self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.effective.metadata.generics, self.context)
    }
    /// Returns where this method's reflected implementation came from.
    pub fn implementation_source(&self) -> MethodImplementationSource {
        self.effective.source
    }
    /// Returns whether this declaration has a directly callable implementation.
    pub fn availability(&self) -> MethodAvailability {
        if !self.effective.metadata.is_generic {
            return self.effective.availability;
        }
        match self.specializations.as_ref() {
            [] => MethodAvailability::RequiresSpecialization,
            [specialization] => match specialization.availability {
                InvocationAvailability::Callable => MethodAvailability::Callable,
                InvocationAvailability::Unavailable { reasons } => MethodAvailability::Unavailable { reasons },
            },
            specializations => MethodAvailability::RequiresSpecializationSelection {
                count: specializations.len(),
            },
        }
    }
    /// Returns the concrete implementations bound for this method.
    pub fn specializations(&self) -> MethodSpecializations<'ctx> {
        MethodSpecializations { method: self.clone() }
    }
    /// Selects the specialization with exactly these generic arguments.
    ///
    /// Returns `LookupError::Missing` when no matching binding exists and
    /// `LookupError::Ambiguous` when multiple identical bindings are present.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Generic arguments or invocation arguments supplied by the
    ///   caller.
    pub fn specialization(
        &self,
        arguments: &MethodArguments,
    ) -> Result<MethodSpecializationDescriptor<'ctx>, LookupError> {
        let matches = self
            .specializations
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| candidate.arguments.matches(&arguments.selectors).then_some(index))
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [index] => {
                let specializations = self.specializations.clone();
                Ok(MethodSpecializationDescriptor {
                    method: self.clone(),
                    specializations,
                    index: *index,
                })
            }
            [] => Err(LookupError::Missing {
                subject: LookupSubject::MethodSpecialization,
                query: LookupQuery::GenericArguments(arguments.arguments.clone().into_boxed_slice()),
            }),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::MethodSpecialization,
                query: LookupQuery::GenericArguments(arguments.arguments.clone().into_boxed_slice()),
                candidates: matches
                    .iter()
                    .enumerate()
                    .map(|(index, _)| specialization_candidate(index))
                    .collect(),
            }),
        }
    }
    /// Invokes the method after validating visibility, safety, receiver, and
    /// arguments.
    ///
    /// Panics from the method are propagated to the caller.
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    pub fn invoke<'call, M: ValueMode>(
        &self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>> {
        match self.invoke_inner(invocation, false, false) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(failure),
            Err(InvocationAttempt::Panicked(_)) => unreachable!("ordinary invoke does not catch"),
        }
    }

    /// Invokes the method and converts a panic raised after invocation starts
    /// into [`CaughtInvocationFailure::Panicked`].
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    pub fn invoke_catching<'call, M: ValueMode>(
        &self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, CaughtInvocationFailure<'call, M>> {
        match self.invoke_inner(invocation, true, false) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(CaughtInvocationFailure::Rejected(failure)),
            Err(InvocationAttempt::Panicked(panic)) => Err(CaughtInvocationFailure::Panicked(panic)),
        }
    }

    /// Invokes a method while bypassing its visibility and unsafe-call checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    ///
    /// # Parameters
    ///
    /// - `M`: Value mode that determines the invocation adapter and storage
    ///   bounds.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    /// # Safety
    /// The caller must satisfy the method's Rust safety contract and any
    /// invariants enforced by its source visibility.
    #[allow(unsafe_code)]
    pub unsafe fn invoke_unchecked<'call, M: ValueMode>(
        &self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>> {
        match self.invoke_inner(invocation, false, true) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(failure),
            Err(InvocationAttempt::Panicked(_)) => unreachable!("ordinary invoke does not catch"),
        }
    }

    /// Invokes a method with a pinned receiver.
    ///
    /// Preflight failures return the original pinned receiver and all supplied
    /// arguments in [`PinnedInvocationFailure`].
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    pub fn invoke_pinned<'call, T: ?Sized + 'static, M: ValueMode>(
        &self,
        invocation: PinnedInvocation<'call, M, T>,
    ) -> Result<InvocationOutput<'call, M>, PinnedInvocationFailure<'call, M, T>> {
        let specialization = match self.specializations.as_ref() {
            [specialization] => specialization,
            [] if !self.effective.metadata.is_generic => {
                let availability = match self.effective.availability {
                    MethodAvailability::Callable => InvocationAvailability::Callable,
                    MethodAvailability::Unavailable { reasons } => InvocationAvailability::Unavailable { reasons },
                    MethodAvailability::RequiresSpecialization => {
                        return Err(PinnedInvocationFailure::new(
                            InvocationError::new(InvocationErrorKind::SpecializationRequired),
                            invocation,
                        ));
                    }
                    MethodAvailability::RequiresSpecializationSelection { count } => {
                        return Err(PinnedInvocationFailure::new(
                            InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired { count }),
                            invocation,
                        ));
                    }
                };
                let adapters = self.effective.adapters.unwrap_or(NO_ADAPTERS);
                return self.invoke_pinned_with(availability, &adapters, invocation);
            }
            [] => {
                return Err(PinnedInvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationRequired),
                    invocation,
                ));
            }
            many => {
                return Err(PinnedInvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired { count: many.len() }),
                    invocation,
                ));
            }
        };
        let specialization = specialization.clone();
        self.invoke_pinned_with(specialization.availability, &specialization.adapters, invocation)
    }

    /// Invokes a pinned receiver through the adapter matching its exact type
    /// and pin mode. # Parameters
    ///
    /// - `availability`: Availability decision and its structured reasons.
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `availability`: Availability decision and its structured reasons.
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    fn invoke_pinned_with<'call, T: ?Sized + 'static, M: ValueMode + ModeAdapter>(
        &self,
        availability: InvocationAvailability,
        adapters: &InvocationAdapters,
        invocation: PinnedInvocation<'call, M, T>,
    ) -> Result<InvocationOutput<'call, M>, PinnedInvocationFailure<'call, M, T>> {
        let expected_kind = invocation.pinned_kind_and_type().0;
        if self.effective.metadata.visibility != MethodVisibility::Public {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::VisibilityDenied),
                invocation,
            ));
        }
        if self.qualifiers().is_unsafe {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::UnsafeMethod),
                invocation,
            ));
        }
        if let InvocationAvailability::Unavailable { reasons } = availability {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::MethodUnavailable { reasons }),
                invocation,
            ));
        }
        let validated = invocation.validate(self.clone())?;
        let Some(adapter) = M::select_pinned_adapter(adapters) else {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::ReceiverAdapterUnavailable),
                validated.into_invocation(),
            ));
        };
        if adapter.receiver_type_id() != TypeId::of::<T>() || adapter.receiver_kind() != expected_kind {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::ReceiverAdapterUnavailable),
                validated.into_invocation(),
            ));
        }
        let (receiver, invocation) = validated.into_parts();
        Ok(adapter.invoke(receiver, invocation))
    }

    /// Runs the appropriate generated adapter after preflight validation.
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// - `catching`: Whether the selected adapter catches method panics.
    /// - `unchecked`: Adapter slot that bypasses reflected visibility checks.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// - `catching`: Whether the selected adapter catches method panics.
    /// - `unchecked`: Adapter slot that bypasses reflected visibility checks.
    fn invoke_inner<'call, M: ValueMode + ModeAdapter>(
        &self,
        invocation: Invocation<'call, M>,
        catching: bool,
        unchecked: bool,
    ) -> Result<InvocationOutput<'call, M>, InvocationAttempt<'call, M>> {
        let specialization = match self.specializations.as_ref() {
            [specialization] => specialization,
            [] if !self.effective.metadata.is_generic => {
                let availability = self.effective.availability;
                let adapters = self.effective.adapters.unwrap_or(NO_ADAPTERS);
                return self.invoke_with(availability, &adapters, invocation, catching, unchecked);
            }
            [] => {
                return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationRequired),
                    invocation,
                )));
            }
            many => {
                return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired { count: many.len() }),
                    invocation,
                )));
            }
        };
        let specialization = specialization.clone();
        self.invoke_with_availability(
            specialization.availability,
            &specialization.adapters,
            invocation,
            catching,
            unchecked,
        )
    }
    /// Invokes the selected method using the validated receiver and arguments.
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `availability`: Availability decision and its structured reasons.
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// - `catching`: Whether the selected adapter catches method panics.
    /// - `unchecked`: Adapter slot that bypasses reflected visibility checks.
    fn invoke_with<'call, M: ValueMode + ModeAdapter>(
        &self,
        availability: MethodAvailability,
        adapters: &InvocationAdapters,
        invocation: Invocation<'call, M>,
        catching: bool,
        unchecked: bool,
    ) -> Result<InvocationOutput<'call, M>, InvocationAttempt<'call, M>> {
        let availability = match availability {
            MethodAvailability::Callable => InvocationAvailability::Callable,
            MethodAvailability::RequiresSpecialization => {
                return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationRequired),
                    invocation,
                )));
            }
            MethodAvailability::RequiresSpecializationSelection { count } => {
                return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired { count }),
                    invocation,
                )));
            }
            MethodAvailability::Unavailable { reasons } => InvocationAvailability::Unavailable { reasons },
        };
        self.invoke_with_availability(availability, adapters, invocation, catching, unchecked)
    }
    /// Checks invocation availability before dispatching to an adapter.
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `availability`: Availability decision and its structured reasons.
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// - `catching`: Whether the selected adapter catches method panics.
    /// - `unchecked`: Adapter slot that bypasses reflected visibility checks.
    fn invoke_with_availability<'call, M: ValueMode + ModeAdapter>(
        &self,
        availability: InvocationAvailability,
        adapters: &InvocationAdapters,
        invocation: Invocation<'call, M>,
        catching: bool,
        unchecked: bool,
    ) -> Result<InvocationOutput<'call, M>, InvocationAttempt<'call, M>> {
        if !unchecked && self.effective.metadata.visibility != MethodVisibility::Public {
            return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                InvocationError::new(InvocationErrorKind::VisibilityDenied),
                invocation,
            )));
        }
        if !unchecked && self.qualifiers().is_unsafe {
            return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                InvocationError::new(InvocationErrorKind::UnsafeMethod),
                invocation,
            )));
        }
        if let InvocationAvailability::Unavailable { reasons } = availability {
            return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                InvocationError::new(InvocationErrorKind::MethodUnavailable { reasons }),
                invocation,
            )));
        }
        if catching && (cfg!(panic = "abort") || self.qualifiers().is_async) {
            return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                InvocationError::new(InvocationErrorKind::ValueModeUnavailable),
                invocation,
            )));
        }
        let bound = match invocation.validate(self.clone()) {
            Ok(bound) => bound,
            Err(failure) => return Err(InvocationAttempt::Rejected(failure)),
        };
        let adapter = if catching {
            M::select_catching_adapter(adapters)
        } else {
            adapters.adapter::<M>()
        };
        let Some(adapter) = adapter else {
            return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                InvocationError::new(InvocationErrorKind::ValueModeUnavailable),
                bound.into_invocation(),
            )));
        };
        if catching {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| adapter.invoke(bound))) {
                Ok(output) => Ok(output),
                Err(payload) => Err(InvocationAttempt::Panicked(InvocationPanic::from_payload(payload))),
            }
        } else {
            Ok(adapter.invoke(bound))
        }
    }
}

impl InvocationAdapters {
    /// Starts building a typed adapter set with no mutable slots exposed.
    pub const fn builder() -> InvocationAdaptersBuilder {
        InvocationAdaptersBuilder {
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            pinned_local: None,
            pinned_thread_safe: None,
        }
    }

    /// Returns the generated invocation adapter for the selected value mode.
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    fn adapter<M: ValueMode + ModeAdapter>(&self) -> Option<&'static dyn InvocationAdapter<M>> {
        M::select_adapter(self)
    }
}

/// An explicit set of methods used by context registration to build lookups.
#[derive(Clone)]
pub(crate) struct MethodCatalog<'ctx> {
    /// Concrete target type whose method metadata is being registered.
    target: TypeDescriptor<'ctx>,
    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    methods: MethodCatalogMethods<'ctx>,
}

/// Storage strategy for methods in a catalog tied to a reflection context.
#[derive(Clone)]
#[allow(dead_code)] // Retained for crate-level tests that construct static catalogs directly.
enum MethodCatalogMethods<'ctx> {
    /// Borrows static effective method metadata for the catalog lifetime.
    Borrowed(
        /// Methods shared by all contexts for a given target type.
        &'ctx [EffectiveMethod],
    ),
    /// Shares context-bound method metadata through reference counting.
    Shared(
        /// Methods resolved for the catalog's reflection context.
        std::sync::Arc<[ContextEffectiveMethod]>,
    ),
}

/// One effective method viewed from either catalog storage strategy.
enum MethodCatalogMethod<'a> {
    /// References method metadata independent of a specific context.
    Static(
        /// Static registration used to build the effective descriptor.
        &'a EffectiveMethod,
    ),
    /// References method metadata resolved for a particular context.
    Context(
        /// Context-bound registration and its resolved specialization data.
        &'a ContextEffectiveMethod,
    ),
}

impl MethodCatalogMethod<'_> {
    /// Returns the static effective method record shared by the catalog view.
    fn effective(&self) -> &EffectiveMethod {
        match self {
            Self::Static(method) => method,
            Self::Context(method) => &method.effective,
        }
    }
}

/// Iterates over effective methods in either borrowed or shared storage.
enum MethodCatalogMethodIter<'a> {
    /// Iterates static method registrations.
    Borrowed(
        /// Source-ordered static methods for the target type.
        std::slice::Iter<'a, EffectiveMethod>,
    ),
    /// Iterates context-bound method registrations.
    Shared(
        /// Methods resolved for the current reflection context.
        std::slice::Iter<'a, ContextEffectiveMethod>,
    ),
}

impl<'a> Iterator for MethodCatalogMethodIter<'a> {
    /// Provides the item operation used to resolve or dispatch reflected
    /// methods.
    type Item = MethodCatalogMethod<'a>;

    /// Advances the catalog iterator and wraps the next effective method.
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Borrowed(methods) => methods.next().map(MethodCatalogMethod::Static),
            Self::Shared(methods) => methods.next().map(MethodCatalogMethod::Context),
        }
    }
}

/// Static source metadata for a target's effective method catalog.
pub struct MethodCatalogRegistration {
    /// Returns the target type identity used to match catalog registrations.
    type_id: fn() -> TypeId,
    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    methods: &'static [EffectiveMethod],
    /// Calls the registration provider to obtain method metadata for the
    /// target.
    provider: Option<fn() -> &'static [EffectiveMethod]>,
}

impl MethodCatalogRegistration {
    /// Creates a catalog registration for a reflected target type.
    ///
    /// # Parameters
    ///
    /// - `methods`: Method records registered for the target type.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `methods`: Method records registered for the target type.
    pub const fn new<T: super::Reflect>(methods: &'static [EffectiveMethod]) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            methods,
            provider: None,
        }
    }

    /// Creates a registration whose method slice is initialized by a typed
    /// provider.
    ///
    /// # Parameters
    ///
    /// - `provider`: Function that supplies the target type’s static method
    ///   records.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `provider`: Function that supplies the target type’s static method
    ///   records.
    pub const fn with_provider<T: super::Reflect>(provider: fn() -> &'static [EffectiveMethod]) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            methods: &[],
            provider: Some(provider),
        }
    }

    /// Returns the target type identity used to match catalog registrations.
    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    pub(crate) fn methods(&self) -> &'static [EffectiveMethod] {
        match self.provider {
            Some(provider) => provider(),
            None => self.methods,
        }
    }
}

inventory::collect!(MethodCatalogRegistration);

impl<'ctx> MethodCatalog<'ctx> {
    /// Creates a catalog borrowing the static method records for `target`.
    ///
    /// # Parameters
    ///
    /// - `target`: Type descriptor whose effective methods this catalog
    ///   exposes.
    /// - `methods`: Static method records registered for the target type.
    ///
    /// # Returns
    ///
    /// Returns a catalog that borrows `methods` for the context lifetime.
    #[allow(dead_code)] // Used by crate-level tests for hand-written method metadata.
    pub(crate) fn new(target: TypeDescriptor<'ctx>, methods: &'ctx [EffectiveMethod]) -> Self {
        Self {
            target,
            methods: MethodCatalogMethods::Borrowed(methods),
        }
    }

    /// Creates a catalog sharing context-resolved method records through an
    /// `Arc`.
    ///
    /// # Parameters
    ///
    /// - `target`: Type descriptor whose effective methods this catalog
    ///   exposes.
    /// - `methods`: Context-bound method records resolved for this reflection
    ///   context.
    ///
    /// # Returns
    ///
    /// Returns a catalog sharing ownership of `methods`.
    pub(crate) fn from_shared(target: TypeDescriptor<'ctx>, methods: std::sync::Arc<[ContextEffectiveMethod]>) -> Self {
        Self {
            target,
            methods: MethodCatalogMethods::Shared(methods),
        }
    }

    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    fn methods(&self) -> MethodCatalogMethodIter<'_> {
        match &self.methods {
            MethodCatalogMethods::Borrowed(methods) => MethodCatalogMethodIter::Borrowed(methods.iter()),
            MethodCatalogMethods::Shared(methods) => MethodCatalogMethodIter::Shared(methods.iter()),
        }
    }
    /// Looks up methods by source name in the catalog's effective method set.
    ///
    /// # Parameters
    ///
    /// - `name`: Method name to match against the target type's methods.
    ///
    /// # Returns
    ///
    /// Returns the lookup result, preserving missing and ambiguous cases.
    pub(crate) fn lookup_method(&self, name: &str) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(self.lookup(|method| method.metadata.name == name))
    }
    /// Looks up a method by name and requires exactly one effective match.
    ///
    /// # Parameters
    ///
    /// - `name`: Method name that must identify a unique catalog entry.
    ///
    /// # Returns
    ///
    /// Returns the unique descriptor or an error for missing or ambiguous
    /// names.
    #[allow(dead_code)] // Used by crate-level tests for hand-written method metadata.
    pub(crate) fn method(&self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.lookup_method(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Method, LookupQuery::Name(name.into()))
    }
    #[cfg(test)]
    pub(crate) fn find_inherent_method(&self, name: &str) -> Result<Option<MethodDescriptor<'ctx>>, ContextError> {
        match self.lookup(|method| {
            method.metadata.name == name
                && matches!(method.metadata.namespace, MethodNamespaceMetadata::Inherent { .. })
        }) {
            Lookup::Missing => Ok(None),
            Lookup::Unique(method) => Ok(Some(method)),
            Lookup::Ambiguous(_) => {
                unreachable!("inherent method names are unique within one namespace")
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn inherent_method(&self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.find_inherent_method(name)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name.into()),
            })
    }
    #[cfg(test)]
    pub(crate) fn lookup_trait_method(
        &self,
        trait_: &str,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(self.lookup(|method| method.metadata.name == name && matches!(method.metadata.namespace, MethodNamespaceMetadata::Trait { trait_: candidate, .. } if candidate == trait_)))
    }
    #[cfg(test)]
    pub(crate) fn find_trait_method(
        &self,
        trait_: &str,
        name: &str,
    ) -> Result<Option<MethodDescriptor<'ctx>>, ContextError> {
        match self.lookup_trait_method(trait_, name)? {
            Lookup::Missing => Ok(None),
            Lookup::Unique(method) => Ok(Some(method)),
            Lookup::Ambiguous(_) => {
                unreachable!("method names are unique within one trait namespace")
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn trait_method(&self, trait_: &str, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.find_trait_method(trait_, name)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name.into()),
            })
    }
    /// Provides the lookup operation used to resolve or dispatch reflected
    /// methods. # Parameters
    ///
    /// - `predicate`: Condition used to select matching effective method
    ///   entries.
    /// # Parameters
    ///
    /// - `predicate`: Condition used to filter the catalog’s effective methods.
    fn lookup(&self, predicate: impl Fn(&EffectiveMethod) -> bool) -> Lookup<MethodDescriptor<'ctx>> {
        let mut matches = self
            .methods()
            .filter(|method| {
                (method.effective().target)().type_id() == self.target.type_id() && predicate(method.effective())
            })
            .map(|method| match method {
                MethodCatalogMethod::Static(method) => MethodDescriptor::new(*method, self.target.context()),
                MethodCatalogMethod::Context(method) => MethodDescriptor::with_resolved_specializations(
                    method.effective,
                    method.specializations.clone(),
                    method.implementation,
                    self.target.context(),
                ),
            });
        let Some(first) = matches.next() else {
            return Lookup::Missing;
        };
        match matches.next() {
            None => Lookup::Unique(first),
            Some(second) => Lookup::Ambiguous(
                std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(matches)
                    .collect(),
            ),
        }
    }
}

impl crate::runtime::identity::IntoLookupCandidate for MethodDescriptor<'_> {
    /// Consumes a lookup result into the candidate used by specialization.
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.name().into();
        let namespace_id = match self.effective.metadata.namespace {
            MethodNamespaceMetadata::Inherent { implementation } => {
                format!("inherent:{implementation}")
            }
            MethodNamespaceMetadata::Trait { trait_, implementation } => format!("trait:{trait_}:{implementation}"),
            MethodNamespaceMetadata::TraitApplication {
                trait_,
                implementation,
                identity,
                ..
            } => {
                let arguments = identity
                    .iter()
                    .map(|identity| match identity {
                        crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity::Type(type_id) => {
                            format!("{:?}", type_id())
                        }
                        crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity::Const(value) => {
                            format!("{value:?}")
                        }
                        crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity::Lifetime(value)
                        | crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity::Expression(value) => {
                            (*value).to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("trait:{trait_}<{arguments}>:{implementation}")
            }
        };
        let stable_id: Box<str> = format!(
            "{}::{namespace_id}::{}",
            self.target().type_identity().rust_name,
            self.name()
        )
        .into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Method,
                stable_id: StableItemId(stable_id),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

/// Concrete type and const arguments used to select a method specialization.
/// Concrete type and const arguments used to select a method specialization.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MethodArguments {
    /// Generic arguments supplied when selecting a method specialization.
    arguments: Vec<GenericArgumentIdentity>,
    /// Returns the selector list used to compare generic arguments.
    selectors: Vec<GenericArgumentSelector>,
}

impl MethodArguments {
    /// Creates an empty generic argument list.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends a concrete type argument.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    pub fn type_of<T: crate::runtime::descriptor::Reflect + ?Sized>(mut self) -> Self {
        self.arguments
            .push(GenericArgumentIdentity::Type(std::any::type_name::<T>().into()));
        self.selectors.push(GenericArgumentSelector::Type(TypeId::of::<T>()));
        self
    }
    /// Appends a concrete const argument.
    ///
    /// Primitive integer, boolean, and character values participate in exact
    /// specialization selection. Other `Any` values are retained for error
    /// diagnostics but never treated as a matching const argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn constant<T: Any>(mut self, value: T) -> Self {
        let (display, selector) = const_argument_identity(&value);
        self.arguments.push(GenericArgumentIdentity::Const(display));
        self.selectors.push(selector);
        self
    }

    /// Returns the number of supplied generic arguments.
    pub fn len(&self) -> usize {
        self.arguments.len()
    }

    /// Returns whether no generic arguments were supplied.
    pub fn is_empty(&self) -> bool {
        self.arguments.is_empty()
    }
}

/// Builds the stable identity used to compare const generic arguments.
/// # Parameters
///
/// - `value`: Owned value retained for validation and construction.
///
/// # Type Parameters
///
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
///
/// # Parameters
///
/// - `value`: Owned value retained for validation and construction.
fn const_argument_identity<T: Any>(value: &T) -> (Box<str>, GenericArgumentSelector) {
    macro_rules! integer {
        ($($ty:ty),+ $(,)?) => {
            $(if let Some(value) = (value as &dyn Any).downcast_ref::<$ty>() {
                let text = value.to_string();
                return (
                    format!("{}:{text}", std::any::type_name::<$ty>()).into(),
                    GenericArgumentSelector::Const {
                        type_id: TypeId::of::<$ty>(),
                        value: text.into_boxed_str(),
                    },
                );
            })+
        };
    }
    integer!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
    if let Some(value) = (value as &dyn Any).downcast_ref::<bool>() {
        let text = value.to_string();
        return (
            format!("bool:{text}").into(),
            GenericArgumentSelector::Const {
                type_id: TypeId::of::<bool>(),
                value: text.into_boxed_str(),
            },
        );
    }
    if let Some(value) = (value as &dyn Any).downcast_ref::<char>() {
        return (
            format!("char:{value:?}").into(),
            GenericArgumentSelector::Const {
                type_id: TypeId::of::<char>(),
                value: u32::from(*value).to_string().into_boxed_str(),
            },
        );
    }
    (
        format!("{}:<unsupported>", std::any::type_name::<T>()).into(),
        GenericArgumentSelector::UnsupportedConst(TypeId::of::<T>()),
    )
}

/// A view over the concrete implementations registered for one method.
pub struct MethodSpecializations<'ctx> {
    /// Returns the unique method descriptor for the requested name.
    method: MethodDescriptor<'ctx>,
}

impl<'ctx> MethodSpecializations<'ctx> {
    /// Iterates over the registered specializations in registration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = MethodSpecializationDescriptor<'ctx>> + 'ctx {
        let method = self.method;
        let specializations = method.specializations.clone();
        (0..specializations.len()).map(move |index| MethodSpecializationDescriptor {
            method: method.clone(),
            specializations: specializations.clone(),
            index,
        })
    }
    /// Returns the number of registered specializations.
    pub fn len(&self) -> usize {
        self.method.specializations.len()
    }

    /// Returns whether this method has no concrete specializations.
    pub fn is_empty(&self) -> bool {
        self.method.specializations.is_empty()
    }
}

/// One compile-time-bound method implementation.
#[derive(Clone)]
pub struct MethodSpecializationDescriptor<'ctx> {
    /// Returns the unique method descriptor for the requested name.
    method: MethodDescriptor<'ctx>,
    /// Registered concrete generic instantiations available for this method.
    specializations: std::sync::Arc<[ResolvedMethodSpecialization]>,
    /// Returns the parameter’s zero-based position, excluding the receiver.
    index: usize,
}

impl<'ctx> MethodSpecializationDescriptor<'ctx> {
    /// Provides the resolved operation used to resolve or dispatch reflected
    /// methods.
    fn resolved(&self) -> &ResolvedMethodSpecialization {
        &self.specializations[self.index]
    }
    /// Returns the source method declaration for this specialization.
    pub fn method(&self) -> MethodDescriptor<'ctx> {
        self.method.clone()
    }
    /// Returns the concrete type and const arguments for this specialization.
    pub fn arguments(&self) -> crate::runtime::descriptor::generic::GenericArguments<'ctx> {
        let arguments = self
            .resolved()
            .argument_metadata
            .clone()
            .unwrap_or_else(|| std::sync::Arc::from([]));
        crate::runtime::descriptor::generic::GenericArguments::from_shared(arguments, self.method.context)
    }
    /// Returns whether the generated adapter can be called in each value mode.
    pub fn availability(&self) -> InvocationAvailability {
        self.resolved().availability
    }
    /// Invokes this specialization after validating the receiver and arguments.
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    pub fn invoke<'call, M: ValueMode>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>> {
        let availability = self.resolved().availability;
        let adapters = self.resolved().adapters;
        match self
            .method
            .invoke_with_availability(availability, &adapters, invocation, false, false)
        {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(failure),
            Err(InvocationAttempt::Panicked(_)) => unreachable!("ordinary invoke does not catch"),
        }
    }

    /// Invokes this specialization with a pinned receiver.
    ///
    /// Returns [`PinnedInvocationFailure`] with the original pinned receiver
    /// and arguments when the adapter is unavailable or receiver and argument
    /// validation rejects the call. Panics raised by the method propagate.
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Generic type or lifetime parameter carried by the descriptor.
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    pub fn invoke_pinned<'call, T: ?Sized + 'static, M: ValueMode>(
        &self,
        invocation: PinnedInvocation<'call, M, T>,
    ) -> Result<InvocationOutput<'call, M>, PinnedInvocationFailure<'call, M, T>> {
        let selected = self.resolved();
        self.method
            .invoke_pinned_with(selected.availability, &selected.adapters, invocation)
    }
}

/// Resolves one parameter candidate from the source-level type expression.
/// # Parameters
///
/// - `parameter`: Declared parameter considered during argument matching.
fn parameter_candidate(parameter: ParameterDescriptor<'_>) -> LookupCandidate {
    let display_name: Box<str> = parameter.name().unwrap_or("<pattern>").into();
    LookupCandidate {
        identity: ItemIdentity {
            kind: ItemKind::Parameter,
            stable_id: StableItemId(parameter.index().to_string().into()),
            display_name: display_name.clone(),
        },
        display_name,
        origin: None,
    }
}

/// Finds the registered specialization compatible with the supplied generic
/// arguments. # Parameters
///
/// - `index`: Zero-based field position to inspect.
/// # Parameters
///
/// - `index`: Zero-based field position to inspect.
fn specialization_candidate(index: usize) -> LookupCandidate {
    let display_name: Box<str> = format!("specialization {index}").into();
    LookupCandidate {
        identity: ItemIdentity {
            kind: ItemKind::MethodSpecialization,
            stable_id: StableItemId(index.to_string().into()),
            display_name: display_name.clone(),
        },
        display_name,
        origin: None,
    }
}

#[cfg(test)]
mod descriptor_behavior_tests {
    use super::EffectiveMethod;
    use super::ExpressionData;
    use super::FunctionAbi;
    use super::InvocationAdapter;
    use super::InvocationAdapters;
    use super::InvocationOutput;
    use super::Local;
    use super::MethodArguments;
    use super::MethodAvailability;
    use super::MethodDescriptor;
    use super::MethodImplementationSource;
    use super::MethodMetadata;
    use super::MethodMetadataSpec;
    use super::MethodNamespaceMetadata;
    use super::MethodQualifiers;
    use super::MethodSpecializationArgument;
    use super::MethodSpecializationSpec;
    use super::MethodVisibility;
    use super::NO_ADAPTERS;
    use super::ParameterMetadata;
    use super::ParameterPattern;
    use super::PassingMode;
    use super::ReceiverDescriptor;
    use super::ReceiverKind;
    use super::ReturnKind;
    use super::ReturnMetadata;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::descriptor::Reflect;
    use crate::runtime::descriptor::TypeRegistration;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::generic::GenericParameterMetadata;
    use crate::runtime::error::LookupError;
    use crate::runtime::invoke::Invocation;
    use crate::runtime::invoke::InvocationErrorKind;
    use crate::runtime::invoke::PinnedFunctionInvocationAdapter;
    use crate::runtime::invoke::PinnedInvocation;
    use crate::runtime::invoke::PinnedReceiver;
    use crate::runtime::invoke::ThreadSafe;
    use crate::runtime::invoke::ValidatedInvocation;

    struct Target;

    impl Reflect for Target {
        fn __reflect_registration() -> &'static TypeRegistration<Self> {
            static REGISTRATION: TypeRegistration<Target> = TypeRegistration::new();
            &REGISTRATION
        }
    }

    struct ResultType;

    impl Reflect for ResultType {
        fn __reflect_registration() -> &'static TypeRegistration<Self> {
            static REGISTRATION: TypeRegistration<ResultType> = TypeRegistration::new();
            &REGISTRATION
        }
    }

    static TYPE_EXPRESSION: ExpressionData = ExpressionData::Named {
        path: "ResultType",
        definition: None,
        arguments: &[],
    };
    static U32_EXPRESSION: ExpressionData = ExpressionData::Named {
        path: "u32",
        definition: None,
        arguments: &[],
    };
    static PARAMETER: ParameterMetadata = ParameterMetadata::new(
        0,
        ParameterPattern::Identifier("value"),
        PassingMode::Shared,
        &TYPE_EXPRESSION,
    )
    .with_concrete_type::<ResultType>();
    static PARAMETERS: [ParameterMetadata; 2] = [
        PARAMETER,
        ParameterMetadata::new(1, ParameterPattern::Wildcard, PassingMode::Owned, &TYPE_EXPRESSION),
    ];
    static DUPLICATE_PARAMETERS: [ParameterMetadata; 2] = [
        ParameterMetadata::new(
            0,
            ParameterPattern::Identifier("same"),
            PassingMode::Owned,
            &TYPE_EXPRESSION,
        ),
        ParameterMetadata::new(
            1,
            ParameterPattern::Identifier("same"),
            PassingMode::Owned,
            &TYPE_EXPRESSION,
        ),
    ];
    static OUTPUT: ReturnMetadata =
        ReturnMetadata::new(ReturnKind::Owned, &TYPE_EXPRESSION).with_concrete_type::<ResultType>();
    static GENERIC_PARAMETERS: [GenericParameterMetadata; 1] =
        [GenericParameterMetadata::type_parameter("T", 0, &[], None)];
    static CONST_GENERIC_PARAMETERS: [GenericParameterMetadata; 1] = [GenericParameterMetadata::const_parameter(
        "N",
        0,
        &TYPE_EXPRESSION,
        None,
    )];
    static CONST_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &CONST_GENERIC_PARAMETERS,
        predicates: &[],
    };
    static CONST_FOR_TYPE_ARGUMENT: [MethodSpecializationArgument; 1] = [MethodSpecializationArgument::u8(1)];
    static TYPE_FOR_CONST_ARGUMENT: [MethodSpecializationArgument; 1] =
        [MethodSpecializationArgument::type_argument::<u32>()];
    static NO_SPECIALIZATION_ARGUMENTS: [MethodSpecializationArgument; 0] = [];
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &GENERIC_PARAMETERS,
        predicates: &[],
    };
    static METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
        "inspect",
        MethodNamespaceMetadata::Inherent {
            implementation: "Target::inspect",
        },
        MethodVisibility::Public,
        Some(ReceiverDescriptor::for_type::<Target>(ReceiverKind::Shared).with_expression(&TYPE_EXPRESSION)),
        &PARAMETERS,
        &OUTPUT,
        MethodQualifiers::new(true, true, false, false).with_abi(Some(FunctionAbi::C)),
        &GENERICS,
        true,
        false,
    ));

    fn descriptor(metadata: &'static MethodMetadata) -> MethodDescriptor<'static> {
        let context = Box::leak(Box::new(ReflectionContext::empty()));
        MethodDescriptor::new(EffectiveMethod::for_type::<Target>(metadata), context)
    }

    #[test]
    fn test_descriptor_exposes_receiver_parameters_qualifiers_and_return_metadata() {
        let method = descriptor(&METHOD);
        assert_eq!(method.name(), "inspect");
        assert_eq!(method.target().type_id(), std::any::TypeId::of::<Target>());
        assert_eq!(method.implementation_source(), MethodImplementationSource::Declared);
        assert_eq!(
            method.visibility(),
            crate::runtime::descriptor::trait_descriptor::Visibility::Public
        );
        assert_eq!(method.availability(), MethodAvailability::RequiresSpecialization);

        let receiver = method.receiver().expect("declared receiver");
        assert_eq!(receiver.kind(), ReceiverKind::Shared);
        assert_eq!(
            receiver.type_metadata.expect("bound receiver type")().type_id(),
            std::any::TypeId::of::<Target>()
        );
        assert!(receiver.expression.is_some());

        let parameters = method.parameters();
        assert_eq!(parameters.len(), 2);
        assert!(!parameters.is_empty());
        assert_eq!(parameters.context().id(), method.context().id());
        let mut iter = parameters.iter();
        let first = iter.next().expect("first parameter");
        assert_eq!(first.index(), 0);
        assert_eq!(first.name(), Some("value"));
        assert_eq!(first.pattern(), ParameterPattern::Identifier("value"));
        assert_eq!(first.passing(), PassingMode::Shared);
        assert_eq!(first.context().id(), method.context().id());
        assert!(first.find_concrete_type().is_some());
        assert_eq!(first.type_expression().display().to_string(), "ResultType");
        assert_eq!(parameters.get_at(1).expect("second parameter").name(), None);
        assert!(parameters.get_at(2).is_none());
        assert!(parameters.get("value").is_some());
        assert!(parameters.get("missing").is_none());
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());

        let output = method.output();
        assert_eq!(output.context().id(), method.context().id());
        assert_eq!(output.kind(), ReturnKind::Owned);
        assert_eq!(
            output
                .type_expression()
                .expect("return expression")
                .display()
                .to_string(),
            "ResultType"
        );
        assert!(output.find_concrete_type().is_some());

        let qualifiers = method.qualifiers();
        assert!(qualifiers.is_const);
        assert!(qualifiers.is_async);
        assert!(!qualifiers.is_unsafe);
        assert_eq!(qualifiers.abi, Some(FunctionAbi::C));
        assert!(method.generics().parameter("T").is_ok());
    }

    #[test]
    fn test_parameter_name_lookup_reports_missing_and_duplicate_identifiers() {
        let method = descriptor(&METHOD);
        assert!(matches!(
            method.parameter("absent"),
            Err(LookupError::Missing { subject, .. }) if subject == crate::runtime::error::LookupSubject::Parameter
        ));

        static DUPLICATE_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
            "duplicates",
            MethodNamespaceMetadata::Inherent {
                implementation: "Target::duplicates",
            },
            MethodVisibility::Public,
            None,
            &DUPLICATE_PARAMETERS,
            &ReturnMetadata::unit(),
            MethodQualifiers::new(false, false, false, false),
            &GenericDefinitionMetadata {
                parameters: &[],
                predicates: &[],
            },
            false,
            false,
        ));
        assert!(matches!(
            descriptor(&DUPLICATE_METHOD).parameter("same"),
            Err(LookupError::Ambiguous { subject, candidates, .. })
                if subject == crate::runtime::error::LookupSubject::Parameter && candidates.len() == 2
        ));
        assert!(descriptor(&DUPLICATE_METHOD).parameters().get("same").is_none());
    }

    struct UnitAdapter;

    impl InvocationAdapter<Local> for UnitAdapter {
        fn invoke<'call>(&self, _invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
            InvocationOutput::owned(37_u32)
        }
    }

    static UNIT_ADAPTER: UnitAdapter = UnitAdapter;
    static ADAPTERS: InvocationAdapters = InvocationAdapters::builder().local(&UNIT_ADAPTER).build();
    static CALLABLE_RECEIVER: ReceiverDescriptor = ReceiverDescriptor::for_type::<Target>(ReceiverKind::Shared);
    static ADAPTER_OUTPUT: ReturnMetadata =
        ReturnMetadata::new(ReturnKind::Owned, &U32_EXPRESSION).with_concrete_type::<u32>();

    struct ThreadSafeAdapter;

    impl InvocationAdapter<ThreadSafe> for ThreadSafeAdapter {
        fn invoke<'call>(
            &self,
            _invocation: ValidatedInvocation<'call, ThreadSafe>,
        ) -> InvocationOutput<'call, ThreadSafe> {
            InvocationOutput::owned(37_u32)
        }
    }

    static THREAD_SAFE_ADAPTER: ThreadSafeAdapter = ThreadSafeAdapter;
    static THREAD_SAFE_ONLY_ADAPTERS: InvocationAdapters =
        InvocationAdapters::builder().thread_safe(&THREAD_SAFE_ADAPTER).build();

    fn pinned_unit_adapter<'call>(
        _receiver: PinnedReceiver<'call>,
        _invocation: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
        InvocationOutput::owned(41_u32)
    }

    static PINNED_LOCAL_ADAPTER: PinnedFunctionInvocationAdapter<Local, Target> =
        PinnedFunctionInvocationAdapter::shared(pinned_unit_adapter);
    static PINNED_ADAPTERS: InvocationAdapters = InvocationAdapters::builder()
        .pinned_local(&PINNED_LOCAL_ADAPTER)
        .build();

    #[test]
    fn test_invocation_adapter_runs_and_missing_adapter_is_a_typed_preflight_error() {
        static CALLABLE_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
            "value",
            MethodNamespaceMetadata::Inherent {
                implementation: "Target::value",
            },
            MethodVisibility::Public,
            Some(CALLABLE_RECEIVER),
            &[],
            &ADAPTER_OUTPUT,
            MethodQualifiers::new(false, false, false, false),
            &GenericDefinitionMetadata {
                parameters: &[],
                predicates: &[],
            },
            false,
            false,
        ));
        let callable = EffectiveMethod::for_type::<Target>(&CALLABLE_METHOD).with_adapters(ADAPTERS);
        let method = MethodDescriptor::new(callable, Box::leak(Box::new(ReflectionContext::empty())));
        let target = Target;
        let output = match method.invoke(Invocation::<Local>::on_ref(&target)) {
            Ok(output) => output,
            Err(_) => panic!("the registered adapter is callable"),
        };
        let value = match output.into_owned::<u32>() {
            Ok(value) => value,
            Err(_) => panic!("adapter returns a typed result"),
        };
        assert_eq!(value, 37);

        let unavailable = MethodDescriptor::new(
            EffectiveMethod::for_type::<Target>(&CALLABLE_METHOD).with_adapters(THREAD_SAFE_ONLY_ADAPTERS),
            Box::leak(Box::new(ReflectionContext::empty())),
        );
        let failure = match unavailable.invoke(Invocation::<Local>::on_ref(&target)) {
            Err(failure) => failure,
            Ok(_) => panic!("no adapter is registered"),
        };
        assert_eq!(*failure.error().kind(), InvocationErrorKind::ValueModeUnavailable);
    }

    #[test]
    fn test_specialization_resolution_rejects_argument_kind_and_arity_mismatches() {
        let empty_adapters = InvocationAdapters::builder().build();
        let error = match MethodSpecializationSpec::new(&CONST_FOR_TYPE_ARGUMENT, empty_adapters)
            .resolve(&GENERIC_PARAMETERS)
        {
            Ok(_) => panic!("const argument cannot bind a type parameter"),
            Err(error) => error,
        };
        assert_eq!(error, "const specialization argument does not target a const parameter");

        let error = match MethodSpecializationSpec::new(&TYPE_FOR_CONST_ARGUMENT, empty_adapters)
            .resolve(&CONST_GENERIC_PARAMETERS)
        {
            Ok(_) => panic!("type argument cannot bind a const parameter"),
            Err(error) => error,
        };
        assert_eq!(error, "type specialization argument does not target a type parameter");

        let error = match MethodSpecializationSpec::new(&NO_SPECIALIZATION_ARGUMENTS, empty_adapters)
            .resolve(&CONST_GENERIC_PARAMETERS)
        {
            Ok(_) => panic!("specialization must bind every parameter"),
            Err(error) => error,
        };
        assert_eq!(
            error,
            "method specialization argument count does not match its generic parameters"
        );
    }

    #[test]
    fn test_specialization_arguments_cover_lifetimes_and_primitive_const_kinds() {
        static PARAMETERS: [GenericParameterMetadata; 15] = [
            GenericParameterMetadata::lifetime("a", 0, &[]),
            GenericParameterMetadata::const_parameter("flag", 1, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("character", 2, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i8_value", 3, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i16_value", 4, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i32_value", 5, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i64_value", 6, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("i128_value", 7, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("isize_value", 8, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u8_value", 9, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u16_value", 10, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u32_value", 11, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u64_value", 12, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("u128_value", 13, &TYPE_EXPRESSION, None),
            GenericParameterMetadata::const_parameter("usize_value", 14, &TYPE_EXPRESSION, None),
        ];
        static ARGUMENTS: [MethodSpecializationArgument; 15] = [
            MethodSpecializationArgument::lifetime("'static"),
            MethodSpecializationArgument::const_bool(true),
            MethodSpecializationArgument::const_char('R'),
            MethodSpecializationArgument::i8(-8),
            MethodSpecializationArgument::i16(-16),
            MethodSpecializationArgument::i32(-32),
            MethodSpecializationArgument::i64(-64),
            MethodSpecializationArgument::i128(-128),
            MethodSpecializationArgument::isize(-1),
            MethodSpecializationArgument::u8(8),
            MethodSpecializationArgument::u16(16),
            MethodSpecializationArgument::u32(32),
            MethodSpecializationArgument::u64(64),
            MethodSpecializationArgument::u128(128),
            MethodSpecializationArgument::usize(1),
        ];

        let resolved = MethodSpecializationSpec::new(&ARGUMENTS, NO_ADAPTERS)
            .resolve(&PARAMETERS)
            .expect("all declared arguments have compatible kinds");
        assert_eq!(
            resolved.arguments.as_slice(),
            &[
                crate::runtime::identity::GenericArgumentIdentity::Lifetime("'static".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("bool:true".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("char:'R'".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("i8:-8".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("i16:-16".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("i32:-32".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("i64:-64".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("i128:-128".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("isize:-1".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("u8:8".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("u16:16".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("u32:32".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("u64:64".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("u128:128".into()),
                crate::runtime::identity::GenericArgumentIdentity::Const("usize:1".into()),
            ]
        );
    }

    #[test]
    fn test_specialization_selection_reports_missing_and_ambiguous_bindings() {
        static ARGUMENT_ONE: [MethodSpecializationArgument; 1] = [MethodSpecializationArgument::u8(1)];
        static ARGUMENT_TWO: [MethodSpecializationArgument; 1] = [MethodSpecializationArgument::u8(2)];
        static SPECS: [MethodSpecializationSpec; 2] = [
            MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS),
            MethodSpecializationSpec::new(&ARGUMENT_TWO, ADAPTERS)
                .with_unavailable(&[super::InvocationUnavailableReason::AdapterUnavailable]),
        ];
        static GENERIC_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
            "generic",
            MethodNamespaceMetadata::Inherent {
                implementation: "Target::generic",
            },
            MethodVisibility::Public,
            Some(CALLABLE_RECEIVER),
            &[],
            &ADAPTER_OUTPUT,
            MethodQualifiers::new(false, false, false, false),
            &CONST_GENERICS,
            true,
            false,
        ));

        let effective = EffectiveMethod::for_type::<Target>(&GENERIC_METHOD).with_specializations(&SPECS);
        let resolved = effective.resolve_specializations().expect("valid specs resolve");
        let method = MethodDescriptor::with_resolved_specializations(
            effective,
            std::sync::Arc::from(resolved),
            None,
            Box::leak(Box::new(ReflectionContext::empty())),
        );
        assert_eq!(
            method.availability(),
            MethodAvailability::RequiresSpecializationSelection { count: 2 }
        );

        let specializations = method.specializations();
        assert_eq!(specializations.len(), 2);
        assert!(!specializations.is_empty());
        let mut iter = specializations.iter();
        let callable = iter.next().expect("first specialization");
        assert_eq!(callable.method().name(), "generic");
        assert!(matches!(
            callable.availability(),
            super::InvocationAvailability::Callable
        ));
        assert_eq!(callable.arguments().iter().len(), 1);

        let arguments = MethodArguments::new().constant(1_u8);
        assert_eq!(arguments.len(), 1);
        assert!(!arguments.is_empty());
        assert_eq!(
            MethodArguments::new()
                .type_of::<ResultType>()
                .constant(true)
                .constant('R')
                .len(),
            3
        );
        let selected = method
            .specialization(&arguments)
            .expect("the matching specialization is unique");
        let target = Target;
        let output = match selected.invoke(Invocation::<Local>::on_ref(&target)) {
            Ok(output) => output,
            Err(_) => panic!("the selected adapter is callable"),
        };
        let value = match output.into_owned::<u32>() {
            Ok(value) => value,
            Err(_) => panic!("the selected adapter returns the declared type"),
        };
        assert_eq!(value, 37);

        let unavailable = iter.next().expect("second specialization");
        assert!(matches!(
            unavailable.availability(),
            super::InvocationAvailability::Unavailable { .. }
        ));
        let failure = match unavailable.invoke(Invocation::<Local>::on_ref(&target)) {
            Err(failure) => failure,
            Ok(_) => panic!("unavailable specialization must reject invocation"),
        };
        assert_eq!(
            *failure.error().kind(),
            InvocationErrorKind::MethodUnavailable {
                reasons: &[super::InvocationUnavailableReason::AdapterUnavailable],
            }
        );
        assert!(iter.next().is_none());

        assert!(matches!(
            method.specialization(&MethodArguments::new().constant(3_u8)),
            Err(LookupError::Missing {
                subject: crate::runtime::error::LookupSubject::MethodSpecialization,
                ..
            })
        ));

        static DUPLICATE_SPECS: [MethodSpecializationSpec; 2] = [
            MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS),
            MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS),
        ];
        let duplicate_effective =
            EffectiveMethod::for_type::<Target>(&GENERIC_METHOD).with_specializations(&DUPLICATE_SPECS);
        let duplicate_method = MethodDescriptor::with_resolved_specializations(
            duplicate_effective,
            std::sync::Arc::from(
                duplicate_effective
                    .resolve_specializations()
                    .expect("duplicate specs still resolve"),
            ),
            None,
            method.context(),
        );
        assert!(matches!(
            duplicate_method.specialization(&MethodArguments::new().constant(1_u8)),
            Err(LookupError::Ambiguous {
                subject: crate::runtime::error::LookupSubject::MethodSpecialization,
                candidates,
                ..
            }) if candidates.len() == 2
        ));

        static SINGLE_CALLABLE_SPECS: [MethodSpecializationSpec; 1] =
            [MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS)];
        static SINGLE_UNAVAILABLE_SPECS: [MethodSpecializationSpec; 1] =
            [MethodSpecializationSpec::new(&ARGUMENT_ONE, ADAPTERS)
                .with_unavailable(&[super::InvocationUnavailableReason::AdapterUnavailable])];
        for (specs, expected) in [
            (&SINGLE_CALLABLE_SPECS[..], MethodAvailability::Callable),
            (
                &SINGLE_UNAVAILABLE_SPECS[..],
                MethodAvailability::Unavailable {
                    reasons: &[super::InvocationUnavailableReason::AdapterUnavailable],
                },
            ),
        ] {
            let effective = EffectiveMethod::for_type::<Target>(&GENERIC_METHOD).with_specializations(specs);
            let single = MethodDescriptor::with_resolved_specializations(
                effective,
                std::sync::Arc::from(
                    effective
                        .resolve_specializations()
                        .expect("single specialization resolves"),
                ),
                None,
                method.context(),
            );
            assert_eq!(single.availability(), expected);
        }
    }

    #[test]
    fn test_registration_equality_compares_source_availability_and_adapter_identity() {
        static METADATA: MethodMetadata = MethodMetadata::inherent("unit", "Target::unit");
        let base = EffectiveMethod::for_type::<Target>(&METADATA);
        assert!(base.same_registration(&EffectiveMethod::for_type::<Target>(&METADATA)));
        assert!(!base.same_registration(&base.with_source(MethodImplementationSource::Overridden)));
        assert!(!base.same_registration(&base.with_adapters(ADAPTERS)));
        assert!(
            EffectiveMethod::for_type::<Target>(&METADATA)
                .with_adapters(ADAPTERS)
                .same_registration(&EffectiveMethod::for_type::<Target>(&METADATA).with_adapters(ADAPTERS))
        );
        assert!(
            !EffectiveMethod::for_type::<Target>(&METADATA)
                .with_adapters(ADAPTERS)
                .same_registration(
                    &EffectiveMethod::for_type::<Target>(&METADATA).with_adapters(THREAD_SAFE_ONLY_ADAPTERS)
                )
        );
    }

    #[test]
    fn test_pinned_method_invocation_checks_and_dispatches_the_bound_receiver() {
        static PINNED_RECEIVER: ReceiverDescriptor = ReceiverDescriptor::for_type::<Target>(ReceiverKind::PinnedShared);
        static NO_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
            parameters: &[],
            predicates: &[],
        };
        static PINNED_METHOD: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
            "pinned",
            MethodNamespaceMetadata::Inherent {
                implementation: "Target::pinned",
            },
            MethodVisibility::Public,
            Some(PINNED_RECEIVER),
            &[],
            &ADAPTER_OUTPUT,
            MethodQualifiers::new(false, false, false, false),
            &NO_GENERICS,
            false,
            false,
        ));
        let method = MethodDescriptor::new(
            EffectiveMethod::for_type::<Target>(&PINNED_METHOD).with_adapters(PINNED_ADAPTERS),
            Box::leak(Box::new(ReflectionContext::empty())),
        );
        let target = Box::pin(Target);
        let output = match method.invoke_pinned(PinnedInvocation::<Local, _>::shared(target.as_ref())) {
            Ok(output) => output,
            Err(_) => panic!("matching pinned receiver adapter is callable"),
        };
        let value = match output.into_owned::<u32>() {
            Ok(value) => value,
            Err(_) => panic!("pinned adapter returns the declared type"),
        };
        assert_eq!(value, 41);

        let unavailable = MethodDescriptor::new(
            EffectiveMethod::for_type::<Target>(&PINNED_METHOD).with_adapters(ADAPTERS),
            Box::leak(Box::new(ReflectionContext::empty())),
        );
        let failure = match unavailable.invoke_pinned(PinnedInvocation::<Local, _>::shared(target.as_ref())) {
            Err(failure) => failure,
            Ok(_) => panic!("a method without a pinned adapter must reject invocation"),
        };
        assert_eq!(*failure.error().kind(), InvocationErrorKind::ReceiverAdapterUnavailable);
    }

    #[test]
    fn test_inherent_metadata_constructors_keep_their_documented_defaults() {
        let metadata = MethodMetadata::inherent("unit", "Target::unit");
        assert_eq!(metadata.name, "unit");
        assert!(matches!(
            metadata.namespace,
            MethodNamespaceMetadata::Inherent {
                implementation: "Target::unit"
            }
        ));
        assert_eq!(metadata.visibility, MethodVisibility::Public);
        assert!(metadata.receiver.is_none());
        assert!(metadata.parameters.is_empty());
        assert_eq!(metadata.output.kind, ReturnKind::Unit);
        assert!(!metadata.is_generic);
        assert!(!metadata.default_body);

        let spec = MethodMetadataSpec::inherent("another", "Target::another");
        assert_eq!(spec.name, "another");
        assert!(spec.receiver.is_none());
        assert!(spec.parameters.is_empty());
        assert_eq!(spec.output.kind, ReturnKind::Unit);
        assert!(!spec.qualifiers.is_async);
        assert!(!spec.is_generic);
    }

    #[test]
    fn test_runtime_specialization_builders_resolve_each_public_argument_constructor() {
        static U8_TYPE_EXPRESSION: ExpressionData = ExpressionData::Named {
            path: "u8",
            definition: None,
            arguments: &[],
        };
        let parameters = Box::leak(
            vec![
                GenericParameterMetadata::type_parameter("T", 0, &[], None),
                GenericParameterMetadata::lifetime("a", 1, &[]),
                GenericParameterMetadata::const_parameter("flag", 2, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("character", 3, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("i8_value", 4, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("i16_value", 5, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("i32_value", 6, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("i64_value", 7, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("i128_value", 8, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("isize_value", 9, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("u8_value", 10, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("u16_value", 11, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("u32_value", 12, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("u64_value", 13, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("u128_value", 14, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("usize_value", 15, &TYPE_EXPRESSION, None),
                GenericParameterMetadata::const_parameter("literal", 16, &U8_TYPE_EXPRESSION, None),
            ]
            .into_boxed_slice(),
        );
        let arguments = Box::leak(
            vec![
                MethodSpecializationArgument::type_argument::<Target>(),
                MethodSpecializationArgument::lifetime("'static"),
                MethodSpecializationArgument::const_bool(std::hint::black_box(true)),
                MethodSpecializationArgument::const_char(std::hint::black_box('R')),
                MethodSpecializationArgument::i8(std::hint::black_box(-8)),
                MethodSpecializationArgument::i16(std::hint::black_box(-16)),
                MethodSpecializationArgument::i32(std::hint::black_box(-32)),
                MethodSpecializationArgument::i64(std::hint::black_box(-64)),
                MethodSpecializationArgument::i128(std::hint::black_box(-128)),
                MethodSpecializationArgument::isize(std::hint::black_box(-1)),
                MethodSpecializationArgument::u8(std::hint::black_box(8)),
                MethodSpecializationArgument::u16(std::hint::black_box(16)),
                MethodSpecializationArgument::u32(std::hint::black_box(32)),
                MethodSpecializationArgument::u64(std::hint::black_box(64)),
                MethodSpecializationArgument::u128(std::hint::black_box(128)),
                MethodSpecializationArgument::usize(std::hint::black_box(1)),
                MethodSpecializationArgument::const_integer_literal(
                    std::hint::black_box(false),
                    std::hint::black_box(255),
                ),
            ]
            .into_boxed_slice(),
        );
        let reasons = Box::leak(vec![super::InvocationUnavailableReason::AdapterUnavailable].into_boxed_slice());
        let spec = MethodSpecializationSpec::new(arguments, ADAPTERS).with_unavailable(std::hint::black_box(reasons));
        let resolved = spec
            .resolve(std::hint::black_box(parameters))
            .expect("all argument constructors produce matching metadata");
        assert!(matches!(
            resolved.availability,
            super::InvocationAvailability::Unavailable { .. }
        ));
        assert_eq!(resolved.arguments.as_slice().len(), parameters.len());
    }

    #[test]
    fn test_integer_expression_paths_and_normalization_cover_signed_and_unsigned_ranges() {
        static EXPRESSIONS: [ExpressionData; 12] = [
            ExpressionData::Named {
                path: "i8",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "i16",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "i32",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "i64",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "i128",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "isize",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "u8",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "u16",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "u32",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "u64",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "u128",
                definition: None,
                arguments: &[],
            },
            ExpressionData::Named {
                path: "usize",
                definition: None,
                arguments: &[],
            },
        ];
        static EXPECTED: [&str; 12] = [
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        ];
        for (expression, expected) in EXPRESSIONS.iter().zip(EXPECTED) {
            assert_eq!(
                super::integer_kind_for_expression(expression).map(super::ConstIntegerKind::type_name),
                Some(expected)
            );
        }

        static NAMED_EXPRESSION_WITH_ARGUMENT: ExpressionData = ExpressionData::Named {
            path: "u8",
            definition: None,
            arguments: &[crate::runtime::expression::GenericExpressionArgument::Type(
                &TYPE_EXPRESSION,
            )],
        };
        static OPAQUE_EXPRESSION: ExpressionData = ExpressionData::Tuple(&[]);
        assert!(super::integer_kind_for_expression(&NAMED_EXPRESSION_WITH_ARGUMENT).is_none());
        assert!(super::integer_kind_for_expression(&OPAQUE_EXPRESSION).is_none());

        let signed = [
            (super::ConstIntegerKind::I8, 8),
            (super::ConstIntegerKind::I16, 16),
            (super::ConstIntegerKind::I32, 32),
            (super::ConstIntegerKind::I64, 64),
            (super::ConstIntegerKind::I128, 128),
            (super::ConstIntegerKind::Isize, usize::BITS),
        ];
        for (kind, bits) in signed {
            let maximum = (1_u128 << (bits - 1)) - 1;
            let minimum_magnitude = 1_u128 << (bits - 1);
            assert!(kind.normalize(false, maximum).is_some());
            assert!(kind.normalize(true, minimum_magnitude).is_some());
            assert!(kind.normalize(false, minimum_magnitude).is_none());
            assert!(kind.normalize(true, minimum_magnitude + 1).is_none());
        }

        let unsigned = [
            (super::ConstIntegerKind::U8, 8),
            (super::ConstIntegerKind::U16, 16),
            (super::ConstIntegerKind::U32, 32),
            (super::ConstIntegerKind::U64, 64),
            (super::ConstIntegerKind::U128, 128),
            (super::ConstIntegerKind::Usize, usize::BITS),
        ];
        for (kind, bits) in unsigned {
            assert_eq!(kind.normalize(false, 0).as_deref(), Some("0"));
            assert!(kind.normalize(true, 1).is_none());
            if bits < 128 {
                assert!(kind.normalize(false, 1_u128 << bits).is_none());
            }
        }
    }

    #[test]
    fn test_receiver_and_optional_return_metadata_preserve_absence() {
        static NO_RECEIVER: MethodMetadata = MethodMetadata::from_spec(MethodMetadataSpec::new(
            "unit",
            MethodNamespaceMetadata::Inherent {
                implementation: "Target::unit",
            },
            MethodVisibility::Private,
            None,
            &[],
            &ReturnMetadata::unit(),
            MethodQualifiers::new(false, false, true, true),
            &GenericDefinitionMetadata {
                parameters: &[],
                predicates: &[],
            },
            false,
            true,
        ));
        let method = descriptor(&NO_RECEIVER);
        assert!(method.receiver().is_none());
        assert_eq!(method.parameters().len(), 0);
        assert!(method.parameters().is_empty());
        assert_eq!(method.output().kind(), ReturnKind::Unit);
        assert!(method.output().type_expression().is_none());
        assert!(method.output().find_concrete_type().is_none());
        assert!(method.qualifiers().is_unsafe);
        assert!(method.qualifiers().is_variadic);
        assert!(method.generics().is_empty());
        assert_eq!(
            method.visibility(),
            crate::runtime::descriptor::trait_descriptor::Visibility::Private
        );
        assert!(method.availability().eq(&MethodAvailability::Unavailable {
            reasons: &[super::InvocationUnavailableReason::AdapterUnavailable],
        }));
    }
}
