//! Application fixture that proves linked dependency fragments are aggregated.

#[cfg(test)]
mod tests {
    use qubit_reflect::Reflect;
    use qubit_reflect::registry::ReflectRegistry;
    use registry_impl_a::LabelA;
    use registry_impl_b::LabelB;
    use registry_types::RegistryUser;

    #[test]
    fn registry_discovers_impl_fragments_from_two_dependencies() {
        let user = RegistryUser { id: 3 };
        assert_eq!(user.label_a(), 13);
        assert_eq!(user.label_b(), 23);

        let registry = ReflectRegistry::initialize().expect("dependency fragments must form one registry");
        let implementations = registry.implementations(RegistryUser::type_descriptor().type_id());
        assert_eq!(implementations.len(), 2);
        assert!(implementations.iter().all(|implementation| implementation.implemented_trait().is_some()));

        let effective = registry.effective_view(RegistryUser::type_descriptor().type_id());
        assert_eq!(effective.implementations().len(), 2);
        assert_eq!(effective.methods().len(), 2);
        let names: Vec<_> = effective
            .methods()
            .iter()
            .map(|method| method.declaration().query_name())
            .collect();
        assert_eq!(names, ["label_a", "label_b"]);
    }
}
