// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Struct construction adapter contract and descriptor-bound dispatch.

use std::any::Any;
use std::any::TypeId;
use std::fmt;

use crate::construct::ConstructionError;
use crate::construct::ConstructionField;
use crate::construct::ConstructionRecovery;
use crate::construct::ConstructionShape;
use crate::construct::NamedConstructionInput;
use crate::construct::TupleConstructionInput;
use crate::construct::ValidatedConstructionInput;
use crate::descriptor::StructKind;
use crate::descriptor::TypeDescriptor;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::Mode;
use crate::value::ThreadSafe;

/// A mode-specific safe adapter generated inside the declaring struct module.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode accepted and returned by the adapter.
pub type StructConstructionAdapter<M> = fn(ValidatedConstructionInput<M>) -> DynamicOwned<M>;

/// A descriptor-bound two-phase constructor for one concrete struct root.
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
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct User {
///     name: String,
/// }
///
/// let constructor = TypeDescriptor::of::<User>()
///     .struct_construction()
///     .expect("derived constructor")
///     .local_constructor();
/// let input = NamedConstructionInput::new([("name", DynamicOwned::<Local>::new(String::from("Ada")))]);
/// let user = constructor.construct_named(input).expect("valid field input");
/// assert!(user.downcast_ref::<User>().is_some());
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
pub struct StructConstructor<M: Mode + 'static> {
    /// Exact reflected root that the generated adapter constructs.
    descriptor: &'static TypeDescriptor,
    /// Direct fields and their policies in descriptor source order.
    fields: &'static [ConstructionField<M>],
    /// Generated constructor invoked after validation succeeds.
    adapter: StructConstructionAdapter<M>,
}

impl<M: Mode + 'static> StructConstructor<M> {
    /// Creates a constructor from generated immutable descriptor data.
    ///
    /// `fields` must correspond to the descriptor's direct fields in source
    /// order, and `adapter` must return the descriptor's exact root type.
    ///
    /// # Type Parameters
    ///
    /// - `M`: The dynamic ownership mode handled by `adapter`.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The exact reflected struct root.
    /// - `fields`: Its direct fields paired with construction policies.
    /// - `adapter`: Generated code that consumes validated values.
    ///
    /// # Returns
    ///
    /// Returns a constructor over the supplied immutable metadata.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new(
        descriptor: &'static TypeDescriptor,
        fields: &'static [ConstructionField<M>],
        adapter: StructConstructionAdapter<M>,
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
    /// Returns the exact root descriptor associated with this constructor.
    #[must_use]
    #[inline]
    pub const fn descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor
    }

    /// Returns field construction policies in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns the policies corresponding to direct fields by source index.
    #[inline]
    pub const fn fields(&self) -> &'static [ConstructionField<M>] {
        self.fields
    }

    /// Returns the shape required by this struct constructor.
    ///
    /// # Panics
    ///
    /// Panics if generated or manually assembled metadata associates this
    /// constructor with a non-struct descriptor.
    #[must_use]
    #[inline]
    pub fn shape(&self) -> ConstructionShape {
        match self
            .descriptor
            .as_struct()
            .unwrap_or_else(|| panic!("a StructConstructor requires a struct descriptor"))
            .kind()
        {
            StructKind::Named => ConstructionShape::Named,
            StructKind::Tuple | StructKind::Newtype => ConstructionShape::Tuple,
            StructKind::Unit => ConstructionShape::Unit,
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
    /// Returns recovery when shape, field names, policies, or exact field types
    /// do not match the descriptor.
    ///
    /// # Panics
    ///
    /// Panics if generated descriptor metadata or adapter output violates the
    /// constructor's exact-root contract.
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
    /// do not match the descriptor.
    ///
    /// # Panics
    ///
    /// Panics if generated descriptor metadata or adapter output violates the
    /// constructor's exact-root contract.
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
    /// Returns the constructed unit value or an empty-input recovery error.
    ///
    /// # Errors
    ///
    /// Returns recovery when the descriptor is not unit-shaped or required
    /// fields are unavailable.
    ///
    /// # Panics
    ///
    /// Panics if generated descriptor metadata or adapter output violates the
    /// constructor's exact-root contract.
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

    /// Invokes generated code and enforces its exact-root output invariant.
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
    /// Panics if generated code returns a value whose root type differs from
    /// the descriptor.
    fn execute(
        &self,
        validated: ValidatedConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> DynamicOwned<M> {
        let output = (self.adapter)(validated);
        assert_eq!(
            value_type_id(&output),
            self.descriptor.type_id(),
            "a struct construction adapter must return its exact declared root type"
        );
        output
    }

    /// Enforces generated descriptor/field alignment before accepting input.
    ///
    /// # Panics
    ///
    /// Panics if the descriptor is not a struct or its fields do not exactly
    /// match the generated field-policy slice.
    fn assert_descriptor_contract(&self) {
        assert!(
            self.descriptor.as_struct().is_some(),
            "a StructConstructor requires a struct descriptor"
        );
        assert_eq!(
            self.descriptor.fields().len(),
            self.fields.len(),
            "construction policy must cover every direct struct field"
        );
        for (descriptor_field, construction_field) in self.descriptor.fields().iter().zip(self.fields) {
            assert!(
                std::ptr::eq(descriptor_field, construction_field.descriptor()),
                "construction policy fields must be the descriptor's own fields"
            );
        }
    }
}

impl StructConstructor<Local> {
    /// Validates and constructs a local named struct.
    ///
    /// # Parameters
    ///
    /// - `input`: Named values to validate against the struct fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed local value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with all caller-owned values when validation fails.
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

    /// Validates and constructs a local tuple or newtype struct.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional values to validate against the struct fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed local value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with all caller-owned values when validation fails.
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

    /// Validates and constructs a local unit struct.
    ///
    /// # Returns
    ///
    /// Returns the constructed local value on success.
    ///
    /// # Errors
    ///
    /// Returns an empty-input recovery when the target is not constructible as
    /// a unit struct.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_unit(&self) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_unit_with(local_type_id)
    }
}

impl StructConstructor<ThreadSafe> {
    /// Validates and constructs a thread-safe named struct.
    ///
    /// # Parameters
    ///
    /// - `input`: Named values to validate against the struct fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed thread-safe value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with all caller-owned values when validation fails.
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

    /// Validates and constructs a thread-safe tuple or newtype struct.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional values to validate against the struct fields.
    ///
    /// # Returns
    ///
    /// Returns the constructed thread-safe value on success.
    ///
    /// # Errors
    ///
    /// Returns recovery with all caller-owned values when validation fails.
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

    /// Validates and constructs a thread-safe unit struct.
    ///
    /// # Returns
    ///
    /// Returns the constructed thread-safe value on success.
    ///
    /// # Errors
    ///
    /// Returns an empty-input recovery when the target is not constructible as
    /// a unit struct.
    ///
    /// # Panics
    ///
    /// Panics if generated metadata or adapter output violates its contract.
    #[inline]
    pub fn construct_unit(&self) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_unit_with(thread_safe_type_id)
    }
}

impl<M: Mode + 'static> fmt::Debug for StructConstructor<M> {
    /// Formats descriptor and policy facts without exposing adapter addresses.
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
            .debug_struct("StructConstructor")
            .field("descriptor", &self.descriptor.type_name())
            .field("shape", &self.shape())
            .field("fields", &self.fields)
            .finish()
    }
}

/// Returns the exact local erased value type identity.
///
/// # Parameters
///
/// - `value`: The local dynamic value whose concrete type is inspected.
///
/// # Returns
///
/// Returns its process-local concrete `TypeId`.
///
/// # Panics
///
/// Panics only if local dynamic storage violates its `Any` compatibility
/// invariant.
#[must_use]
#[inline]
pub(crate) fn local_type_id(value: &DynamicOwned<Local>) -> TypeId {
    value
        .as_any()
        .map(Any::type_id)
        .unwrap_or_else(|| unreachable!("owned local values are Any-compatible"))
}

/// Returns the exact thread-safe erased value type identity.
///
/// # Parameters
///
/// - `value`: The thread-safe dynamic value whose concrete type is inspected.
///
/// # Returns
///
/// Returns its process-local concrete `TypeId`.
///
/// # Panics
///
/// Panics only if thread-safe dynamic storage violates its `Any` compatibility
/// invariant.
#[must_use]
#[inline]
pub(crate) fn thread_safe_type_id(value: &DynamicOwned<ThreadSafe>) -> TypeId {
    value
        .as_any()
        .map(Any::type_id)
        .unwrap_or_else(|| unreachable!("owned thread-safe values are Any-compatible"))
}
