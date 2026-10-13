// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Atomic owned-struct update validation and adapter dispatch.

use std::any::Any;
use std::any::TypeId;
use std::fmt;

use crate::construct::ConstructionRecovery;
use crate::construct::StructUpdateInput;
use crate::construct::UpdateField;
use crate::construct::ValidatedUpdateInput;
use crate::construct::struct_constructor::local_type_id;
use crate::descriptor::TypeDescriptor;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::Mode;

/// A mode-specific safe adapter generated inside the declaring struct module.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode accepted and returned by the adapter.
pub type StructUpdateAdapter<M> = fn(ValidatedUpdateInput<M>) -> DynamicOwned<M>;

/// A descriptor-bound atomic updater for one concrete struct root.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode supported by the updater.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::construct::StructUpdateInput;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// mod example {
///     use qubit_reflect::Reflect;
///     #[Reflect]
///     pub struct User { pub name: String }
/// }
///
/// let descriptor = TypeDescriptor::of::<example::User>();
/// let updater = descriptor.struct_construction().expect("generated updater").local_updater().expect("update enabled");
/// let input = StructUpdateInput::new(
///     example::User { name: String::from("Ada") },
///     NamedConstructionInput::new().field("name", String::from("Grace")),
/// );
/// let updated = updater.update::<example::User>(input).expect("valid replacement");
/// assert_eq!(updated.name, "Grace");
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
pub struct StructUpdater<M: Mode + 'static> {
    /// Exact reflected root whose owned fields may be replaced.
    descriptor: &'static TypeDescriptor,
    /// Direct fields and their update policies in descriptor source order.
    fields: &'static [UpdateField],
    /// Generated updater invoked after all validation succeeds.
    adapter: StructUpdateAdapter<M>,
}

impl<M: Mode + 'static> StructUpdater<M> {
    /// Creates an updater from generated immutable descriptor data.
    ///
    /// `fields` must correspond to every direct struct field in source order,
    /// and `adapter` must return the descriptor's exact root type.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode handled by `adapter`.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The exact reflected struct root.
    /// - `fields`: Every direct field paired with its update policy.
    /// - `adapter`: Generated code that consumes validated base and overrides.
    ///
    /// # Returns
    ///
    /// Returns an updater over the supplied immutable metadata.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new(
        descriptor: &'static TypeDescriptor,
        fields: &'static [UpdateField],
        adapter: StructUpdateAdapter<M>,
    ) -> Self {
        Self {
            descriptor,
            fields,
            adapter,
        }
    }

    /// Returns the concrete struct root descriptor.
    ///
    /// # Returns
    ///
    /// Returns the exact root descriptor associated with this updater.
    #[must_use]
    #[inline]
    pub const fn descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor
    }

    /// Returns update policies in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns the policies corresponding to direct fields by source index.
    #[inline]
    pub const fn fields(&self) -> &'static [UpdateField] {
        self.fields
    }

    /// Validates the base and all overrides before invoking generated code.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode of the input and result.
    ///
    /// # Parameters
    ///
    /// - `input`: The owned base and caller-ordered field replacements.
    /// - `value_type_id`: The exact runtime type identity function for `M`.
    ///
    /// # Returns
    ///
    /// Returns the updated owned value or recovery retaining every input.
    ///
    /// # Errors
    ///
    /// Returns recovery when the base type, field names, update policy, or
    /// replacement types do not match the descriptor.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates the exact-root
    /// contract.
    fn update_with(
        &self,
        input: StructUpdateInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
        allow_private: bool,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        let validated = crate::construct::validated::validate_update_with_visibility(
            input,
            self.descriptor.type_id(),
            self.fields,
            value_type_id,
            allow_private,
        )?;
        let output = (self.adapter)(validated);
        assert_eq!(
            value_type_id(&output),
            self.descriptor.type_id(),
            "a struct update adapter must return its exact declared root type"
        );
        Ok(output)
    }

    /// Enforces generated descriptor/field alignment before accepting input.
    ///
    /// # Panics
    ///
    /// Panics if the descriptor is not a struct or its fields do not exactly
    /// match the generated update-policy slice.
    fn assert_descriptor_contract(&self) {
        assert!(
            self.descriptor.as_struct().is_some(),
            "a StructUpdater requires a struct descriptor"
        );
        assert_eq!(
            self.descriptor.fields().len(),
            self.fields.len(),
            "update policy must cover every direct struct field"
        );
        for (descriptor_field, construction_field) in self.descriptor.fields().iter().zip(self.fields) {
            assert!(
                std::ptr::eq(descriptor_field, construction_field.descriptor()),
                "update policy fields must be the descriptor's own fields"
            );
        }
    }
}

impl StructUpdater<Local> {
    /// Atomically validates and updates a local owned struct value.
    ///
    /// # Parameters
    ///
    /// - `input`: The base value and named whole-field replacements.
    ///
    /// # Returns
    ///
    /// Returns the updated local value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with the untouched base and overrides when validation
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn update<T: Any>(&self, input: StructUpdateInput<Local>) -> Result<T, ConstructionRecovery<Local>> {
        if TypeId::of::<T>() != self.descriptor.type_id() {
            return Err(
                input.into_recovery(crate::construct::ConstructionError::TargetTypeMismatch {
                    expected: self.descriptor.type_id(),
                    actual: TypeId::of::<T>(),
                }),
            );
        }
        let value = self.update_with(input, local_type_id, false)?;
        Ok(value
            .downcast::<T>()
            .unwrap_or_else(|_| unreachable!("the generated updater must match its root descriptor")))
    }

    /// Updates a value dynamically and returns its exact owned Rust value.
    pub fn update_any(&self, input: StructUpdateInput<Local>) -> Result<Box<dyn Any>, ConstructionRecovery<Local>> {
        self.update_with(input, local_type_id, false).map(|value| {
            value
                .into_any()
                .unwrap_or_else(|_| unreachable!("owned update results are Any values"))
        })
    }

    /// Updates a local struct while allowing writes to private fields.
    ///
    /// # Safety
    ///
    /// The caller must ensure replacement values preserve every invariant
    /// normally enforced by the target type's private fields.
    #[allow(unsafe_code)]
    pub unsafe fn update_unsafe<T: Any>(
        &self,
        input: StructUpdateInput<Local>,
    ) -> Result<T, ConstructionRecovery<Local>> {
        if TypeId::of::<T>() != self.descriptor.type_id() {
            return Err(
                input.into_recovery(crate::construct::ConstructionError::TargetTypeMismatch {
                    expected: self.descriptor.type_id(),
                    actual: TypeId::of::<T>(),
                }),
            );
        }
        let value = self.update_with(input, local_type_id, true)?;
        Ok(value
            .downcast::<T>()
            .unwrap_or_else(|_| unreachable!("the generated updater must match its root descriptor")))
    }

    /// Updates a local struct dynamically while allowing writes to private
    /// fields.
    ///
    /// # Safety
    ///
    /// The caller must ensure replacement values preserve every invariant
    /// normally enforced by private fields of the target type.
    #[allow(unsafe_code)]
    pub unsafe fn update_any_unsafe(
        &self,
        input: StructUpdateInput<Local>,
    ) -> Result<Box<dyn Any>, ConstructionRecovery<Local>> {
        self.update_with(input, local_type_id, true).map(|value| {
            value
                .into_any()
                .unwrap_or_else(|_| unreachable!("owned update results are Any values"))
        })
    }
}

impl<M: Mode + 'static> fmt::Debug for StructUpdater<M> {
    /// Formats descriptor and policy facts without exposing adapter addresses.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the updater metadata.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StructUpdater")
            .field("descriptor", &self.descriptor.type_name())
            .field("fields", &self.fields)
            .finish()
    }
}
