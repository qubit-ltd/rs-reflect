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
    #[path = "multi_version_tests.rs"]
    mod multi_version_tests;

    use registry_impl_a::LabelA;
    use registry_impl_b::LabelB;
    use registry_types::BoundedConstService;
    use registry_types::CrossCrateService;
    use registry_types::RegistryDynParent;
    use registry_types::RegistryUser;
    use renamed_reflect::Invocation;
    use renamed_reflect::ReflectionContext;
    use renamed_reflect::register_method_binding;
    use renamed_reflect::register_trait_object;
    use renamed_reflect::value::Local;

    #[renamed_reflect::Reflect]
    trait RenamedDependencyChild: RegistryDynParent {
        fn child(&self) -> usize;
    }

    register_trait_object!(dyn RenamedDependencyChild<Item = u8>);

    register_method_binding!(BoundedConstService::<u16, 5>::score as fn(&BoundedConstService<u16, 5>, u16) -> u64);

    #[renamed_reflect::Reflect]
    struct RenamedDependencyProbe;

    #[renamed_reflect::ReflectImpl(reflected_trait = registry_types::registry_dyn_parent::RegistryDynParent)]
    impl RegistryDynParent for RenamedDependencyProbe {
        type Item = u8;

        fn parent(&self) -> u8 {
            1
        }
    }

    #[renamed_reflect::ReflectImpl(reflected_trait = RenamedDependencyChild)]
    impl RenamedDependencyChild for RenamedDependencyProbe {
        fn child(&self) -> usize {
            2
        }
    }

    #[test]
    fn global_context_discovers_impl_methods_from_two_dependencies() {
        let context = ReflectionContext::builder()
            .include_linked()
            .build()
            .expect("linked dependency registrations build");
        let user = RegistryUser { id: 3 };
        assert_eq!(user.label_a(), 13);
        assert_eq!(user.label_b(), 23);

        let descriptor = context.type_of::<RegistryUser>();
        let output = descriptor
            .method("label_a")
            .expect("dependency A method is registered")
            .invoke(Invocation::<Local>::on_ref(&user))
            .unwrap_or_else(|failure| panic!("dependency A invocation unavailable: {:?}", failure.error().kind()))
            .into_owned::<u8>()
            .unwrap_or_else(|_| panic!("dependency A returns an owned byte"));
        assert_eq!(output, 13);

        let output = descriptor
            .method("label_b")
            .expect("dependency B method is registered")
            .invoke(Invocation::<Local>::on_ref(&user))
            .unwrap_or_else(|failure| panic!("dependency B invocation unavailable: {:?}", failure.error().kind()))
            .into_owned::<u8>()
            .unwrap_or_else(|_| panic!("dependency B returns an owned byte"));
        assert_eq!(output, 23);
    }

    #[test]
    fn call_site_crate_binds_and_invokes_a_public_dependency_method() {
        let context = ReflectionContext::builder()
            .include_linked()
            .build()
            .expect("linked call-site method binding builds");
        let service = CrossCrateService(40);
        let output = context
            .type_of::<CrossCrateService>()
            .method("add")
            .expect("call-site binding is registered")
            .invoke(Invocation::<Local>::on_ref(&service).arg(2_u8))
            .unwrap_or_else(|failure| panic!("call-site shim invocation unavailable: {:?}", failure.error().kind()))
            .into_owned::<u8>()
            .unwrap_or_else(|_| panic!("public dependency method returns an owned byte"));
        assert_eq!(output, 42);
    }

    #[test]
    fn renamed_facade_binds_cross_crate_const_impl_with_trait_bound() {
        let context = ReflectionContext::builder()
            .include_linked()
            .build()
            .expect("linked const-generic method binding builds");
        let target = BoundedConstService::<u16, 5>(10);
        let output = context
            .type_of::<BoundedConstService<u16, 5>>()
            .method("score")
            .expect("const-generic binding is registered")
            .invoke(Invocation::<Local>::on_ref(&target).arg(7_u16))
            .unwrap_or_else(|failure| {
                panic!(
                    "the call-site adapter invokes the bounded generic method: {:?}",
                    failure.error().kind()
                )
            })
            .into_owned::<u64>()
            .unwrap_or_else(|_| panic!("the method returns an owned integer"));
        assert_eq!(output, 22);
    }

    #[test]
    fn renamed_runtime_dependency_links_default_method_adapter() {
        let context = ReflectionContext::builder()
            .include_linked()
            .build()
            .expect("renamed dependency registrations build");
        assert_eq!(RegistryDynParent::parent(&RenamedDependencyProbe), 1);
        assert_eq!(RenamedDependencyChild::child(&RenamedDependencyProbe), 2);
        let descriptor = context
            .trait_object::<dyn RenamedDependencyChild<Item = u8>>()
            .expect("global context builds")
            .expect("renamed dependency trait object is registered");
        assert_eq!(descriptor.trait_path(), "dyn RenamedDependencyChild < Item = u8 >");
        let definitions = context
            .trait_definitions_by_short_name("RenamedDependencyChild")
            .expect("global context exposes reflected trait definitions");
        assert_eq!(definitions.len(), 1);
        assert_eq!(
            definitions
                .iter()
                .next()
                .unwrap()
                .supertraits()
                .iter()
                .next()
                .unwrap()
                .path(),
            "RegistryDynParent"
        );

        for path in ["RegistryDynParent", "RenamedDependencyChild"] {
            let applications = context
                .trait_applications_by_path(path)
                .expect("reflected trait applications are available");
            assert_eq!(applications.len(), 1, "one application is registered for {path}");
            let application = applications.iter().next().expect("one application exists");
            assert!(
                application.find_definition().is_some(),
                "the explicit reflected trait link resolves for {path}"
            );
            assert!(
                context
                    .type_of::<RenamedDependencyProbe>()
                    .trait_impls()
                    .expect("target trait implementations are available")
                    .find_trait_application(*application)
                    .expect("the exact application lookup completes")
                    .is_some(),
                "the target implementation is linked to {path}"
            );
        }

        let parent_application = context
            .trait_applications_by_path("RegistryDynParent")
            .expect("the parent trait application is registered");
        let parent_application = parent_application
            .iter()
            .next()
            .expect("the parent trait application exists");
        let parent_impl = context
            .type_of::<RenamedDependencyProbe>()
            .trait_impls()
            .expect("target trait implementations are available")
            .find_trait_application(*parent_application)
            .expect("the exact parent application lookup completes")
            .expect("the parent trait implementation is linked");
        let target = RenamedDependencyProbe;
        let value = parent_impl
            .method("default_value")
            .expect("the trait default method is available")
            .invoke(Invocation::<Local>::on_ref(&target))
            .unwrap_or_else(|failure| {
                panic!(
                    "the cross-crate default adapter is unavailable: {:?}",
                    failure.error().kind()
                )
            })
            .into_owned::<u8>()
            .unwrap_or_else(|_| panic!("the default method returns an owned byte"));
        assert_eq!(value, 9);
    }
}
