//! Method declarations, effective implementations, and callable specializations.

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::GenericDefinitionDescriptor;
use crate::runtime::error::{ContextError, LookupError, LookupQuery, LookupSubject};
use crate::runtime::expression::{ExpressionData, TypeExpression};
use crate::runtime::identity::{
    GenericArgumentIdentity, ItemIdentity, ItemKind, LookupCandidate, StableItemId,
};
use crate::runtime::invoke::{
    CaughtInvocationFailure, Invocation, InvocationAdapter, InvocationAttempt,
    InvocationAvailability, InvocationError, InvocationErrorKind, InvocationFailure,
    InvocationOutput, InvocationPanic, InvocationUnavailableReason, Local, ModeAdapter,
    PinnedInvocation, PinnedInvocationAdapter, PinnedInvocationFailure, ThreadSafe, ValueMode,
};
use crate::runtime::lookup::Lookup;
use std::any::TypeId;

pub(crate) use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
pub(crate) use crate::runtime::descriptor::metadata::TypeMetadata;

/// Visibility recorded for a method declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodVisibility {
    Public,
    Restricted,
    Private,
}

/// The receiver form declared by a method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiverKind {
    Owned,
    Shared,
    Mutable,
    PinnedShared,
    PinnedMutable,
}

/// Metadata for the receiver parameter.
#[derive(Clone, Copy)]
pub struct ReceiverDescriptor {
    pub(crate) kind: ReceiverKind,
    pub(crate) type_metadata:
        Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
    pub(crate) expression: Option<&'static ExpressionData>,
}

impl ReceiverDescriptor {
    /// Creates receiver metadata for one concrete reflected target type.
    pub const fn for_type<T: super::Reflect>(kind: ReceiverKind) -> Self {
        Self {
            kind,
            type_metadata: Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
            expression: None,
        }
    }

    /// Creates receiver metadata whose concrete type is supplied by an effective method.
    pub const fn trait_self(kind: ReceiverKind, expression: &'static ExpressionData) -> Self {
        Self {
            kind,
            type_metadata: None,
            expression: Some(expression),
        }
    }

    /// Binds an unbound receiver declaration to its effective target type.
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
    pub const fn with_expression(mut self, expression: &'static ExpressionData) -> Self {
        self.expression = Some(expression);
        self
    }
}

/// Source pattern retained for a non-receiver parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterPattern {
    Identifier(&'static str),
    Wildcard,
    Destructure,
    DestructurePattern(&'static str),
}

/// How a method parameter receives its value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PassingMode {
    Owned,
    Shared,
    Mutable,
}

/// Static facts about one declared parameter.
#[derive(Clone, Copy)]
pub struct ParameterMetadata {
    pub(crate) index: usize,
    pub(crate) pattern: ParameterPattern,
    pub(crate) passing: PassingMode,
    pub(crate) expression: &'static ExpressionData,
    pub(crate) concrete_type:
        Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
}

impl ParameterMetadata {
    /// Creates static metadata for one method parameter.
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
    pub const fn with_concrete_type<T: super::Reflect>(mut self) -> Self {
        self.concrete_type = Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>);
        self
    }
}

/// A context-bound view of one method parameter.
#[derive(Clone, Copy)]
pub(crate) struct ParameterDescriptor<'ctx> {
    metadata: &'static ParameterMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ParameterDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static ParameterMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn index(self) -> usize {
        self.metadata.index
    }
    pub(crate) const fn name(self) -> Option<&'static str> {
        match self.metadata.pattern {
            ParameterPattern::Identifier(name) => Some(name),
            _ => None,
        }
    }
    pub(crate) const fn pattern(self) -> ParameterPattern {
        self.metadata.pattern
    }
    pub(crate) const fn passing(self) -> PassingMode {
        self.metadata.passing
    }
    pub(crate) fn type_expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.expression, self.context)
    }
    pub(crate) fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete_type
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
}

/// The declared kind and optional type expression for a return value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnKind {
    Never,
    Unit,
    Owned,
    Shared,
    OptionalShared,
    SharedSlice,
    Mutable,
    Future,
    Opaque,
}

/// Static return metadata for a method declaration.
#[derive(Clone, Copy)]
pub struct ReturnMetadata {
    pub(crate) kind: ReturnKind,
    pub(crate) expression: Option<&'static ExpressionData>,
    pub(crate) concrete_type:
        Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata>,
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
    pub const fn new(kind: ReturnKind, expression: &'static ExpressionData) -> Self {
        Self {
            kind,
            expression: Some(expression),
            concrete_type: None,
        }
    }

    /// Records the exact concrete Rust type when the return value has one.
    pub const fn with_concrete_type<T: super::Reflect>(mut self) -> Self {
        self.concrete_type = Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>);
        self
    }
}

/// A context-bound view of a method's declared return value.
#[derive(Clone, Copy)]
pub(crate) struct ReturnDescriptor<'ctx> {
    metadata: &'static ReturnMetadata,
    context: &'ctx ReflectionContext,
}

impl<'ctx> ReturnDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static ReturnMetadata,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { metadata, context }
    }
    pub(crate) const fn kind(self) -> ReturnKind {
        self.metadata.kind
    }
    pub(crate) fn type_expression(self) -> Option<TypeExpression<'ctx>> {
        self.metadata
            .expression
            .map(|expression| TypeExpression::new(expression, self.context))
    }
    pub(crate) fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.metadata
            .concrete_type
            .map(|metadata| TypeDescriptor::new(metadata(), self.context))
    }
}

/// Source-level method qualifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodQualifiers {
    pub(crate) is_const: bool,
    pub(crate) is_async: bool,
    pub(crate) is_unsafe: bool,
    pub(crate) abi: Option<FunctionAbi>,
    pub(crate) is_variadic: bool,
}

impl MethodQualifiers {
    /// Creates source qualifiers without a custom ABI.
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
    pub const fn with_abi(mut self, abi: Option<FunctionAbi>) -> Self {
        self.abi = abi;
        self
    }
}

/// Function ABI facts retained from the source declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FunctionAbi {
    Rust,
    C,
    System,
    Other(&'static str),
}

/// Source namespace for a method declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MethodNamespace<'ctx> {
    Inherent {
        implementation: &'ctx str,
    },
    Trait {
        trait_: &'ctx str,
        implementation: &'ctx str,
    },
}

/// Structured qualified method name.
#[derive(Clone, Copy)]
pub(crate) enum MethodQualifiedName<'ctx> {
    Inherent {
        target: TypeDescriptor<'ctx>,
        name: &'static str,
    },
    Trait {
        target: TypeDescriptor<'ctx>,
        trait_: &'ctx str,
        name: &'static str,
    },
}

/// Why the effective implementation came from its current source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodImplementationSource {
    Declared,
    Required,
    Defaulted,
    Overridden,
}

/// Callability reasons recorded during registration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodAvailability {
    Callable,
    RequiresSpecialization,
    RequiresSpecializationSelection {
        count: usize,
    },
    Unavailable {
        reasons: &'static [InvocationUnavailableReason],
    },
}

/// Source declaration metadata, shared by all effective implementations.
pub struct MethodMetadata {
    pub(crate) name: &'static str,
    pub(crate) namespace: MethodNamespaceMetadata,
    pub(crate) visibility: MethodVisibility,
    pub(crate) receiver: Option<ReceiverDescriptor>,
    pub(crate) parameters: &'static [ParameterMetadata],
    pub(crate) output: &'static ReturnMetadata,
    pub(crate) qualifiers: MethodQualifiers,
    pub(crate) generics: &'static GenericDefinitionMetadata,
    pub(crate) is_generic: bool,
    pub(crate) default_body: bool,
}

/// Static namespace identity used before it is bound to a reflection context.
#[derive(Clone, Copy)]
pub enum MethodNamespaceMetadata {
    Inherent {
        implementation: &'static str,
    },
    Trait {
        trait_: &'static str,
        implementation: &'static str,
    },
}

static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: &[],
};
static NO_METHOD_ADAPTER: [InvocationUnavailableReason; 1] =
    [InvocationUnavailableReason::AdapterUnavailable];
static UNIT_OUTPUT: ReturnMetadata = ReturnMetadata {
    kind: ReturnKind::Unit,
    expression: None,
    concrete_type: None,
};

impl MethodMetadata {
    /// Creates method metadata from a complete typed specification.
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
}

impl MethodMetadataSpec {
    /// Creates an inherent method specification with no receiver or parameters.
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
    pub const fn for_type<T: super::Reflect>(metadata: &'static MethodMetadata) -> Self {
        Self {
            target: crate::runtime::descriptor::metadata::TypeMetadata::of::<T>,
            metadata,
            source: MethodImplementationSource::Declared,
            availability: MethodAvailability::Unavailable {
                reasons: &NO_METHOD_ADAPTER,
            },
            specializations: &[],
            adapters: None,
        }
    }

    /// Binds generated adapters to the method's exact target type.
    pub const fn with_adapters(mut self, adapters: InvocationAdapters) -> Self {
        self.availability = MethodAvailability::Callable;
        self.specializations = &[];
        self.adapters = Some(adapters);
        self
    }
}

/// The callable implementation selected for a target type.
#[derive(Clone, Copy)]
pub struct EffectiveMethod {
    pub(crate) target: fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata,
    pub(crate) metadata: &'static MethodMetadata,
    pub(crate) source: MethodImplementationSource,
    pub(crate) availability: MethodAvailability,
    pub(crate) specializations: &'static [ResolvedMethodSpecialization],
    pub(crate) adapters: Option<InvocationAdapters>,
}

/// A compile-time-bound implementation of one generic method.
pub(crate) struct ResolvedMethodSpecialization {
    pub(crate) arguments: &'static [GenericArgumentIdentity],
    pub(crate) availability: InvocationAvailability,
    pub(crate) adapters: &'static InvocationAdapters,
}

/// Mode-specific adapters generated for a concrete specialization.
#[derive(Clone, Copy)]
pub struct InvocationAdapters {
    pub(crate) local: Option<&'static dyn InvocationAdapter<Local>>,
    pub(crate) thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    pub(crate) catching_local: Option<&'static dyn InvocationAdapter<Local>>,
    pub(crate) catching_thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    pub(crate) pinned_local: Option<&'static dyn PinnedInvocationAdapter<Local>>,
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
    local: Option<&'static dyn InvocationAdapter<Local>>,
    thread_safe: Option<&'static dyn InvocationAdapter<ThreadSafe>>,
    pinned_local: Option<&'static dyn PinnedInvocationAdapter<Local>>,
    pinned_thread_safe: Option<&'static dyn PinnedInvocationAdapter<ThreadSafe>>,
}

impl InvocationAdaptersBuilder {
    /// Sets the local adapter.
    pub const fn local(mut self, adapter: &'static dyn InvocationAdapter<Local>) -> Self {
        self.local = Some(adapter);
        self
    }

    /// Sets the thread-safe adapter.
    pub const fn thread_safe(
        mut self,
        adapter: &'static dyn InvocationAdapter<ThreadSafe>,
    ) -> Self {
        self.thread_safe = Some(adapter);
        self
    }

    /// Sets the local pinned receiver adapter.
    pub const fn pinned_local<T: ?Sized + 'static>(
        mut self,
        adapter: &'static crate::runtime::invoke::PinnedFunctionInvocationAdapter<Local, T>,
    ) -> Self {
        self.pinned_local = Some(adapter);
        self
    }

    /// Sets the thread-safe pinned receiver adapter.
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
            catching_local: None,
            catching_thread_safe: None,
            pinned_local: self.pinned_local,
            pinned_thread_safe: self.pinned_thread_safe,
        }
    }
}

/// A method declaration combined with its effective implementation in one context.
#[derive(Clone, Copy)]
pub struct MethodDescriptor<'ctx> {
    effective: &'ctx EffectiveMethod,
    context: &'ctx ReflectionContext,
}

impl<'ctx> MethodDescriptor<'ctx> {
    pub(crate) const fn new(
        effective: &'ctx EffectiveMethod,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { effective, context }
    }
    pub(crate) fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    pub(crate) fn target(self) -> TypeDescriptor<'ctx> {
        TypeDescriptor::new((self.effective.target)(), self.context)
    }
    pub fn name(self) -> &'static str {
        self.effective.metadata.name
    }
    pub(crate) fn namespace(self) -> MethodNamespace<'ctx> {
        match &self.effective.metadata.namespace {
            MethodNamespaceMetadata::Inherent { implementation } => {
                MethodNamespace::Inherent { implementation }
            }
            MethodNamespaceMetadata::Trait {
                trait_,
                implementation,
            } => MethodNamespace::Trait {
                trait_,
                implementation,
            },
        }
    }
    pub(crate) fn qualified_name(self) -> MethodQualifiedName<'ctx> {
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
    pub(crate) const fn visibility(self) -> MethodVisibility {
        self.effective.metadata.visibility
    }
    pub(crate) const fn receiver(self) -> Option<ReceiverDescriptor> {
        match self.effective.metadata.receiver {
            Some(receiver) => Some(receiver.for_target(self.effective.target)),
            None => None,
        }
    }
    pub(crate) fn parameters(
        self,
    ) -> impl ExactSizeIterator<Item = ParameterDescriptor<'ctx>> + 'ctx {
        self.effective
            .metadata
            .parameters
            .iter()
            .map(move |parameter| ParameterDescriptor::new(parameter, self.context))
    }
    pub(crate) fn parameter(self, name: &str) -> Result<ParameterDescriptor<'ctx>, LookupError> {
        let mut matches = self.effective.metadata.parameters.iter().filter(|parameter| matches!(parameter.pattern, ParameterPattern::Identifier(candidate) if candidate == name));
        let Some(first) = matches.next() else {
            return Err(LookupError::Missing {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
            });
        };
        let mut candidates = vec![ParameterDescriptor::new(first, self.context)];
        candidates
            .extend(matches.map(|parameter| ParameterDescriptor::new(parameter, self.context)));
        match candidates.as_slice() {
            [only] => Ok(*only),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::Parameter,
                query: LookupQuery::Name(name.into()),
                candidates: candidates.into_iter().map(parameter_candidate).collect(),
            }),
        }
    }
    pub(crate) fn output(self) -> ReturnDescriptor<'ctx> {
        ReturnDescriptor::new(self.effective.metadata.output, self.context)
    }
    pub(crate) const fn qualifiers(self) -> MethodQualifiers {
        self.effective.metadata.qualifiers
    }
    pub(crate) fn generics(self) -> GenericDefinitionDescriptor<'ctx> {
        GenericDefinitionDescriptor::new(self.effective.metadata.generics, self.context)
    }
    pub(crate) const fn implementation_source(self) -> MethodImplementationSource {
        self.effective.source
    }
    pub(crate) fn availability(self) -> MethodAvailability {
        if !self.effective.metadata.is_generic {
            return self.effective.availability;
        }
        match self.effective.specializations {
            [] => MethodAvailability::RequiresSpecialization,
            [specialization] => match specialization.availability {
                InvocationAvailability::Callable => MethodAvailability::Callable,
                InvocationAvailability::Unavailable { reasons } => {
                    MethodAvailability::Unavailable { reasons }
                }
            },
            specializations => MethodAvailability::RequiresSpecializationSelection {
                count: specializations.len(),
            },
        }
    }
    pub(crate) fn specializations(self) -> MethodSpecializations<'ctx> {
        MethodSpecializations { method: self }
    }
    pub(crate) fn specialization(
        self,
        arguments: &MethodArguments,
    ) -> Result<MethodSpecializationDescriptor<'ctx>, LookupError> {
        let matches = self
            .effective
            .specializations
            .iter()
            .filter(|candidate| candidate.arguments == arguments.arguments)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [resolved] => Ok(MethodSpecializationDescriptor {
                method: self,
                resolved,
            }),
            [] => Err(LookupError::Missing {
                subject: LookupSubject::MethodSpecialization,
                query: LookupQuery::GenericArguments(
                    arguments.arguments.clone().into_boxed_slice(),
                ),
            }),
            _ => Err(LookupError::Ambiguous {
                subject: LookupSubject::MethodSpecialization,
                query: LookupQuery::GenericArguments(
                    arguments.arguments.clone().into_boxed_slice(),
                ),
                candidates: matches
                    .iter()
                    .enumerate()
                    .map(|(index, _)| specialization_candidate(index))
                    .collect(),
            }),
        }
    }
    pub(crate) fn invoke<'call, M: ValueMode + ModeAdapter>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>> {
        match self.invoke_inner(invocation, false, false) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(failure),
            Err(InvocationAttempt::Panicked(_)) => unreachable!("ordinary invoke does not catch"),
        }
    }

    pub(crate) fn invoke_catching<'call, M: ValueMode + ModeAdapter>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, CaughtInvocationFailure<'call, M>> {
        match self.invoke_inner(invocation, true, false) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => {
                Err(CaughtInvocationFailure::Rejected(failure))
            }
            Err(InvocationAttempt::Panicked(panic)) => {
                Err(CaughtInvocationFailure::Panicked(panic))
            }
        }
    }

    /// Invokes a method while bypassing its visibility and unsafe-call checks.
    ///
    /// # Safety
    /// The caller must satisfy the method's Rust safety contract and any invariants
    /// enforced by its source visibility.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn invoke_unchecked<'call, M: ValueMode + ModeAdapter>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>> {
        match self.invoke_inner(invocation, false, true) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(failure),
            Err(InvocationAttempt::Panicked(_)) => unreachable!("ordinary invoke does not catch"),
        }
    }

    pub(crate) fn invoke_pinned<'call, T: ?Sized + 'static, M: ValueMode + ModeAdapter>(
        self,
        invocation: PinnedInvocation<'call, M, T>,
    ) -> Result<InvocationOutput<'call, M>, PinnedInvocationFailure<'call, M, T>> {
        let specialization = match self.effective.specializations {
            [specialization] => specialization,
            [] if !self.effective.metadata.is_generic => {
                let availability = match self.effective.availability {
                    MethodAvailability::Callable => InvocationAvailability::Callable,
                    MethodAvailability::Unavailable { reasons } => {
                        InvocationAvailability::Unavailable { reasons }
                    }
                    MethodAvailability::RequiresSpecialization => {
                        return Err(PinnedInvocationFailure::new(
                            InvocationError::new(InvocationErrorKind::SpecializationRequired),
                            invocation,
                        ));
                    }
                    MethodAvailability::RequiresSpecializationSelection { count } => {
                        return Err(PinnedInvocationFailure::new(
                            InvocationError::new(
                                InvocationErrorKind::SpecializationSelectionRequired { count },
                            ),
                            invocation,
                        ));
                    }
                };
                return self.invoke_pinned_with(
                    availability,
                    self.effective.adapters.as_ref().unwrap_or(&NO_ADAPTERS),
                    invocation,
                );
            }
            [] => {
                return Err(PinnedInvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationRequired),
                    invocation,
                ));
            }
            many => {
                return Err(PinnedInvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired {
                        count: many.len(),
                    }),
                    invocation,
                ));
            }
        };
        self.invoke_pinned_with(
            specialization.availability,
            specialization.adapters,
            invocation,
        )
    }

    fn invoke_pinned_with<'call, T: ?Sized + 'static, M: ValueMode + ModeAdapter>(
        self,
        availability: InvocationAvailability,
        adapters: &InvocationAdapters,
        invocation: PinnedInvocation<'call, M, T>,
    ) -> Result<InvocationOutput<'call, M>, PinnedInvocationFailure<'call, M, T>> {
        let expected_kind = invocation.pinned_kind_and_type().0;
        if self.visibility() != MethodVisibility::Public {
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
        let validated = match invocation.validate(self) {
            Ok(validated) => validated,
            Err(failure) => return Err(failure),
        };
        let Some(adapter) = M::select_pinned_adapter(adapters) else {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::ReceiverAdapterUnavailable),
                validated.into_invocation(),
            ));
        };
        if adapter.receiver_type_id() != TypeId::of::<T>()
            || adapter.receiver_kind() != expected_kind
        {
            return Err(PinnedInvocationFailure::new(
                InvocationError::new(InvocationErrorKind::ReceiverAdapterUnavailable),
                validated.into_invocation(),
            ));
        }
        let (receiver, invocation) = validated.into_parts();
        Ok(adapter.invoke(receiver, invocation))
    }

    fn invoke_inner<'call, M: ValueMode + ModeAdapter>(
        self,
        invocation: Invocation<'call, M>,
        catching: bool,
        unchecked: bool,
    ) -> Result<InvocationOutput<'call, M>, InvocationAttempt<'call, M>> {
        let specialization = match self.effective.specializations {
            [specialization] => specialization,
            [] if !self.effective.metadata.is_generic => {
                return self.invoke_with(
                    self.effective.availability,
                    self.effective.adapters.as_ref().unwrap_or(&NO_ADAPTERS),
                    invocation,
                    catching,
                    unchecked,
                );
            }
            [] => {
                return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationRequired),
                    invocation,
                )));
            }
            many => {
                return Err(InvocationAttempt::Rejected(InvocationFailure::new(
                    InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired {
                        count: many.len(),
                    }),
                    invocation,
                )));
            }
        };
        self.invoke_with_availability(
            specialization.availability,
            specialization.adapters,
            invocation,
            catching,
            unchecked,
        )
    }
    fn invoke_with<'call, M: ValueMode + ModeAdapter>(
        self,
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
                    InvocationError::new(InvocationErrorKind::SpecializationSelectionRequired {
                        count,
                    }),
                    invocation,
                )));
            }
            MethodAvailability::Unavailable { reasons } => {
                InvocationAvailability::Unavailable { reasons }
            }
        };
        self.invoke_with_availability(availability, adapters, invocation, catching, unchecked)
    }
    fn invoke_with_availability<'call, M: ValueMode + ModeAdapter>(
        self,
        availability: InvocationAvailability,
        adapters: &InvocationAdapters,
        invocation: Invocation<'call, M>,
        catching: bool,
        unchecked: bool,
    ) -> Result<InvocationOutput<'call, M>, InvocationAttempt<'call, M>> {
        if !unchecked && self.visibility() != MethodVisibility::Public {
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
        let bound = match invocation.validate(self) {
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
                Err(payload) => Err(InvocationAttempt::Panicked(InvocationPanic::from_payload(
                    payload,
                ))),
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
            pinned_local: None,
            pinned_thread_safe: None,
        }
    }

    fn adapter<M: ValueMode + ModeAdapter>(&self) -> Option<&'static dyn InvocationAdapter<M>> {
        M::select_adapter(self)
    }
}

/// An explicit set of methods used by context registration to build lookups.
#[derive(Clone, Copy)]
pub(crate) struct MethodCatalog<'ctx> {
    target: TypeDescriptor<'ctx>,
    methods: &'ctx [EffectiveMethod],
}

/// Static source metadata for a target's effective method catalog.
pub struct MethodCatalogRegistration {
    type_id: fn() -> TypeId,
    methods: &'static [EffectiveMethod],
}

impl MethodCatalogRegistration {
    pub const fn new<T: super::Reflect>(methods: &'static [EffectiveMethod]) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            methods,
        }
    }

    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn methods(&self) -> &'static [EffectiveMethod] {
        self.methods
    }
}

inventory::collect!(MethodCatalogRegistration);

impl<'ctx> MethodCatalog<'ctx> {
    pub(crate) const fn new(
        target: TypeDescriptor<'ctx>,
        methods: &'ctx [EffectiveMethod],
    ) -> Self {
        Self { target, methods }
    }
    pub(crate) fn lookup_method(
        self,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(self.lookup(|method| method.metadata.name == name))
    }
    pub(crate) fn method(self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.lookup_method(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Method, LookupQuery::Name(name.into()))
    }
    pub(crate) fn find_inherent_method(
        self,
        name: &str,
    ) -> Result<Option<MethodDescriptor<'ctx>>, ContextError> {
        match self.lookup(|method| {
            method.metadata.name == name
                && matches!(
                    method.metadata.namespace,
                    MethodNamespaceMetadata::Inherent { .. }
                )
        }) {
            Lookup::Missing => Ok(None),
            Lookup::Unique(method) => Ok(Some(method)),
            Lookup::Ambiguous(_) => {
                unreachable!("inherent method names are unique within one namespace")
            }
        }
    }
    pub(crate) fn inherent_method(self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.find_inherent_method(name)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name.into()),
            })
    }
    pub(crate) fn lookup_trait_method(
        self,
        trait_: &str,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(self.lookup(|method| method.metadata.name == name && matches!(method.metadata.namespace, MethodNamespaceMetadata::Trait { trait_: candidate, .. } if candidate == trait_)))
    }
    pub(crate) fn find_trait_method(
        self,
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
    pub(crate) fn trait_method(
        self,
        trait_: &str,
        name: &str,
    ) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.find_trait_method(trait_, name)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name.into()),
            })
    }
    fn lookup(
        self,
        predicate: impl Fn(&EffectiveMethod) -> bool,
    ) -> Lookup<MethodDescriptor<'ctx>> {
        let mut matches = self
            .methods
            .iter()
            .filter(|method| {
                (method.target)().type_id() == self.target.type_id() && predicate(method)
            })
            .map(|method| MethodDescriptor::new(method, self.target.context()));
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
    fn into_lookup_candidate(self) -> LookupCandidate {
        let display_name: Box<str> = self.name().into();
        let namespace_id = match self.effective.metadata.namespace {
            MethodNamespaceMetadata::Inherent { implementation } => {
                format!("inherent:{implementation}")
            }
            MethodNamespaceMetadata::Trait {
                trait_,
                implementation,
            } => format!("trait:{trait_}:{implementation}"),
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct MethodArguments {
    arguments: Vec<GenericArgumentIdentity>,
}

impl MethodArguments {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn type_of<T: 'static>(mut self) -> Self {
        self.arguments.push(GenericArgumentIdentity::Type(
            std::any::type_name::<T>().into(),
        ));
        self
    }
    pub(crate) fn constant<T: std::fmt::Debug + 'static>(mut self, value: T) -> Self {
        self.arguments.push(GenericArgumentIdentity::Const(
            format!("{}:{value:?}", std::any::type_name::<T>()).into(),
        ));
        self
    }
}

/// A view over the concrete implementations registered for one method.
pub(crate) struct MethodSpecializations<'ctx> {
    method: MethodDescriptor<'ctx>,
}

impl<'ctx> MethodSpecializations<'ctx> {
    pub(crate) fn iter(
        self,
    ) -> impl ExactSizeIterator<Item = MethodSpecializationDescriptor<'ctx>> + 'ctx {
        self.method
            .effective
            .specializations
            .iter()
            .map(move |resolved| MethodSpecializationDescriptor {
                method: self.method,
                resolved,
            })
    }
    pub(crate) fn len(self) -> usize {
        self.method.effective.specializations.len()
    }
}

/// One compile-time-bound method implementation.
#[derive(Clone, Copy)]
pub(crate) struct MethodSpecializationDescriptor<'ctx> {
    method: MethodDescriptor<'ctx>,
    resolved: &'ctx ResolvedMethodSpecialization,
}

impl<'ctx> MethodSpecializationDescriptor<'ctx> {
    pub(crate) const fn method(self) -> MethodDescriptor<'ctx> {
        self.method
    }
    pub(crate) fn arguments(self) -> &'ctx [GenericArgumentIdentity] {
        self.resolved.arguments
    }
    pub(crate) const fn availability(self) -> InvocationAvailability {
        self.resolved.availability
    }
    pub(crate) fn invoke<'call, M: ValueMode + ModeAdapter>(
        self,
        invocation: Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>> {
        match self.method.invoke_with_availability(
            self.resolved.availability,
            self.resolved.adapters,
            invocation,
            false,
            false,
        ) {
            Ok(output) => Ok(output),
            Err(InvocationAttempt::Rejected(failure)) => Err(failure),
            Err(InvocationAttempt::Panicked(_)) => unreachable!("ordinary invoke does not catch"),
        }
    }
}

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

    /// Records whether the callable implementation is declared, defaulted, or overridden.
    pub const fn with_source(mut self, source: MethodImplementationSource) -> Self {
        self.source = source;
        self
    }
