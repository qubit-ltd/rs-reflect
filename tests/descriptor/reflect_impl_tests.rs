//! Integration coverage for implementation registration expansion.

use std::sync::OnceLock;

use qubit_reflect::descriptor::StructKind;
use qubit_reflect::{Reflect, reflect, reflect_impl};
use qubit_reflect::registry::ReflectRegistry;

struct Sample;

impl Reflect for Sample {
    fn type_descriptor() -> &'static qubit_reflect::TypeDescriptor {
        static DESCRIPTOR: OnceLock<qubit_reflect::TypeDescriptor> = OnceLock::new();
        DESCRIPTOR.get_or_init(|| {
            qubit_reflect::__private::descriptor::struct_type::<Sample>(
                "Sample",
                StructKind::Named,
                &[],
            )
        })
    }
}

trait External {
    fn value(&self) -> u8;
}

#[reflect]
trait Reflected {
    fn reflected_value(&self) -> u8;

    fn default_value(&self) -> u8 {
        13
    }
}

#[reflect_impl]
impl Reflected for Sample {
    fn reflected_value(&self) -> u8 {
        11
    }
}

#[reflect_impl(external_trait_id = "example.external")]
impl External for Sample {
    fn value(&self) -> u8 {
        7
    }
}

#[reflect_impl]
impl Sample {
    fn private_method(&self) -> u8 {
        3
    }

    pub fn public_method() -> u8 {
        5
    }
}

#[test]
fn test_reflect_impl_registers_inherent_and_external_fragments() {
    let registry = ReflectRegistry::initialize().expect("generated impl fragments must validate");
    assert!(registry.get(Sample::type_descriptor().type_id()).is_none());
    assert_eq!(Sample::public_method(), 5);
    assert_eq!(Sample.value(), 7);
    assert_eq!(Sample.reflected_value(), 11);
    assert_eq!(Sample.default_value(), 13);
    assert_eq!(Sample.private_method(), 3);
}
