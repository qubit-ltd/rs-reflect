// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Concrete method specializations and descriptor-aware invocation.

use std::fmt;

use super::InvocationAdapter;
use super::InvocationUnavailableReason;
use super::MethodDescriptor;
use crate::expression::GenericArgument;

/// The effective source of a concrete method instance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodImplementationSource {
    /// A method declared directly by an inherent impl.
    Declared,
    /// A required trait declaration has no implementation adapter.
    Required,
    /// The instance uses a trait default for the concrete target type.
    Defaulted,
    /// The implementation explicitly overrides the trait declaration.
    Overridden,
}

/// A concrete specialization of one method declaration.
///
/// Invoke with the same registry used for lookup. Descriptor-aware entries
/// bind named arguments before validating and adapting the receiver. A static
/// mode without an entry returns `None`; pre-execution errors return
/// `Some(Err(...))` with caller-ordered recovery. Outputs, futures, and
/// recovery retain input lifetimes without borrowing the selected registry.
#[derive(Clone, Debug)]
pub struct MethodInstanceDescriptor {
    declaration: &'static MethodDescriptor,
    implementation_method: Option<&'static MethodDescriptor>,
    implementation_source: MethodImplementationSource,
    adapter: Option<&'static InvocationAdapter>,
    arguments: Box<[GenericArgument]>,
    unavailable_reasons: Box<[InvocationUnavailableReason]>,
}

/// An inconsistent method implementation source or invocation capability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodInstanceBuildError {
    /// An inherent instance does not reference an impl-owned declaration.
    DeclaredMethodNotOwnedByImpl,
    /// A trait instance does not reference a trait-owned declaration.
    TraitMethodNotOwnedByTrait,
    /// A required method incorrectly advertises an invocation adapter.
    RequiredMethodHasAdapter,
    /// An overridden instance does not name its concrete impl method.
    OverriddenMethodMissingImplementation,
    /// A non-overridden instance names a concrete impl method.
    UnexpectedImplementationMethod,
    /// An available adapter and unavailable reasons were supplied together.
    AdapterHasUnavailableReasons,
    /// No adapter and no structured unavailable reason were supplied.
    UnavailableMethodMissingReasons,
}

impl fmt::Display for MethodInstanceBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeclaredMethodNotOwnedByImpl => {
                formatter.write_str("a declared inherent method must be owned by an impl")
            }
            Self::TraitMethodNotOwnedByTrait => formatter.write_str("a trait method instance must be owned by a trait"),
            Self::RequiredMethodHasAdapter => {
                formatter.write_str("a required method cannot have an invocation adapter")
            }
            Self::OverriddenMethodMissingImplementation => {
                formatter.write_str("an overridden method must name its impl method")
            }
            Self::UnexpectedImplementationMethod => {
                formatter.write_str("only an overridden method can name an impl method")
            }
            Self::AdapterHasUnavailableReasons => {
                formatter.write_str("an available invocation adapter cannot have unavailable reasons")
            }
            Self::UnavailableMethodMissingReasons => {
                formatter.write_str("an unavailable method must provide a structured reason")
            }
        }
    }
}

impl std::error::Error for MethodInstanceBuildError {}

impl MethodInstanceDescriptor {
    /// Creates a concrete method instance.
    ///
    /// `adapter` is present only when `unavailable_reasons` is empty and the
    /// later invocation layer supplied a safe entry point.
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

    /// Binds and invokes this concrete method through its thread-safe
    /// panic-catching adapter.
    ///
    /// Returns `None` when this instance has no explicitly generated
    /// thread-safe catching adapter.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_catching_thread_safe<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<crate::invoke::CatchingInvocationResult<'call, crate::value::ThreadSafe>> {
        let entry_point = self.adapter?.catching_thread_safe?;
        Some(
            match invocation.bind_arguments(self.effective_method().identity(), self.effective_method().parameters()) {
                Ok(invocation) => entry_point(registry, invocation),
                Err(failure) => Err(failure),
            },
        )
    }

    /// Creates a concrete method specialization with its generic arguments in
    /// declaration order.
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
    #[must_use]
    #[inline(always)]
    pub const fn declaration(&self) -> &'static MethodDescriptor {
        self.declaration
    }

    /// Returns the explicit impl method used by an overridden instance.
    ///
    /// `None` means the instance is required or uses its trait default.
    #[must_use]
    #[inline(always)]
    pub const fn implementation_method(&self) -> Option<&'static MethodDescriptor> {
        self.implementation_method
    }

    /// Returns the effective declaration or explicit implementation method.
    #[must_use]
    #[inline(always)]
    pub const fn effective_method(&self) -> &'static MethodDescriptor {
        match self.implementation_method {
            Some(method) => method,
            None => self.declaration,
        }
    }

    /// Returns whether the implementation is required, defaulted, or
    /// overridden.
    #[must_use]
    #[inline(always)]
    pub const fn implementation_source(&self) -> MethodImplementationSource {
        self.implementation_source
    }

    /// Returns the safe invocation adapter when one is available.
    ///
    /// `None` means callers must inspect [`Self::unavailable_reasons`].
    #[must_use]
    #[inline(always)]
    pub const fn adapter(&self) -> Option<&'static InvocationAdapter> {
        self.adapter
    }

    /// Returns the concrete type and const arguments of this method
    /// specialization in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns stable reasons that prevent dynamic invocation.
    #[must_use]
    #[inline(always)]
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
    /// Returns `None` when this instance has no local adapter. Otherwise the
    /// result contains either the invocation output or a structured
    /// pre-execution failure.
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
        let entry_point = self.adapter?.local?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(|invocation| entry_point(registry, invocation)),
        )
    }

    /// Binds and invokes this concrete method through its thread-safe adapter.
    ///
    /// Binding uses the same interleaved named/positional rules and complete
    /// pre-execution recovery contract as [`Self::invoke_local`].
    ///
    /// Returns `None` when this instance has no explicitly generated
    /// thread-safe adapter. Otherwise the result contains either the invocation
    /// output or a structured pre-execution failure.
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
        let entry_point = self.adapter?.thread_safe?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(|invocation| entry_point(registry, invocation)),
        )
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
    /// Returns `None` when this instance has no explicitly generated local
    /// catching adapter.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_catching_local<'call>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<crate::invoke::CatchingInvocationResult<'call, crate::value::Local>> {
        let entry_point = self.adapter?.catching_local?;
        Some(
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
    /// Returns `None` when this instance has no pinned shared adapter for the
    /// exact receiver type `T`.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_pinned_ref_local<'call, T: 'static>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::PinnedRefInvocation<'call, T, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedRefInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        let entry_point = self
            .adapter?
            .pinned_ref_local?
            .downcast_ref::<crate::invoke::PinnedRefAdapter<T, crate::value::Local>>()?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(|invocation| entry_point(registry, invocation)),
        )
    }

    /// Binds and invokes this concrete method through a typed local
    /// `Pin<&mut T>` adapter.
    ///
    /// Named and positional arguments follow the same descriptor-aware rules
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// The receiver remains typed and pinned throughout binding and adapter
    /// validation.
    ///
    /// Returns `None` when this instance has no pinned mutable adapter for the
    /// exact receiver type `T`.
    ///
    /// `registry` selects receiver capabilities without global fallback.
    /// Outputs, futures, and recovery borrow only invocation inputs, never
    /// the registry.
    #[must_use]
    pub fn invoke_pinned_mut_local<'call, T: 'static>(
        &self,
        registry: &crate::registry::ReflectRegistry,
        invocation: crate::invoke::PinnedMutInvocation<'call, T, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedMutInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        let entry_point = self
            .adapter?
            .pinned_mut_local?
            .downcast_ref::<crate::invoke::PinnedMutAdapter<T, crate::value::Local>>()?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(|invocation| entry_point(registry, invocation)),
        )
    }
}
