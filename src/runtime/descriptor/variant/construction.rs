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

    /// Constructs this enum variant into a typed owned value.
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

    /// Constructs this enum variant into a dynamic owned value.
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

    /// Constructs this variant while bypassing reflected visibility checks.
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

    /// Constructs this variant dynamically while bypassing visibility checks.
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
}
