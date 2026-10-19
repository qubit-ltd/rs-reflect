use qubit_reflect::{Invocation, Reflect, ReflectImpl, TypeDescriptor};

#[Reflect]
struct Customer {
    pub id: u64,
    pub suspended: bool,
    pub suspension_reason: Option<String>,
}

#[ReflectImpl]
impl Customer {
    pub fn suspend(&mut self, reason: String) -> bool {
        if self.suspended {
            return false;
        }
        self.suspended = true;
        self.suspension_reason = Some(reason);
        true
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut customer = Customer {
        id: 1001,
        suspended: false,
        suspension_reason: None,
    };

    let result = TypeDescriptor::of::<Customer>()
        .method("suspend")?
        .invoke(Invocation::on_mut(&mut customer).arg(String::from("chargeback dispute")));
    let output = match result {
        Ok(output) => output,
        Err(failure) => {
            let (error, _recovery) = failure.into_parts();
            return Err(format!("suspension failed: {error:?}").into());
        }
    };
    let changed = output.into_owned::<bool>()?;
    assert!(changed);
    assert!(customer.suspended);
    assert_eq!(
        customer.suspension_reason.as_deref(),
        Some("chargeback dispute")
    );
    Ok(())
}
