// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// reflect-example-start
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;

use qubit_reflect::FieldAccessError;
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

#[derive(Clone)]
#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub display_name: String,
    pub credit_limit_cents: u64,
}

struct FieldChange {
    field: String,
    value: Box<dyn Any>,
}

enum PatchError {
    UnknownField {
        field: String,
        value: Box<dyn Any>,
    },
    Failed {
        field: String,
        error: FieldAccessError,
        value: Box<dyn Any>,
    },
}

/// Applies changes in order to a reflected target, stopping at the first
/// invalid field or value.
///
/// Earlier successful changes remain applied. Rejected values are returned for
/// unknown or mismatched fields; access failures retain their diagnostic error
/// and original replacement value.
#[allow(clippy::result_large_err)] // The example preserves rejected values inline for callers.
fn apply_patch<T: Reflect>(target: &mut T, changes: Vec<FieldChange>) -> Result<(), PatchError> {
    let descriptor = TypeDescriptor::of::<T>();
    for change in changes {
        let Some(field) = descriptor.field(&change.field) else {
            return Err(PatchError::UnknownField {
                field: change.field,
                value: change.value,
            });
        };
        if let Err(failure) = field.set(target, change.value) {
            let field_name = change.field;
            let (error, recovery) = failure.into_parts();
            let Some(recovery) = recovery else {
                unreachable!("preflight set failures retain the input")
            };
            return Err(PatchError::Failed {
                field: field_name,
                error,
                value: recovery.into_value(),
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
            value: Box::new(String::from("ada@corp.example")),
        }],
    ) {
        Ok(()) => {}
        Err(PatchError::UnknownField { field, value }) => {
            drop(value);
            panic!("unsupported change to field {field}")
        }
        Err(PatchError::Failed { field, error, value }) => {
            drop(value);
            panic!("change to field {field} failed: {error:?}")
        }
    }
    repository.save(&customer).expect("persist");

    let updated = repository.load(1001).expect("reload");
    assert_eq!(updated.email, "ada@corp.example");
    assert_eq!(updated.display_name, "Ada Lovelace");
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use super::Customer;
    use super::CustomerRepository;
    use super::FieldChange;
    use super::PatchError;
    use super::apply_patch;

    #[test]
    fn test_earlier_change_remains_in_memory_when_later_change_has_wrong_type() {
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
        let result = apply_patch(
            &mut customer,
            vec![
                FieldChange {
                    field: String::from("email"),
                    value: Box::new(String::from("ada@corp.example")) as Box<dyn Any>,
                },
                FieldChange {
                    field: String::from("email"),
                    value: Box::new(2002_u64) as Box<dyn Any>,
                },
            ],
        );
        let Err(PatchError::Failed { field, error, value }) = result else {
            panic!("a mismatched type must reject the second change");
        };
        assert_eq!(field, "email");
        assert!(matches!(error, FieldAccessError::ValueTypeMismatch { .. }));
        assert_eq!(value.downcast_ref::<u64>(), Some(&2002));
        assert_eq!(customer.email, "ada@corp.example");
        assert_eq!(customer.id, 1001);
        let stored = repository.load(1001).expect("stored customer");
        assert_eq!(stored.email, "ada@example.com");
        assert_eq!(stored.id, 1001);
    }
}
// reflect-example-end
