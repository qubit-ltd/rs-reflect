//! First independent implementation fragment for the cross-crate fixture.

use registry_types::RegistryUser;

pub trait LabelA {
    fn label_a(&self) -> u8;
}

#[qubit_reflect::reflect_impl(external_trait_id = "fixture.registry.label_a")]
impl LabelA for RegistryUser {
    fn label_a(&self) -> u8 {
        self.id + 10
    }
}
