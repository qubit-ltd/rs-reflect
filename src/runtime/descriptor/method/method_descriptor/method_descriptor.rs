// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

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
                return self.invoke_pinned_with(availability, &adapters, None, invocation);
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
        self.invoke_pinned_with(
            specialization.availability,
            &specialization.adapters,
            specialization.parameters,
            invocation,
        )
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
        parameters: Option<&'static [ParameterMetadata]>,
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
        let validated = invocation.validate(self.clone(), parameters)?;
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
