// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Generated method invocation adapters and mode availability.

use std::any::Any;

/// A stable reason why a concrete method instance cannot be invoked
/// dynamically.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationUnavailableReason {
    /// The receiver form has no safe adapter.
    UnsupportedReceiver,
    /// A parameter pattern has no safe adapter.
    UnsupportedParameterPattern,
    /// The declaration is generic and has no registered specialization.
    UnspecializedGeneric,
    /// A concrete specialization was registered but has no safe generated
    /// invocation adapter.
    UnsupportedSpecialization,
    /// The declaration is unsafe.
    UnsafeMethod,
    /// The declared ABI has no safe adapter.
    UnsupportedAbi,
    /// Variadic invocation is not supported.
    Variadic,
    /// The return borrow cannot be related safely to an input borrow.
    UnsupportedBorrowedReturn,
    /// An opaque return value cannot cross the dynamic boundary.
    OpaqueReturn,
    /// An unsized value has no dedicated safe adapter.
    UnsupportedUnsizedValue,
    /// A default method has call-site bounds that reflection cannot prove.
    UnprovenDefaultConstraint,
    /// A default method depends on an associated type that is not proven at
    /// the declaration hook.
    UnprovenAssociatedType,
    /// A pinned receiver conflicts with the requested async, thread-safe,
    /// catching, or borrowed-output mode.
    PinnedModeConflict,
    /// Invocation was disabled by reflection policy.
    DisabledByPolicy,
}

/// Availability of an explicitly requested panic-catching invocation entry
/// point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatchingAvailability {
    /// No catching adapter was requested for this method.
    NotRequested,
    /// The explicitly requested catching adapter is callable.
    Available,
    /// Catching was requested but the binary uses abort-on-panic semantics.
    UnavailablePanicAbort,
}

/// Generated invocation entry points and their static mode availability.
///
/// This descriptor layer records adapter identity and availability. The
/// invocation layer owns argument validation and the complete call contract.
/// A present special-receiver entry still requires the matching capability in
/// the registry selected for invocation. Missing capabilities yield a failure
/// with recovery, not a missing entry. Ordinary entries propagate user panics;
/// explicitly requested catching entries capture only supported user panics.
#[derive(Clone, Copy, Debug)]
pub struct InvocationAdapter {
    entry_point: fn(),
    pub(super) local:
        Option<crate::invoke::InvocationAdapter<crate::value::Local>>,
    pub(super) thread_safe:
        Option<crate::invoke::InvocationAdapter<crate::value::ThreadSafe>>,
    pub(super) catching_local:
        Option<crate::invoke::CatchingInvocationAdapter<crate::value::Local>>,
    pub(super) catching_thread_safe: Option<
        crate::invoke::CatchingInvocationAdapter<crate::value::ThreadSafe>,
    >,
    catching_availability: CatchingAvailability,
    pub(super) pinned_ref_local: Option<&'static (dyn Any + Send + Sync)>,
    pub(super) pinned_mut_local: Option<&'static (dyn Any + Send + Sync)>,
}

impl InvocationAdapter {
    /// Creates an opaque adapter token for generated descriptor data.
    #[doc(hidden)]
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
    #[doc(hidden)]
    pub const fn local(
        entry_point: crate::invoke::InvocationAdapter<crate::value::Local>,
    ) -> Self {
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
    /// `Send` boundary required by [`ThreadSafe`](crate::value::ThreadSafe).
    #[doc(hidden)]
    pub const fn thread_safe(
        entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>,
    ) -> Self {
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
    #[doc(hidden)]
    pub const fn local_with_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::Local>,
        catching_entry_point: crate::invoke::CatchingInvocationAdapter<
            crate::value::Local,
        >,
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
    #[doc(hidden)]
    pub const fn thread_safe_with_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>,
        catching_entry_point: crate::invoke::CatchingInvocationAdapter<
            crate::value::ThreadSafe,
        >,
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
    #[doc(hidden)]
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
    #[doc(hidden)]
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
    #[doc(hidden)]
    pub const fn pinned_ref_local<T: 'static>(
        entry_point: &'static crate::invoke::PinnedRefAdapter<
            T,
            crate::value::Local,
        >,
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
    #[doc(hidden)]
    pub const fn pinned_mut_local<T: 'static>(
        entry_point: &'static crate::invoke::PinnedMutAdapter<
            T,
            crate::value::Local,
        >,
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
    #[doc(hidden)]
    #[must_use]
    #[inline(always)]
    pub const fn entry_point(&self) -> fn() {
        self.entry_point
    }

    /// Reports whether an explicitly requested panic-catching entry point is
    /// callable in this binary.
    #[must_use]
    #[inline(always)]
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
    /// Returns `None` for legacy descriptor-only entries and for adapters that
    /// are available exclusively in another invocation mode.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_local<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::InvocationFailure<'call, crate::value::Local>,
        >,
    > {
        self.local
            .map(|entry_point| entry_point(registry, invocation))
    }

    /// Invokes the thread-safe generated entry point when this descriptor has
    /// one.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_thread_safe`](crate::descriptor::MethodInstanceDescriptor::invoke_thread_safe)
    /// for named bindings.
    ///
    /// Returns `None` when this method was not explicitly generated with a
    /// thread-safe adapter.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_thread_safe<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::ThreadSafe>,
            crate::invoke::InvocationFailure<'call, crate::value::ThreadSafe>,
        >,
    > {
        self.thread_safe
            .map(|entry_point| entry_point(registry, invocation))
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
    #[must_use]
    pub fn invoke_catching_local<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<
        crate::invoke::CatchingInvocationResult<'call, crate::value::Local>,
    > {
        self.catching_local
            .map(|entry_point| entry_point(registry, invocation))
    }

    /// Invokes the explicit thread-safe catching entry point when one was
    /// generated.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_catching_thread_safe<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<
        crate::invoke::CatchingInvocationResult<
            'call,
            crate::value::ThreadSafe,
        >,
    > {
        self.catching_thread_safe
            .map(|entry_point| entry_point(registry, invocation))
    }

    /// Invokes a typed local `Pin<&T>` entry point when its exact receiver
    /// type matches this method's generated adapter.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_pinned_ref_local`](crate::descriptor::MethodInstanceDescriptor::invoke_pinned_ref_local)
    /// for named bindings.
    ///
    /// `None` means this method has no such adapter or `T` is not its exact
    /// receiver type. The `Err` case preserves the original pin and arguments.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_pinned_ref_local<'call, T: 'static>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::PinnedRefInvocation<
            'call,
            T,
            crate::value::Local,
        >,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedRefInvocationFailure<
                'call,
                T,
                crate::value::Local,
            >,
        >,
    > {
        self.pinned_ref_local
            .and_then(|entry_point| {
                entry_point.downcast_ref::<crate::invoke::PinnedRefAdapter<T, crate::value::Local>>()
            })
            .map(|entry_point| entry_point(registry, invocation))
    }

    /// Invokes a typed local `Pin<&mut T>` entry point when its exact receiver
    /// type matches this method's generated adapter.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_pinned_mut_local`](crate::descriptor::MethodInstanceDescriptor::invoke_pinned_mut_local)
    /// for named bindings.
    ///
    /// `None` means this method has no such adapter or `T` is not its exact
    /// receiver type. The `Err` case preserves the original pin and arguments.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_pinned_mut_local<'call, T: 'static>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::PinnedMutInvocation<
            'call,
            T,
            crate::value::Local,
        >,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedMutInvocationFailure<
                'call,
                T,
                crate::value::Local,
            >,
        >,
    > {
        self.pinned_mut_local
            .and_then(|entry_point| {
                entry_point.downcast_ref::<crate::invoke::PinnedMutAdapter<T, crate::value::Local>>()
            })
            .map(|entry_point| entry_point(registry, invocation))
    }
}

/// Serves as a stable opaque token for adapters whose real entry point is
/// typed.
fn unavailable_entry_point() {}
