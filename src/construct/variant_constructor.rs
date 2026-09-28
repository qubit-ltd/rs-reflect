// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Enum-variant construction adapter contract and descriptor-bound dispatch.

use std::any::TypeId;
use std::fmt;

use crate::construct::ConstructionError;
use crate::construct::ConstructionField;
use crate::construct::ConstructionRecovery;
use crate::construct::ConstructionShape;
use crate::construct::NamedConstructionInput;
use crate::construct::TupleConstructionInput;
use crate::construct::ValidatedConstructionInput;
use crate::construct::struct_constructor::local_type_id;
use crate::construct::struct_constructor::thread_safe_type_id;
use crate::descriptor::VariantDescriptor;
use crate::descriptor::VariantKind;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::Mode;
use crate::value::ThreadSafe;

/// A mode-specific adapter generated inside the declaring enum module.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode accepted and returned by the adapter.
pub type VariantConstructionAdapter<M> = fn(ValidatedConstructionInput<M>) -> DynamicOwned<M>;

/// A descriptor-bound two-phase constructor for one concrete enum variant.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode supported by the constructor.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::construct::TupleConstructionInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// enum Event {
///     Text(String),
/// }
///
/// let constructor = TypeDescriptor::of::<Event>()
///     .variant("Text")
///     .expect("derived variant")
///     .construction()
///     .expect("generated constructor")
///     .local_constructor();
/// let input = TupleConstructionInput::new([DynamicOwned::<Local>::new(String::from("hello"))]);
/// let event = constructor.construct_tuple(input).expect("valid tuple field");
/// assert!(event.downcast_ref::<Event>().is_some());
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[must_use]
pub struct VariantConstructor<M: Mode + 'static> {
    /// Immutable variant whose fields and constructor are described.
    variant: &'static VariantDescriptor,
    /// Variant fields and their policies in descriptor source order.
    fields: &'static [ConstructionField<M>],
    /// Generated adapter that constructs the declaring enum value.
    adapter: VariantConstructionAdapter<M>,
}

impl<M: Mode + 'static> VariantConstructor<M> {
    /// Creates a variant constructor from generated immutable descriptor data.
    ///
    /// `fields` must correspond to the variant's fields in source order, and
    /// `adapter` must return the declaring enum's exact root type.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode handled by `adapter`.
    ///
    /// # Parameters
    ///
    /// - `variant`: The immutable enum variant descriptor.
    /// - `fields`: The variant fields paired with construction policies.
    /// - `adapter`: Generated code that consumes validated values.
    ///
    /// # Returns
    ///
    /// Returns a constructor over the supplied immutable metadata.
    #[doc(hidden)]
    #[inline]
    pub const fn new(
        variant: &'static VariantDescriptor,
        fields: &'static [ConstructionField<M>],
        adapter: VariantConstructionAdapter<M>,
    ) -> Self {
        Self {
            variant,
            fields,
            adapter,
        }
    }

    /// Returns the immutable enum variant descriptor.
    ///
    /// # Returns
    ///
    /// Returns the variant associated with this constructor.
    #[must_use]
    #[inline]
    pub const fn variant(&self) -> &'static VariantDescriptor {
        self.variant
    }

    /// Returns field construction policies in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns policies corresponding to the variant fields by source index.
    #[must_use = "the field descriptors define the constructor inputs"]
    #[inline]
    pub const fn fields(&self) -> &'static [ConstructionField<M>] {
        self.fields
    }

    /// Returns the input shape required by this variant.
    ///
    /// # Returns
    ///
    /// Returns `Named`, `Tuple`, or `Unit` according to the variant form.
    #[must_use]
    #[inline]
    pub const fn shape(&self) -> ConstructionShape {
        match self.variant.kind() {
            VariantKind::Struct => ConstructionShape::Named,
            VariantKind::Tuple => ConstructionShape::Tuple,
            VariantKind::Unit => ConstructionShape::Unit,
        }
    }

    /// Executes an adapter only after named input validation succeeds.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode of the input and result.
    ///
    /// # Parameters
    ///
    /// - `input`: Named caller-owned values to validate.
    /// - `value_type_id`: The exact runtime type identity function for `M`.
    ///
    /// # Returns
    ///
    /// Returns the constructed value or recovery containing all input values.
    ///
    /// # Errors
    ///
    /// Returns recovery when shape, names, policies, or exact field types do
    /// not match the variant descriptor.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    fn construct_named_with(
        &self,
        input: NamedConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        if self.shape() != ConstructionShape::Named {
            return Err(input.into_recovery(ConstructionError::WrongShape {
                expected: self.shape(),
                actual: ConstructionShape::Named,
            }));
        }
        let validated = crate::construct::validated::validate_named(input, self.fields, value_type_id)?;
        Ok(self.execute(validated, value_type_id))
    }

    /// Executes an adapter only after positional input validation succeeds.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode of the input and result.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional caller-owned values to validate.
    /// - `value_type_id`: The exact runtime type identity function for `M`.
    ///
    /// # Returns
    ///
    /// Returns the constructed value or recovery containing all input values.
    ///
    /// # Errors
    ///
    /// Returns recovery when shape, field count, policies, or exact field types
    /// do not match the variant descriptor.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    fn construct_tuple_with(
        &self,
        input: TupleConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        if self.shape() != ConstructionShape::Tuple {
            return Err(input.into_recovery(ConstructionError::WrongShape {
                expected: self.shape(),
                actual: ConstructionShape::Tuple,
            }));
        }
        let validated = crate::construct::validated::validate_tuple(input, self.fields, value_type_id)?;
        Ok(self.execute(validated, value_type_id))
    }

    /// Executes an adapter only after unit-shape validation succeeds.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode of the result.
    ///
    /// # Parameters
    ///
    /// - `value_type_id`: The exact runtime type identity function for `M`.
    ///
    /// # Returns
    ///
    /// Returns the constructed unit variant or an empty-input recovery error.
    ///
    /// # Errors
    ///
    /// Returns recovery when the variant is not unit-shaped or its fields are
    /// unavailable.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    fn construct_unit_with(
        &self,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        if self.shape() != ConstructionShape::Unit {
            return Err(ConstructionRecovery::new(
                ConstructionError::WrongShape {
                    expected: self.shape(),
                    actual: ConstructionShape::Unit,
                },
                Vec::new(),
            ));
        }
        let validated = crate::construct::validated::validate_unit(self.fields)
            .map_err(|error| ConstructionRecovery::new(error, Vec::new()))?;
        Ok(self.execute(validated, value_type_id))
    }

    /// Invokes generated code and enforces its exact enum-root output
    /// invariant.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode of validated values and the result.
    ///
    /// # Parameters
    ///
    /// - `validated`: The descriptor-ordered values after validation.
    /// - `value_type_id`: The exact runtime type identity function for `M`.
    ///
    /// # Returns
    ///
    /// Returns the value produced by the generated adapter.
    ///
    /// # Panics
    ///
    /// Panics if the generated adapter returns a value with a different root
    /// type than the declaring enum.
    fn execute(
        &self,
        validated: ValidatedConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> DynamicOwned<M> {
        let output = (self.adapter)(validated);
        assert_eq!(
            value_type_id(&output),
            self.variant.declaring_type().type_id(),
            "a variant construction adapter must return its exact declaring enum type"
        );
        output
    }

    /// Enforces generated variant/field alignment before accepting input.
    ///
    /// # Panics
    ///
    /// Panics if the field policies do not reference the variant's own fields
    /// in descriptor order.
    fn assert_descriptor_contract(&self) {
        assert_eq!(
            self.variant.fields().len(),
            self.fields.len(),
            "construction policy must cover every variant field"
        );
        for (descriptor_field, construction_field) in self.variant.fields().iter().zip(self.fields) {
            assert!(
                std::ptr::eq(descriptor_field, construction_field.descriptor()),
                "construction policy fields must be the variant's own fields"
            );
        }
    }
}

impl VariantConstructor<Local> {
    /// Validates and constructs a local named enum variant.
    ///
    /// # Parameters
    ///
    /// - `input`: Named values to validate against the variant fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed local enum value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with every caller-owned value when validation fails.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_named(
        &self,
        input: NamedConstructionInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_named_with(input, local_type_id)
    }

    /// Validates and constructs a local tuple enum variant.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional values to validate against the variant fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed local enum value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with every caller-owned value when validation fails.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_tuple_with(input, local_type_id)
    }

    /// Validates and constructs a local unit enum variant.
    ///
    /// # Returns
    ///
    /// Returns the constructed local enum value on success.
    ///
    /// # Errors
    ///
    /// Returns an empty-input recovery when the variant cannot be constructed.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_unit(&self) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_unit_with(local_type_id)
    }
}

impl VariantConstructor<ThreadSafe> {
    /// Validates and constructs a thread-safe named enum variant.
    ///
    /// # Parameters
    ///
    /// - `input`: Named values to validate against the variant fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed thread-safe enum value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with every caller-owned value when validation fails.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_named(
        &self,
        input: NamedConstructionInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_named_with(input, thread_safe_type_id)
    }

    /// Validates and constructs a thread-safe tuple enum variant.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional values to validate against the variant fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed thread-safe enum value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with every caller-owned value when validation fails.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_tuple_with(input, thread_safe_type_id)
    }

    /// Validates and constructs a thread-safe unit enum variant.
    ///
    /// # Returns
    ///
    /// Returns the constructed thread-safe enum value on success.
    ///
    /// # Errors
    ///
    /// Returns an empty-input recovery when the variant cannot be constructed.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_unit(&self) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_unit_with(thread_safe_type_id)
    }
}

impl<M: Mode + 'static> fmt::Debug for VariantConstructor<M> {
    /// Formats variant and policy facts without exposing adapter addresses.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the constructor metadata.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VariantConstructor")
            .field("variant", &self.variant.rust_name())
            .field("shape", &self.shape())
            .field("fields", &self.fields)
            .finish()
    }
}
