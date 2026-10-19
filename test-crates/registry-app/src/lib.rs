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
    use renamed_reflect::{Invocation, Reflect, TypeDescriptor, register_method_binding};

    #[renamed_reflect::Reflect]
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

    register_method_binding!(CrossCrateService::add as fn(&CrossCrateService, u8) -> u8);
    register_method_binding!(
        BoundedConstService::<u16, 5>::score as fn(&BoundedConstService<u16, 5>, u16) -> u64
    );

    #[test]
    fn global_context_discovers_impl_methods_from_two_dependencies() {
        let user = RegistryUser { id: 3 };
        assert_eq!(user.label_a(), 13);
        assert_eq!(user.label_b(), 23);

        let descriptor = TypeDescriptor::of::<RegistryUser>();
        let output = descriptor
            .method("label_a")
            .expect("dependency A method is registered")
            .invoke(Invocation::on_ref(&user))
            .expect("dependency A adapter invokes against the global context")
            .into_owned::<u8>()
            .expect("dependency A returns an owned byte");
        assert_eq!(output, 13);

        let output = descriptor
            .method("label_b")
            .expect("dependency B method is registered")
            .invoke(Invocation::on_ref(&user))
            .expect("dependency B adapter invokes against the global context")
            .into_owned::<u8>()
            .expect("dependency B returns an owned byte");
        assert_eq!(output, 23);
    }

    #[test]
    fn call_site_crate_binds_and_invokes_a_public_dependency_method() {
        let service = CrossCrateService(40);
        let output = TypeDescriptor::of::<CrossCrateService>()
            .method("add")
            .expect("call-site binding is registered")
            .invoke(Invocation::on_ref(&service).arg(2_u8))
            .expect("call-site shim invokes the public dependency method")
            .into_owned::<u8>()
            .expect("public dependency method returns an owned byte");
        assert_eq!(output, 42);
    }

    #[test]
    fn renamed_facade_binds_cross_crate_const_impl_with_trait_bound() {
        let target = BoundedConstService::<u16, 5>(10);
        let output = TypeDescriptor::of::<BoundedConstService<u16, 5>>()
            .method("score")
            .expect("const-generic binding is registered")
            .invoke(Invocation::on_ref(&target).arg(7_u16))
            .expect("the call-site adapter invokes the bounded generic method")
            .into_owned::<u64>()
            .expect("the method returns an owned integer");
        assert_eq!(output, 22);
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
