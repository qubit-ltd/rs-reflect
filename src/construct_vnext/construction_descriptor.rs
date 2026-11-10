// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]
#![allow(clippy::result_large_err)]
use super::ConstructionAvailability;
use super::ConstructionFailure;
use super::ConstructionFields;
use super::ConstructionInput;
use super::ConstructionInputMode;
use super::ConstructionMode;
use super::ConstructionShape;
use super::ConstructionTarget;
use super::RuntimeConstructionInput;
use super::UpdateFailure;
use super::UpdateInput;
use crate::runtime::construct as runtime;
use crate::value::DynamicOwned;
use crate::value::ValueFor;

/// Construction descriptor that always carries availability and field policy.
#[derive(Clone, Copy)]
pub struct ConstructionDescriptor<'ctx> {
    /// Runtime input storage that owns the values until construction succeeds
    /// or failure recovery.
    inner: runtime::ConstructionDescriptor<'ctx>,
    /// Public descriptor identifying the struct or selected variant.
    target: ConstructionTarget<'ctx>,
}

impl<'ctx> ConstructionDescriptor<'ctx> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// - `target`: Target type whose effective method metadata is being
    ///   resolved.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// - `target`: Target descriptor whose construction or method metadata is
    ///   queried.
    pub(crate) fn new(inner: runtime::ConstructionDescriptor<'ctx>, target: ConstructionTarget<'ctx>) -> Self {
        Self { inner, target }
    }

    /// Returns the selected struct or variant target.
    pub fn target(self) -> ConstructionTarget<'ctx> {
        self.target
    }

    /// Returns the required input shape.
    pub fn shape(self) -> ConstructionShape {
        self.inner.shape()
    }

    /// Returns construction availability, including structured unavailable
    /// reasons.
    pub fn availability(self) -> ConstructionAvailability {
        self.inner.availability()
    }

    /// Returns source-ordered field policy metadata.
    pub fn fields(&self) -> ConstructionFields<'_> {
        ConstructionFields::new(self.inner.fields())
    }

    /// Delegates typed construction to the runtime descriptor and preserves its
    /// recovery input. # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) fn construct_runtime<T, M>(
        self,
        input: RuntimeConstructionInput<M>,
    ) -> Result<T, runtime::ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: runtime::ConstructionMode,
    {
        self.inner.construct::<T, M, _>(input)
    }

    /// Delegates typed construction while bypassing the visibility policy.
    ///
    /// # Safety
    ///
    /// The caller must preserve invariants normally enforced by reflected field
    /// visibility. # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) unsafe fn construct_unchecked_runtime<T, M>(
        self,
        input: RuntimeConstructionInput<M>,
    ) -> Result<T, runtime::ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: runtime::ConstructionMode,
    {
        unsafe { self.inner.construct_unchecked::<T, M, _>(input) }
    }

    /// Delegates dynamic construction and preserves runtime validation
    /// failures. # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) fn construct_dynamic_runtime<M>(
        self,
        input: RuntimeConstructionInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::ConstructionFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        self.inner.construct_dynamic::<M, _>(input)
    }

    /// Delegates dynamic construction while bypassing the visibility policy.
    ///
    /// # Safety
    ///
    /// The caller must preserve invariants normally enforced by reflected field
    /// visibility. # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) unsafe fn construct_dynamic_unchecked_runtime<M>(
        self,
        input: RuntimeConstructionInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::ConstructionFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        unsafe { self.inner.construct_dynamic_unchecked::<M, _>(input) }
    }

    /// Delegates typed update and preserves runtime recovery data.
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) fn update_runtime<T, M>(self, input: UpdateInput<M>) -> Result<T, runtime::UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: runtime::ConstructionMode,
    {
        self.inner.update::<T, M>(input.inner)
    }

    /// Delegates typed update while bypassing the visibility policy.
    ///
    /// # Safety
    ///
    /// The caller must preserve invariants normally enforced by reflected field
    /// visibility. # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) unsafe fn update_unchecked_runtime<T, M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<T, runtime::UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: runtime::ConstructionMode,
    {
        unsafe { self.inner.update_unchecked::<T, M>(input.inner) }
    }

    /// Delegates dynamic update to the runtime descriptor.
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) fn update_dynamic_runtime<M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::UpdateFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        self.inner.update_dynamic(input.inner)
    }

    /// Delegates dynamic update while bypassing the visibility policy.
    ///
    /// # Safety
    ///
    /// The caller must preserve invariants normally enforced by reflected field
    /// visibility. # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub(super) unsafe fn update_dynamic_unchecked_runtime<M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::UpdateFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        unsafe { self.inner.update_dynamic_unchecked(input.inner) }
    }

    /// Constructs a typed value.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `I`: Input type carrying values for the operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub fn construct<T, I>(self, input: I) -> Result<T, ConstructionFailure<I::Mode>>
    where
        T: ValueFor<I::Mode>,
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        I::Mode::construct(self, input)
    }

    /// Constructs a typed value while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `I`: Input type carrying values for the operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted
    /// fields.
    pub unsafe fn construct_unchecked<T, I>(self, input: I) -> Result<T, ConstructionFailure<I::Mode>>
    where
        T: ValueFor<I::Mode>,
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        unsafe { I::Mode::construct_unchecked(self, input) }
    }

    /// Constructs a dynamically owned value.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `I`: Input type carrying values for the operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub fn construct_dynamic<I>(self, input: I) -> Result<DynamicOwned<I::Mode>, ConstructionFailure<I::Mode>>
    where
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        I::Mode::construct_dynamic(self, input)
    }

    /// Constructs a dynamic value while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// - `I`: Input type carrying values for the operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted
    /// fields.
    pub unsafe fn construct_dynamic_unchecked<I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<I::Mode>, ConstructionFailure<I::Mode>>
    where
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        unsafe { I::Mode::construct_dynamic_unchecked(self, input) }
    }

    /// Updates a typed owned struct value.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub fn update<T, M>(self, input: UpdateInput<M>) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        M::update(self, input)
    }

    /// Updates a dynamic owned struct value.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    pub fn update_dynamic<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        M::update_dynamic(self, input)
    }

    /// Updates a typed owned struct value while bypassing visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted
    /// fields.
    pub unsafe fn update_unchecked<T, M>(self, input: UpdateInput<M>) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        unsafe { M::update_unchecked(self, input) }
    }

    /// Updates a dynamic owned struct value while bypassing visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    ///
    /// # Parameters
    ///
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted
    /// fields.
    pub unsafe fn update_dynamic_unchecked<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        unsafe { M::update_dynamic_unchecked(self, input) }
    }
}
