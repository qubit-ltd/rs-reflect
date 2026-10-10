// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Generated method invocation adapter entries.

use std::any::Any;

use super::catching_availability::CatchingAvailability;
use crate::invoke::InvocationDispatchMode;
use crate::invoke::InvocationDispatchReason;
use crate::invoke::InvocationUnavailable;

/// Generated invocation entry points and their static mode availability.
///
/// This descriptor layer records adapter identity and availability. The
/// invocation layer owns argument validation and the complete call contract.
/// A present special-receiver entry still requires the matching capability in
/// the registry selected for invocation. Missing capabilities yield a failure
/// with recovery, not a missing entry. Ordinary entries propagate user panics;
/// explicitly requested catching entries capture only supported user panics.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::{CatchingAvailability, InvocationAdapter};
///
/// fn opaque_entry() {}
/// let adapter = InvocationAdapter::new(opaque_entry);
/// assert_eq!(adapter.catching_availability(), CatchingAvailability::NotRequested);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct InvocationAdapter {
    /// Opaque identity retained by the method declaration.
    entry_point: fn(),
    /// Local dynamic-value invocation entry point, when generated.
    pub(crate) local: Option<crate::invoke::InvocationAdapter<crate::value::Local>>,
    /// Thread-safe dynamic-value invocation entry point, when generated.
    pub(crate) thread_safe: Option<crate::invoke::InvocationAdapter<crate::value::ThreadSafe>>,
    /// Local panic-catching entry point, when explicitly requested.
    pub(crate) catching_local: Option<crate::invoke::CatchingInvocationAdapter<crate::value::Local>>,
    /// Thread-safe panic-catching entry point, when explicitly requested.
    pub(crate) catching_thread_safe: Option<crate::invoke::CatchingInvocationAdapter<crate::value::ThreadSafe>>,
    /// Availability classification for a requested catching entry point.
    catching_availability: CatchingAvailability,
    /// Typed local pinned shared-receiver entry point, when generated.
    pub(crate) pinned_ref_local: Option<&'static (dyn Any + Send + Sync)>,
    /// Typed local pinned mutable-receiver entry point, when generated.
    pub(crate) pinned_mut_local: Option<&'static (dyn Any + Send + Sync)>,
}

impl InvocationAdapter {
    /// Creates an opaque adapter token for generated descriptor data.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Stable function identity retained by the descriptor.
    ///
    /// # Returns
    ///
    /// Returns a token with no callable invocation modes.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(entry_point: fn()) -> Self {
        Self {
            entry_point,
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for one callable local invocation entry point.
    ///
    /// The generated function is higher-ranked over the invocation lifetime,
    /// so it cannot extend erased input borrows beyond one invocation.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Generated local invocation function.
    ///
    /// # Returns
    ///
    /// Returns an adapter descriptor with a local entry point.
    #[doc(hidden)]
    #[must_use]
    pub const fn local(entry_point: crate::invoke::InvocationAdapter<crate::value::Local>) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: Some(entry_point),
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for one callable thread-safe invocation entry
    /// point.
    ///
    /// The entry point's type preserves both the call lifetime and the runtime
    /// `Send` boundary required by [`crate::value::ThreadSafe`].
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Generated thread-safe invocation function.
    ///
    /// # Returns
    ///
    /// Returns an adapter descriptor with a thread-safe entry point.
    #[doc(hidden)]
    #[must_use]
    pub const fn thread_safe(entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: Some(entry_point),
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a local adapter paired with an explicitly generated catching
    /// entry point.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Generated local invocation function.
    /// - `catching_entry_point`: Generated local panic-catching function.
    ///
    /// # Returns
    ///
    /// Returns an adapter descriptor with both local entry points available.
    #[doc(hidden)]
    #[must_use]
    pub const fn local_with_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::Local>,
        catching_entry_point: crate::invoke::CatchingInvocationAdapter<crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: Some(entry_point),
            thread_safe: None,
            catching_local: Some(catching_entry_point),
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::Available,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a thread-safe adapter paired with an explicitly generated
    /// catching entry point.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Generated thread-safe invocation function.
    /// - `catching_entry_point`: Generated thread-safe panic-catching function.
    ///
    /// # Returns
    ///
    /// Returns an adapter descriptor with both thread-safe entry points.
    #[doc(hidden)]
    #[must_use]
    pub const fn thread_safe_with_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>,
        catching_entry_point: crate::invoke::CatchingInvocationAdapter<crate::value::ThreadSafe>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: Some(entry_point),
            catching_local: None,
            catching_thread_safe: Some(catching_entry_point),
            catching_availability: CatchingAvailability::Available,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a local adapter whose requested catching entry point is
    /// unavailable because this binary aborts on panic.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Generated local invocation function.
    ///
    /// # Returns
    ///
    /// Returns a local adapter marked as unavailable for panic catching.
    #[doc(hidden)]
    #[must_use]
    pub const fn local_with_unavailable_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: Some(entry_point),
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::UnavailablePanicAbort,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a thread-safe adapter whose requested catching entry point is
    /// unavailable because this binary aborts on panic.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Generated thread-safe invocation function.
    ///
    /// # Returns
    ///
    /// Returns a thread-safe adapter marked as unavailable for panic catching.
    #[doc(hidden)]
    #[must_use]
    pub const fn thread_safe_with_unavailable_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: Some(entry_point),
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::UnavailablePanicAbort,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for a typed local `Pin<&T>` entry point.
    ///
    /// The concrete adapter remains behind `Any` only for descriptor storage;
    /// [`Self::invoke_pinned_ref_local`] downcasts it by the caller's `T`
    /// without erasing or reconstructing the pin proof.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact concrete receiver type accepted by the pinned adapter.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Static adapter specialized for `Pin<&T>`.
    ///
    /// # Returns
    ///
    /// Returns an adapter descriptor with the typed pinned entry point.
    #[doc(hidden)]
    #[must_use]
    pub const fn pinned_ref_local<T: 'static>(
        entry_point: &'static crate::invoke::PinnedRefAdapter<T, crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: Some(entry_point),
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for a typed local `Pin<&mut T>` entry point.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Exact concrete receiver type accepted by the pinned adapter.
    ///
    /// # Parameters
    ///
    /// - `entry_point`: Static adapter specialized for `Pin<&mut T>`.
    ///
    /// # Returns
    ///
    /// Returns an adapter descriptor with the typed pinned entry point.
    #[doc(hidden)]
    #[must_use]
    pub const fn pinned_mut_local<T: 'static>(
        entry_point: &'static crate::invoke::PinnedMutAdapter<T, crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: Some(entry_point),
        }
    }

    /// Returns the opaque entry-point identity.
    ///
    /// # Returns
    ///
    /// Returns the function pointer retained as this adapter's identity.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn entry_point(&self) -> fn() {
        self.entry_point
    }

    /// Reports whether an explicitly requested panic-catching entry point is
    /// callable in this binary.
    ///
    /// # Returns
    ///
    /// Returns whether catching was not requested, is available, or is
    /// unavailable under abort-on-panic semantics.
    #[must_use]
    #[inline]
    pub const fn catching_availability(&self) -> CatchingAvailability {
        self.catching_availability
    }

    /// Invokes the local generated entry point when this descriptor has one.
    ///
    /// This raw adapter entry point accepts positional inputs only. Named
    /// bindings return `NamedBindingRequiresDescriptor`; use
    /// [`MethodInstanceDescriptor::invoke_local`](crate::descriptor::MethodInstanceDescriptor::invoke_local)
    /// for descriptor-aware binding.
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
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Local receiver and positional arguments to invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure retains caller-owned inputs rejected before they
    /// are consumed.
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
        let adapter = self;
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
        Ok(entry_point(registry, invocation))
    }

    /// Invokes the thread-safe generated entry point when this descriptor has
    /// one.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_thread_safe`](crate::descriptor::MethodInstanceDescriptor::invoke_thread_safe)
    /// for named bindings.
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
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Thread-safe receiver and positional arguments to invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure retains caller-owned inputs rejected before they
    /// are consumed.
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
        let adapter = self;
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
        Ok(entry_point(registry, invocation))
    }

    /// Invokes the explicit local catching entry point when one was generated.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_catching_local`](crate::descriptor::MethodInstanceDescriptor::invoke_catching_local)
    /// for named bindings.
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
    /// - `invocation`: Local receiver and positional arguments to invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable. Otherwise, the dispatched result reports
    /// validation failure and its inner result distinguishes a normal
    /// output from a caught panic.
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
        let adapter = self;
        let entry_point = match adapter.catching_local {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    if adapter.local.is_none() {
                        InvocationDispatchReason::MissingEntry
                    } else {
                        match adapter.catching_availability {
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
        Ok(entry_point(registry, invocation))
    }

    /// Invokes the explicit thread-safe catching entry point when one was
    /// generated.
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
    /// - `invocation`: Thread-safe receiver and positional arguments to invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable. Otherwise, the outer result reports validation
    /// failure and the inner result distinguishes a normal output from a
    /// caught panic.
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
        let adapter = self;
        let entry_point = match adapter.catching_thread_safe {
            Some(entry_point) => entry_point,
            None => {
                return Err(InvocationUnavailable::new(
                    mode,
                    if adapter.thread_safe.is_none() {
                        InvocationDispatchReason::MissingEntry
                    } else {
                        match adapter.catching_availability {
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
        Ok(entry_point(registry, invocation))
    }

    /// Invokes a typed local `Pin<&T>` entry point when its exact receiver
    /// type matches this method's generated adapter.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_pinned_ref_local`](crate::descriptor::MethodInstanceDescriptor::invoke_pinned_ref_local)
    /// for named bindings.
    ///
    /// Dispatch errors retain the original pin and arguments when the slot is
    /// missing or its exact receiver type differs from `T`.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    /// - `T`: Exact concrete receiver type accepted by the pinned adapter.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Pinned shared receiver and positional arguments to
    ///   invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure retains the original pin and arguments when they
    /// are rejected before execution.
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
        let adapter = self;
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
        Ok(entry_point(registry, invocation))
    }

    /// Invokes a typed local `Pin<&mut T>` entry point when its exact receiver
    /// type matches this method's generated adapter.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_pinned_mut_local`](crate::descriptor::MethodInstanceDescriptor::invoke_pinned_mut_local)
    /// for named bindings.
    ///
    /// Dispatch errors retain the original pin and arguments when the slot is
    /// missing or its exact receiver type differs from `T`.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    ///
    /// # Type Parameters
    ///
    /// - `'call`: Lifetime of the receiver and argument borrows.
    /// - `T`: Exact concrete receiver type accepted by the pinned adapter.
    ///
    /// # Parameters
    ///
    /// - `registry`: Immutable snapshot used to resolve receiver capabilities.
    /// - `invocation`: Pinned mutable receiver and positional arguments to
    ///   invoke.
    ///
    /// # Returns
    ///
    /// Returns an outer `Err` retaining the original input when the requested
    /// entry is unavailable.
    ///
    /// # Errors
    ///
    /// The returned failure retains the original pin and arguments when they
    /// are rejected before execution.
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
        let adapter = self;
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
        Ok(entry_point(registry, invocation))
    }
}

/// Serves as a stable opaque token for adapters whose real entry point is
/// typed.
///
/// This placeholder is stored only when a typed adapter is the callable entry;
/// invoking it directly is not supported.
fn unavailable_entry_point() {}
