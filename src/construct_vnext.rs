// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Public construction and owned-update facade over the crate-private runtime.

#![allow(unsafe_code)]
// Construction and update failures retain the original owned inputs so callers
// can recover them without cloning or losing values.
#![allow(clippy::result_large_err)]

use std::fmt;

use runtime::NamedFields as RuntimeNamedFields;
use runtime::PositionalFields as RuntimePositionalFields;
use runtime::UnitFields as RuntimeUnitFields;
use runtime::UpdateInput as RuntimeUpdateInput;

pub use crate::__private::codegen_v4::construct::ConstructField;
pub use crate::__private::codegen_v4::construct::ConstructionAvailability;
pub use crate::__private::codegen_v4::construct::ConstructionFieldPolicy;
pub use crate::__private::codegen_v4::construct::ConstructionPlan;
pub use crate::__private::codegen_v4::construct::ConstructionProviders;
pub use crate::__private::codegen_v4::construct::ConstructionShape;
pub use crate::__private::codegen_v4::construct::ConstructionUnavailableReason;
pub use crate::__private::codegen_v4::construct::UpdateFieldPolicy;
pub use crate::__private::codegen_v4::construct::UpdateUnavailableReason;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::VariantDescriptor;
use crate::runtime::construct as runtime;
use crate::runtime::error::RecoverableFailure;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::ThreadSafe;
use crate::value::ValueFor;
use crate::value::ValueMode;

/// The reflected target selected for a construction operation.
#[derive(Clone, Copy)]
pub enum ConstructionTarget<'ctx> {
    /// A reflected struct or tuple struct.
    Struct(
        /// Descriptor whose type is the construction target.
        TypeDescriptor<'ctx>,
    ),
    /// An explicitly selected enum variant.
    Variant(
        /// Descriptor of the variant selected for construction.
        VariantDescriptor<'ctx>,
    ),
}

/// A view of the declaration-order fields used by construction and update.
#[derive(Clone, Copy)]
pub struct ConstructionFields<'a> {
    /// Field metadata borrowed from the target's construction plan.
    fields: &'a [ConstructField],
}

impl<'a> ConstructionFields<'a> {
    /// Creates a field view over metadata owned by a construction descriptor.
    ///
    /// # Parameters
    ///
    /// - `fields`: Field metadata exposed by the target descriptor in
    ///   declaration order.
    fn new(fields: &'a [ConstructField]) -> Self {
        Self { fields }
    }

    /// Returns the number of declared construction fields.
    pub fn len(self) -> usize {
        self.fields.len()
    }

    /// Returns whether the target has no fields.
    pub fn is_empty(self) -> bool {
        self.fields.is_empty()
    }

    /// Returns field metadata at a declaration-order index without allocation.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based declaration-order field index.
    ///
    /// # Returns
    ///
    /// Returns `Some` with the borrowed field metadata when `index` is in
    /// range, or `None` when it is outside the target's fields.
    pub fn get(self, index: usize) -> Option<&'a ConstructField> {
        self.fields.get(index)
    }

    /// Iterates over fields in declaration order.
    pub fn iter(self) -> std::slice::Iter<'a, ConstructField> {
        self.fields.iter()
    }
}

/// Named values supplied to struct-like construction or update.
///
/// Values remain owned by this builder until construction succeeds or a
/// failure returns them for recovery.
///
/// # Type Parameters
///
/// - `M`: Value storage mode, local by default.
pub struct NamedFields<M: ValueMode = Local> {
    /// Runtime collection that retains the named values in insertion order.
    inner: RuntimeNamedFields<M>,
}

impl<M: ValueMode> NamedFields<M> {
    /// Creates an empty named input.
    pub fn new() -> Self {
        Self {
            inner: RuntimeNamedFields::new(),
        }
    }

    /// Adds a typed value under `name`; validation is deferred until
    /// construction.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type accepted by the selected value mode.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for the construction attempt.
    ///
    /// # Returns
    ///
    /// Returns this builder with the named value appended.
    pub fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.inner = self.inner.field(name, value);
        self
    }

    /// Adds a dynamically typed owned value under `name`.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Dynamic value retained for validation and construction.
    ///
    /// # Returns
    ///
    /// Returns this builder with the named value appended.
    pub fn field_dynamic(mut self, name: impl Into<Box<str>>, value: DynamicOwned<M>) -> Self {
        self.inner = self.inner.field_dynamic(name, value);
        self
    }
}

impl<M: ValueMode> Default for NamedFields<M> {
    /// Creates an empty field input that can be populated with builder methods.
    fn default() -> Self {
        Self::new()
    }
}

/// Positional values supplied to tuple-like construction.
///
/// Values are retained in insertion order and matched to fields by position.
///
/// # Type Parameters
///
/// - `M`: Value storage mode, local by default.
pub struct PositionalFields<M: ValueMode = Local> {
    /// Runtime collection that retains positional values in insertion order.
    inner: RuntimePositionalFields<M>,
}

impl<M: ValueMode> PositionalFields<M> {
    /// Creates an empty positional input.
    pub fn new() -> Self {
        Self {
            inner: RuntimePositionalFields::new(),
        }
    }

    /// Appends a typed value at the next positional field index.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete value type accepted by the selected value mode.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned positional value retained for construction.
    ///
    /// # Returns
    ///
    /// Returns this builder with the value appended.
    pub fn value<T: ValueFor<M>>(mut self, value: T) -> Self {
        self.inner = self.inner.value(value);
        self
    }

    /// Appends a dynamically typed value at the next positional field index.
    ///
    /// # Parameters
    ///
    /// - `value`: Dynamic value retained for validation and construction.
    ///
    /// # Returns
    ///
    /// Returns this builder with the value appended.
    pub fn value_dynamic(mut self, value: DynamicOwned<M>) -> Self {
        self.inner = self.inner.value_dynamic(value);
        self
    }
}

impl<M: ValueMode> Default for PositionalFields<M> {
    /// Creates an empty field input that can be populated with builder methods.
    fn default() -> Self {
        Self::new()
    }
}

/// Empty input for unit structs and unit enum variants.
#[derive(Default)]
pub struct UnitFields;

impl UnitFields {
    /// Creates a unit input.
    pub const fn new() -> Self {
        Self
    }
}

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

/// The value mode associated with a sealed construction input type.
#[doc(hidden)]
pub trait ConstructionInputMode {
    /// Storage mode used by the input values.
    type Mode: ConstructionMode;
}

impl<M: ConstructionMode> ConstructionInputMode for NamedFields<M> {
    /// Value mode used to store this input’s values.
    type Mode = M;
}
impl<M: ConstructionMode> ConstructionInputMode for PositionalFields<M> {
    /// Value mode used to store this input’s values.
    type Mode = M;
}
impl ConstructionInputMode for UnitFields {
    /// Value mode used to store this input’s values.
    type Mode = Local;
}

/// Opaque bridge used by the public input contract to reach runtime validation.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[doc(hidden)]
pub struct RuntimeConstructionInput<M: ValueMode> {
    /// Runtime input storage that owns the values until construction succeeds
    /// or failure recovery.
    inner: runtime::input::ConstructionInputValue<M>,
}

impl<M: ValueMode> RuntimeConstructionInput<M> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    fn new(inner: runtime::input::ConstructionInputValue<M>) -> Self {
        Self { inner }
    }
}

impl<M: runtime::ConstructionMode> runtime::input::sealed::Sealed<M> for RuntimeConstructionInput<M> {
    /// Transfers the retained input values into the construction runtime.
    fn into_input(self) -> runtime::input::ConstructionInputValue<M> {
        self.inner
    }
}
impl<M: runtime::ConstructionMode> runtime::ConstructionInput<M> for RuntimeConstructionInput<M> {}

/// Input shape accepted by construction operations.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[doc(hidden)]
pub trait ConstructionInput<M: ConstructionMode> {
    /// Returns the shape of this input.
    fn shape(&self) -> ConstructionShape;

    /// Moves this public input into the sealed runtime validation carrier.
    #[doc(hidden)]
    fn __into_runtime(self) -> RuntimeConstructionInput<M>;
}

impl<M: ConstructionMode> ConstructionInput<M> for NamedFields<M> {
    /// Returns the input layout used to match values with target fields.
    fn shape(&self) -> ConstructionShape {
        ConstructionShape::Named
    }
    /// Moves this public input into the sealed runtime validation carrier.
    fn __into_runtime(self) -> RuntimeConstructionInput<M> {
        RuntimeConstructionInput::new(self.inner.into_input())
    }
}
impl<M: ConstructionMode> ConstructionInput<M> for PositionalFields<M> {
    /// Returns the input layout used to match values with target fields.
    fn shape(&self) -> ConstructionShape {
        ConstructionShape::Positional
    }
    /// Moves this public input into the sealed runtime validation carrier.
    fn __into_runtime(self) -> RuntimeConstructionInput<M> {
        RuntimeConstructionInput::new(self.inner.into_input())
    }
}
impl<M: ConstructionMode> ConstructionInput<M> for UnitFields {
    /// Returns the input layout used to match values with target fields.
    fn shape(&self) -> ConstructionShape {
        ConstructionShape::Unit
    }
    /// Moves this public input into the sealed runtime validation carrier.
    fn __into_runtime(self) -> RuntimeConstructionInput<M> {
        RuntimeConstructionInput::new(RuntimeUnitFields::new().into_input())
    }
}

/// Public construction error categories and details.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionError {
    /// The selected target has no registered construction provider.
    MissingProvider,
    /// More than one provider matches the target.
    AmbiguousProvider {
        /// Rust type name of the ambiguous target.
        target: &'static str,
        /// Number of matching providers.
        count: usize,
    },
    /// Construction was requested for a type or descriptor that is not
    /// constructible.
    WrongTargetKind,
    /// The selected target is unavailable for the reported reasons.
    Unavailable(
        /// Structured reasons this target cannot be constructed.
        ConstructionAvailability,
    ),
    /// The requested output type does not match the provider target type.
    TargetTypeMismatch(
        /// Expected target type and type supplied by the caller.
        crate::error::TypeMismatch,
    ),
    /// The input layout does not match the target's construction shape.
    ShapeMismatch {
        /// Shape required by the target.
        expected: ConstructionShape,
        /// Shape supplied by the caller.
        actual: ConstructionShape,
    },
    /// A required field value was omitted.
    MissingField {
        /// Name of the missing field.
        field: &'static str,
    },
    /// A named field was supplied more than once.
    DuplicateField {
        /// Name of the duplicated field.
        field: &'static str,
    },
    /// The input names a field that is not part of the target.
    UnknownField {
        /// Name supplied by the caller.
        field: Box<str>,
    },
    /// The caller supplied a field that only its provider may initialize.
    ProviderOnlyField {
        /// Name of the provider-only field.
        field: &'static str,
    },
    /// The selected field is unavailable for the reported reason.
    FieldUnavailable {
        /// Name of the unavailable field.
        field: &'static str,
        /// Why the field cannot be constructed.
        reason: ConstructionUnavailableReason,
    },
    /// The caller attempted to set a field that is not public.
    FieldNotPublic {
        /// Name of the restricted field.
        field: &'static str,
    },
    /// A supplied field value has the wrong reflected type.
    ValueTypeMismatch {
        /// Name of the field with the wrong value type.
        field: &'static str,
        /// Expected and actual value type identities.
        mismatch: crate::error::TypeMismatch,
    },
    /// Positional input contains more values than the target accepts.
    TooManyValues {
        /// Maximum number of accepted values.
        expected_max: usize,
        /// Number of values supplied.
        actual: usize,
    },
}

impl fmt::Display for ConstructionError {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ConstructionError {}

/// Copies the expected and actual type identities into the public error type.
/// # Parameters
///
/// - `inner`: Runtime representation being adapted to the public API.
fn public_mismatch(inner: &crate::runtime::value::TypeMismatch) -> crate::error::TypeMismatch {
    inner.clone()
}

/// Maps every runtime construction error variant to its public counterpart.
/// # Parameters
///
/// - `inner`: Runtime representation being adapted to the public API.
fn public_construction_error(inner: &runtime::ConstructionError) -> ConstructionError {
    use runtime::ConstructionError as RuntimeError;
    match inner {
        RuntimeError::MissingProvider => ConstructionError::MissingProvider,
        RuntimeError::AmbiguousProvider { target, count } => ConstructionError::AmbiguousProvider {
            target: target.rust_name,
            count: *count,
        },
        RuntimeError::WrongTargetKind => ConstructionError::WrongTargetKind,
        RuntimeError::Unavailable(reason) => ConstructionError::Unavailable(reason.clone()),
        RuntimeError::TargetTypeMismatch(mismatch) => ConstructionError::TargetTypeMismatch(public_mismatch(mismatch)),
        RuntimeError::ShapeMismatch { expected, actual } => ConstructionError::ShapeMismatch {
            expected: *expected,
            actual: *actual,
        },
        RuntimeError::MissingField { field } => ConstructionError::MissingField { field },
        RuntimeError::DuplicateField { field } => ConstructionError::DuplicateField { field },
        RuntimeError::UnknownField { field } => ConstructionError::UnknownField { field: field.clone() },
        RuntimeError::ProviderOnlyField { field } => ConstructionError::ProviderOnlyField { field },
        RuntimeError::FieldUnavailable { field, reason } => {
            ConstructionError::FieldUnavailable { field, reason: *reason }
        }
        RuntimeError::FieldNotPublic { field } => ConstructionError::FieldNotPublic { field },
        RuntimeError::ValueTypeMismatch { field, mismatch } => ConstructionError::ValueTypeMismatch {
            field,
            mismatch: public_mismatch(mismatch),
        },
        RuntimeError::TooManyValues { expected_max, actual } => ConstructionError::TooManyValues {
            expected_max: *expected_max,
            actual: *actual,
        },
    }
}

/// Public update error categories and details.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UpdateError {
    /// The supplied base value is not of the type being updated.
    BaseTypeMismatch(
        /// Expected base type and type supplied by the caller.
        crate::error::TypeMismatch,
    ),
    /// The requested output type does not match the provider target type.
    TargetTypeMismatch(
        /// Expected update target type and type requested by the caller.
        crate::error::TypeMismatch,
    ),
    /// Update was requested for a type or descriptor that cannot be updated.
    WrongTargetKind,
    /// The selected target has no registered update provider.
    MissingProvider,
    /// More than one provider matches the target.
    AmbiguousProvider {
        /// Rust type name of the ambiguous target.
        target: &'static str,
        /// Number of matching providers.
        count: usize,
    },
    /// The selected target is unavailable for the reported reasons.
    Unavailable(
        /// Structured reasons this target cannot be updated.
        ConstructionAvailability,
    ),
    /// The input layout does not match the target's update shape.
    ShapeMismatch {
        /// Shape required by the target.
        expected: ConstructionShape,
        /// Shape supplied by the caller.
        actual: ConstructionShape,
    },
    /// A required replacement value was omitted.
    MissingField {
        /// Name of the missing field.
        field: &'static str,
    },
    /// A named field was supplied more than once.
    DuplicateField {
        /// Name of the duplicated field.
        field: &'static str,
    },
    /// The input names a field that is not part of the target.
    UnknownField {
        /// Name supplied by the caller.
        field: Box<str>,
    },
    /// The caller supplied a field that only its provider may initialize.
    ProviderOnlyField {
        /// Name of the provider-only field.
        field: &'static str,
    },
    /// The selected field is unavailable for the reported reason.
    FieldUnavailable {
        /// Name of the unavailable field.
        field: &'static str,
        /// Why the field cannot be changed.
        reason: ConstructionUnavailableReason,
    },
    /// The caller attempted to replace a field that is not public.
    FieldNotPublic {
        /// Name of the restricted field.
        field: &'static str,
    },
    /// A replacement value has the wrong reflected type.
    ValueTypeMismatch {
        /// Name of the field with the wrong value type.
        field: &'static str,
        /// Expected and actual value type identities.
        mismatch: crate::error::TypeMismatch,
    },
    /// Positional input contains more replacements than the target accepts.
    TooManyValues {
        /// Maximum number of accepted replacements.
        expected_max: usize,
        /// Number of replacements supplied.
        actual: usize,
    },
    /// The caller tried to replace a field whose update policy preserves it.
    FieldPreserved {
        /// Name of the preserved field.
        field: &'static str,
    },
    /// The field cannot be updated for the reported reason.
    FieldUpdateUnavailable {
        /// Name of the unavailable field.
        field: &'static str,
        /// Why the field cannot be changed.
        reason: UpdateUnavailableReason,
    },
    /// The target cannot be updated for the reported reason.
    UpdateUnavailable(
        /// Structured reason the target has no usable update path.
        UpdateUnavailableReason,
    ),
}

impl fmt::Display for UpdateError {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for UpdateError {}

/// Maps every runtime update error variant to its public counterpart.
/// # Parameters
///
/// - `inner`: Runtime representation being adapted to the public API.
fn public_update_error(inner: &runtime::UpdateError) -> UpdateError {
    use runtime::UpdateError as RuntimeError;
    match inner {
        RuntimeError::BaseTypeMismatch(mismatch) => UpdateError::BaseTypeMismatch(public_mismatch(mismatch)),
        RuntimeError::TargetTypeMismatch(mismatch) => UpdateError::TargetTypeMismatch(public_mismatch(mismatch)),
        RuntimeError::WrongTargetKind => UpdateError::WrongTargetKind,
        RuntimeError::MissingProvider => UpdateError::MissingProvider,
        RuntimeError::AmbiguousProvider { target, count } => UpdateError::AmbiguousProvider {
            target: target.rust_name,
            count: *count,
        },
        RuntimeError::Unavailable(reason) => UpdateError::Unavailable(reason.clone()),
        RuntimeError::ShapeMismatch { expected, actual } => UpdateError::ShapeMismatch {
            expected: *expected,
            actual: *actual,
        },
        RuntimeError::MissingField { field } => UpdateError::MissingField { field },
        RuntimeError::DuplicateField { field } => UpdateError::DuplicateField { field },
        RuntimeError::UnknownField { field } => UpdateError::UnknownField { field: field.clone() },
        RuntimeError::ProviderOnlyField { field } => UpdateError::ProviderOnlyField { field },
        RuntimeError::FieldUnavailable { field, reason } => UpdateError::FieldUnavailable { field, reason: *reason },
        RuntimeError::FieldNotPublic { field } => UpdateError::FieldNotPublic { field },
        RuntimeError::ValueTypeMismatch { field, mismatch } => UpdateError::ValueTypeMismatch {
            field,
            mismatch: public_mismatch(mismatch),
        },
        RuntimeError::TooManyValues { expected_max, actual } => UpdateError::TooManyValues {
            expected_max: *expected_max,
            actual: *actual,
        },
        RuntimeError::FieldPreserved { field } => UpdateError::FieldPreserved { field },
        RuntimeError::FieldUpdateUnavailable { field, reason } => {
            UpdateError::FieldUpdateUnavailable { field, reason: *reason }
        }
        RuntimeError::UpdateUnavailable(reason) => UpdateError::UpdateUnavailable(*reason),
    }
}

/// Construction failure with recoverable owned input.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[must_use]
pub struct ConstructionFailure<M: ValueMode> {
    /// Structured reason the construction or update operation failed.
    error: ConstructionError,
    /// Original input retained so its values can be recovered after failure.
    recovery: RecoveredConstructionInput<M>,
}

impl<M: ValueMode> fmt::Debug for ConstructionFailure<M> {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructionFailure")
            .field("error", &self.error)
            .field("shape", &self.recovery().shape())
            .finish()
    }
}

impl<M: ValueMode> ConstructionFailure<M> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    fn new(inner: runtime::ConstructionFailure<M>) -> Self {
        let error = public_construction_error(inner.error());
        let recovery = recovered_input(inner.into_input());
        Self { error, recovery }
    }

    /// Returns the structured construction error.
    pub fn error(&self) -> &ConstructionError {
        &self.error
    }

    /// Recovers the original input shape and values.
    pub fn recovery(&self) -> ConstructionRecovery<'_, M> {
        ConstructionRecovery { input: &self.recovery }
    }

    /// Recovers the original input shape and values.
    pub fn into_input(self) -> RecoveredConstructionInput<M> {
        self.recovery
    }
}

/// Recovered construction values, preserving declaration-independent input
/// order.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub enum RecoveredConstructionInput<M: ValueMode> {
    /// Recovered named field values.
    Named(
        /// Builder containing the original name/value pairs.
        NamedFields<M>,
    ),
    /// Recovered positional values in their original order.
    Positional(
        /// Builder containing the original positional values.
        PositionalFields<M>,
    ),
    /// Recovered empty input for a unit target.
    Unit(
        /// Empty builder corresponding to the original unit input.
        UnitFields,
    ),
}

/// Converts runtime recovery values back into the corresponding public builder.
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
fn recovered_input<M: ValueMode>(input: runtime::RecoveredConstructionInput<M>) -> RecoveredConstructionInput<M> {
    match input {
        runtime::RecoveredConstructionInput::Named(inner) => RecoveredConstructionInput::Named(NamedFields { inner }),
        runtime::RecoveredConstructionInput::Positional(inner) => {
            RecoveredConstructionInput::Positional(PositionalFields { inner })
        }
        runtime::RecoveredConstructionInput::Unit(_) => RecoveredConstructionInput::Unit(UnitFields),
    }
}

/// Recoverable construction input view.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct ConstructionRecovery<'a, M: ValueMode> {
    /// Borrowed recovered input retained by the failure object.
    input: &'a RecoveredConstructionInput<M>,
}

impl<M: ValueMode> ConstructionRecovery<'_, M> {
    /// Returns the shape retained for recovery.
    pub fn shape(&self) -> ConstructionShape {
        match self.input {
            RecoveredConstructionInput::Named(_) => ConstructionShape::Named,
            RecoveredConstructionInput::Positional(_) => ConstructionShape::Positional,
            RecoveredConstructionInput::Unit(_) => ConstructionShape::Unit,
        }
    }
}

impl<M: ValueMode> RecoverableFailure for ConstructionFailure<M> {
    /// Public error category returned by the recoverable failure contract.
    type Error = ConstructionError;
    /// Owned construction or update input retained on failure.
    type Recovery = RecoveredConstructionInput<M>;

    /// Structured reason the construction or update operation failed.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Original input retained so its values can be recovered after failure.
    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    /// Separates the structured error from the values retained for recovery.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        (self.error, self.recovery)
    }
}

/// Failure from an owned update, retaining the base and every replacement.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[must_use]
pub struct UpdateFailure<M: ValueMode> {
    /// Structured reason the construction or update operation failed.
    error: UpdateError,
    /// Original input retained so its values can be recovered after failure.
    recovery: UpdateInput<M>,
}

impl<M: ValueMode> fmt::Debug for UpdateFailure<M> {
    /// Formats the structured error using its stable debug representation.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<M: ValueMode> UpdateFailure<M> {
    /// Wraps a runtime input value produced by the corresponding public
    /// builder. # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    /// # Parameters
    ///
    /// - `inner`: Runtime representation being adapted to the public API.
    fn new(inner: runtime::UpdateFailure<M>) -> Self {
        let error = public_update_error(inner.error());
        let recovery = UpdateInput {
            inner: inner.into_input(),
        };
        Self { error, recovery }
    }

    /// Returns the structured update error.
    pub fn error(&self) -> &UpdateError {
        &self.error
    }

    /// Returns the retained base value.
    pub fn base(&self) -> &DynamicOwned<M> {
        &self.recovery.inner.base
    }

    /// Borrows the retained update input.
    pub fn recovery(&self) -> UpdateRecovery<'_, M> {
        UpdateRecovery { input: &self.recovery }
    }

    /// Returns the retained update input.
    pub fn into_input(self) -> UpdateInput<M> {
        self.recovery
    }
}

impl<M: ValueMode> RecoverableFailure for UpdateFailure<M> {
    /// Public error category returned by the recoverable failure contract.
    type Error = UpdateError;
    /// Owned construction or update input retained on failure.
    type Recovery = UpdateInput<M>;

    /// Structured reason the construction or update operation failed.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Original input retained so its values can be recovered after failure.
    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    /// Separates the structured error from the values retained for recovery.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        (self.error, self.recovery)
    }
}

/// Owned base value plus its named replacements.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct UpdateInput<M: ValueMode = Local> {
    /// Runtime input storage that owns the values until construction succeeds
    /// or failure recovery.
    inner: RuntimeUpdateInput<M>,
}

impl<M: ValueMode> UpdateInput<M> {
    /// Starts an update from a typed owned value.
    ///
    /// # Parameters
    ///
    /// - `base`: Owned base value whose fields may be replaced.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `base`: Owned base value whose fields may be replaced.
    pub fn new<T: ValueFor<M>>(base: T) -> Self {
        Self {
            inner: RuntimeUpdateInput::new(base),
        }
    }

    /// Starts an update from a dynamic owned value.
    ///
    /// # Parameters
    ///
    /// - `base`: Owned base value whose fields may be replaced.
    pub fn from_dynamic(base: DynamicOwned<M>) -> Self {
        Self {
            inner: RuntimeUpdateInput::from_dynamic(base),
        }
    }

    /// Adds a typed replacement.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.inner = self.inner.field(name, value);
        self
    }

    /// Adds a dynamically owned replacement.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn field_dynamic(mut self, name: impl Into<Box<str>>, value: DynamicOwned<M>) -> Self {
        self.inner = self.inner.field_dynamic(name, value);
        self
    }
}

/// Recovery view for an update failure.
///
/// # Type Parameters
///
/// - `'a`: Lifetime of the borrowed input or view.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct UpdateRecovery<'a, M: ValueMode> {
    /// Borrowed recovered input retained by the failure object.
    input: &'a UpdateInput<M>,
}

impl<M: ValueMode> UpdateRecovery<'_, M> {
    /// Returns the preserved base value.
    pub fn base(&self) -> &DynamicOwned<M> {
        &self.input.inner.base
    }
}

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
    fn new(inner: runtime::ConstructionDescriptor<'ctx>, target: ConstructionTarget<'ctx>) -> Self {
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
    fn construct_runtime<T, M>(self, input: RuntimeConstructionInput<M>) -> Result<T, runtime::ConstructionFailure<M>>
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
    unsafe fn construct_unchecked_runtime<T, M>(
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
    fn construct_dynamic_runtime<M>(
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
    unsafe fn construct_dynamic_unchecked_runtime<M>(
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
    fn update_runtime<T, M>(self, input: UpdateInput<M>) -> Result<T, runtime::UpdateFailure<M>>
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
    unsafe fn update_unchecked_runtime<T, M>(self, input: UpdateInput<M>) -> Result<T, runtime::UpdateFailure<M>>
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
    fn update_dynamic_runtime<M>(self, input: UpdateInput<M>) -> Result<DynamicOwned<M>, runtime::UpdateFailure<M>>
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
    unsafe fn update_dynamic_unchecked_runtime<M>(
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

#[cfg(all(test, feature = "derive"))]
mod tests {
    use super::ConstructField;
    use super::ConstructionAvailability;
    use super::ConstructionError;
    use super::ConstructionFieldPolicy;
    use super::ConstructionFields;
    use super::ConstructionRecovery;
    use super::ConstructionShape;
    use super::ConstructionTarget;
    use super::ConstructionUnavailableReason;
    use super::Local;
    use super::NamedFields;
    use super::ThreadSafe;
    use super::UnitFields;
    use super::UpdateError;
    use super::UpdateFieldPolicy;
    use super::UpdateInput;
    use super::UpdateUnavailableReason;
    use super::public_construction_error;
    use super::public_update_error;
    use super::recovered_input;
    use super::runtime;
    use crate::runtime::construct::ConstructionError as RuntimeConstructionError;
    use crate::runtime::construct::RecoveredConstructionInput as RuntimeRecoveredInput;
    use crate::runtime::construct::UpdateError as RuntimeUpdateError;
    use crate::runtime::descriptor::field::Visibility;
    use crate::runtime::identity::TypeIdentity;
    use crate::runtime::value::TypeMismatch;

    #[crate::Reflect]
    #[derive(Debug, PartialEq)]
    pub struct PublicRecord {
        pub id: u32,
        pub name: String,
    }

    #[crate::Reflect]
    #[derive(Debug, PartialEq)]
    pub enum PublicEvent {
        Named { id: u32 },
        Unit,
    }

    fn mismatch() -> TypeMismatch {
        TypeMismatch::new(
            TypeIdentity {
                type_id: std::any::TypeId::of::<u32>(),
                rust_name: "u32",
            },
            TypeIdentity {
                type_id: std::any::TypeId::of::<String>(),
                rust_name: "String",
            },
        )
    }

    fn target() -> TypeIdentity {
        TypeIdentity {
            type_id: std::any::TypeId::of::<u32>(),
            rust_name: "u32",
        }
    }

    #[test]
    fn test_construction_errors_preserve_every_runtime_error_detail() {
        let mismatch = mismatch();
        let unavailable = ConstructionAvailability::Unavailable {
            reasons: Box::new([ConstructionUnavailableReason::PrivateRequiredField]),
        };
        let cases = [
            (
                RuntimeConstructionError::MissingProvider,
                ConstructionError::MissingProvider,
            ),
            (
                RuntimeConstructionError::AmbiguousProvider {
                    target: target(),
                    count: 2,
                },
                ConstructionError::AmbiguousProvider {
                    target: "u32",
                    count: 2,
                },
            ),
            (
                RuntimeConstructionError::WrongTargetKind,
                ConstructionError::WrongTargetKind,
            ),
            (
                RuntimeConstructionError::Unavailable(unavailable.clone()),
                ConstructionError::Unavailable(unavailable),
            ),
            (
                RuntimeConstructionError::TargetTypeMismatch(mismatch.clone()),
                ConstructionError::TargetTypeMismatch(mismatch.clone()),
            ),
            (
                RuntimeConstructionError::ShapeMismatch {
                    expected: ConstructionShape::Named,
                    actual: ConstructionShape::Positional,
                },
                ConstructionError::ShapeMismatch {
                    expected: ConstructionShape::Named,
                    actual: ConstructionShape::Positional,
                },
            ),
            (
                RuntimeConstructionError::MissingField { field: "name" },
                ConstructionError::MissingField { field: "name" },
            ),
            (
                RuntimeConstructionError::DuplicateField { field: "name" },
                ConstructionError::DuplicateField { field: "name" },
            ),
            (
                RuntimeConstructionError::UnknownField { field: "extra".into() },
                ConstructionError::UnknownField { field: "extra".into() },
            ),
            (
                RuntimeConstructionError::ProviderOnlyField { field: "id" },
                ConstructionError::ProviderOnlyField { field: "id" },
            ),
            (
                RuntimeConstructionError::FieldUnavailable {
                    field: "id",
                    reason: ConstructionUnavailableReason::FieldNotReflectable,
                },
                ConstructionError::FieldUnavailable {
                    field: "id",
                    reason: ConstructionUnavailableReason::FieldNotReflectable,
                },
            ),
            (
                RuntimeConstructionError::FieldNotPublic { field: "id" },
                ConstructionError::FieldNotPublic { field: "id" },
            ),
            (
                RuntimeConstructionError::ValueTypeMismatch {
                    field: "id",
                    mismatch: mismatch.clone(),
                },
                ConstructionError::ValueTypeMismatch {
                    field: "id",
                    mismatch: mismatch.clone(),
                },
            ),
            (
                RuntimeConstructionError::TooManyValues {
                    expected_max: 2,
                    actual: 3,
                },
                ConstructionError::TooManyValues {
                    expected_max: 2,
                    actual: 3,
                },
            ),
        ];

        for (runtime, expected) in cases {
            assert_eq!(public_construction_error(&runtime), expected);
        }
    }

    #[test]
    fn test_update_errors_preserve_every_runtime_error_detail() {
        let mismatch = mismatch();
        let unavailable = ConstructionAvailability::Unavailable {
            reasons: Box::new([ConstructionUnavailableReason::UnsupportedShape]),
        };
        let cases = [
            (
                RuntimeUpdateError::BaseTypeMismatch(mismatch.clone()),
                UpdateError::BaseTypeMismatch(mismatch.clone()),
            ),
            (
                RuntimeUpdateError::TargetTypeMismatch(mismatch.clone()),
                UpdateError::TargetTypeMismatch(mismatch.clone()),
            ),
            (RuntimeUpdateError::WrongTargetKind, UpdateError::WrongTargetKind),
            (RuntimeUpdateError::MissingProvider, UpdateError::MissingProvider),
            (
                RuntimeUpdateError::AmbiguousProvider {
                    target: target(),
                    count: 3,
                },
                UpdateError::AmbiguousProvider {
                    target: "u32",
                    count: 3,
                },
            ),
            (
                RuntimeUpdateError::Unavailable(unavailable.clone()),
                UpdateError::Unavailable(unavailable),
            ),
            (
                RuntimeUpdateError::ShapeMismatch {
                    expected: ConstructionShape::Named,
                    actual: ConstructionShape::Unit,
                },
                UpdateError::ShapeMismatch {
                    expected: ConstructionShape::Named,
                    actual: ConstructionShape::Unit,
                },
            ),
            (
                RuntimeUpdateError::MissingField { field: "name" },
                UpdateError::MissingField { field: "name" },
            ),
            (
                RuntimeUpdateError::DuplicateField { field: "name" },
                UpdateError::DuplicateField { field: "name" },
            ),
            (
                RuntimeUpdateError::UnknownField { field: "extra".into() },
                UpdateError::UnknownField { field: "extra".into() },
            ),
            (
                RuntimeUpdateError::ProviderOnlyField { field: "id" },
                UpdateError::ProviderOnlyField { field: "id" },
            ),
            (
                RuntimeUpdateError::FieldUnavailable {
                    field: "id",
                    reason: ConstructionUnavailableReason::NoProvider,
                },
                UpdateError::FieldUnavailable {
                    field: "id",
                    reason: ConstructionUnavailableReason::NoProvider,
                },
            ),
            (
                RuntimeUpdateError::FieldNotPublic { field: "id" },
                UpdateError::FieldNotPublic { field: "id" },
            ),
            (
                RuntimeUpdateError::ValueTypeMismatch {
                    field: "id",
                    mismatch: mismatch.clone(),
                },
                UpdateError::ValueTypeMismatch {
                    field: "id",
                    mismatch: mismatch.clone(),
                },
            ),
            (
                RuntimeUpdateError::TooManyValues {
                    expected_max: 1,
                    actual: 2,
                },
                UpdateError::TooManyValues {
                    expected_max: 1,
                    actual: 2,
                },
            ),
            (
                RuntimeUpdateError::FieldPreserved { field: "id" },
                UpdateError::FieldPreserved { field: "id" },
            ),
            (
                RuntimeUpdateError::FieldUpdateUnavailable {
                    field: "id",
                    reason: UpdateUnavailableReason::FieldNotReplaceable,
                },
                UpdateError::FieldUpdateUnavailable {
                    field: "id",
                    reason: UpdateUnavailableReason::FieldNotReplaceable,
                },
            ),
            (
                RuntimeUpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider),
                UpdateError::UpdateUnavailable(UpdateUnavailableReason::NoProvider),
            ),
        ];

        for (runtime, expected) in cases {
            assert_eq!(public_update_error(&runtime), expected);
        }
    }

    #[test]
    fn test_recovered_inputs_and_field_views_preserve_shape_and_bounds() {
        let named = recovered_input(RuntimeRecoveredInput::Named(runtime::NamedFields::<Local>::new()));
        let positional = recovered_input(RuntimeRecoveredInput::Positional(
            runtime::PositionalFields::<Local>::new(),
        ));
        let unit = recovered_input(RuntimeRecoveredInput::<Local>::Unit(runtime::UnitFields::new()));
        assert_eq!(ConstructionRecovery { input: &named }.shape(), ConstructionShape::Named);
        assert_eq!(
            ConstructionRecovery { input: &positional }.shape(),
            ConstructionShape::Positional
        );
        assert_eq!(ConstructionRecovery { input: &unit }.shape(), ConstructionShape::Unit);

        let fields = [ConstructField::typed::<u32>(
            "id",
            Visibility::Public,
            ConstructionFieldPolicy::Required,
            UpdateFieldPolicy::Replaceable,
        )];
        let view = ConstructionFields::new(&fields);
        assert_eq!(view.len(), 1);
        assert!(!view.is_empty());
        assert_eq!(view.get(0).map(ConstructField::name), Some("id"));
        assert!(view.get(1).is_none());
        assert!(view.get(usize::MAX).is_none());
        assert_eq!(view.iter().map(ConstructField::name).collect::<Vec<_>>(), ["id"]);

        let empty = ConstructionFields::new(&[]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert!(empty.get(0).is_none());
        assert_eq!(empty.iter().count(), 0);
    }

    #[test]
    fn test_public_facade_constructs_and_updates_local_and_thread_safe_values() {
        let descriptor = crate::TypeDescriptor::of::<PublicRecord>();
        let plan = descriptor.construction();
        assert!(matches!(plan.target(), ConstructionTarget::Struct(_)));
        assert_eq!(plan.shape(), ConstructionShape::Named);
        assert_eq!(
            plan.fields().iter().map(ConstructField::name).collect::<Vec<_>>(),
            ["id", "name"]
        );

        let local: PublicRecord = descriptor
            .construct(
                NamedFields::<Local>::new()
                    .field("id", 1_u32)
                    .field("name", "local".to_owned()),
            )
            .expect("named fields construct a local typed value");
        assert_eq!(
            local,
            PublicRecord {
                id: 1,
                name: "local".to_owned()
            }
        );

        let thread_safe: PublicRecord = descriptor
            .construct(
                NamedFields::<ThreadSafe>::new()
                    .field("id", 2_u32)
                    .field("name", "safe".to_owned()),
            )
            .expect("named fields construct a thread-safe typed value");
        assert_eq!(
            thread_safe,
            PublicRecord {
                id: 2,
                name: "safe".to_owned()
            }
        );

        let dynamic = descriptor
            .construct_dynamic(
                NamedFields::<ThreadSafe>::new()
                    .field("id", 3_u32)
                    .field("name", "dynamic".to_owned()),
            )
            .expect("named fields construct a thread-safe dynamic value");
        assert_eq!(
            dynamic.downcast_ref::<PublicRecord>(),
            Some(&PublicRecord {
                id: 3,
                name: "dynamic".to_owned()
            })
        );

        let unchecked: PublicRecord = unsafe {
            descriptor.construct_unchecked(
                NamedFields::<ThreadSafe>::new()
                    .field("id", 4_u32)
                    .field("name", "unchecked".to_owned()),
            )
        }
        .expect("unchecked typed construction preserves valid field values");
        assert_eq!(unchecked.id, 4);

        let dynamic_unchecked = unsafe {
            descriptor.construct_dynamic_unchecked(
                NamedFields::<ThreadSafe>::new()
                    .field("id", 5_u32)
                    .field("name", "dynamic unchecked".to_owned()),
            )
        }
        .expect("unchecked dynamic construction preserves valid field values");
        assert_eq!(
            dynamic_unchecked.downcast_ref::<PublicRecord>().map(|value| value.id),
            Some(5)
        );

        let updated: PublicRecord = descriptor
            .update::<_, ThreadSafe>(UpdateInput::<ThreadSafe>::new(thread_safe).field("name", "updated".to_owned()))
            .expect("thread-safe typed updates replace selected fields");
        assert_eq!(
            updated,
            PublicRecord {
                id: 2,
                name: "updated".to_owned()
            }
        );

        let unchecked_typed_update: PublicRecord = unsafe {
            plan.update_unchecked::<_, ThreadSafe>(
                UpdateInput::<ThreadSafe>::new(PublicRecord {
                    id: 9,
                    name: "before".to_owned(),
                })
                .field("name", "typed unchecked".to_owned()),
            )
        }
        .expect("thread-safe unchecked typed update preserves valid replacements");
        assert_eq!(unchecked_typed_update.name, "typed unchecked");

        let local_dynamic_update = descriptor
            .update_dynamic::<Local>(
                UpdateInput::<Local>::new(PublicRecord {
                    id: 10,
                    name: "before".to_owned(),
                })
                .field("name", "local dynamic".to_owned()),
            )
            .expect("local dynamic update replaces selected fields");
        assert_eq!(
            local_dynamic_update
                .downcast_ref::<PublicRecord>()
                .map(|value| value.name.as_str()),
            Some("local dynamic")
        );

        let dynamic_updated = descriptor
            .update_dynamic::<ThreadSafe>(UpdateInput::from_dynamic(dynamic).field("id", 6_u32))
            .expect("dynamic updates accept a dynamic base value");
        assert_eq!(
            dynamic_updated.downcast_ref::<PublicRecord>().map(|value| value.id),
            Some(6)
        );

        let dynamic_unchecked_update = unsafe {
            descriptor.update_dynamic_unchecked::<ThreadSafe>(
                UpdateInput::<ThreadSafe>::new(PublicRecord {
                    id: 7,
                    name: "before".to_owned(),
                })
                .field("name", "after".to_owned()),
            )
        }
        .expect("unchecked dynamic update handles a valid replacement");
        assert_eq!(
            dynamic_unchecked_update
                .downcast_ref::<PublicRecord>()
                .map(|value| value.name.as_str()),
            Some("after")
        );

        let local_unchecked_update = unsafe {
            descriptor.update_dynamic_unchecked::<Local>(
                UpdateInput::<Local>::new(PublicRecord {
                    id: 8,
                    name: "before".to_owned(),
                })
                .field("name", "local after".to_owned()),
            )
        }
        .expect("local unchecked dynamic update handles a valid replacement");
        assert_eq!(
            local_unchecked_update
                .downcast_ref::<PublicRecord>()
                .map(|value| value.name.as_str()),
            Some("local after")
        );
    }

    #[test]
    fn test_enum_variant_facade_covers_named_and_unit_construction_paths() {
        let descriptor = crate::TypeDescriptor::of::<PublicEvent>();
        let named = descriptor.variant("Named").expect("named variant is reflected");
        assert!(matches!(named.construction().target(), ConstructionTarget::Variant(_)));
        let event: PublicEvent = named
            .construct(NamedFields::<Local>::new().field("id", 11_u32))
            .expect("named enum variant is constructible by field name");
        assert_eq!(event, PublicEvent::Named { id: 11 });

        let dynamic = named
            .construct_dynamic(NamedFields::<ThreadSafe>::new().field("id", 12_u32))
            .expect("named enum variant supports dynamic construction");
        assert_eq!(
            dynamic.downcast_ref::<PublicEvent>(),
            Some(&PublicEvent::Named { id: 12 })
        );

        let unit = descriptor.variant("Unit").expect("unit variant is reflected");
        let event: PublicEvent = unit
            .construct(UnitFields::new())
            .expect("unit enum variant is constructed with unit input");
        assert_eq!(event, PublicEvent::Unit);

        let dynamic_unchecked = unsafe { unit.construct_dynamic_unchecked(UnitFields::new()) }
            .expect("unit enum variant supports unchecked dynamic construction");
        assert_eq!(
            dynamic_unchecked.downcast_ref::<PublicEvent>(),
            Some(&PublicEvent::Unit)
        );
    }

    #[test]
    fn test_public_failures_format_and_retain_update_and_construction_inputs() {
        let descriptor = crate::TypeDescriptor::of::<PublicRecord>();
        let failure = descriptor
            .construct::<PublicRecord, _>(NamedFields::<Local>::new().field("id", "wrong".to_owned()))
            .expect_err("wrong field types are reported");
        assert!(matches!(
            failure.error(),
            ConstructionError::ValueTypeMismatch { field: "id", .. }
        ));
        assert!(format!("{failure:?}").contains("shape: Named"));
        assert_eq!(failure.recovery().shape(), ConstructionShape::Named);

        let update_failure = descriptor
            .update::<PublicRecord, Local>(
                UpdateInput::<Local>::new(PublicRecord {
                    id: 1,
                    name: "base".to_owned(),
                })
                .field("id", "wrong".to_owned()),
            )
            .expect_err("wrong update field types are reported");
        assert!(matches!(
            update_failure.error(),
            UpdateError::ValueTypeMismatch { field: "id", .. }
        ));
        assert!(format!("{update_failure:?}").contains("UpdateFailure"));
        assert_eq!(
            update_failure.base().downcast_ref::<PublicRecord>(),
            Some(&PublicRecord {
                id: 1,
                name: "base".to_owned()
            })
        );
        assert_eq!(
            update_failure.recovery().base().downcast_ref::<PublicRecord>(),
            Some(&PublicRecord {
                id: 1,
                name: "base".to_owned()
            })
        );
    }
}
