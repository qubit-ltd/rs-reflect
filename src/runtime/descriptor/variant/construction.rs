// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]
#![allow(clippy::result_large_err)]
use super::VariantDescriptor;
use crate::construct_vnext::ConstructionDescriptor;
use crate::construct_vnext::ConstructionFailure;
use crate::construct_vnext::ConstructionInput;
use crate::construct_vnext::ConstructionInputMode;
use crate::construct_vnext::ConstructionTarget;
use crate::value::DynamicOwned;
use crate::value::ValueFor;

impl<'ctx> VariantDescriptor<'ctx> {
    /// Returns the construction plan and availability for this selected
    /// variant.
    pub fn construction(self) -> ConstructionDescriptor<'ctx> {
        ConstructionDescriptor::new(self.construction_runtime(), ConstructionTarget::Variant(self))
    }
    /// Constructs this enum variant as a typed owned value after validating its
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
    /// Constructs this enum variant as a dynamic owned value after validating
    /// its input shape and field values.
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
    /// Constructs this enum variant as a typed owned value while bypassing
    /// reflected field visibility checks. Target type and input validation
    /// still apply.
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
        unsafe { self.construction().construct_unchecked(input) }
    }
    /// Constructs this enum variant as a dynamic owned value while bypassing
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
        unsafe { self.construction().construct_dynamic_unchecked(input) }
    }
}
