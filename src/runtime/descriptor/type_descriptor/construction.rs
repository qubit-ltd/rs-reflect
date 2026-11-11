// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]
#![allow(clippy::result_large_err)]
use super::TypeDescriptor;
use crate::construct_vnext::ConstructionDescriptor;
use crate::construct_vnext::ConstructionFailure;
use crate::construct_vnext::ConstructionInput;
use crate::construct_vnext::ConstructionInputMode;
use crate::construct_vnext::ConstructionMode;
use crate::construct_vnext::ConstructionTarget;
use crate::construct_vnext::UpdateFailure;
use crate::construct_vnext::UpdateInput;
use crate::value::DynamicOwned;
use crate::value::ValueFor;

impl<'ctx> TypeDescriptor<'ctx> {
    /// Returns the construction plan and availability for this struct target.
    pub fn construction(self) -> ConstructionDescriptor<'ctx> {
        ConstructionDescriptor::new(self.construction_runtime(), ConstructionTarget::Struct(self))
    }

    /// Constructs this struct as a typed owned value after validating the
    /// target type, input shape, and field values.
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
        self.construction().construct(input)
    }

    /// Constructs this struct as a typed owned value while bypassing reflected
    /// field visibility checks. Target type and input validation still apply.
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
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility. This method does not bypass target type or input validation.
    pub unsafe fn construct_unchecked<T, I>(self, input: I) -> Result<T, ConstructionFailure<I::Mode>>
    where
        T: ValueFor<I::Mode>,
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        unsafe { self.construction().construct_unchecked(input) }
    }

    /// Constructs this struct as a dynamic owned value after validating its
    /// input shape and field values.
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
        self.construction().construct_dynamic(input)
    }

    /// Constructs this struct as a dynamic owned value while bypassing
    /// reflected field visibility checks. Input validation still applies.
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
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility. This method does not bypass input validation.
    pub unsafe fn construct_dynamic_unchecked<I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<I::Mode>, ConstructionFailure<I::Mode>>
    where
        I: ConstructionInputMode + ConstructionInput<I::Mode>,
    {
        unsafe { self.construction().construct_dynamic_unchecked(input) }
    }

    /// Updates an owned struct value after validating its type and every field
    /// replacement.
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
        self.construction().update(input)
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
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility. This method does not bypass type or replacement validation.
    pub unsafe fn update_unchecked<T, M>(self, input: UpdateInput<M>) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        unsafe { self.construction().update_unchecked(input) }
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
        self.construction().update_dynamic(input)
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
    /// The caller must uphold invariants normally enforced by reflected field
    /// visibility. This method does not bypass type or replacement validation.
    pub unsafe fn update_dynamic_unchecked<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        unsafe { self.construction().update_dynamic_unchecked(input) }
    }
}
