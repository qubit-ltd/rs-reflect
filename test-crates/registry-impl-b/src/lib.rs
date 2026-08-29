//! Second independent implementation fragment for the cross-crate fixture.

use registry_types::RegistryUser;

pub trait LabelB {
    fn label_b(&self) -> u8;
}

#[qubit_reflect::reflect_impl(external_trait_id = "fixture.registry.label_b")]
impl LabelB for RegistryUser {
    fn label_b(&self) -> u8 {
        self.id + 20
    }
}
