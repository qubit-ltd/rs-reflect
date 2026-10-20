// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Integration tests for context-bound trait and impl descriptors.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::MethodDefinitionOwner;
use qubit_reflect::descriptor::MethodImplementationSource;
use qubit_reflect::descriptor::MethodNamespace;
use qubit_reflect::descriptor::ReceiverKind;
use qubit_reflect::descriptor::TraitVisibility;
use qubit_reflect::lookup::Lookup;

#[Reflect]
struct Service;

#[Reflect]
trait ServiceContract {
    type Output: qubit_reflect::descriptor::Reflect;

    const LIMIT: u32;

    fn execute(&self, input: u32) -> Self::Output;
}

#[ReflectImpl(reflected_trait = ServiceContract)]
impl ServiceContract for Service {
    type Output = u64;

    const LIMIT: u32 = 16;

    fn execute(&self, input: u32) -> Self::Output {
        u64::from(input)
    }
}

#[Reflect]
trait GenericSource<T> {
    fn source(&self) -> T;
}

#[ReflectImpl(reflected_trait = GenericSource::<u8>)]
impl GenericSource<u8> for Service {
    fn source(&self) -> u8 {
        1
    }
}

#[ReflectImpl(reflected_trait = GenericSource::<String>)]
impl GenericSource<String> for Service {
    fn source(&self) -> String {
        String::from("value")
    }
}

#[Reflect]
#[allow(dead_code)]
trait GenericSupertrait<T> {}

#[Reflect]
#[allow(dead_code)]
trait GenericChild<T>: GenericSupertrait<Vec<T>> {}

#[Reflect]
struct GenericSupertraitService;

impl<T> GenericSupertrait<Vec<T>> for GenericSupertraitService {}

#[ReflectImpl(reflected_trait = GenericChild::<u8>)]
impl GenericChild<u8> for GenericSupertraitService {}

#[ReflectImpl(reflected_trait = GenericChild::<u16>)]
impl GenericChild<u16> for GenericSupertraitService {}

#[Reflect]
#[allow(dead_code)]
trait ConstSupertrait<const N: usize> {}

#[Reflect]
#[allow(dead_code)]
trait ConstChild<const N: usize>: ConstSupertrait<3> {}

#[Reflect]
struct ConstSupertraitService;

impl<const N: usize> ConstSupertrait<N> for ConstSupertraitService {}

#[ReflectImpl(reflected_trait = ConstChild::<3>)]
impl ConstChild<3> for ConstSupertraitService {}

#[ReflectImpl(reflected_trait = ConstChild::<7>)]
impl ConstChild<7> for ConstSupertraitService {}

#[Reflect]
#[allow(dead_code)]
trait BorrowingSupertrait<T> {}

#[Reflect]
#[allow(dead_code)]
trait BorrowingChild<T>: BorrowingSupertrait<T> {}

#[Reflect]
struct BorrowingService;

// Keep the explicit binder to test the implementation for every borrowed
// lifetime.
#[allow(clippy::needless_lifetimes)]
impl<'a> BorrowingSupertrait<&'a str> for BorrowingService {}

// Keep the explicit binder to test the child trait for every borrowed lifetime.
#[allow(clippy::needless_lifetimes)]
#[ReflectImpl(reflected_trait = BorrowingChild::<&'a str>)]
impl<'a> BorrowingChild<&'a str> for BorrowingService {}

#[Reflect]
#[allow(dead_code)]
trait LifetimeParent<T> {}

#[Reflect]
#[allow(dead_code)]
trait LifetimeOnlyChild<'a>: LifetimeParent<&'a str> {}

#[Reflect]
struct LifetimeOnlyService;

impl LifetimeParent<&'static str> for LifetimeOnlyService {}

#[ReflectImpl(reflected_trait = LifetimeOnlyChild::<'static>)]
impl LifetimeOnlyChild<'static> for LifetimeOnlyService {}

#[Reflect]
#[allow(dead_code)]
trait MixedLifetimeParent<T> {}

#[Reflect]
#[allow(dead_code)]
trait MixedLifetimeChild<'a, T>: MixedLifetimeParent<(&'a str, T)> {}

#[Reflect]
struct MixedLifetimeService;

impl MixedLifetimeParent<(&'static str, u8)> for MixedLifetimeService {}

#[ReflectImpl(reflected_trait = MixedLifetimeChild::<'static, u8>)]
impl MixedLifetimeChild<'static, u8> for MixedLifetimeService {}

#[Reflect]
trait SameNameTrait {
    fn duplicate(&self) -> u8;
}

#[Reflect]
trait DefaultMethodContract {
    fn required(&self);

    fn status(&self) -> u8 {
        1
    }
}

#[Reflect]
struct DefaultMethodService;

#[ReflectImpl(reflected_trait = DefaultMethodContract)]
impl DefaultMethodContract for DefaultMethodService {
    fn required(&self) {}
}

#[Reflect]
struct OverriddenMethodService;

#[ReflectImpl(reflected_trait = DefaultMethodContract)]
impl DefaultMethodContract for OverriddenMethodService {
    fn required(&self) {}

    fn status(&self) -> u8 {
        2
    }
}

#[Reflect]
struct GenericDefaultService;

#[Reflect]
trait GenericDefaultContract<T> {
    fn status(&self) -> usize {
        1
    }
}

#[ReflectImpl(reflected_trait = GenericDefaultContract::<u8>)]
impl GenericDefaultContract<u8> for GenericDefaultService {}

#[ReflectImpl(reflected_trait = GenericDefaultContract::<u16>)]
impl GenericDefaultContract<u16> for GenericDefaultService {}

#[ReflectImpl(reflected_trait = SameNameTrait)]
impl SameNameTrait for Service {
    fn duplicate(&self) -> u8 {
        2
    }
}

#[ReflectImpl]
impl Service {
    fn duplicate(&self) -> u8 {
        3
    }
}

/// Verifies a trait definition exposes source methods and associated items.
#[test]
fn test_trait_definition_preserves_declarations() {
    let context = ReflectionContext::global();
    context.initialize().expect("linked trait metadata validates");
    let definition = context
        .trait_definition_by_name(concat!(module_path!(), "::ServiceContract"))
        .expect("the reflected trait definition is discoverable");

    assert_eq!(definition.name(), "ServiceContract");
    assert_eq!(definition.path(), concat!(module_path!(), "::ServiceContract"));
    assert_eq!(definition.methods().len(), 1);
    assert_eq!(definition.method("execute").unwrap().name(), "execute");
    assert_eq!(definition.associated_types().len(), 1);
    assert_eq!(definition.associated_type("Output").unwrap().name(), "Output");
    assert_eq!(definition.associated_consts().len(), 1);
    assert_eq!(definition.associated_const("LIMIT").unwrap().name(), "LIMIT");
}

/// Verifies a concrete type exposes its reflected trait implementation and
/// associated bindings.
#[test]
fn test_trait_impl_preserves_application_and_associated_bindings() {
    assert_eq!(<Service as ServiceContract>::LIMIT, 16);
    let descriptor = TypeDescriptor::of::<Service>();
    let implementation = descriptor
        .trait_impls()
        .expect("linked trait implementations are available")
        .iter()
        .find(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == "ServiceContract")
        })
        .expect("the service contract implementation is present");

    assert_eq!(implementation.target().type_id(), descriptor.type_id());
    assert_eq!(implementation.method("execute").unwrap().name(), "execute");
    assert_eq!(implementation.associated_types().len(), 1);
    assert_eq!(
        implementation.associated_type("Output").unwrap().definition().name(),
        "Output"
    );
    assert_eq!(implementation.associated_consts().len(), 1);
    assert_eq!(
        implementation.associated_const("LIMIT").unwrap().definition().name(),
        "LIMIT"
    );
}

#[test]
fn test_generic_supertrait_applications_use_concrete_impl_arguments() {
    let implementations = TypeDescriptor::of::<GenericSupertraitService>()
        .trait_impls()
        .expect("generic supertrait impl metadata initializes");

    for (child_argument, expected) in [
        (TypeDescriptor::of::<u8>(), TypeDescriptor::of::<Vec<u8>>()),
        (TypeDescriptor::of::<u16>(), TypeDescriptor::of::<Vec<u16>>()),
    ] {
        let implementation = implementations
            .iter()
            .filter_map(|implementation| implementation.implemented_trait())
            .find(|trait_| {
                trait_.name() == "GenericChild"
                    && trait_
                        .arguments()
                        .get("T")
                        .and_then(|argument| argument.type_argument().ok())
                        .and_then(|argument| argument.concrete_type().ok())
                        .is_some_and(|concrete| concrete.type_id() == child_argument.type_id())
            })
            .expect("the requested generic child application exists");
        let application = implementation;
        let supertrait = application
            .direct_supertraits()
            .iter()
            .next()
            .expect("the declared generic supertrait is retained");

        assert_eq!(supertrait.name(), "GenericSupertrait");
        assert_eq!(supertrait.arguments().len(), 1);
        assert_eq!(
            supertrait
                .arguments()
                .get_at(0)
                .unwrap()
                .type_argument()
                .unwrap()
                .concrete_type()
                .unwrap()
                .type_id(),
            expected.type_id()
        );
        assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
    }
}

#[test]
fn test_const_supertrait_applications_use_concrete_impl_arguments() {
    let implementations = TypeDescriptor::of::<ConstSupertraitService>()
        .trait_impls()
        .expect("const supertrait impl metadata initializes");
    let mut values = Vec::new();

    for implementation in implementations.iter().filter(|implementation| {
        implementation
            .implemented_trait()
            .is_some_and(|trait_| trait_.name() == "ConstChild")
    }) {
        let application = implementation
            .implemented_trait()
            .expect("the const child application is retained");
        let supertrait = application
            .direct_supertraits()
            .iter()
            .next()
            .expect("the declared const supertrait is retained");
        let value = supertrait
            .arguments()
            .get_at(0)
            .expect("the const argument is retained")
            .const_argument()
            .expect("the supertrait argument is const");
        values.push(format!("{:?}", value.expression()));
        assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
    }

    assert_eq!(values.len(), 2);
    assert!(
        values.iter().all(|value| value == "SignedInteger(3)"),
        "unexpected const supertrait arguments: {values:?}"
    );
}

/// Verifies generic trait applications remain distinct on one target type.
#[test]
fn test_generic_trait_implementations_preserve_each_application() {
    let implementations: Vec<_> = TypeDescriptor::of::<Service>()
        .trait_impls()
        .expect("linked trait implementations are available")
        .iter()
        .filter(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == "GenericSource")
        })
        .collect();

    assert_eq!(implementations.len(), 2);
    let mut type_arguments = Vec::new();
    for implementation in implementations {
        let application = implementation.implemented_trait().expect("trait application exists");
        let argument = application
            .arguments()
            .get("T")
            .expect("the generic trait application retains T")
            .type_argument()
            .expect("T is a type argument");
        assert_eq!(argument.parameter().map(|parameter| parameter.name()), Some("T"));
        type_arguments.push(argument.find_concrete_type().unwrap().type_id());
        assert_eq!(implementation.method("source").unwrap().name(), "source");
    }
    assert_eq!(type_arguments.len(), 2);
    assert!(type_arguments.contains(&std::any::TypeId::of::<String>()));
    assert!(type_arguments.contains(&std::any::TypeId::of::<u8>()));
}

/// Verifies an impl exposes whether a trait default body was inherited or
/// replaced by a concrete method.
#[test]
fn test_trait_method_reports_default_and_override_sources() {
    let defaulted = TypeDescriptor::of::<DefaultMethodService>()
        .trait_impls()
        .expect("linked implementations are available")
        .iter()
        .find(|implementation| implementation.implemented_trait().is_some())
        .expect("the default method trait implementation is linked")
        .method("status")
        .expect("the inherited default method is effective");
    let overridden = TypeDescriptor::of::<OverriddenMethodService>()
        .trait_impls()
        .expect("linked implementations are available")
        .iter()
        .find(|implementation| implementation.implemented_trait().is_some())
        .expect("the overriding trait implementation is linked")
        .method("status")
        .expect("the overridden method is effective");

    assert_eq!(defaulted.implementation_source(), MethodImplementationSource::Defaulted);
    assert_eq!(
        overridden.implementation_source(),
        MethodImplementationSource::Overridden
    );
}

/// Verifies methods from separate namespaces retain explicit ambiguity.
#[test]
fn test_trait_and_inherent_methods_keep_separate_namespaces() {
    let descriptor = TypeDescriptor::of::<Service>();

    assert!(matches!(
        descriptor.lookup_method("duplicate").unwrap(),
        Lookup::Ambiguous(_)
    ));
    assert!(matches!(descriptor.lookup_method("missing").unwrap(), Lookup::Missing));
}

#[test]
fn test_effective_method_exposes_definition_visibility_receiver_and_owner() {
    let inherent = TypeDescriptor::of::<Service>()
        .inherent_impls()
        .expect("inherent impl applications are available")
        .iter()
        .find_map(|implementation| implementation.methods().get("duplicate"))
        .expect("the inherent method is reflected");
    let inherent_definition = inherent.definition();
    assert!(matches!(inherent_definition.owner(), MethodDefinitionOwner::Impl(_)));
    assert_eq!(inherent.visibility(), TraitVisibility::Private);
    assert_eq!(inherent_definition.visibility(), TraitVisibility::Private);
    assert_eq!(
        inherent.receiver().expect("method has a receiver").kind(),
        ReceiverKind::Shared
    );

    let defaulted = TypeDescriptor::of::<DefaultMethodService>()
        .trait_impls()
        .expect("trait impl applications are available")
        .iter()
        .find(|implementation| implementation.implemented_trait().is_some())
        .expect("the default method trait implementation is linked")
        .method("status")
        .expect("the inherited method is effective");
    let default_definition = defaulted.definition();
    assert!(matches!(default_definition.owner(), MethodDefinitionOwner::Trait(_)));
    assert_eq!(defaulted.visibility(), TraitVisibility::Public);
    assert_eq!(default_definition.visibility(), TraitVisibility::Public);
    assert_eq!(
        defaulted.receiver().expect("method has a receiver").kind(),
        ReceiverKind::Shared
    );
}

#[test]
fn test_generic_default_methods_keep_their_exact_impl_application() {
    let implementations: Vec<_> = TypeDescriptor::of::<GenericDefaultService>()
        .trait_impls()
        .expect("generic trait impl applications are available")
        .iter()
        .filter(|implementation| {
            implementation
                .implemented_trait()
                .is_some_and(|trait_| trait_.name() == "GenericDefaultContract")
        })
        .collect();
    assert_eq!(implementations.len(), 2);

    let mut argument_types = Vec::new();
    for implementation in &implementations {
        let application = implementation.implemented_trait().expect("trait application exists");
        argument_types.push(
            application
                .arguments()
                .get("T")
                .expect("generic argument T is retained")
                .type_argument()
                .expect("T is a type argument")
                .find_concrete_type()
                .expect("T resolves to a concrete type")
                .type_id(),
        );

        let method = implementation
            .method("status")
            .expect("the default method is effective");
        match method.namespace() {
            MethodNamespace::Trait {
                implementation: owner, ..
            } => {
                assert!(owner.same_application(*implementation));
            }
            MethodNamespace::Inherent { .. } => {
                panic!("a trait default method has a trait namespace")
            }
        }
        assert!(matches!(method.definition().owner(), MethodDefinitionOwner::Trait(_)));
        assert!(format!("{}", method.qualified_name()).contains("GenericDefaultContract"));
    }
    assert!(argument_types.contains(&std::any::TypeId::of::<u8>()));
    assert!(argument_types.contains(&std::any::TypeId::of::<u16>()));

    let candidates = TypeDescriptor::of::<GenericDefaultService>()
        .lookup_method("status")
        .expect("the generic default method catalog is available");
    assert!(matches!(candidates, Lookup::Ambiguous(_)));
    let mut matched = vec![false; implementations.len()];
    for method in candidates.candidates() {
        let MethodNamespace::Trait {
            implementation: owner, ..
        } = method.namespace()
        else {
            panic!("a generic trait default method has a trait namespace");
        };
        let index = implementations
            .iter()
            .position(|expected| owner.same_application(*expected))
            .expect("the method namespace must retain its exact trait impl application");
        assert!(
            !matched[index],
            "each generic application contributes one method candidate"
        );
        matched[index] = true;
        assert!(matches!(method.definition().owner(), MethodDefinitionOwner::Trait(_)));
        assert!(format!("{}", method.qualified_name()).contains("GenericDefaultContract"));
    }
    assert!(matched.into_iter().all(|matched| matched));
}
