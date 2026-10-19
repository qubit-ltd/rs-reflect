use std::collections::HashMap;
use std::sync::Mutex;

use qubit_reflect::{Reflect, TypeDescriptor};

#[derive(Clone)]
#[Reflect]
struct Customer {
    pub id: u64,
    pub email: String,
    pub display_name: String,
    pub credit_limit_cents: u64,
}

/// The application repository owns loading and persistence; reflection only
/// applies a checked update to the loaded Rust value.
struct CustomerRepository(Mutex<HashMap<u64, Customer>>);

impl CustomerRepository {
    fn load(&self, id: u64) -> Result<Customer, String> {
        self.0
            .lock()
            .map_err(|error| error.to_string())?
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("customer {id} not found"))
    }

    fn save(&self, customer: &Customer) -> Result<(), String> {
        self.0
            .lock()
            .map_err(|error| error.to_string())?
            .insert(customer.id, customer.clone());
        Ok(())
    }
}

fn apply_email_patch(
    customer: &mut Customer,
    email: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let field = TypeDescriptor::of::<Customer>().field("email")?;
    if let Err(failure) = field.set(customer, email) {
        let (error, _rejected_email) = failure.into_parts();
        return Err(format!("email update was rejected: {error:?}").into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repository = CustomerRepository(Mutex::new(HashMap::from([(
        1001,
        Customer {
            id: 1001,
            email: String::from("ada@example.com"),
            display_name: String::from("Ada Lovelace"),
            credit_limit_cents: 50_000,
        },
    )])));

    let mut customer = repository.load(1001)?;
    apply_email_patch(&mut customer, String::from("ada@corp.example"))?;
    repository.save(&customer)?;

    let updated = repository.load(1001)?;
    assert_eq!(updated.email, "ada@corp.example");
    assert_eq!(updated.display_name, "Ada Lovelace");
    assert_eq!(updated.credit_limit_cents, 50_000);
    Ok(())
}
