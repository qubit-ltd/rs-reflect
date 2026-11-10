// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Static source facts used to create a method declaration.
pub struct MethodMetadataSpec {
    /// Returns the unique source identifier when the parameter pattern binds a
    /// name.
    pub(super) name: &'static str,
    /// Inherent or trait implementation that supplies this effective method.
    namespace: MethodNamespaceMetadata,
    /// Visibility recorded for this method declaration.
    visibility: MethodVisibility,
    /// Receiver kind and type metadata used to validate invocation binding.
    pub(super) receiver: Option<ReceiverDescriptor>,
    /// Source-ordered metadata for the method’s non-receiver arguments.
    pub(super) parameters: &'static [ParameterMetadata],
    /// Return category and type metadata for values produced by the method.
    pub(super) output: &'static ReturnMetadata,
    /// Const, async, unsafe, ABI, and variadic facts copied from the
    /// declaration.
    pub(super) qualifiers: MethodQualifiers,
    /// Generic parameter metadata required to select a method specialization.
    generics: &'static GenericDefinitionMetadata,
    /// Whether the declaration requires specialization before invocation.
    pub(super) is_generic: bool,
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
    pub const fn for_type<T: crate::runtime::descriptor::Reflect>(metadata: &'static MethodMetadata) -> Self {
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
            specialization_parameters: None,
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
        self.specialization_parameters = None;
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
        self.specialization_parameters = None;
        self
    }

    /// Adds specializations and retains this generated binding's call
    /// signature.
    ///
    /// # Parameters
    ///
    /// - `specializations`: Concrete generic instantiations registered for the
    ///   method.
    pub(crate) const fn with_bound_specializations(
        mut self,
        specializations: &'static [MethodSpecializationSpec],
    ) -> Self {
        self = self.with_specializations(specializations);
        self.specialization_parameters = Some(self.metadata.parameters);
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
        self.specialization_parameters = None;
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
            let mut resolved = self
                .specialization_specs
                .iter()
                .copied()
                .map(|spec| spec.resolve(self.metadata.generics.parameters))
                .collect::<Result<Vec<_>, _>>()?;
            for specialization in &mut resolved {
                specialization.parameters = self.specialization_parameters;
            }
            Ok(resolved.into_boxed_slice())
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
