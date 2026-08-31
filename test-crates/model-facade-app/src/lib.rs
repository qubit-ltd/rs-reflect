//! Terminal fixture: it depends only on the two downstream facade crates.

use model_facade_derive::model_reflect;

#[model_reflect]
/// A model-facade fixture whose descriptor is generated through delegation.
pub struct FacadeUser {
    /// The reflected identifier used by facade integration assertions.
    pub id: u64,
}

#[cfg(test)]
mod tests {
    use model_facade_runtime::descriptor::Reflect;
    use model_facade_runtime::registry::ReflectRegistry;

    use super::FacadeUser;

    #[test]
    fn downstream_facade_delegates_reflect_derive() {
        let descriptor = FacadeUser::type_descriptor();
        assert!(descriptor.type_name().ends_with("FacadeUser"));

        let registry = ReflectRegistry::initialize().expect("facade fragment registers");
        assert!(registry.get(descriptor.type_id()).is_some());
    }
}
