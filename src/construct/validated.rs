// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Validated adapter input and shared pre-execution validation helpers.

use std::any::TypeId;
use std::fmt;

use crate::construct::ConstructionError;
use crate::construct::ConstructionField;
use crate::construct::ConstructionFieldId;
use crate::construct::ConstructionFieldPolicy;
use crate::construct::ConstructionRecovery;
use crate::construct::ConstructionUnavailableReason;
use crate::construct::NamedConstructionInput;
use crate::construct::StructUpdateInput;
use crate::construct::TupleConstructionInput;
use crate::construct::UpdateField;
use crate::construct::UpdateFieldPolicy;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::value::DynamicOwned;
use crate::value::Mode;

/// Descriptor-ordered owned values proven safe for a construction adapter.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the validated values.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ValidatedConstructionInput;
/// use qubit_reflect::value::Local;
///
/// fn generated_adapter_input(input: ValidatedConstructionInput<Local>) {
///     assert!(input.values().is_empty());
/// }
///
/// let _adapter: fn(ValidatedConstructionInput<Local>) = generated_adapter_input;
/// ```
#[must_use]
pub struct ValidatedConstructionInput<M: Mode> {
    /// Caller and provider values reordered to match direct-field indices.
    values: Box<[DynamicOwned<M>]>,
}

impl<M: Mode> ValidatedConstructionInput<M> {
    /// Returns values in exact descriptor source-index order.
    ///
    /// # Returns
    ///
    /// Returns all validated owned values without transferring ownership.
    #[must_use]
    #[inline]
    pub fn values(&self) -> &[DynamicOwned<M>] {
        &self.values
    }

    /// Consumes validation state so generated code may downcast every value.
    ///
    /// # Returns
    ///
    /// Returns the validated values in descriptor source-index order.
    #[must_use]
    #[inline]
    pub fn into_values(self) -> Box<[DynamicOwned<M>]> {
        self.values
    }
}

impl<M: Mode> fmt::Debug for ValidatedConstructionInput<M> {
    /// Formats the value count without requiring erased values to be `Debug`.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the value count.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedConstructionInput")
            .field("value_count", &self.values.len())
            .finish()
    }
}

/// One descriptor-indexed override proven to have the exact field type.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the validated replacement.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ValidatedOverride;
/// use qubit_reflect::value::Local;
///
/// fn generated_update_input(input: &ValidatedOverride<Local>) {
///     assert_eq!(input.index(), 0);
/// }
///
/// let _adapter: fn(&ValidatedOverride<Local>) = generated_update_input;
/// ```
#[must_use]
pub struct ValidatedOverride<M: Mode> {
    /// Zero-based field index in the reflected source declaration.
    index: usize,
    /// Replacement whose concrete type exactly matches that field.
    value: DynamicOwned<M>,
}

impl<M: Mode> ValidatedOverride<M> {
    /// Returns the declared source field index.
    ///
    /// # Returns
    ///
    /// Returns the zero-based index used by generated field assignment.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the exact validated whole-field replacement value.
    ///
    /// # Returns
    ///
    /// Returns a borrow of the replacement value with the field's exact type.
    #[must_use]
    #[inline]
    pub const fn value(&self) -> &DynamicOwned<M> {
        &self.value
    }

    /// Consumes the override for generated field assignment.
    ///
    /// # Returns
    ///
    /// Returns the source index and owned replacement value.
    #[must_use]
    pub fn into_parts(self) -> (usize, DynamicOwned<M>) {
        (self.index, self.value)
    }
}

impl<M: Mode> fmt::Debug for ValidatedOverride<M> {
    /// Formats the source field index without formatting the erased value.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the field index.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedOverride")
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
}

/// An exact owned root and descriptor-ordered validated whole-field overrides.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the root and replacement values.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ValidatedUpdateInput;
/// use qubit_reflect::value::Local;
///
/// fn generated_update_adapter(input: ValidatedUpdateInput<Local>) {
///     assert!(input.overrides().is_empty());
/// }
///
/// let _adapter: fn(ValidatedUpdateInput<Local>) = generated_update_adapter;
/// ```
#[must_use]
pub struct ValidatedUpdateInput<M: Mode> {
    /// Exact root value whose type matched the descriptor.
    base: DynamicOwned<M>,
    /// Validated replacements ordered by descriptor field index.
    overrides: Box<[ValidatedOverride<M>]>,
}

impl<M: Mode> ValidatedUpdateInput<M> {
    /// Returns the exact validated owned root value.
    ///
    /// # Returns
    ///
    /// Returns a borrow of the root whose runtime type matches the descriptor.
    #[must_use]
    #[inline]
    pub const fn base(&self) -> &DynamicOwned<M> {
        &self.base
    }

    /// Returns supplied overrides in descriptor source-index order.
    ///
    /// # Returns
    ///
    /// Returns the validated replacements in generated assignment order.
    #[inline]
    pub fn overrides(&self) -> &[ValidatedOverride<M>] {
        &self.overrides
    }

    /// Consumes validation state for generated downcasts and field assignment.
    ///
    /// # Returns
    ///
    /// Returns the owned root and descriptor-ordered replacements.
    #[must_use]
    pub fn into_parts(self) -> (DynamicOwned<M>, Box<[ValidatedOverride<M>]>) {
        (self.base, self.overrides)
    }
}

impl<M: Mode> fmt::Debug for ValidatedUpdateInput<M> {
    /// Formats the override count without formatting erased values.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the override count.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedUpdateInput")
            .field("override_count", &self.overrides.len())
            .finish()
    }
}

/// Validates named from-zero input without extracting any caller-owned value.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the input and validated values.
///
/// # Parameters
///
/// - `input`: Named caller-owned values to validate.
/// - `fields`: The target fields and their generated construction policies.
/// - `value_type_id`: The exact runtime type identity function for `M`.
///
/// # Returns
///
/// Returns descriptor-ordered validated values on success.
///
/// # Errors
///
/// Returns recovery containing the input values when validation fails.
///
/// # Panics
///
/// Panics if a generated value provider panics or metadata violates the
/// descriptor-order contract.
pub(crate) fn validate_named<M: Mode>(
    input: NamedConstructionInput<M>,
    fields: &'static [ConstructionField<M>],
    value_type_id: fn(&DynamicOwned<M>) -> TypeId,
) -> Result<ValidatedConstructionInput<M>, ConstructionRecovery<M>> {
    assert_field_contract(fields);
    if let Some(error) = unavailable_constructor_field(fields) {
        return Err(input.into_recovery(error));
    }

    let positions = match validate_construction_bindings(input.fields(), fields, value_type_id) {
        Ok(positions) => positions,
        Err(error) => return Err(input.into_recovery(error)),
    };

    for (index, field) in fields.iter().enumerate() {
        if positions[index].is_none() && matches!(field.policy(), ConstructionFieldPolicy::Required) {
            return Err(input.into_recovery(ConstructionError::MissingField {
                field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
            }));
        }
    }

    let mut defaults: Vec<Option<DynamicOwned<M>>> = std::iter::repeat_with(|| None).take(fields.len()).collect();
    for (index, field) in fields.iter().enumerate() {
        if positions[index].is_some() {
            continue;
        }
        match field.policy() {
            ConstructionFieldPolicy::Default(provider) | ConstructionFieldPolicy::ProviderOnly(provider) => {
                let value = provider();
                if let Err(error) = validate_value(field.descriptor(), &value, value_type_id) {
                    return Err(input.into_recovery(error));
                }
                defaults[index] = Some(value);
            }
            ConstructionFieldPolicy::Unavailable(_) => {
                unreachable!("constructor availability is validated before providers")
            }
            ConstructionFieldPolicy::Required => {
                unreachable!("every missing required field is rejected before providers")
            }
        }
    }

    let mut raw = input.into_fields().into_iter().map(Some).collect::<Vec<_>>();
    let mut values = Vec::with_capacity(fields.len());
    for index in 0..fields.len() {
        if let Some(input_index) = positions[index] {
            let (_, value) = raw[input_index]
                .take()
                .unwrap_or_else(|| unreachable!("each input position is selected once"));
            values.push(value);
        } else {
            values.push(
                defaults[index]
                    .take()
                    .unwrap_or_else(|| unreachable!("every omitted field has a validated provider")),
            );
        }
    }
    Ok(ValidatedConstructionInput {
        values: values.into_boxed_slice(),
    })
}

/// Validates tuple input, permitting only trailing explicitly defaulted fields.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the input and validated values.
///
/// # Parameters
///
/// - `input`: Positional caller-owned values to validate.
/// - `fields`: The target fields and their generated construction policies.
/// - `value_type_id`: The exact runtime type identity function for `M`.
///
/// # Returns
///
/// Returns descriptor-ordered validated values on success.
///
/// # Errors
///
/// Returns recovery containing the input values when shape or validation
/// fails.
///
/// # Panics
///
/// Panics if a generated value provider panics or metadata violates the
/// descriptor-order contract.
pub(crate) fn validate_tuple<M: Mode>(
    input: TupleConstructionInput<M>,
    fields: &'static [ConstructionField<M>],
    value_type_id: fn(&DynamicOwned<M>) -> TypeId,
) -> Result<ValidatedConstructionInput<M>, ConstructionRecovery<M>> {
    assert_field_contract(fields);
    if let Some(error) = unavailable_constructor_field(fields) {
        return Err(input.into_recovery(error));
    }
    let caller_field_indices = fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            (!matches!(field.policy(), ConstructionFieldPolicy::ProviderOnly(_))).then_some(index)
        })
        .collect::<Vec<_>>();
    if input.values().len() > caller_field_indices.len() {
        return Err(input.into_recovery(ConstructionError::UnknownPosition {
            index: caller_field_indices.len(),
        }));
    }
    let mut positions = vec![None; fields.len()];
    for (input_index, (&field_index, value)) in caller_field_indices.iter().zip(input.values()).enumerate() {
        let field = &fields[field_index];
        if let Err(error) = validate_value(field.descriptor(), value, value_type_id) {
            return Err(input.into_recovery(error));
        }
        positions[field_index] = Some(input_index);
    }

    for (index, field) in fields.iter().enumerate() {
        if positions[index].is_none() && matches!(field.policy(), ConstructionFieldPolicy::Required) {
            return Err(input.into_recovery(ConstructionError::MissingField {
                field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
            }));
        }
    }

    let mut defaults: Vec<Option<DynamicOwned<M>>> = std::iter::repeat_with(|| None).take(fields.len()).collect();
    for (index, field) in fields.iter().enumerate() {
        if positions[index].is_some() {
            continue;
        }
        match field.policy() {
            ConstructionFieldPolicy::Default(provider) | ConstructionFieldPolicy::ProviderOnly(provider) => {
                let value = provider();
                if let Err(error) = validate_value(field.descriptor(), &value, value_type_id) {
                    return Err(input.into_recovery(error));
                }
                defaults[index] = Some(value);
            }
            ConstructionFieldPolicy::Unavailable(_) => {
                unreachable!("constructor availability is validated before providers")
            }
            ConstructionFieldPolicy::Required => {
                unreachable!("every missing required field is rejected before providers")
            }
        }
    }

    let mut raw = input.into_values().into_iter().map(Some).collect::<Vec<_>>();
    let mut values = Vec::with_capacity(fields.len());
    for index in 0..fields.len() {
        if let Some(input_index) = positions[index] {
            values.push(
                raw[input_index]
                    .take()
                    .unwrap_or_else(|| unreachable!("each tuple position is selected once")),
            );
        } else {
            values.push(
                defaults[index]
                    .take()
                    .unwrap_or_else(|| unreachable!("every omitted field has a validated provider")),
            );
        }
    }
    Ok(ValidatedConstructionInput {
        values: values.into_boxed_slice(),
    })
}

/// Validates a unit target's empty generated field contract.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of validated values.
///
/// # Parameters
///
/// - `fields`: The generated fields and their construction policies.
///
/// # Returns
///
/// Returns an empty validated input for a valid unit target.
///
/// # Errors
///
/// Returns the missing-field or unavailable-construction error.
///
/// # Panics
///
/// Panics if generated field indices violate the descriptor-order contract.
pub(crate) fn validate_unit<M: Mode>(
    fields: &'static [ConstructionField<M>],
) -> Result<ValidatedConstructionInput<M>, ConstructionError> {
    assert_field_contract(fields);
    if let Some(error) = unavailable_constructor_field(fields) {
        return Err(error);
    }
    if let Some(field) = fields.first() {
        return Err(ConstructionError::MissingField {
            field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
        });
    }
    Ok(ValidatedConstructionInput { values: Box::new([]) })
}

/// Validates an exact update base and every supplied override atomically.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the base and replacement values.
///
/// # Parameters
///
/// - `input`: The owned base and named overrides to validate.
/// - `expected_root`: The exact root `TypeId` required by the updater.
/// - `fields`: The generated update policy for every direct field.
/// - `value_type_id`: The exact runtime type identity function for `M`.
///
/// # Returns
///
/// Returns the validated root and ordered overrides on success.
///
/// # Errors
///
/// Returns recovery with the original inputs when any validation fails.
///
/// # Panics
///
/// Panics if generated field indices violate the descriptor-order contract.
pub(crate) fn validate_update<M: Mode>(
    input: StructUpdateInput<M>,
    expected_root: TypeId,
    fields: &'static [UpdateField],
    value_type_id: fn(&DynamicOwned<M>) -> TypeId,
) -> Result<ValidatedUpdateInput<M>, ConstructionRecovery<M>> {
    assert_update_field_contract(fields);
    let actual_root = value_type_id(input.base());
    if actual_root != expected_root {
        return Err(input.into_recovery(ConstructionError::BaseTypeMismatch {
            mismatch: TypeMismatch::new(expected_root, actual_root),
        }));
    }

    let positions = match validate_update_bindings(input.overrides().fields(), fields, value_type_id) {
        Ok(positions) => positions,
        Err(error) => return Err(input.into_recovery(error)),
    };

    let (base, overrides) = input.into_parts();
    let mut raw = overrides.into_fields().into_iter().map(Some).collect::<Vec<_>>();
    let mut validated = Vec::with_capacity(raw.len());
    for (index, position) in positions.into_iter().enumerate() {
        if let Some(position) = position {
            let (_, value) = raw[position]
                .take()
                .unwrap_or_else(|| unreachable!("each override position is selected once"));
            validated.push(ValidatedOverride { index, value });
        }
    }
    Ok(ValidatedUpdateInput {
        base,
        overrides: validated.into_boxed_slice(),
    })
}

/// Validates one dynamic value against a concrete resolved or opaque field.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the value.
///
/// # Parameters
///
/// - `field`: The reflected field whose concrete type is required.
/// - `value`: The caller-owned value being checked without extraction.
/// - `value_type_id`: The exact runtime type identity function for `M`.
///
/// # Returns
///
/// Returns `Ok(())` when the runtime type exactly matches the field.
///
/// # Errors
///
/// Returns an unavailable or type-mismatch construction error.
fn validate_value<M: Mode>(
    field: &FieldDescriptor,
    value: &DynamicOwned<M>,
    value_type_id: fn(&DynamicOwned<M>) -> TypeId,
) -> Result<(), ConstructionError> {
    let expected = match field.field_type() {
        TypeRef::Resolved(descriptor) => descriptor.type_id(),
        TypeRef::Opaque(descriptor) => descriptor.type_id(),
        TypeRef::Symbolic(_) => {
            return Err(ConstructionError::Unavailable {
                field: Box::new(ConstructionFieldId::from_descriptor(field)),
                reason: ConstructionUnavailableReason::SymbolicFieldType,
            });
        }
    };
    let actual = value_type_id(value);
    if actual == expected {
        Ok(())
    } else {
        Err(ConstructionError::ValueTypeMismatch {
            field: Box::new(ConstructionFieldId::from_descriptor(field)),
            mismatch: TypeMismatch::new(expected, actual),
        })
    }
}

/// Checks names, uniqueness, policy, and exact value types without extraction.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of supplied values.
///
/// # Parameters
///
/// - `input`: Ordered names and values supplied by the caller.
/// - `fields`: The target fields and their construction policies.
/// - `value_type_id`: The exact runtime type identity function for `M`.
///
/// # Returns
///
/// Returns the input position selected for each descriptor field.
///
/// # Errors
///
/// Returns the first unknown, duplicate, unavailable, or mistyped field error.
fn validate_construction_bindings<M: Mode>(
    input: &[(Box<str>, DynamicOwned<M>)],
    fields: &[ConstructionField<M>],
    value_type_id: fn(&DynamicOwned<M>) -> TypeId,
) -> Result<Vec<Option<usize>>, ConstructionError> {
    let mut positions = vec![None; fields.len()];
    for (input_index, (name, value)) in input.iter().enumerate() {
        let Some(field_index) = fields
            .iter()
            .position(|field| field.descriptor().query_name() == Some(name.as_ref()))
        else {
            return Err(ConstructionError::UnknownField { name: name.clone() });
        };
        if positions[field_index].replace(input_index).is_some() {
            return Err(ConstructionError::DuplicateField { name: name.clone() });
        }
        let field = &fields[field_index];
        match field.policy() {
            ConstructionFieldPolicy::ProviderOnly(_) => {
                return Err(ConstructionError::Unavailable {
                    field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
                    reason: ConstructionUnavailableReason::CallerValueForbidden,
                });
            }
            ConstructionFieldPolicy::Unavailable(reason) => {
                return Err(ConstructionError::Unavailable {
                    field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
                    reason,
                });
            }
            ConstructionFieldPolicy::Required | ConstructionFieldPolicy::Default(_) => {}
        }
        validate_value(field.descriptor(), value, value_type_id)?;
    }
    Ok(positions)
}

/// Returns the first field whose policy makes from-zero construction
/// unavailable.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the field policies.
///
/// # Parameters
///
/// - `fields`: The fields checked in descriptor order.
///
/// # Returns
///
/// Returns the first unavailable-field error, or `None` when all policies can
/// construct a value.
fn unavailable_constructor_field<M: Mode>(fields: &[ConstructionField<M>]) -> Option<ConstructionError> {
    fields.iter().find_map(|field| match field.policy() {
        ConstructionFieldPolicy::Unavailable(reason) => Some(ConstructionError::Unavailable {
            field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
            reason,
        }),
        ConstructionFieldPolicy::Required
        | ConstructionFieldPolicy::Default(_)
        | ConstructionFieldPolicy::ProviderOnly(_) => None,
    })
}

/// Checks update names, uniqueness, policy, and exact types without extraction.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of supplied values.
///
/// # Parameters
///
/// - `input`: Ordered names and values supplied as update overrides.
/// - `fields`: The target fields and their update policies.
/// - `value_type_id`: The exact runtime type identity function for `M`.
///
/// # Returns
///
/// Returns the input position selected for each descriptor field.
///
/// # Errors
///
/// Returns the first unknown, duplicate, unavailable, or mistyped field error.
fn validate_update_bindings<M: Mode>(
    input: &[(Box<str>, DynamicOwned<M>)],
    fields: &[UpdateField],
    value_type_id: fn(&DynamicOwned<M>) -> TypeId,
) -> Result<Vec<Option<usize>>, ConstructionError> {
    let mut positions = vec![None; fields.len()];
    for (input_index, (name, value)) in input.iter().enumerate() {
        let Some(field_index) = fields
            .iter()
            .position(|field| field.descriptor().query_name() == Some(name.as_ref()))
        else {
            return Err(ConstructionError::UnknownField { name: name.clone() });
        };
        if positions[field_index].replace(input_index).is_some() {
            return Err(ConstructionError::DuplicateField { name: name.clone() });
        }
        let field = &fields[field_index];
        if let UpdateFieldPolicy::Unavailable(reason) = field.policy() {
            return Err(ConstructionError::Unavailable {
                field: Box::new(ConstructionFieldId::from_descriptor(field.descriptor())),
                reason,
            });
        }
        validate_value(field.descriptor(), value, value_type_id)?;
    }
    Ok(positions)
}

/// Enforces the generated contract that fields are in contiguous source order.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the construction policies.
///
/// # Parameters
///
/// - `fields`: The fields whose descriptor indices are checked.
///
/// # Panics
///
/// Panics if any descriptor index differs from its position in the slice.
fn assert_field_contract<M: Mode>(fields: &[ConstructionField<M>]) {
    for (expected_index, field) in fields.iter().enumerate() {
        assert_eq!(
            field.descriptor().index(),
            expected_index,
            "construction fields must be ordered by contiguous descriptor index"
        );
    }
}

/// Enforces contiguous source order for an independent update field policy.
///
/// # Parameters
///
/// - `fields`: The fields whose descriptor indices are checked.
///
/// # Panics
///
/// Panics if any descriptor index differs from its position in the slice.
fn assert_update_field_contract(fields: &[UpdateField]) {
    for (expected_index, field) in fields.iter().enumerate() {
        assert_eq!(
            field.descriptor().index(),
            expected_index,
            "update fields must be ordered by contiguous descriptor index"
        );
    }
}
