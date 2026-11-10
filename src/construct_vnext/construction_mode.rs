// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]
#![allow(clippy::result_large_err)]
use super::ConstructionDescriptor;
use super::ConstructionFailure;
use super::ConstructionInput;
use super::ConstructionInputMode;
use super::UpdateFailure;
use super::UpdateInput;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ThreadSafe;
use crate::value::ValueFor;
use crate::value::ValueMode;

/// Value modes supported by the construction runtime.
#[doc(hidden)]
pub trait ConstructionMode: ValueMode {
    /// Constructs a typed output after validating the selected target and
    /// input.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type requested by the caller.
    /// - `I`: Sealed input shape and value collection.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction policy.
    /// - `input`: Owned values to validate and apply.
    ///
    /// # Returns
    ///
    /// Returns the constructed value or a failure retaining the input.
    #[doc(hidden)]
    fn construct<T, I>(descriptor: ConstructionDescriptor<'_>, input: I) -> Result<T, ConstructionFailure<Self>>
    where
        T: ValueFor<Self>,
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>;

    /// Constructs a typed output without enforcing reflected visibility policy.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete output type requested by the caller.
    /// - `I`: Sealed input shape and value collection.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction policy.
    /// - `input`: Owned values to validate and apply.
    ///
    /// # Returns
    ///
    /// Returns the constructed value or a failure retaining the input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by field visibility.
    #[doc(hidden)]
    unsafe fn construct_unchecked<T, I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<T, ConstructionFailure<Self>>
    where
        T: ValueFor<Self>,
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>;

    /// Constructs a dynamic owned output after validating target and input.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Sealed input shape and value collection.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction policy.
    /// - `input`: Owned values to validate and apply.
    ///
    /// # Returns
    ///
    /// Returns the dynamic value or a failure retaining the input.
    #[doc(hidden)]
    fn construct_dynamic<I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<DynamicOwned<Self>, ConstructionFailure<Self>>
    where
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>;

    /// Constructs a dynamic output without enforcing reflected visibility
    /// policy.
    ///
    /// # Type Parameters
    ///
    /// - `I`: Sealed input shape and value collection.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction policy.
    /// - `input`: Owned values to validate and apply.
    ///
    /// # Returns
    ///
    /// Returns the dynamic value or a failure retaining the input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by field visibility.
    #[doc(hidden)]
    unsafe fn construct_dynamic_unchecked<I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<DynamicOwned<Self>, ConstructionFailure<Self>>
    where
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>;

    /// Applies named replacements to an owned base and returns the typed
    /// result.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete updated output type.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and update policy.
    /// - `input`: Base value and owned replacements.
    ///
    /// # Returns
    ///
    /// Returns the updated value or a failure retaining the update input.
    #[doc(hidden)]
    fn update<T>(descriptor: ConstructionDescriptor<'_>, input: UpdateInput<Self>) -> Result<T, UpdateFailure<Self>>
    where
        T: ValueFor<Self>;

    /// Applies replacements while bypassing reflected visibility policy.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete updated output type.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and update policy.
    /// - `input`: Base value and owned replacements.
    ///
    /// # Returns
    ///
    /// Returns the updated value or a failure retaining the update input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by field visibility.
    #[doc(hidden)]
    unsafe fn update_unchecked<T>(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<T, UpdateFailure<Self>>
    where
        T: ValueFor<Self>;

    /// Applies replacements and returns the result as a dynamic owned value.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and update policy.
    /// - `input`: Base value and owned replacements.
    ///
    /// # Returns
    ///
    /// Returns the dynamic result or a failure retaining the update input.
    #[doc(hidden)]
    fn update_dynamic(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<DynamicOwned<Self>, UpdateFailure<Self>>;

    /// Applies replacements dynamically without enforcing visibility policy.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and update policy.
    /// - `input`: Base value and owned replacements.
    ///
    /// # Returns
    ///
    /// Returns the dynamic result or a failure retaining the update input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants normally enforced by field visibility.
    #[doc(hidden)]
    unsafe fn update_dynamic_unchecked(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<DynamicOwned<Self>, UpdateFailure<Self>>;
}

impl ConstructionMode for Local {
    /// Constructs a typed output through the runtime for this value mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn construct<T, I>(descriptor: ConstructionDescriptor<'_>, input: I) -> Result<T, ConstructionFailure<Self>>
    where
        T: ValueFor<Self>,
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        descriptor
            .construct_runtime::<T, Self>(input.__into_runtime())
            .map_err(ConstructionFailure::new)
    }
    /// Constructs a typed output while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn construct_unchecked<T, I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<T, ConstructionFailure<Self>>
    where
        T: ValueFor<Self>,
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        unsafe { descriptor.construct_unchecked_runtime::<T, Self>(input.__into_runtime()) }
            .map_err(ConstructionFailure::new)
    }
    /// Constructs a dynamic output through the runtime for this value mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn construct_dynamic<I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<DynamicOwned<Self>, ConstructionFailure<Self>>
    where
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        descriptor
            .construct_dynamic_runtime::<Self>(input.__into_runtime())
            .map_err(ConstructionFailure::new)
    }
    /// Constructs a dynamic output while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn construct_dynamic_unchecked<I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<DynamicOwned<Self>, ConstructionFailure<Self>>
    where
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        unsafe { descriptor.construct_dynamic_unchecked_runtime::<Self>(input.__into_runtime()) }
            .map_err(ConstructionFailure::new)
    }
    /// Applies named replacements to the owned base through the runtime.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn update<T>(descriptor: ConstructionDescriptor<'_>, input: UpdateInput<Self>) -> Result<T, UpdateFailure<Self>>
    where
        T: ValueFor<Self>,
    {
        descriptor.update_runtime::<T, Self>(input).map_err(UpdateFailure::new)
    }
    /// Applies replacements while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn update_unchecked<T>(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<T, UpdateFailure<Self>>
    where
        T: ValueFor<Self>,
    {
        unsafe { descriptor.update_unchecked_runtime::<T, Self>(input) }.map_err(UpdateFailure::new)
    }
    /// Applies named replacements and returns a dynamic owned value.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn update_dynamic(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<DynamicOwned<Self>, UpdateFailure<Self>> {
        descriptor
            .update_dynamic_runtime::<Self>(input)
            .map_err(UpdateFailure::new)
    }
    /// Applies replacements dynamically while bypassing visibility checks.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn update_dynamic_unchecked(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<DynamicOwned<Self>, UpdateFailure<Self>> {
        unsafe { descriptor.update_dynamic_unchecked_runtime::<Self>(input) }.map_err(UpdateFailure::new)
    }
}

impl ConstructionMode for ThreadSafe {
    /// Constructs a typed output through the runtime for this value mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn construct<T, I>(descriptor: ConstructionDescriptor<'_>, input: I) -> Result<T, ConstructionFailure<Self>>
    where
        T: ValueFor<Self>,
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        descriptor
            .construct_runtime::<T, Self>(input.__into_runtime())
            .map_err(ConstructionFailure::new)
    }
    /// Constructs a typed output while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn construct_unchecked<T, I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<T, ConstructionFailure<Self>>
    where
        T: ValueFor<Self>,
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        unsafe { descriptor.construct_unchecked_runtime::<T, Self>(input.__into_runtime()) }
            .map_err(ConstructionFailure::new)
    }
    /// Constructs a dynamic output through the runtime for this value mode.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn construct_dynamic<I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<DynamicOwned<Self>, ConstructionFailure<Self>>
    where
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        descriptor
            .construct_dynamic_runtime::<Self>(input.__into_runtime())
            .map_err(ConstructionFailure::new)
    }
    /// Constructs a dynamic output while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `I`: Input type carrying values for the operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn construct_dynamic_unchecked<I>(
        descriptor: ConstructionDescriptor<'_>,
        input: I,
    ) -> Result<DynamicOwned<Self>, ConstructionFailure<Self>>
    where
        I: ConstructionInputMode<Mode = Self> + ConstructionInput<Self>,
    {
        unsafe { descriptor.construct_dynamic_unchecked_runtime::<Self>(input.__into_runtime()) }
            .map_err(ConstructionFailure::new)
    }
    /// Applies named replacements to the owned base through the runtime.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn update<T>(descriptor: ConstructionDescriptor<'_>, input: UpdateInput<Self>) -> Result<T, UpdateFailure<Self>>
    where
        T: ValueFor<Self>,
    {
        descriptor.update_runtime::<T, Self>(input).map_err(UpdateFailure::new)
    }
    /// Applies replacements while bypassing reflected visibility checks.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and its construction or update policy.
    /// - `input`: Owned values supplied for validation and application.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn update_unchecked<T>(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<T, UpdateFailure<Self>>
    where
        T: ValueFor<Self>,
    {
        unsafe { descriptor.update_unchecked_runtime::<T, Self>(input) }.map_err(UpdateFailure::new)
    }
    /// Applies named replacements and returns a dynamic owned value.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    fn update_dynamic(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<DynamicOwned<Self>, UpdateFailure<Self>> {
        descriptor
            .update_dynamic_runtime::<Self>(input)
            .map_err(UpdateFailure::new)
    }
    /// Applies replacements dynamically while bypassing visibility checks.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: Target metadata and construction or update policy.
    /// - `input`: Owned input values retained for validation and recovery.
    ///
    /// # Returns
    ///
    /// Returns the value or a failure that retains the supplied input.
    ///
    /// # Safety
    ///
    /// The caller must uphold invariants protected by field visibility.
    unsafe fn update_dynamic_unchecked(
        descriptor: ConstructionDescriptor<'_>,
        input: UpdateInput<Self>,
    ) -> Result<DynamicOwned<Self>, UpdateFailure<Self>> {
        unsafe { descriptor.update_dynamic_unchecked_runtime::<Self>(input) }.map_err(UpdateFailure::new)
    }
}
