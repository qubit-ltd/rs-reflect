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
    /// Wraps a runtime construction descriptor with its public target.
    ///
    /// # Parameters
    ///
    /// - `inner`: Runtime descriptor that performs construction and update
    ///   operations.
    /// - `target`: Struct or variant descriptor identifying the construction
    ///   target.
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

    /// Delegates typed construction to the runtime descriptor, retaining input
    /// on failure.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by the target.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Runtime construction input to validate and consume.
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

    /// Delegates typed construction while bypassing reflected field visibility
    /// checks. Target type and input validation still apply.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by the target.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Runtime construction input to validate and consume.
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

    /// Delegates dynamic construction and retains runtime validation input on
    /// failure.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Runtime construction input to validate and consume.
    pub(super) fn construct_dynamic_runtime<M>(
        self,
        input: RuntimeConstructionInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::ConstructionFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        self.inner.construct_dynamic::<M, _>(input)
    }

    /// Delegates dynamic construction while bypassing reflected field
    /// visibility checks. Input validation still applies.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Runtime construction input to validate and consume.
    pub(super) unsafe fn construct_dynamic_unchecked_runtime<M>(
        self,
        input: RuntimeConstructionInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::ConstructionFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        unsafe { self.inner.construct_dynamic_unchecked::<M, _>(input) }
    }

    /// Delegates typed update and retains the original value and replacements
    /// on failure.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by the target.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Original value and requested field replacements.
    pub(super) fn update_runtime<T, M>(self, input: UpdateInput<M>) -> Result<T, runtime::UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: runtime::ConstructionMode,
    {
        self.inner.update::<T, M>(input.inner)
    }

    /// Delegates typed update while bypassing reflected field visibility
    /// checks. Type and replacement validation still apply.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by the target.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Original value and requested field replacements.
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

    /// Delegates dynamic update and retains the original value and replacements
    /// on failure.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Original value and requested field replacements.
    pub(super) fn update_dynamic_runtime<M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::UpdateFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        self.inner.update_dynamic(input.inner)
    }

    /// Delegates dynamic update while bypassing reflected field visibility
    /// checks. Type and replacement validation still apply.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: Original value and requested field replacements.
    pub(super) unsafe fn update_dynamic_unchecked_runtime<M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, runtime::UpdateFailure<M>>
    where
        M: runtime::ConstructionMode,
    {
        unsafe { self.inner.update_dynamic_unchecked(input.inner) }
    }

    /// Constructs a typed value after validating the target type, input shape,
    /// and field values.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by this descriptor.
    /// - `I`: Construction input whose associated mode selects value storage.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned construction fields to validate and consume. On
    ///   failure, the error retains the input for recovery.
    pub fn construct<T, I>(self, input: I) -> Result<T, ConstructionFailure<I::Mode>>
    where
        T: ValueFor<I::Mode>,
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        I::Mode::construct(self, input)
    }

    /// Constructs a typed value while bypassing reflected field visibility
    /// checks. Target type and input validation still apply.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by this descriptor.
    /// - `I`: Construction input whose associated mode selects value storage.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned construction fields to validate and consume. On
    ///   failure, the error retains the input for recovery.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    pub unsafe fn construct_unchecked<T, I>(self, input: I) -> Result<T, ConstructionFailure<I::Mode>>
    where
        T: ValueFor<I::Mode>,
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        unsafe { I::Mode::construct_unchecked(self, input) }
    }

    /// Constructs a dynamic owned value after validating the input shape and
    /// field values.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Construction input whose associated mode selects value storage.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned construction fields to validate and consume. On
    ///   failure, the error retains the input for recovery.
    pub fn construct_dynamic<I>(self, input: I) -> Result<DynamicOwned<I::Mode>, ConstructionFailure<I::Mode>>
    where
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        I::Mode::construct_dynamic(self, input)
    }

    /// Constructs a dynamic owned value while bypassing reflected field
    /// visibility checks. Input validation still applies.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Construction input whose associated mode selects value storage.
    ///
    /// # Parameters
    ///
    /// - `input`: Owned construction fields to validate and consume. On
    ///   failure, the error retains the input for recovery.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    pub unsafe fn construct_dynamic_unchecked<I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<I::Mode>, ConstructionFailure<I::Mode>>
    where
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        unsafe { I::Mode::construct_dynamic_unchecked(self, input) }
    }

    /// Updates a typed owned struct value after validating its type and field
    /// replacements.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by this descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: The owned struct and field replacements to validate. On
    ///   failure, the error retains both for recovery.
    pub fn update<T, M>(self, input: UpdateInput<M>) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        M::update(self, input)
    }

    /// Updates a dynamic owned struct value after validating its type and field
    /// replacements.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: The owned struct and field replacements to validate. On
    ///   failure, the error retains both for recovery.
    pub fn update_dynamic<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        M::update_dynamic(self, input)
    }

    /// Updates a typed owned struct value while bypassing reflected field
    /// visibility checks. Type and replacement validation still apply.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type represented by this descriptor.
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: The owned struct and field replacements to validate. On
    ///   failure, the error retains both for recovery.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    pub unsafe fn update_unchecked<T, M>(self, input: UpdateInput<M>) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        unsafe { M::update_unchecked(self, input) }
    }

    /// Updates a dynamic owned struct value while bypassing reflected field
    /// visibility checks. Type and replacement validation still apply.
    ///
    /// # Type Parameters
    ///
    /// - `M`: Value mode that governs storage and thread-safety bounds.
    ///
    /// # Parameters
    ///
    /// - `input`: The owned struct and field replacements to validate. On
    ///   failure, the error retains both for recovery.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility.
    pub unsafe fn update_dynamic_unchecked<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        unsafe { M::update_dynamic_unchecked(self, input) }
    }
}
