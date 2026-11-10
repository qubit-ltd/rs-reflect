// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// One compile-time-bound method implementation.
///
/// A binding supplies the concrete call signature used to validate a pinned
/// invocation after its generic arguments select this specialization.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// # #[cfg(feature = "derive")]
/// # fn main() {
/// mod example {
///     use std::pin::Pin;
///     use qubit_reflect::Reflect;
///     use qubit_reflect::ReflectImpl;
///     use qubit_reflect::ReflectionContext;
///     use qubit_reflect::bind_method;
///     use qubit_reflect::descriptor::MethodArguments;
///     use qubit_reflect::invoke::PinnedInvocation;
///     use qubit_reflect::value::Local;
///
///     #[Reflect]
///     pub struct PinnedCounter;
///
///     #[ReflectImpl]
///     impl PinnedCounter {
///         pub fn echo<T>(self: Pin<&mut Self>, value: T) -> T {
///             let _ = self;
///             value
///         }
///     }
///
///     pub fn run() {
///         let binding = bind_method!(PinnedCounter::echo::<u32>
///             as fn(Pin<&mut PinnedCounter>, u32) -> u32);
///         let context = ReflectionContext::builder()
///             .include_linked()
///             .bind(binding)
///             .build()
///             .expect("the concrete pinned binding is valid");
///         let method = context.type_of::<PinnedCounter>().method("echo").expect("method");
///         let specialization = method.specialization(&MethodArguments::new().type_of::<u32>())
///             .expect("u32 selects the binding");
///         let mut receiver = Box::pin(PinnedCounter);
///         let output = match specialization.invoke_pinned(
///             PinnedInvocation::<Local, _>::mutable(Pin::as_mut(&mut receiver)).arg(7_u32),
///         ) {
///             Ok(output) => output,
///             Err(failure) => panic!("pinned invocation failed: {:?}", failure.error().kind()),
///         };
///         assert_eq!(output.into_owned::<u32>().ok(), Some(7));
///     }
/// }
/// example::run();
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
#[derive(Clone)]
pub struct MethodSpecializationDescriptor<'ctx> {
    /// Source method declaration shared by all of its registered
    /// specializations.
    method: MethodDescriptor<'ctx>,
    /// Registered concrete generic instantiations available for this method.
    specializations: std::sync::Arc<[ResolvedMethodSpecialization]>,
    /// Index of this specialization in the method's shared specialization
    /// array.
    index: usize,
}

impl<'ctx> MethodSpecializationDescriptor<'ctx> {
    /// Returns the resolved data for this descriptor's selected specialization.
    fn resolved(&self) -> &ResolvedMethodSpecialization {
        &self.specializations[self.index]
    }
    /// Returns the source method declaration that owns this specialization.
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
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of borrowed receiver and argument values.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
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
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the pinned receiver and borrowed arguments.
    /// - `T`: Concrete receiver type protected by the pin.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    pub fn invoke_pinned<'call, T: ?Sized + 'static, M: ValueMode>(
        &self,
        invocation: PinnedInvocation<'call, M, T>,
    ) -> Result<InvocationOutput<'call, M>, PinnedInvocationFailure<'call, M, T>> {
        let selected = self.resolved();
        self.method.invoke_pinned_with(
            selected.availability,
            &selected.adapters,
            selected.parameters,
            invocation,
        )
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

/// Constructs the lookup candidate identifying one method specialization.
///
/// # Parameters
///
/// - `index`: Zero-based specialization index.
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
