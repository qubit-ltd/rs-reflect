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
    use registry_types::BoundedConstService;
    use registry_types::CrossCrateService;
    use registry_types::RegistryDynParent;
    use registry_types::RegistryUser;
    use renamed_reflect::Reflect;
    use renamed_reflect::TypeDescriptor;
    use renamed_reflect::bind_method;
    use renamed_reflect::descriptor::MethodLookup;
    use renamed_reflect::identity::FragmentIdentity;
    use renamed_reflect::invoke::Invocation;
    use renamed_reflect::invoke::InvocationOutput;
    use renamed_reflect::registry::ReflectRegistry;
    use renamed_reflect::registry::RegistrySnapshotBuilder;
    use renamed_reflect::value::DynamicOwned;
    use renamed_reflect::value::Local;

    #[renamed_reflect::Reflect(supertrait(RegistryDynParent), dyn_compatible(RegistryDynParent::Item))]
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

        let registry = ReflectRegistry::initialize().expect("dependency fragments must form one registry");
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
    fn call_site_crate_binds_and_invokes_a_public_dependency_method() {
        let mut builder = RegistrySnapshotBuilder::new();
        builder.include_linked_fragments();
        builder.add_method_binding(
            bind_method!(CrossCrateService::add as fn(&CrossCrateService, u8) -> u8),
            FragmentIdentity::new("registry-app", module_path!(), line!(), 1, "binding", 1),
        );
        let registry = builder
            .build()
            .expect("cross-crate declaration links to the call-site shim");
        let service = CrossCrateService(40);
        let MethodLookup::Unique(method) = CrossCrateService::type_descriptor().methods_named_in(&registry, "add")
        else {
            panic!("dependency method has one bound adapter");
        };
        let InvocationOutput::Owned(output) = method
            .invoke(&registry, Invocation::receiver_ref(&service).arg(2_u8))
            .expect("call-site crate shim invokes the public dependency method")
        else {
            panic!("public dependency method returns an owned value");
        };
        assert_eq!(DynamicOwned::<Local>::downcast_ref(&output), Some(&42_u8));
    }

    #[test]
    fn renamed_facade_binds_cross_crate_const_impl_with_trait_bound() {
        let mut builder = RegistrySnapshotBuilder::new();
        builder.include_linked_fragments();
        builder.add_method_binding(
            bind_method!(BoundedConstService::<u16, 5>::score as fn(&BoundedConstService<u16, 5>, u16) -> u64),
            FragmentIdentity::new("registry-app", module_path!(), line!(), 2, "generic-binding", 1),
        );
        let registry = builder.build().expect("const-generic impl method binding resolves");
        let target = BoundedConstService::<u16, 5>(10);
        let MethodLookup::Unique(method) =
            BoundedConstService::<u16, 5>::type_descriptor().methods_named_in(&registry, "score")
        else {
            panic!("the concrete const-generic method must resolve uniquely");
        };
        let InvocationOutput::Owned(output) = method
            .invoke(&registry, Invocation::receiver_ref(&target).arg(7_u16))
            .expect("the call-site adapter invokes the bounded generic method")
        else {
            panic!("the method returns an owned integer");
        };
        assert_eq!(DynamicOwned::<Local>::downcast_ref(&output), Some(&22_u64));
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
