// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Application fixture that proves linked dependency fragments are aggregated.

#[cfg(test)]
mod tests {
    use registry_impl_a::LabelA;
    use registry_impl_b::LabelB;
    use registry_types::RegistryDynParent;
    use registry_types::RegistryUser;
    use renamed_reflect::Reflect;
    use renamed_reflect::TypeDescriptor;
    use renamed_reflect::reflect;
    use renamed_reflect::registry::ReflectRegistry;

    #[reflect(supertrait(RegistryDynParent), dyn_compatible(RegistryDynParent::Item))]
    trait RenamedDependencyChild: RegistryDynParent {
        fn child(&self) -> usize;
    }

    struct RenamedDependencyProbe;

    impl RegistryDynParent for RenamedDependencyProbe {
        type Item = u8;

        fn parent(&self) -> u8 {
            1
        }
    }

    impl RenamedDependencyChild for RenamedDependencyProbe {
        fn child(&self) -> usize {
            2
        }
    }

    #[test]
    fn registry_discovers_impl_fragments_from_two_dependencies() {
        let user = RegistryUser { id: 3 };
        assert_eq!(user.label_a(), 13);
        assert_eq!(user.label_b(), 23);

        let registry =
            ReflectRegistry::initialize().expect("dependency fragments must form one registry");
        let implementations = registry.implementations(RegistryUser::type_descriptor().type_id());
        assert_eq!(implementations.len(), 2);
        assert!(
            implementations
                .iter()
                .all(|implementation| { implementation.implemented_trait().is_some() })
        );

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

    #[test]
    fn renamed_runtime_dependency_generates_dyn_supertrait_navigation() {
        assert_eq!(RegistryDynParent::parent(&RenamedDependencyProbe), 1);
        assert_eq!(RenamedDependencyChild::child(&RenamedDependencyProbe), 2);
        let descriptor = TypeDescriptor::of::<dyn RenamedDependencyChild<Item = u8>>();
        assert_eq!(
            descriptor
                .as_trait_object()
                .expect("renamed dependency must generate a dyn descriptor")
                .trait_descriptor()
                .direct_supertraits()[0]
                .rust_name(),
            "RegistryDynParent"
        );
    }
}
