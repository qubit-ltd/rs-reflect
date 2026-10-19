#![allow(unsafe_code)]

//! Runtime validation, execution, and input recovery for reflected construction.

mod input;
mod recovery;
mod schema;

pub(crate) use input::{NamedFields, PositionalFields, UnitFields, UpdateInput};
pub(crate) use recovery::{
    ConstructionFailure, ConstructionRecovery, RecoveredConstructionInput, UpdateFailure,
    UpdateRecovery,
};
pub use schema::{
    ConstructField, ConstructionAvailability, ConstructionFieldPolicy, ConstructionPlan,
    ConstructionProviders, ConstructionShape, ConstructionUnavailableReason,
    TypedConstructProvider, UpdateFieldPolicy, UpdateUnavailableReason,
};
pub(crate) use schema::{
    ConstructProvider, ConstructVisibility, ConstructionDescriptor, ConstructionError,
    ConstructionTarget, ConstructionTargetKind, UpdateError, resolve_plan, resolve_variant_plan,
};

use crate::runtime::value::{DynamicOwned, TypeMismatch, ValueFor, ValueMode};

use self::input::{ConstructionInput, ConstructionInputValue, InputEntryRef};
use self::schema::ConstructionMode;

impl<'ctx> ConstructionDescriptor<'ctx> {
    pub(crate) fn construct<T, M, I>(self, input: I) -> Result<T, ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
        I: ConstructionInput<M>,
    {
        if self.plan.target.kind == ConstructionTargetKind::Unsupported {
            return Err(ConstructionFailure::new(
                ConstructionError::WrongTargetKind,
                input.into_input(),
            ));
        }
        let expected = self.plan.target.identity();
        let actual = T::identity();
        if expected.type_id != actual.type_id {
            return Err(ConstructionFailure::new(
                ConstructionError::TargetTypeMismatch(TypeMismatch::new(expected, actual)),
                input.into_input(),
            ));
        }
        self.construct_inner(input.into_input(), false)
    }

    /// Constructs a value while bypassing reflected field visibility checks.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted fields.
    pub(crate) unsafe fn construct_unchecked<T, M, I>(
        self,
        input: I,
    ) -> Result<T, ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
        I: ConstructionInput<M>,
    {
        if self.plan.target.kind == ConstructionTargetKind::Unsupported {
            return Err(ConstructionFailure::new(
                ConstructionError::WrongTargetKind,
                input.into_input(),
            ));
        }
        let expected = self.plan.target.identity();
        let actual = T::identity();
        if expected.type_id != actual.type_id {
            return Err(ConstructionFailure::new(
                ConstructionError::TargetTypeMismatch(TypeMismatch::new(expected, actual)),
                input.into_input(),
            ));
        }
        self.construct_inner(input.into_input(), true)
    }

    fn construct_inner<T, M>(
        self,
        input: ConstructionInputValue<M>,
        unchecked_visibility: bool,
    ) -> Result<T, ConstructionFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        let target = &self.plan.target;
        let availability = self.plan.availability(unchecked_visibility);
        if let unavailable @ ConstructionAvailability::Unavailable { .. } = availability {
            return Err(ConstructionFailure::new(
                ConstructionError::Unavailable(unavailable),
                input,
            ));
        }
        let Some(provider) = self.plan.provider.for_mode::<M>() else {
            let error = ConstructionError::MissingProvider;
            return Err(ConstructionFailure::new(error, input));
        };

        let binding = match bind_input(&self.plan, &input, unchecked_visibility) {
            Ok(binding) => binding,
            Err(error) => return Err(ConstructionFailure::new(error, input)),
        };

        let values = take_bound_values(input, binding);
        let value = provider.construct(values);
        debug_assert_eq!(value.type_id(), target.identity().type_id);
        match value.downcast::<T>() {
            Ok(value) => Ok(value),
            Err(_) => panic!("construction provider returned a value with the wrong owner type"),
        }
    }

    pub(crate) fn construct_dynamic<M, I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<M>, ConstructionFailure<M>>
    where
        M: ConstructionMode,
        I: ConstructionInput<M>,
    {
        self.construct_dynamic_inner(input.into_input(), false)
    }

    /// Constructs a dynamic value while bypassing reflected field visibility checks.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted fields.
    pub(crate) unsafe fn construct_dynamic_unchecked<M, I>(
        self,
        input: I,
    ) -> Result<DynamicOwned<M>, ConstructionFailure<M>>
    where
        M: ConstructionMode,
        I: ConstructionInput<M>,
    {
        self.construct_dynamic_inner(input.into_input(), true)
    }

    fn construct_dynamic_inner<M: ConstructionMode>(
        self,
        input: ConstructionInputValue<M>,
        unchecked_visibility: bool,
    ) -> Result<DynamicOwned<M>, ConstructionFailure<M>> {
        if self.plan.target.kind == ConstructionTargetKind::Unsupported {
            return Err(ConstructionFailure::new(
                ConstructionError::WrongTargetKind,
                input,
            ));
        }
        let availability = self.plan.availability(unchecked_visibility);
        if let unavailable @ ConstructionAvailability::Unavailable { .. } = availability {
            return Err(ConstructionFailure::new(
                ConstructionError::Unavailable(unavailable),
                input,
            ));
        }
        let Some(provider) = self.plan.provider.for_mode::<M>() else {
            let error = ConstructionError::MissingProvider;
            return Err(ConstructionFailure::new(error, input));
        };
        let binding = match bind_input(&self.plan, &input, unchecked_visibility) {
            Ok(binding) => binding,
            Err(error) => return Err(ConstructionFailure::new(error, input)),
        };
        let value = provider.construct(take_bound_values(input, binding));
        if value.type_id() != self.plan.target.identity().type_id {
            panic!("construction provider returned a value with the wrong owner type");
        }
        Ok(value)
    }

    pub(crate) fn update<T, M>(self, input: UpdateInput<M>) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        let expected = self.plan.target.identity();
        let actual = T::identity();
        if expected.type_id != actual.type_id {
            return Err(UpdateFailure::new(
                UpdateError::TargetTypeMismatch(TypeMismatch::new(expected, actual)),
                input,
            ));
        }
        self.update_inner(input, false)
    }

    /// Updates an owned value while bypassing reflected field visibility checks.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted fields.
    pub(crate) unsafe fn update_unchecked<T, M>(
        self,
        input: UpdateInput<M>,
    ) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        let expected = self.plan.target.identity();
        let actual = T::identity();
        if expected.type_id != actual.type_id {
            return Err(UpdateFailure::new(
                UpdateError::TargetTypeMismatch(TypeMismatch::new(expected, actual)),
                input,
            ));
        }
        self.update_inner(input, true)
    }

    fn update_inner<T, M>(
        self,
        input: UpdateInput<M>,
        unchecked_visibility: bool,
    ) -> Result<T, UpdateFailure<M>>
    where
        T: ValueFor<M>,
        M: ConstructionMode,
    {
        let base_type = input.base.type_id();
        if base_type != self.plan.target.identity().type_id {
            let expected = self.plan.target.identity();
            let actual = input.base.ty().type_identity();
            return Err(UpdateFailure::new(
                UpdateError::BaseTypeMismatch(TypeMismatch::new(expected, actual)),
                input,
            ));
        }
        if self.plan.target.kind != ConstructionTargetKind::Struct {
            return Err(UpdateFailure::new(UpdateError::WrongTargetKind, input));
        }
        if let Some(reason) = self.plan.update_unavailable_reason() {
            return Err(UpdateFailure::new(
                UpdateError::UpdateUnavailable(reason),
                input,
            ));
        }
        let Some(provider) = self.plan.provider.for_mode::<M>() else {
            return Err(UpdateFailure::new(UpdateError::MissingProvider, input));
        };
        let binding = match bind_replacements(&self.plan, &input.replacements, unchecked_visibility)
        {
            Ok(binding) => binding,
            Err(error) => return Err(UpdateFailure::new(error, input)),
        };
        let UpdateInput { base, replacements } = input;
        let values = take_bound_values(replacements.into_input(), binding);
        let value = provider.update(base, values);
        if value.type_id() != self.plan.target.identity().type_id {
            panic!("update provider returned a value with the wrong owner type");
        }
        match value.downcast::<T>() {
            Ok(value) => Ok(value),
            Err(_) => panic!("update provider returned a value with the wrong owner type"),
        }
    }

    pub(crate) fn update_dynamic<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        self.update_dynamic_inner(input, false)
    }

    /// Updates a dynamic value while bypassing reflected field visibility checks.
    ///
    /// # Safety
    /// The caller must uphold invariants protected by private or restricted fields.
    pub(crate) unsafe fn update_dynamic_unchecked<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        self.update_dynamic_inner(input, true)
    }

    fn update_dynamic_inner<M: ConstructionMode>(
        self,
        input: UpdateInput<M>,
        unchecked_visibility: bool,
    ) -> Result<DynamicOwned<M>, UpdateFailure<M>> {
        if input.base.type_id() != self.plan.target.identity().type_id {
            return Err(UpdateFailure::new(
                UpdateError::BaseTypeMismatch(TypeMismatch::new(
                    self.plan.target.identity(),
                    input.base.ty().type_identity(),
                )),
                input,
            ));
        }
        if self.plan.target.kind != ConstructionTargetKind::Struct {
            return Err(UpdateFailure::new(UpdateError::WrongTargetKind, input));
        }
        if let Some(reason) = self.plan.update_unavailable_reason() {
            return Err(UpdateFailure::new(
                UpdateError::UpdateUnavailable(reason),
                input,
            ));
        }
        let Some(provider) = self.plan.provider.for_mode::<M>() else {
            return Err(UpdateFailure::new(UpdateError::MissingProvider, input));
        };
        let binding = match bind_replacements(&self.plan, &input.replacements, unchecked_visibility)
        {
            Ok(binding) => binding,
            Err(error) => return Err(UpdateFailure::new(error, input)),
        };
        let UpdateInput { base, replacements } = input;
        let value = provider.update(base, take_bound_values(replacements.into_input(), binding));
        if value.type_id() != self.plan.target.identity().type_id {
            panic!("update provider returned a value with the wrong owner type");
        }
        Ok(value)
    }
}

struct Binding {
    input_indices: Vec<Option<usize>>,
}

fn bind_input<M: ValueMode>(
    plan: &ConstructionPlan,
    input: &ConstructionInputValue<M>,
    unchecked_visibility: bool,
) -> Result<Binding, ConstructionError> {
    let shape = input.shape();
    if shape != plan.shape {
        return Err(ConstructionError::ShapeMismatch {
            expected: plan.shape,
            actual: shape,
        });
    }
    if let Some(field) = plan
        .fields
        .iter()
        .find(|field| matches!(field.policy, ConstructionFieldPolicy::Unavailable(_)))
    {
        if let ConstructionFieldPolicy::Unavailable(reason) = field.policy {
            return Err(ConstructionError::FieldUnavailable {
                field: field.name,
                reason,
            });
        }
    }
    let entries = input.entries();
    bind_entries(plan, &entries, unchecked_visibility, true)
}

fn bind_replacements<M: ValueMode>(
    plan: &ConstructionPlan,
    input: &NamedFields<M>,
    unchecked_visibility: bool,
) -> Result<Binding, UpdateError> {
    let entries = input.entries();
    let binding =
        bind_entries(plan, &entries, unchecked_visibility, false).map_err(UpdateError::from)?;
    for (index, value) in binding.input_indices.iter().enumerate() {
        if value.is_some() {
            match plan.fields[index].update_policy {
                UpdateFieldPolicy::Replaceable => {}
                UpdateFieldPolicy::Preserved => {
                    return Err(UpdateError::FieldPreserved {
                        field: plan.fields[index].name,
                    });
                }
                UpdateFieldPolicy::Unavailable(reason) => {
                    return Err(UpdateError::FieldUpdateUnavailable {
                        field: plan.fields[index].name,
                        reason,
                    });
                }
            }
        }
    }
    Ok(binding)
}

fn bind_entries<M: ValueMode>(
    plan: &ConstructionPlan,
    entries: &[InputEntryRef<'_, M>],
    unchecked_visibility: bool,
    check_required: bool,
) -> Result<Binding, ConstructionError> {
    let mut binding = Binding {
        input_indices: vec![None; plan.fields.len()],
    };
    let mut seen = vec![false; plan.fields.len()];

    if plan.shape == ConstructionShape::Positional {
        let mut position = 0;
        for (field_index, field) in plan.fields.iter().enumerate() {
            if matches!(field.policy, ConstructionFieldPolicy::ProviderOnly) {
                continue;
            }
            if let ConstructionFieldPolicy::Unavailable(reason) = field.policy {
                return Err(ConstructionError::FieldUnavailable {
                    field: field.name,
                    reason,
                });
            }
            if position == entries.len() {
                if matches!(field.policy, ConstructionFieldPolicy::Required) {
                    return Err(ConstructionError::MissingField { field: field.name });
                }
                continue;
            }
            let InputEntryRef::Positional(value) = &entries[position] else {
                unreachable!();
            };
            validate_value(field, value)?;
            validate_visibility(field, unchecked_visibility)?;
            binding.input_indices[field_index] = Some(position);
            position += 1;
        }
        if position != entries.len() {
            return Err(ConstructionError::TooManyValues {
                expected_max: position,
                actual: entries.len(),
            });
        }
    } else {
        for (input_index, entry) in entries.iter().enumerate() {
            let InputEntryRef::Named { name, value } = entry else {
                continue;
            };
            let Some((field_index, field)) = plan
                .fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == *name)
            else {
                return Err(ConstructionError::UnknownField {
                    field: (*name).into(),
                });
            };
            if seen[field_index] {
                return Err(ConstructionError::DuplicateField { field: field.name });
            }
            seen[field_index] = true;
            if matches!(field.policy, ConstructionFieldPolicy::ProviderOnly) {
                return Err(ConstructionError::ProviderOnlyField { field: field.name });
            }
            if let ConstructionFieldPolicy::Unavailable(reason) = field.policy {
                return Err(ConstructionError::FieldUnavailable {
                    field: field.name,
                    reason,
                });
            }
            validate_visibility(field, unchecked_visibility)?;
            validate_value(field, value)?;
            binding.input_indices[field_index] = Some(input_index);
        }
        if check_required {
            for (index, field) in plan.fields.iter().enumerate() {
                if !seen[index] && matches!(field.policy, ConstructionFieldPolicy::Required) {
                    return Err(ConstructionError::MissingField { field: field.name });
                }
            }
        }
    }
    Ok(binding)
}

fn validate_visibility(
    field: &ConstructField,
    unchecked_visibility: bool,
) -> Result<(), ConstructionError> {
    if !unchecked_visibility && !field.visibility.is_public() {
        return Err(ConstructionError::FieldNotPublic { field: field.name });
    }
    Ok(())
}

fn validate_value<M: ValueMode>(
    field: &ConstructField,
    value: &DynamicOwned<M>,
) -> Result<(), ConstructionError> {
    let field_identity = field.value_identity();
    if value.type_id() != field_identity.type_id {
        return Err(ConstructionError::ValueTypeMismatch {
            field: field.name,
            mismatch: TypeMismatch::new(field_identity, value.ty().type_identity()),
        });
    }
    Ok(())
}

fn take_bound_values<M: ValueMode>(
    input: ConstructionInputValue<M>,
    binding: Binding,
) -> Vec<Option<DynamicOwned<M>>> {
    let mut entries = input.into_entries();
    binding
        .input_indices
        .into_iter()
        .map(|index| index.map(|index| entries[index].take_value()))
        .collect()
}
