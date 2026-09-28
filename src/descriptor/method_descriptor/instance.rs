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
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodImplementationSource;
/// assert_eq!(MethodImplementationSource::Declared, MethodImplementationSource::Declared);
/// ```
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
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// {
/// use qubit_reflect::TypeDescriptor;
/// use qubit_reflect::descriptor::MethodLookup;
/// use qubit_reflect::registry::ReflectRegistry;
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
/// }
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

/// An inconsistent method implementation source or invocation capability.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MethodInstanceBuildError;
/// let error = MethodInstanceBuildError::RequiredMethodHasAdapter;
/// assert_eq!(error, MethodInstanceBuildError::RequiredMethodHasAdapter);
/// ```
#[must_use]
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination for the diagnostic message.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the message.
    ///
    /// # Errors
    ///
    /// Returns the formatter error if the destination rejects the message.
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

    /// Binds and invokes this concrete method through its thread-safe
    /// panic-catching adapter.
    ///
    /// Returns `None` when this instance has no explicitly generated
    /// thread-safe catching adapter.
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
    /// Returns `None` when no thread-safe catching adapter exists. Otherwise,
    /// returns `Some(Ok(Ok(output)))` on success, `Some(Ok(Err(panic)))` when
    /// the method panics, or `Some(Err(failure))` when validation fails.
    ///
    /// # Errors
    ///
    /// The outer error contains pre-execution binding or validation recovery;
    /// the inner error contains a panic captured after validation.
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
    /// Returns `None` when this instance has no local adapter. Otherwise the
    /// result contains either the invocation output or a structured
    /// pre-execution failure.
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
    /// Returns `None` when no local adapter exists, `Some(Ok(output))` on
    /// success, or `Some(Err(failure))` when binding, validation, or adapter
    /// execution reports a structured failure.
    ///
    /// # Errors
    ///
    /// The returned failure contains the original invocation inputs when
    /// execution could not safely consume them.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
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
    /// Returns `None` when no thread-safe adapter exists, `Some(Ok(output))`
    /// on success, or `Some(Err(failure))` when binding, validation, or adapter
    /// execution reports a structured failure.
    ///
    /// # Errors
    ///
    /// The returned failure contains the original invocation inputs when
    /// execution could not safely consume them.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
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
    /// Returns `None` when no local catching adapter exists. Otherwise,
    /// returns `Some(Ok(Ok(output)))` on success, `Some(Ok(Err(panic)))` when
    /// the method panics, or `Some(Err(failure))` when validation fails.
    ///
    /// # Errors
    ///
    /// The outer error contains pre-execution binding or validation recovery;
    /// the inner error contains a panic captured after validation.
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
    /// Returns `None` when no adapter exists for `T`; otherwise returns
    /// `Some(Ok(output))` on success or `Some(Err(failure))` when binding,
    /// validation, or execution fails.
    ///
    /// # Errors
    ///
    /// The returned failure retains the pinned receiver and original arguments
    /// when they were rejected before execution.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
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
    /// Returns `None` when no adapter exists for `T`; otherwise returns
    /// `Some(Ok(output))` on success or `Some(Err(failure))` when binding,
    /// validation, or execution fails.
    ///
    /// # Errors
    ///
    /// The returned failure retains the pinned receiver and original arguments
    /// when they were rejected before execution.
    ///
    /// # Panics
    ///
    /// Panics from the invoked Rust method propagate to the caller.
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
