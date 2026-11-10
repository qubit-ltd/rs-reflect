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

    /// Constructs this struct into a typed owned value.
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
        self.construction().construct(input)
    }

    /// Constructs this struct while bypassing reflected visibility checks.
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
        unsafe { self.construction().construct_unchecked(input) }
    }

    /// Constructs this struct into a dynamic owned value.
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
        self.construction().construct_dynamic(input)
    }

    /// Constructs this struct into a dynamic value while bypassing visibility
    /// checks.
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
        unsafe { self.construction().construct_dynamic_unchecked(input) }
    }

    /// Updates an owned struct value after validating every replacement.
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
        self.construction().update(input)
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
        unsafe { self.construction().update_unchecked(input) }
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
        self.construction().update_dynamic(input)
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
        unsafe { self.construction().update_dynamic_unchecked(input) }
    }
}
