// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Concrete method specializations and descriptor-aware invocation.

use super::method_implementation_source::MethodImplementationSource;
use super::method_instance_build_error::MethodInstanceBuildError;

use crate::descriptor::method_descriptor::InvocationAdapter;
use crate::descriptor::method_descriptor::InvocationUnavailableReason;
use crate::descriptor::method_descriptor::MethodDescriptor;
use crate::expression::GenericArgument;
use crate::invoke::InvocationDispatchMode;
use crate::invoke::InvocationDispatchReason;
use crate::invoke::InvocationUnavailable;

/// A concrete specialization of one method declaration.
///
/// Invoke with the same registry used for lookup. Descriptor-aware entries
/// bind named arguments before validating and adapting the receiver. A static
/// mode without an entry returns `Err(InvocationUnavailable)` with unchanged
/// input; selecting an entry returns `Ok(...)` containing its invocation
/// result. Pre-execution validation failures are therefore `Ok(Err(...))`
/// with caller-ordered recovery. Outputs, futures, and
/// recovery retain input lifetimes without borrowing the selected registry.
///
/// # Examples
///
/// ```standalone_crate
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// # #[cfg(feature = "derive")]
/// use qubit_reflect::TypeDescriptor;
/// # #[cfg(feature = "derive")]
/// use qubit_reflect::descriptor::MethodLookup;
/// # #[cfg(feature = "derive")]
/// use qubit_reflect::registry::ReflectRegistry;
/// #[cfg(feature = "derive")]
/// mod example {
///     use qubit_reflect::{Reflect, reflect_impl};
///     #[derive(Reflect)]
///     #[reflect(crate = qubit_reflect)]
///     pub struct Service;
///     #[reflect_impl(crate = qubit_reflect)]
///     impl Service {
///         fn ping(&self) {}
///     }
/// }
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let registry = ReflectRegistry::initialize()?;
/// let MethodLookup::Unique(method) =
///     TypeDescriptor::of::<example::Service>().methods_named_in(registry, "ping")
/// else {
///     panic!("one reflected method")
/// };
/// assert!(method.adapter().is_some());
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
#[derive(Clone, Debug)]
pub struct MethodInstanceDescriptor {
    /// Trait or impl declaration whose signature is specialized.
    declaration: &'static MethodDescriptor,
    /// Concrete impl method for an explicit trait override, when present.
    implementation_method: Option<&'static MethodDescriptor>,
    /// Whether the method is declared, required, defaulted, or overridden.
    implementation_source: MethodImplementationSource,
    /// Generated adapters available for this concrete method instance.
    adapter: Option<&'static InvocationAdapter>,
    /// Concrete type and const arguments in declaration order.
    arguments: Box<[GenericArgument]>,
    /// Stable reasons no invocation adapter is available.
    unavailable_reasons: Box<[InvocationUnavailableReason]>,
}

impl MethodInstanceDescriptor {
    /// Creates a concrete method instance.
    ///
    /// `adapter` is present only when `unavailable_reasons` is empty and the
    /// later invocation layer supplied a safe entry point.
    ///
    /// # Parameters
    ///
    /// - `declaration`: The trait or impl method declaration being specialized.
    /// - `implementation_method`: The concrete impl method for an override.
    /// - `implementation_source`: How this instance obtains its method body.
    /// - `adapter`: Generated entry points, when invocation is available.
    /// - `unavailable_reasons`: Reasons invocation is unavailable.
    ///
    /// # Returns
    ///
    /// Returns a validated method instance.
    ///
    /// # Errors
    ///
    /// Returns a build error when declaration ownership, implementation
    /// source, adapter availability, or unavailable reasons are inconsistent.
    #[doc(hidden)]
    pub fn new(
        declaration: &'static MethodDescriptor,
        implementation_method: Option<&'static MethodDescriptor>,
        implementation_source: MethodImplementationSource,
        adapter: Option<&'static InvocationAdapter>,
        unavailable_reasons: Box<[InvocationUnavailableReason]>,
    ) -> Result<Self, MethodInstanceBuildError> {
        Self::with_arguments(
            declaration,
            implementation_method,
            implementation_source,
            adapter,
            Box::new([]),
            unavailable_reasons,
        )
    }

    /// Creates a concrete method specialization with its generic arguments in
    /// declaration order.
    ///
    /// # Parameters
    ///
    /// - `declaration`: The trait or impl method declaration being specialized.
    /// - `implementation_method`: The concrete impl method for an override.
    /// - `implementation_source`: How this instance obtains its method body.
    /// - `adapter`: Generated entry points, when invocation is available.
    /// - `arguments`: Concrete type and const arguments in declaration order.
    /// - `unavailable_reasons`: Reasons invocation is unavailable.
    ///
    /// # Returns
    ///
    /// Returns a validated method instance with its concrete arguments.
    ///
    /// # Errors
    ///
    /// Returns the matching [`MethodInstanceBuildError`] when the declaration
    /// ownership, implementation method, adapter, or unavailable reasons do
    /// not match the implementation source.
    #[doc(hidden)]
    pub fn with_arguments(
        declaration: &'static MethodDescriptor,
        implementation_method: Option<&'static MethodDescriptor>,
        implementation_source: MethodImplementationSource,
        adapter: Option<&'static InvocationAdapter>,
        arguments: Box<[GenericArgument]>,
        unavailable_reasons: Box<[InvocationUnavailableReason]>,
    ) -> Result<Self, MethodInstanceBuildError> {
        if implementation_source == MethodImplementationSource::Required && adapter.is_some() {
            return Err(MethodInstanceBuildError::RequiredMethodHasAdapter);
        }
        match implementation_source {
            MethodImplementationSource::Declared if declaration.declaring_impl().is_none() => {
                return Err(MethodInstanceBuildError::DeclaredMethodNotOwnedByImpl);
            }
            MethodImplementationSource::Required
            | MethodImplementationSource::Defaulted
            | MethodImplementationSource::Overridden
                if declaration.declaring_trait().is_none() =>
            {
                return Err(MethodInstanceBuildError::TraitMethodNotOwnedByTrait);
            }
            _ => {}
        }
        match (implementation_source, implementation_method) {
            (MethodImplementationSource::Overridden, None) => {
                return Err(MethodInstanceBuildError::OverriddenMethodMissingImplementation);
            }
            (
                MethodImplementationSource::Declared
                | MethodImplementationSource::Required
                | MethodImplementationSource::Defaulted,
                Some(_),
            ) => {
                return Err(MethodInstanceBuildError::UnexpectedImplementationMethod);
            }
            _ => {}
        }
        if adapter.is_some() && !unavailable_reasons.is_empty() {
            return Err(MethodInstanceBuildError::AdapterHasUnavailableReasons);
        }
        if adapter.is_none() && unavailable_reasons.is_empty() {
            return Err(MethodInstanceBuildError::UnavailableMethodMissingReasons);
        }
        Ok(Self {
            declaration,
            implementation_method,
            implementation_source,
            adapter,
            arguments,
            unavailable_reasons,
        })
    }

    /// Returns the declaration shared by this concrete specialization.
    ///
    /// # Returns
    ///
    /// Returns the trait or impl declaration whose signature is specialized.
    #[must_use]
    #[inline]
    pub const fn declaration(&self) -> &'static MethodDescriptor {
        self.declaration
    }

    /// Returns the explicit impl method used by an overridden instance.
    ///
    /// `None` means the instance is required or uses its trait default.
    ///
    /// # Returns
    ///
    /// Returns the explicit impl method, or `None` for required/defaulted
    /// methods.
    #[must_use]
    #[inline]
    pub const fn implementation_method(&self) -> Option<&'static MethodDescriptor> {
        self.implementation_method
    }

    /// Returns the effective declaration or explicit implementation method.
    ///
    /// # Returns
    ///
    /// Returns the method signature and identity used for invocation.
    #[must_use]
    #[inline]
    pub const fn effective_method(&self) -> &'static MethodDescriptor {
        match self.implementation_method {
            Some(method) => method,
            None => self.declaration,
        }
    }

    /// Returns whether the implementation is required, defaulted, or
    /// overridden.
    ///
    /// # Returns
    ///
    /// Returns the source that supplies the concrete method behavior.
    #[must_use]
    #[inline]
    pub const fn implementation_source(&self) -> MethodImplementationSource {
        self.implementation_source
    }

    /// Returns the safe invocation adapter when one is available.
    ///
    /// `None` means callers must inspect [`Self::unavailable_reasons`].
    ///
    /// # Returns
    ///
    /// Returns the safe adapter, or `None` when no invocation mode is
    /// available.
    #[must_use]
    #[inline]
    pub const fn adapter(&self) -> Option<&'static InvocationAdapter> {
        self.adapter
    }

    /// Returns the concrete type and const arguments of this method
    /// specialization in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the specialization arguments, excluding lifetime parameters.
    #[must_use]
    #[inline]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns stable reasons that prevent dynamic invocation.
    ///
    /// # Returns
    ///
    /// Returns every recorded reason no adapter is available.
    #[must_use]
    #[inline]
    pub const fn unavailable_reasons(&self) -> &[InvocationUnavailableReason] {
        &self.unavailable_reasons
    }

    /// Binds and invokes this concrete method through its local adapter.
    ///
    /// Positional inputs bind the next unoccupied declaration-order parameter;
    /// named inputs may be interleaved and bind only a unique simple
    /// identifier parameter. Binding, receiver, mode, and exact-type checks
    /// all occur before the generated adapter extracts any owned value. Their
    /// failures therefore retain the complete original invocation recovery.
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable. Otherwise the result contains either the
    /// invocation output or a structured pre-execution failure.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Local receiver and arguments to bind and invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure contains the original invocation inputs when
    /// execution could not safely consume them.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
    #[must_use = "handle dispatch errors to recover the original invocation inputs"]
    pub fn invoke_local<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> crate::invoke::InvocationDispatchResult<
        crate::invoke::Invocation<'call, crate::value::Local>,
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::InvocationFailure<'call, crate::value::Local>,
        >,
    > {
        let mode = InvocationDispatchMode::Local;
        let adapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::NoAdapter {
                        reasons: self.unavailable_reasons.clone(),
                    },
                    invocation,
                ));
            }
        };
        let entry_point = match adapter.local {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::MissingEntry,
                    invocation,
                ));
            }
        };
        Ok(invocation
            .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
            .and_then(|invocation| entry_point(registry, invocation)))
    }

    /// Binds and invokes this concrete method through its thread-safe adapter.
    ///
    /// Binding uses the same interleaved named/positional rules and complete
    /// pre-execution recovery contract as [`Self::invoke_local`].
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable. Otherwise the result contains either the
    /// invocation output or a structured pre-execution failure.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Thread-safe receiver and arguments to bind and invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure contains the original invocation inputs when
    /// execution could not safely consume them.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
    #[must_use = "handle dispatch errors to recover the original invocation inputs"]
    pub fn invoke_thread_safe<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> crate::invoke::InvocationDispatchResult<
        crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::ThreadSafe>,
            crate::invoke::InvocationFailure<'call, crate::value::ThreadSafe>,
        >,
    > {
        let mode = InvocationDispatchMode::ThreadSafe;
        let adapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::NoAdapter {
                        reasons: self.unavailable_reasons.clone(),
                    },
                    invocation,
                ));
            }
        };
        let entry_point = match adapter.thread_safe {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::MissingEntry,
                    invocation,
                ));
            }
        };
        Ok(invocation
            .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
            .and_then(|invocation| entry_point(registry, invocation)))
    }

    /// Binds and invokes this concrete method through its local panic-catching
    /// adapter.
    ///
    /// Named and positional inputs follow the same descriptor-aware binding
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// A panic after successful validation is returned as
    /// [`InvocationPanic`](crate::invoke::InvocationPanic), independently of
    /// binding or type-validation failures.
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// Catching covers only the synchronous entry call. Panics while polling a
    /// returned future propagate and are not captured by this entry.
    /// Under `panic=abort`, a requested catching entry is unavailable. When the
    /// ordinary entry for the same mode exists, dispatch reports `PanicAbort`;
    /// if that ordinary entry is also missing, it reports `MissingEntry` first.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Local receiver and arguments to bind and invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable. Otherwise, returns `Ok(Ok(Ok(output)))` on
    /// success, `Ok(Ok(Err(panic)))` when the method panics, or
    /// `Ok(Err(failure))` when validation fails.
    ///
    /// # Errors
    ///
    /// The dispatch error retains the original input when no entry is
    /// available. Within dispatched results, validation errors retain
    /// pre-execution recovery; the innermost error contains a panic
    /// captured after validation.
    #[must_use = "handle dispatch errors to recover the original invocation inputs"]
    pub fn invoke_catching_local<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> crate::invoke::InvocationDispatchResult<
        crate::invoke::Invocation<'call, crate::value::Local>,
        crate::invoke::CatchingInvocationResult<'call, crate::value::Local>,
    > {
        let mode = InvocationDispatchMode::CatchingLocal;
        let adapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::NoAdapter {
                        reasons: self.unavailable_reasons.clone(),
                    },
                    invocation,
                ));
            }
        };
        let entry_point = match adapter.catching_local {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    if adapter.local.is_none() {
                        InvocationDispatchReason::MissingEntry
                    } else {
                        match adapter.catching_availability() {
                            crate::descriptor::CatchingAvailability::NotRequested => {
                                InvocationDispatchReason::CatchingNotRequested
                            }
                            crate::descriptor::CatchingAvailability::UnavailablePanicAbort => {
                                InvocationDispatchReason::PanicAbort
                            }
                            crate::descriptor::CatchingAvailability::Available => {
                                InvocationDispatchReason::MissingEntry
                            }
                        }
                    },
                    invocation,
                ));
            }
        };
        Ok(
            match invocation.bind_arguments(self.effective_method().identity(), self.effective_method().parameters()) {
                Ok(invocation) => entry_point(registry, invocation),
                Err(failure) => Err(failure),
            },
        )
    }

    /// Binds and invokes this concrete method through its thread-safe
    /// panic-catching adapter.
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// Catching covers only the synchronous entry call. Panics while polling a
    /// returned future propagate and are not captured by this entry.
    /// Under `panic=abort`, a requested catching entry is unavailable. When the
    /// ordinary entry for the same mode exists, dispatch reports `PanicAbort`;
    /// if that ordinary entry is also missing, it reports `MissingEntry` first.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Thread-safe receiver and arguments to bind and invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable. Otherwise, returns `Ok(Ok(Ok(output)))` on
    /// success, `Ok(Ok(Err(panic)))` when the method panics, or
    /// `Ok(Err(failure))` when validation fails.
    ///
    /// # Errors
    ///
    /// The dispatch error retains the original input when no entry is
    /// available. Within dispatched results, validation errors retain
    /// pre-execution recovery; the innermost error contains a panic
    /// captured after validation.
    #[must_use = "handle dispatch errors to recover the original invocation inputs"]
    pub fn invoke_catching_thread_safe<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> crate::invoke::InvocationDispatchResult<
        crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
        crate::invoke::CatchingInvocationResult<'call, crate::value::ThreadSafe>,
    > {
        let mode = InvocationDispatchMode::CatchingThreadSafe;
        let adapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::NoAdapter {
                        reasons: self.unavailable_reasons.clone(),
                    },
                    invocation,
                ));
            }
        };
        let entry_point = match adapter.catching_thread_safe {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    if adapter.thread_safe.is_none() {
                        InvocationDispatchReason::MissingEntry
                    } else {
                        match adapter.catching_availability() {
                            crate::descriptor::CatchingAvailability::NotRequested => {
                                InvocationDispatchReason::CatchingNotRequested
                            }
                            crate::descriptor::CatchingAvailability::UnavailablePanicAbort => {
                                InvocationDispatchReason::PanicAbort
                            }
                            crate::descriptor::CatchingAvailability::Available => {
                                InvocationDispatchReason::MissingEntry
                            }
                        }
                    },
                    invocation,
                ));
            }
        };
        Ok(
            match invocation.bind_arguments(self.effective_method().identity(), self.effective_method().parameters()) {
                Ok(invocation) => entry_point(registry, invocation),
                Err(failure) => Err(failure),
            },
        )
    }

    /// Binds and invokes this concrete method through a typed local
    /// `Pin<&T>` adapter.
    ///
    /// Named and positional arguments follow the same descriptor-aware rules
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// The receiver remains typed and pinned throughout binding and adapter
    /// validation.
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    /// - `T`: Exact concrete receiver type captured by the typed adapter.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Pinned shared receiver and arguments to bind and invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure retains the pinned receiver and original arguments
    /// when they were rejected before execution.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
    #[must_use = "handle dispatch errors to recover the original invocation inputs"]
    // Keep dispatch and validation recovery explicit for the exact pinned T and call lifetime.
    #[allow(clippy::type_complexity)]
    pub fn invoke_pinned_ref_local<'call, T: 'static>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::PinnedRefInvocation<'call, T, crate::value::Local>,
    ) -> crate::invoke::InvocationDispatchResult<
        crate::invoke::PinnedRefInvocation<'call, T, crate::value::Local>,
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedRefInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        let mode = InvocationDispatchMode::PinnedRefLocal;
        let adapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::NoAdapter {
                        reasons: self.unavailable_reasons.clone(),
                    },
                    invocation,
                ));
            }
        };
        let entry_point = match adapter.pinned_ref_local {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::MissingEntry,
                    invocation,
                ));
            }
        };
        let entry_point = match entry_point.downcast_ref::<crate::invoke::PinnedRefAdapter<T, crate::value::Local>>() {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::PinnedReceiverTypeMismatch,
                    invocation,
                ));
            }
        };
        Ok(invocation
            .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
            .and_then(|invocation| entry_point(registry, invocation)))
    }

    /// Binds and invokes this concrete method through a typed local
    /// `Pin<&mut T>` adapter.
    ///
    /// Named and positional arguments follow the same descriptor-aware rules
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// The receiver remains typed and pinned throughout binding and adapter
    /// validation.
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    /// - `T`: Exact concrete receiver type captured by the typed adapter.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Pinned mutable receiver and arguments to bind and
    ///   invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure retains the pinned receiver and original arguments
    /// when they were rejected before execution.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
    #[must_use = "handle dispatch errors to recover the original invocation inputs"]
    // Keep dispatch and validation recovery explicit for the exact pinned T and call lifetime.
    #[allow(clippy::type_complexity)]
    pub fn invoke_pinned_mut_local<'call, T: 'static>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::PinnedMutInvocation<'call, T, crate::value::Local>,
    ) -> crate::invoke::InvocationDispatchResult<
        crate::invoke::PinnedMutInvocation<'call, T, crate::value::Local>,
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedMutInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        let mode = InvocationDispatchMode::PinnedMutLocal;
        let adapter = match self.adapter {
            Some(adapter) => adapter,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::NoAdapter {
                        reasons: self.unavailable_reasons.clone(),
                    },
                    invocation,
                ));
            }
        };
        let entry_point = match adapter.pinned_mut_local {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::MissingEntry,
                    invocation,
                ));
            }
        };
        let entry_point = match entry_point.downcast_ref::<crate::invoke::PinnedMutAdapter<T, crate::value::Local>>() {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    InvocationDispatchReason::PinnedReceiverTypeMismatch,
                    invocation,
                ));
            }
        };
        Ok(invocation
            .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
            .and_then(|invocation| entry_point(registry, invocation)))
    }
}
