// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Coverage for invoking a reflected trait's default method through a context.

use std::rc::Rc;

use qubit_reflect::Invocation;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::invoke::InvocationErrorKind;
use qubit_reflect::invoke::InvocationUnavailableReason;
use qubit_reflect::lookup::Lookup;
use qubit_reflect::value::Local;
use qubit_reflect::value::ThreadSafe;

#[Reflect]
struct DefaultService;

#[Reflect]
#[derive(Clone)]
struct ConstrainedDefaultService;

#[Reflect]
trait ConstrainedDefaultMethods {
    type Output;

    #[expect(dead_code, reason = "the descriptor records an unavailable constrained default")]
    fn requires_clone(&self) -> usize
    where
        Self: Clone,
    {
        13
    }

    #[expect(dead_code, reason = "the descriptor records an unavailable projected default")]
    fn projected_output(&self) -> Self::Output {
        panic!("the projection is only used to verify conservative adapter availability")
    }

    fn available(&self) -> usize {
        17
    }
}

#[ReflectImpl(reflected_trait = ConstrainedDefaultMethods)]
impl ConstrainedDefaultMethods for ConstrainedDefaultService {
    type Output = String;
}

#[Reflect]
pub trait DefaultRenderer {
    fn render(&self, input: String) -> String {
        format!("rendered: {input}")
    }
}

#[ReflectImpl(reflected_trait = DefaultRenderer)]
impl DefaultRenderer for DefaultService {}

#[Reflect]
trait CanonicalDefaultAliasContract {
    fn alias_default(&self) -> usize {
        31
    }
}

use self::CanonicalDefaultAliasContract as ImportedDefaultAlias;

#[Reflect]
struct DefaultAliasService;

#[ReflectImpl(reflected_trait = self::CanonicalDefaultAliasContract)]
impl ImportedDefaultAlias for DefaultAliasService {}

#[Reflect]
pub trait GenericDefaultRenderer<T: Default + qubit_reflect::descriptor::Reflect> {
    fn generic_render(&self) -> T {
        T::default()
    }
}

#[ReflectImpl(reflected_trait = GenericDefaultRenderer::<String>)]
impl GenericDefaultRenderer<String> for DefaultService {}

#[Reflect]
pub trait ConstDefaultRenderer<const LIMIT: usize> {
    fn const_render(&self) -> usize {
        LIMIT
    }
}

#[ReflectImpl(reflected_trait = ConstDefaultRenderer::<17>)]
impl ConstDefaultRenderer<17> for DefaultService {}

#[Reflect]
pub trait CharConstDefaultRenderer<const VALUE: char> {
    fn char_const_render(&self) -> char {
        VALUE
    }
}

#[ReflectImpl(reflected_trait = CharConstDefaultRenderer::<'x'>)]
impl CharConstDefaultRenderer<'x'> for DefaultService {}

#[Reflect]
struct LifetimeDefaultService;

#[Reflect]
pub trait LifetimeDefaultRenderer<'a> {
    fn context(&self) -> &'a str;

    fn render_lifetime(&self, input: String) -> String {
        format!("{}: {input}", self.context())
    }
}

#[ReflectImpl(reflected_trait = LifetimeDefaultRenderer::<'static>)]
impl LifetimeDefaultRenderer<'static> for LifetimeDefaultService {
    fn context(&self) -> &'static str {
        "lifetime"
    }
}

#[test]
fn test_default_trait_method_can_be_invoked_through_context() {
    let method = ReflectionContext::global()
        .type_of::<DefaultService>()
        .method("render")
        .expect("the reflected default trait method is effective");
    assert_eq!(
        method
            .parameters()
            .get_at(0)
            .and_then(|parameter| parameter.find_concrete_type())
            .map(|ty| ty.type_id()),
        Some(std::any::TypeId::of::<String>()),
        "the default method retains its concrete input type",
    );
    assert_eq!(
        method.output().find_concrete_type().map(|ty| ty.type_id()),
        Some(std::any::TypeId::of::<String>()),
        "the default method retains its concrete output type",
    );
    let service = DefaultService;
    let output = match method.invoke(Invocation::<Local>::on_ref(&service).arg(String::from("example"))) {
        Ok(output) => match output.into_owned::<String>() {
            Ok(value) => value,
            Err(_) => panic!("the default method returns an owned string"),
        },
        Err(_) => panic!("the reflected default method accepts a typed input"),
    };
    assert_eq!(output, "rendered: example");
}

#[test]
fn test_imported_trait_alias_keeps_source_path_and_default_adapter() {
    let context = ReflectionContext::global();
    let applications = context
        .trait_applications_by_path("ImportedDefaultAlias")
        .expect("the impl source alias is registered");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "ImportedDefaultAlias");
    assert_eq!(
        applications[0]
            .find_definition()
            .expect("the explicit canonical provider links the reflected definition")
            .path(),
        concat!(module_path!(), "::CanonicalDefaultAliasContract"),
    );

    let service = DefaultAliasService;
    let method = context
        .type_of::<DefaultAliasService>()
        .method("alias_default")
        .expect("the default method adapter is registered for the imported alias");
    let output = match method.invoke(Invocation::<Local>::on_ref(&service)) {
        Ok(output) => output,
        Err(_) => panic!("the empty impl invokes the reflected trait default adapter"),
    };
    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| { panic!("the default method returns usize") }),
        31,
    );
}

#[Reflect]
struct SkippedOverrideService;

#[Reflect]
trait SkippedOverrideContract {
    fn value(&self) -> usize {
        1
    }

    fn inherited(&self) -> usize {
        3
    }
}

#[ReflectImpl(reflected_trait = SkippedOverrideContract)]
impl SkippedOverrideContract for SkippedOverrideService {
    #[reflect(skip)]
    fn value(&self) -> usize {
        2
    }
}

#[test]
fn test_skipped_trait_override_is_not_exposed_as_default_method() {
    let service = SkippedOverrideService;
    assert_eq!(
        service.value(),
        2,
        "the concrete implementation overrides the trait default"
    );

    let descriptor = ReflectionContext::global().type_of::<SkippedOverrideService>();
    assert!(matches!(
        descriptor.lookup_method("value").expect("method catalog is available"),
        Lookup::Missing,
    ));

    let inherited = descriptor
        .method("inherited")
        .expect("the unrelated default method remains visible");
    assert_eq!(
        inherited.implementation_source(),
        qubit_reflect::descriptor::MethodImplementationSource::Defaulted,
        "only an inherited body is labeled Defaulted",
    );
    let output = match inherited.invoke(Invocation::<Local>::on_ref(&service)) {
        Ok(output) => output,
        Err(_) => panic!("the inherited method remains invocable"),
    };
    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| { panic!("the inherited method returns usize") }),
        3,
    );
}

#[test]
fn test_send_sync_default_trait_method_has_thread_safe_adapter() {
    let method = ReflectionContext::global()
        .type_of::<DefaultService>()
        .method("render")
        .expect("the reflected default trait method is effective");
    let service = DefaultService;
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&service).arg(String::from("shared")))
        .unwrap_or_else(|failure| {
            panic!(
                "a Send + Sync target and owned values have a thread-safe default adapter: {:?}",
                failure.error().kind()
            )
        });

    let output = match output.into_owned::<String>() {
        Ok(output) => output,
        Err(_) => panic!("the default method returns an owned string"),
    };
    assert_eq!(output, "rendered: shared");
}

#[test]
fn test_unproven_default_constraints_keep_descriptors_and_sibling_adapters() {
    let method = ReflectionContext::global()
        .type_of::<ConstrainedDefaultService>()
        .method("requires_clone")
        .expect("the default method descriptor is preserved");
    let service = ConstrainedDefaultService;
    let failure = match method.invoke(Invocation::<Local>::on_ref(&service)) {
        Ok(_) => panic!("the unconstrained provider cannot invoke a Clone-bounded default method"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::MethodUnavailable {
            reasons: &[InvocationUnavailableReason::UnprovenTraitConstraint],
        },
        "the descriptor reports the missing compile-time method bound",
    );

    let projected = ReflectionContext::global()
        .type_of::<ConstrainedDefaultService>()
        .method("projected_output")
        .expect("the associated-type default descriptor is preserved");
    let failure = match projected.invoke(Invocation::<Local>::on_ref(&service)) {
        Ok(_) => panic!("the generic provider cannot prove the associated output is reflectable"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::MethodUnavailable {
            reasons: &[InvocationUnavailableReason::UnprovenTraitConstraint],
        },
        "only methods whose signature depends on the associated type are unavailable",
    );

    let available = ReflectionContext::global()
        .type_of::<ConstrainedDefaultService>()
        .method("available")
        .expect("an unrelated default method remains registered");
    let output = available
        .invoke(Invocation::<ThreadSafe>::on_ref(&service))
        .unwrap_or_else(|_| panic!("an unrelated default method keeps its ThreadSafe adapter"));
    assert_eq!(
        output
            .into_owned::<usize>()
            .unwrap_or_else(|_| panic!("available returns usize")),
        17
    );
}

#[test]
fn test_generic_default_trait_method_has_concrete_thread_safe_adapter() {
    let method = ReflectionContext::global()
        .type_of::<DefaultService>()
        .method("generic_render")
        .expect("the generic trait default method is effective");
    let service = DefaultService;
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "the concrete String application has a ThreadSafe adapter: {:?}",
                failure.error().kind()
            )
        });

    match output.into_owned::<String>() {
        Ok(value) => assert!(value.is_empty()),
        Err(_) => panic!("the generic default method returns String"),
    }
}

#[test]
fn test_const_generic_default_trait_method_has_concrete_thread_safe_adapter() {
    let method = ReflectionContext::global()
        .type_of::<DefaultService>()
        .method("const_render")
        .expect("the const-generic trait default method is effective");
    let service = DefaultService;
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "the concrete const application has a ThreadSafe adapter: {:?}",
                failure.error().kind()
            )
        });

    match output.into_owned::<usize>() {
        Ok(value) => assert_eq!(value, 17),
        Err(_) => panic!("the const-generic default method returns usize"),
    }
}

#[test]
fn test_char_const_default_trait_method_has_concrete_thread_safe_adapter() {
    let method = ReflectionContext::global()
        .type_of::<DefaultService>()
        .method("char_const_render")
        .expect("the char-const trait default method is effective");
    let service = DefaultService;
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "the concrete char const application has a ThreadSafe adapter: {:?}",
                failure.error().kind()
            )
        });

    assert_eq!(
        output
            .into_owned::<char>()
            .unwrap_or_else(|_| panic!("the char-const default method returns char")),
        'x'
    );
}

#[test]
fn test_lifetime_generic_default_method_has_thread_safe_adapter() {
    let method = ReflectionContext::global()
        .type_of::<LifetimeDefaultService>()
        .method("render_lifetime")
        .expect("the lifetime-generic trait default method is effective");
    let service = LifetimeDefaultService;
    let output = method
        .invoke(Invocation::<ThreadSafe>::on_ref(&service).arg(String::from("shared")))
        .unwrap_or_else(|failure| {
            panic!(
                "the concrete 'static lifetime application has a ThreadSafe adapter: {:?}",
                failure.error().kind()
            )
        });

    match output.into_owned::<String>() {
        Ok(value) => assert_eq!(value, "lifetime: shared"),
        Err(_) => panic!("the lifetime-generic default method returns String"),
    }
}

#[Reflect]
struct MixedDefaultService;

#[Reflect]
pub trait MixedDefaultRenderer {
    fn fast(&self) -> usize {
        17
    }

    async fn later(&self) -> usize {
        23
    }
}

#[ReflectImpl(reflected_trait = MixedDefaultRenderer)]
impl MixedDefaultRenderer for MixedDefaultService {}

#[test]
fn test_unsupported_default_method_does_not_disable_thread_safe_siblings() {
    let ty = ReflectionContext::global().type_of::<MixedDefaultService>();
    let service = MixedDefaultService;
    let fast = ty.method("fast").expect("the synchronous default method is effective");
    let output = fast
        .invoke(Invocation::<ThreadSafe>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "the independently adaptable method remains thread-safe: {:?}",
                failure.error().kind()
            )
        });
    match output.into_owned::<usize>() {
        Ok(value) => assert_eq!(value, 17),
        Err(_) => panic!("fast returns usize"),
    }

    let later = ty.method("later").expect("the async default method is effective");
    let failure = match later.invoke(Invocation::<ThreadSafe>::on_ref(&service)) {
        Ok(_) => panic!("the async default method has no ThreadSafe adapter"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.error().kind(),
        &InvocationErrorKind::ValueModeUnavailable,
        "the async method reports that ThreadSafe invocation is unavailable",
    );

    let local = match later.invoke(Invocation::<Local>::on_ref(&service)) {
        Ok(output) => output,
        Err(_) => panic!("the async default method retains its Local adapter"),
    };
    assert!(matches!(local, qubit_reflect::InvocationOutput::Future(_)));
}

#[Reflect]
struct LocalDefaultService {
    marker: Rc<()>,
}

#[Reflect]
struct SendDefaultService;

#[Reflect]
pub trait LocalDefaultRenderer {
    fn render_local(&self) -> usize {
        7
    }
}

#[ReflectImpl(reflected_trait = LocalDefaultRenderer)]
impl LocalDefaultRenderer for LocalDefaultService {}

#[ReflectImpl(reflected_trait = GenericDefaultRenderer::<Rc<()>>)]
impl GenericDefaultRenderer<Rc<()>> for LocalDefaultService {}

#[test]
fn test_non_send_default_trait_target_keeps_local_adapter() {
    let method = ReflectionContext::global()
        .type_of::<LocalDefaultService>()
        .method("render_local")
        .expect("the local default trait method is effective");
    let service = LocalDefaultService { marker: Rc::new(()) };
    assert_eq!(Rc::strong_count(&service.marker), 1);
    let output = method
        .invoke(Invocation::<Local>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "a non-Send target retains its local default adapter: {:?}",
                failure.error().kind()
            )
        });

    let output = match output.into_owned::<usize>() {
        Ok(output) => output,
        Err(_) => panic!("the local default method returns usize"),
    };
    assert_eq!(output, 7);

    let generic_method = ReflectionContext::global()
        .type_of::<LocalDefaultService>()
        .method("generic_render")
        .expect("the generic default method is effective for the local target");
    let local_output = generic_method
        .invoke(Invocation::<Local>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "a local generic trait application retains its Local adapter: {:?}",
                failure.error().kind()
            )
        });
    match local_output.into_owned::<Rc<()>>() {
        Ok(value) => assert_eq!(Rc::strong_count(&value), 1),
        Err(_) => panic!("the generic default method returns Rc<()>"),
    }
}

#[ReflectImpl(reflected_trait = GenericDefaultRenderer::<Rc<()>>)]
impl GenericDefaultRenderer<Rc<()>> for SendDefaultService {}

#[test]
fn test_non_thread_safe_generic_application_keeps_local_default_adapter() {
    let method = ReflectionContext::global()
        .type_of::<SendDefaultService>()
        .method("generic_render")
        .expect("the generic trait default method is effective");
    let service = SendDefaultService;
    let local_output = method
        .invoke(Invocation::<Local>::on_ref(&service))
        .unwrap_or_else(|failure| {
            panic!(
                "the non-ThreadSafe generic application retains its Local adapter: {:?}",
                failure.error().kind()
            )
        });
    match local_output.into_owned::<Rc<()>>() {
        Ok(value) => assert_eq!(Rc::strong_count(&value), 1),
        Err(_) => panic!("the generic default method returns Rc<()>"),
    }

    let thread_safe_failure = match method.invoke(Invocation::<ThreadSafe>::on_ref(&service)) {
        Ok(_) => panic!("Rc<()> has no ThreadSafe default adapter"),
        Err(failure) => failure,
    };
    assert_eq!(
        thread_safe_failure.error().kind(),
        &InvocationErrorKind::ValueModeUnavailable,
    );
}
