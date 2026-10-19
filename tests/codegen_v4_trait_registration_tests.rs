use qubit_reflect::{reflect_external_trait, register_trait_object, ReflectImpl};
use qubit_reflect::__private::codegen_v4::descriptor::ReflectionContext;

reflect_external_trait! {
    pub FmtDisplay = std::fmt::Display;
}

register_trait_object!(dyn std::fmt::Display);
#[derive(Clone)]
struct ExternalDisplay { value: u8 }

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for ExternalDisplay {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::get_or_init_with(
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new,
        )
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(ExternalDisplay);

struct UnregisteredExternalDebug;

impl qubit_reflect::__private::codegen_v4::descriptor::Reflect for UnregisteredExternalDebug {
    fn __reflect_registration() -> &'static qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration<Self> {
        qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::get_or_init_with(
            qubit_reflect::__private::codegen_v4::descriptor::TypeRegistration::new,
        )
    }
}

qubit_reflect::__private::codegen_v4::registration::submit_type!(UnregisteredExternalDebug);

#[ReflectImpl]
impl std::fmt::Debug for UnregisteredExternalDebug {
    #[reflect(skip)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("external-debug")
    }
}

#[ReflectImpl]
impl std::fmt::Display for ExternalDisplay {
    #[reflect(skip)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("external")
    }
}

#[test]
fn test_registered_external_trait_identity_is_available_in_context() {
    let definition = ReflectionContext::global()
        .trait_definition_for::<FmtDisplay>()
        .expect("global context builds")
        .expect("external trait marker is linked");

    assert_eq!(definition.path(), "std::fmt::Display");
    assert_eq!(definition.completeness(), qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete);
}

#[test]
fn test_external_impl_source_path_links_to_external_trait_identity() {
    // Building the global immutable context validates the impl's unresolved
    // source expression against the external handle's exact registered path.
    let definition = ReflectionContext::global()
        .external_trait_definition("std::fmt::Display")
        .expect("global context builds with the external impl")
        .expect("Display external identity is registered");
    assert_eq!(definition.path(), "std::fmt::Display");
    assert_eq!(definition.completeness(), qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete);
    let applications = ReflectionContext::global()
        .trait_applications_by_path("std::fmt::Display")
        .expect("external impl source expression resolves by exact path");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "std::fmt::Display");
    assert_eq!(applications[0].completeness(), definition.completeness());
}

#[test]
fn test_external_impl_without_handle_keeps_context_buildable() {
    // `Debug` has no registered identity handle in this fixture. Its impl
    // remains an incomplete source-path record and does not break freezing.
    let context = ReflectionContext::global();
    let applications = context
        .trait_applications_by_path("std::fmt::Debug")
        .expect("unhandled external impl does not invalidate context");
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].path(), "std::fmt::Debug");
    assert_eq!(
        applications[0].completeness(),
        qubit_reflect::__private::codegen_v4::traits::Completeness::ExternalIncomplete,
    );
}

#[test]
fn test_registered_trait_object_is_queryable_by_exact_rust_type() {
    let object = ReflectionContext::global()
        .trait_object::<dyn std::fmt::Display>()
        .expect("global context builds")
        .expect("trait object is linked");

    assert_eq!(object.trait_path(), "dyn std :: fmt :: Display");
}
