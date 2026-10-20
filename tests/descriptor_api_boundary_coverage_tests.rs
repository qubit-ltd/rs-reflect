// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

//! Exercises public descriptor navigation at the crate boundary.

use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::descriptor::GenericParameterKind;
use qubit_reflect::descriptor::MethodDefinitionOwner;
use qubit_reflect::descriptor::TraitCompleteness;
use qubit_reflect::descriptor::VariantShape;
use qubit_reflect::descriptor::Visibility;
use qubit_reflect::lookup::Lookup;

#[Reflect]
trait BoundaryParent {}

#[Reflect]
trait BoundaryContract<T>: BoundaryParent {
    type Output;

    const LIMIT: u32 = 1;

    fn convert(&self, value: T, retry: bool) -> u64;

    fn ready(&self) -> bool {
        true
    }
}

#[Reflect]
struct BoundaryService {
    pub count: u32,
    hidden: u8,
}

#[ReflectImpl(reflected_trait = BoundaryParent)]
impl BoundaryParent for BoundaryService {}

#[ReflectImpl(reflected_trait = BoundaryContract::<u32>)]
impl BoundaryContract<u32> for BoundaryService {
    type Output = u64;

    const LIMIT: u32 = 32;

    fn convert(&self, value: u32, retry: bool) -> u64 {
        u64::from(value + self.count + u32::from(retry))
    }
}

#[Reflect]
struct GenericBoundary<T, const N: usize> {
    pub value: T,
    pub bytes: [u8; N],
}

#[Reflect]
enum BoundaryChoice {
    Empty,
    Counted { count: u16 },
}

fn service() -> BoundaryService {
    BoundaryService { count: 3, hidden: 9 }
}

#[test]
fn trait_and_method_definition_views_preserve_source_declarations() {
    let context = ReflectionContext::global();
    context.initialize().expect("reflected metadata initializes");
    let definition = context
        .trait_definition_by_name(concat!(module_path!(), "::BoundaryContract"))
        .expect("the reflected trait declaration is registered");

    assert_eq!(definition.context().id(), context.id());
    assert_eq!(definition.name(), "BoundaryContract");
    assert!(definition.path().ends_with("::BoundaryContract"));
    assert_eq!(
        definition.visibility(),
        qubit_reflect::descriptor::TraitVisibility::Private
    );
    assert_eq!(definition.completeness(), TraitCompleteness::Complete);
    assert!(!definition.generics().is_empty());
    assert_eq!(definition.generics().parameter("T").unwrap().name(), "T");
    let supertraits = definition.supertraits();
    assert_eq!(supertraits.len(), 1);
    assert!(!supertraits.is_empty());
    let supertrait_expression = supertraits.iter().next().unwrap();
    assert_eq!(supertrait_expression.context().id(), context.id());
    assert!(supertrait_expression.path().ends_with("BoundaryParent"));
    assert_eq!(
        supertrait_expression.completeness(),
        TraitCompleteness::ExternalIncomplete
    );
    assert!(supertrait_expression.find_definition().is_none());
    assert!(supertrait_expression.arguments().is_empty());

    assert_eq!(definition.associated_types().len(), 1);
    assert!(!definition.associated_types().is_empty());
    let output_definition = definition.associated_types().get("Output").unwrap();
    assert_eq!(output_definition.name(), "Output");
    assert_eq!(
        output_definition.visibility(),
        qubit_reflect::descriptor::TraitVisibility::Public
    );
    assert!(output_definition.default().is_none());
    assert!(output_definition.bounds().is_empty());
    assert!(definition.associated_type("missing").is_err());
    assert_eq!(definition.associated_consts().len(), 1);
    assert!(!definition.associated_consts().is_empty());
    let limit_definition = definition.associated_consts().get("LIMIT").unwrap();
    assert_eq!(limit_definition.name(), "LIMIT");
    assert_eq!(
        limit_definition.visibility(),
        qubit_reflect::descriptor::TraitVisibility::Public
    );
    assert!(limit_definition.ty().display().to_string().contains("u32"));
    assert_eq!(format!("{:?}", limit_definition.default()), "Some(UnsignedInteger(1))");
    assert!(definition.associated_const("missing").is_err());

    let methods = definition.methods();
    assert_eq!(methods.context().id(), context.id());
    assert_eq!(methods.len(), 2);
    assert!(!methods.is_empty());
    let convert = methods.get("convert").expect("convert declaration is present");
    assert_eq!(convert.context().id(), context.id());
    assert_eq!(convert.name(), "convert");
    assert_eq!(convert.visibility(), qubit_reflect::descriptor::TraitVisibility::Public);
    assert!(matches!(convert.owner(), MethodDefinitionOwner::Trait(owner) if owner.id() == definition.id()));
    assert_eq!(convert.parameters().len(), 2);
    assert_eq!(
        convert
            .parameters()
            .map(|parameter| parameter.name())
            .collect::<Vec<_>>(),
        [Some("value"), Some("retry")]
    );
    assert_eq!(format!("{:?}", convert.receiver().unwrap().kind()), "Shared");
    assert!(convert.receiver().unwrap().concrete_type().is_none());
    assert!(convert.output().type_expression().is_some());
    assert_eq!(
        convert.qualifiers(),
        qubit_reflect::descriptor::MethodQualifiers::new(false, false, false, false)
    );
    assert!(!convert.has_default_body());
    assert!(convert.generics().is_empty());
    assert!(definition.method("ready").unwrap().has_default_body());
    assert!(definition.method("missing").is_err());

    let application = definition
        .applications()
        .expect("the linked concrete application is available")
        .get(0)
        .expect("the implementation contributes one application");
    assert_eq!(application.context().id(), context.id());
    assert_eq!(application.name(), "BoundaryContract");
    assert!(application.path().contains("BoundaryContract"));
    assert_eq!(application.completeness(), TraitCompleteness::Complete);
    assert_eq!(application.find_definition().unwrap().id(), definition.id());
    assert_eq!(application.arguments().len(), 1);
    let direct = application.direct_supertraits();
    assert_eq!(direct.context().id(), context.id());
    assert_eq!(direct.len(), 1);
    assert!(!direct.is_empty());
    let parent_application = direct.get(0).unwrap();
    assert_eq!(parent_application.name(), "BoundaryParent");
    assert!(parent_application.find_definition().is_none());
    let closure = application.all_supertraits();
    assert_eq!(closure.len(), 1);
    assert_eq!(closure.iter().next().unwrap().name(), "BoundaryParent");
    assert_eq!(format!("{:?}", application.supertrait_completeness()), "Incomplete");
}

#[test]
fn generic_type_descriptors_cover_named_and_positional_access() {
    let descriptor = TypeDescriptor::of::<GenericBoundary<u32, 4>>();
    assert_eq!(descriptor.context().id(), ReflectionContext::global().id());
    assert_eq!(descriptor.type_id(), std::any::TypeId::of::<GenericBoundary<u32, 4>>());
    assert!(!descriptor.is_registered().unwrap());

    let definition = descriptor
        .find_definition()
        .expect("generic source definition is retained");
    assert_eq!(definition.context().id(), descriptor.context().id());
    assert_eq!(definition.name(), "GenericBoundary");
    assert!(definition.path().ends_with("::GenericBoundary"));
    assert_eq!(format!("{:?}", definition.kind()), "Struct(Named)");
    assert!(definition.generics().parameter("missing").is_err());
    assert_eq!(definition.generics().parameter("T").unwrap().index(), 0);
    assert!(definition.instances().unwrap().is_empty());

    let arguments = descriptor.generic_arguments();
    assert_eq!(arguments.context().id(), descriptor.context().id());
    assert_eq!(arguments.len(), 2);
    assert!(!arguments.is_empty());
    assert_eq!(arguments.iter().len(), 2);
    assert!(arguments.get_at(2).is_none());
    assert!(arguments.argument("missing").is_err());
    let type_argument = arguments.argument("T").unwrap().type_argument().unwrap();
    assert_eq!(type_argument.context().id(), descriptor.context().id());
    assert_eq!(type_argument.parameter().unwrap().name(), "T");
    assert_eq!(
        type_argument.find_concrete_type().unwrap().type_id(),
        std::any::TypeId::of::<u32>()
    );
    assert_eq!(
        type_argument.concrete_type().unwrap().type_id(),
        std::any::TypeId::of::<u32>()
    );
    let const_argument = arguments.get("N").unwrap().const_argument().unwrap();
    assert_eq!(const_argument.parameter().unwrap().name(), "N");
    assert_eq!(format!("{:?}", const_argument.expression()), "UnsignedInteger(4)");

    assert!(matches!(
        definition.generics().parameter("T").unwrap().kind(),
        GenericParameterKind::Type { .. }
    ));
    assert!(matches!(
        definition.generics().parameter("N").unwrap().kind(),
        GenericParameterKind::Const { .. }
    ));
}

#[test]
fn field_variant_and_trait_impl_lookups_report_exact_results() {
    let descriptor = TypeDescriptor::of::<BoundaryService>();
    let value = service();

    let fields = descriptor.fields();
    assert_eq!(fields.len(), 2);
    assert!(!fields.is_empty());
    assert_eq!(fields.iter().len(), 2);
    let count = descriptor.field("count").unwrap();
    assert_eq!(count.context().id(), descriptor.context().id());
    assert_eq!(count.declaring_type().type_id(), descriptor.type_id());
    assert_eq!(count.index(), 0);
    assert_eq!(count.name(), "count");
    assert_eq!(count.visibility(), Visibility::Public);
    let access = count.access();
    assert!(access.readable && access.mutable && access.replaceable && access.constructible);
    assert!(count.find_concrete_type().is_some());
    assert!(count.concrete_type().is_ok());
    assert_eq!(*count.get::<BoundaryService, u32>(&value).unwrap(), 3);
    assert!(count.get::<BoundaryService, u8>(&value).is_err());
    assert!(count.get::<u32, u32>(&3).is_err());
    assert!(count.get_mut::<BoundaryService, u32>(&mut service()).is_ok());
    let mut target = service();
    assert!(count.set(&mut target, 8_u32).is_ok());
    assert_eq!(target.count, 8);
    let failed_set = count
        .set(&mut target, 8_u8)
        .expect_err("wrong replacement type is retained");
    assert!(matches!(
        failed_set.error(),
        qubit_reflect::descriptor::FieldSetError::ValueTypeMismatch(_)
    ));
    assert_eq!(*failed_set.replacement(), 8_u8);
    let (set_error, replacement) = failed_set.into_parts();
    assert!(matches!(
        set_error,
        qubit_reflect::descriptor::FieldSetError::ValueTypeMismatch(_)
    ));
    assert_eq!(replacement, 8_u8);
    let hidden = descriptor.field("hidden").unwrap();
    assert_eq!(hidden.visibility(), Visibility::Private);
    assert!(matches!(
        hidden.get::<BoundaryService, u8>(&value),
        Err(qubit_reflect::descriptor::FieldAccessError::NotPublic)
    ));
    assert!(fields.get("absent").is_none());

    let choice = TypeDescriptor::of::<BoundaryChoice>();
    let variants = choice.variants();
    assert_eq!(variants.len(), 2);
    assert!(!variants.is_empty());
    let empty = choice.variant("Empty").unwrap();
    let counted = choice.variant("Counted").unwrap();
    assert_eq!(empty.context().id(), choice.context().id());
    assert_eq!(empty.declaring_type().type_id(), choice.type_id());
    assert_eq!(empty.index(), 0);
    assert_eq!(empty.shape(), VariantShape::Unit);
    assert_eq!(counted.shape(), VariantShape::Struct);
    assert_eq!(counted.fields().len(), 1);
    assert_eq!(
        counted.field("count").unwrap().declaring_variant().unwrap().name(),
        "Counted"
    );
    assert!(empty.is_active(&BoundaryChoice::Empty).unwrap());
    assert!(!empty.is_active(&BoundaryChoice::Counted { count: 1 }).unwrap());
    assert!(empty.is_active(&0_u8).is_err());
    assert!(variants.get("NotHere").is_none());

    let trait_definition = ReflectionContext::global()
        .trait_definition_by_name(concat!(module_path!(), "::BoundaryContract"))
        .unwrap();
    let application = trait_definition.applications().unwrap().get(0).unwrap();
    assert_eq!(<BoundaryService as BoundaryContract<u32>>::LIMIT, 32);
    let implementation = descriptor.trait_impl(application).unwrap();
    assert_eq!(implementation.target().type_id(), descriptor.type_id());
    let method = implementation.method("convert").unwrap();
    assert_eq!(method.context().id(), descriptor.context().id());
    assert_eq!(method.target().type_id(), descriptor.type_id());
    assert_eq!(method.name(), "convert");
    assert!(matches!(
        method.namespace(),
        qubit_reflect::descriptor::MethodNamespace::Trait { .. }
    ));
    assert!(method.qualified_name().to_string().contains("BoundaryContract"));
    assert_eq!(method.definition().name(), "convert");
    assert_eq!(method.visibility(), qubit_reflect::descriptor::TraitVisibility::Public);
    assert!(method.receiver().is_some());
    let parameters = method.parameters();
    assert_eq!(parameters.context().id(), descriptor.context().id());
    assert_eq!(parameters.len(), 2);
    assert!(!parameters.is_empty());
    assert_eq!(parameters.iter().len(), 2);
    let value_parameter = method.parameter("value").unwrap();
    assert_eq!(value_parameter.index(), 0);
    assert_eq!(value_parameter.name(), Some("value"));
    assert!(value_parameter.type_expression().display().to_string().contains("u32"));
    assert_eq!(
        value_parameter.find_concrete_type().unwrap().type_id(),
        std::any::TypeId::of::<u32>()
    );
    assert_eq!(parameters.get_at(1).unwrap().name(), Some("retry"));
    assert!(parameters.get("absent").is_none());
    assert!(method.parameter("absent").is_err());
    assert!(method.output().type_expression().is_some());
    assert_eq!(
        method.qualifiers(),
        qubit_reflect::descriptor::MethodQualifiers::new(false, false, false, false)
    );
    assert!(method.generics().is_empty());
    assert_eq!(format!("{:?}", method.implementation_source()), "Overridden");
    assert_eq!(format!("{:?}", method.availability()), "Callable");
    assert!(method.specializations().is_empty());
    assert!(descriptor.find_trait_impl(application).unwrap().is_some());
    assert!(matches!(
        descriptor.lookup_trait_impl_by_definition(trait_definition).unwrap(),
        Lookup::Unique(_)
    ));
    assert_eq!(
        descriptor
            .trait_impl_by_definition(trait_definition)
            .unwrap()
            .target()
            .type_id(),
        descriptor.type_id()
    );
    let other_type = TypeDescriptor::of::<GenericBoundary<u32, 4>>();
    assert!(other_type.find_trait_impl(application).unwrap().is_none());
    assert!(other_type.trait_impl(application).is_err());

    // Read retained fixture fields so the integration fixture stays warning-free.
    assert_eq!(value.hidden, 9);
    let BoundaryChoice::Counted { count } = (BoundaryChoice::Counted { count: 2 }) else {
        panic!("the fixture expression constructs Counted");
    };
    assert_eq!(count, 2);
}
