// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// reflect-example-start
use std::collections::HashMap;
use std::sync::Mutex;

use qubit_reflect::FieldAccessError;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectedMut;
use qubit_reflect::ReflectedOwned;
use qubit_reflect::TypeDescriptor;

#[derive(Clone, Reflect)]
#[reflect(crate = qubit_reflect)]
struct Customer {
    #[reflect(read_only)]
    id: u64,
    email: String,
    display_name: String,
    credit_limit_cents: u64,
}

struct FieldChange {
    field: String,
    value: ReflectedOwned,
}

enum PatchError {
    UnknownField {
        field: String,
        value: ReflectedOwned,
    },
    ReadOnly {
        field: String,
        value: ReflectedOwned,
    },
    TypeMismatch {
        field: String,
        value: ReflectedOwned,
    },
    Failed {
        field: String,
        error: Box<FieldAccessError>,
    },
}

/// Applies changes in order to a reflected target, stopping at the first
/// invalid field or value.
///
/// Earlier successful changes remain applied. Rejected values are returned for
/// unknown, read-only, or mismatched fields; other access failures retain their
/// diagnostic error.
fn apply_patch<T: Reflect>(target: &mut T, changes: Vec<FieldChange>) -> Result<(), PatchError> {
    let descriptor = TypeDescriptor::of::<T>();
    for change in changes {
        let Some(field) = descriptor.field(&change.field) else {
            return Err(PatchError::UnknownField {
                field: change.field,
                value: change.value,
            });
        };
        if let Err(failure) = field.set(ReflectedMut::new(target), change.value) {
            let field_name = change.field;
            let (error, recovery) = failure.into_parts();
            return Err(match (error, recovery) {
                (FieldAccessError::ReadOnly { .. }, Some(recovery)) => PatchError::ReadOnly {
                    field: field_name,
                    value: recovery.into_value(),
                },
                (FieldAccessError::ValueTypeMismatch { .. }, Some(recovery)) => PatchError::TypeMismatch {
                    field: field_name,
                    value: recovery.into_value(),
                },
                (error, _) => PatchError::Failed {
                    field: field_name,
                    error: Box::new(error),
                },
            });
        }
    }
    Ok(())
}

struct CustomerRepository(Mutex<HashMap<u64, Customer>>);

impl CustomerRepository {
    /// Clones the stored customer for `id`, returning an error if absent; a
    /// poisoned lock panics.
    fn load(&self, id: u64) -> Result<Customer, String> {
        self.0
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("customer {id} not found"))
    }

    /// Stores a clone of `customer`, replacing its previous record; a poisoned
    /// lock panics.
    fn save(&self, customer: &Customer) -> Result<(), String> {
        self.0.lock().unwrap().insert(customer.id, customer.clone());
        Ok(())
    }
}

/// Runs the example and panics if a business assertion or reflection operation
/// fails.
fn main() {
    let repository = CustomerRepository(Mutex::new(HashMap::from([(
        1001,
        Customer {
            id: 1001,
            email: String::from("ada@example.com"),
            display_name: String::from("Ada Lovelace"),
            credit_limit_cents: 50_000,
        },
    )])));

    let mut customer = repository.load(1001).expect("seed customer");
    match apply_patch(
        &mut customer,
        vec![FieldChange {
            field: String::from("email"),
            value: ReflectedOwned::new(String::from("ada@corp.example")),
        }],
    ) {
        Ok(()) => {}
        Err(PatchError::UnknownField { field, value })
        | Err(PatchError::ReadOnly { field, value })
        | Err(PatchError::TypeMismatch { field, value }) => {
            drop(value);
            panic!("unsupported change to field {field}")
        }
        Err(PatchError::Failed { field, error }) => {
            panic!("change to field {field} failed: {error:?}")
        }
    }
    repository.save(&customer).expect("persist");

    let updated = repository.load(1001).expect("reload");
    assert_eq!(updated.email, "ada@corp.example");
    assert_eq!(updated.display_name, "Ada Lovelace");
}
// reflect-example-end
