// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![cfg(feature = "derive")]

use std::any::TypeId;
use std::sync::LazyLock;

use qubit_reflect::__private::codegen_v3::descriptor::opaque_root;
use qubit_reflect::NamedConstructionInput;
use qubit_reflect::Reflect;
use qubit_reflect::ReflectImpl;
use qubit_reflect::StructUpdateInput;
use qubit_reflect::TupleConstructionInput;
use qubit_reflect::access::FieldAccessError;
use qubit_reflect::access::FieldAccessPolicy;
use qubit_reflect::access::FieldVisibility;
use qubit_reflect::bind_method;
use qubit_reflect::construct::ConstructionError;
use qubit_reflect::descriptor::TraitCompleteness;
use qubit_reflect::descriptor::TraitDefinitionDescriptor;
use qubit_reflect::descriptor::TraitDescriptor;
use qubit_reflect::descriptor::TraitId;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::descriptor::VariantKind;
use qubit_reflect::expression::GenericDefinitionDescriptor;
use qubit_reflect::value::DynamicMut;
use qubit_reflect::value::DynamicOwned;
use qubit_reflect::value::DynamicRef;
use qubit_reflect::value::Local;

#[Reflect]
enum Message {
    Empty,
    Data { id: u32, label: String },
}

#[repr(u8)]
#[Reflect]
enum Code {
    First = 2,
    Second = 9,
}

#[Reflect]
struct DynamicTuple(pub u32, pub String);

#[Reflect]
struct DynamicUnit;

#[Reflect]
#[derive(Debug, PartialEq)]
enum DynamicVariants {
    Empty,
    Named { value: u32 },
    Tuple(u32, String),
}

#[derive(Debug)]
struct NotReflected;

#[Reflect]
#[derive(Debug)]
#[allow(dead_code)]
struct SkippedConstruction {
    pub visible: u32,
    #[reflect(skip)]
    hidden: NotReflected,
}

#[Reflect]
#[allow(dead_code)]
enum SkippedVariantConstruction {
    MissingPayload {
        value: u32,
        #[reflect(skip)]
        hidden: NotReflected,
    },
}

#[Reflect]
#[allow(dead_code)]
struct SkippedTupleConstruction(u32, #[reflect(skip)] NotReflected);

#[Reflect]
#[allow(dead_code)]
enum SkippedTupleVariantConstruction {
    MissingPayload(u32, #[reflect(skip)] NotReflected),
}

#[Reflect]
struct BindingTarget;

#[Reflect(dyn_compatible)]
trait BindingTrait {
    fn label(&self);
}

#[Reflect]
struct FieldRecord {
    pub value: u32,
}

struct HiddenRoot;
static HIDDEN_ROOT: TypeDescriptor = opaque_root::<HiddenRoot>("HiddenRoot");

impl qubit_reflect::Reflect for HiddenRoot {
    fn type_descriptor() -> &'static TypeDescriptor {
        &HIDDEN_ROOT
    }
}

#[ReflectImpl]
impl BindingTarget {
    fn label(&self) {}
}

#[ReflectImpl]
impl BindingTrait for BindingTarget {
    fn label(&self) {}
}

mod generic_fixture {
    use qubit_reflect::Reflect;

    #[Reflect]
    pub struct Envelope<T>(pub T);
}

#[test]
fn variant_descriptor_preserves_shape_discriminant_fields_and_exact_root_checks() {
    let root = TypeDescriptor::of::<Message>();
    let empty = root.variant_at(0).expect("first source variant exists");
    let data = root.variant("Data").expect("Rust name resolves");

    assert_eq!(empty.declaring_type().type_id(), TypeId::of::<Message>());
    assert_eq!(empty.index(), 0);
    assert_eq!(empty.rust_name(), "Empty");
    assert_eq!(empty.kind(), VariantKind::Unit);
    assert_eq!(empty.numeric_discriminant(), None);
    assert_eq!(data.index(), 1);
    assert_eq!(data.rust_name(), "Data");
    assert_eq!(data.query_name(), "Data");
    assert_eq!(data.kind(), VariantKind::Struct);
    assert_eq!(data.numeric_discriminant(), None);
    assert_eq!(data.fields().len(), 2);
    assert_eq!(data.field("id").unwrap().index(), 0);
    assert_eq!(data.field_at(1).unwrap().query_name(), Some("label"));
    assert!(data.field_at(2).is_none());

    let value = Message::Data {
        id: 4,
        label: String::from("x"),
    };
    assert!(data.is_active(&value).expect("exact enum root accepted"));
    assert!(!empty.is_active(&value).expect("inactive variant is false"));
    let mismatch = data.is_active(&4_u32).expect_err("wrong root rejected");
    assert_eq!(mismatch.expected(), TypeId::of::<Message>());
    assert_eq!(mismatch.actual(), TypeId::of::<u32>());
    assert_eq!(
        TypeDescriptor::of::<Code>()
            .variant("First")
            .unwrap()
            .numeric_discriminant(),
        Some(qubit_reflect::descriptor::NumericDiscriminant::U8(2))
    );
}

#[test]
fn concrete_generic_type_views_retain_definition_identity() {
    let u8_type = TypeDescriptor::of::<generic_fixture::Envelope<u8>>();
    let string_type = TypeDescriptor::of::<generic_fixture::Envelope<String>>();
    let u8_generic = u8_type.concrete_generic().expect("generic application retained");
    let string_generic = string_type.concrete_generic().expect("generic application retained");

    assert_ne!(u8_type.type_id(), string_type.type_id());
    assert_eq!(u8_type.definition_id(), string_type.definition_id());
    assert!(std::ptr::eq(u8_generic.definition(), string_generic.definition()));
    assert_eq!(
        u8_type.type_definition().unwrap().id(),
        u8_type.definition_id().unwrap()
    );
    assert_eq!(u8_type.generic_arguments().unwrap().arguments().len(), 1);
    assert_eq!(
        u8_type.as_struct().unwrap().kind(),
        qubit_reflect::descriptor::StructKind::Newtype
    );
    assert_eq!(u8_type.field_at(0).unwrap().index(), 0);
    assert!(u8_type.field("absent").is_none());
}

#[test]
fn trait_descriptor_builder_keeps_declaration_identity_and_empty_application_facts() {
    struct Marker;
    static GENERICS: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
    static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
        TraitDefinitionDescriptor::new(
            TraitId::Reflected(TypeId::of::<Marker>()),
            "MarkerTrait",
            "tests::MarkerTrait",
            "marker_trait",
            TraitCompleteness::Complete,
            &GENERICS,
        )
    });

    let applied = TraitDescriptor::builder(&DEFINITION)
        .build()
        .expect("empty application is valid");
    assert!(std::ptr::eq(applied.definition(), &*DEFINITION));
    assert_eq!(applied.definition().query_name(), "marker_trait");
    assert!(applied.arguments().is_empty());
    assert!(applied.associated_type_arguments().is_empty());
    assert!(applied.direct_supertraits().is_empty());
    assert!(applied.all_supertraits().is_empty());
    assert!(applied.methods().is_empty());
}

#[test]
fn method_binding_identity_distinguishes_trait_ufcs_from_inherent_method() {
    let inherent = bind_method!(BindingTarget::label as fn(&BindingTarget));
    let inherent_again = bind_method!(BindingTarget::label as fn(&BindingTarget));
    let trait_method = bind_method!(<BindingTarget as BindingTrait>::label as fn(&BindingTarget));

    assert_eq!(inherent.identity(), inherent_again.identity());
    assert_ne!(inherent.identity(), trait_method.identity());
    assert_eq!(inherent.identity().target_type_id(), TypeId::of::<BindingTarget>());
    assert_eq!(trait_method.identity().target_type_id(), TypeId::of::<BindingTarget>());
}

#[test]
fn field_descriptor_reports_exact_typed_view_mismatch() {
    let descriptor = TypeDescriptor::of::<BindingTarget>();
    let field = descriptor.field("missing");
    assert!(field.is_none());

    let descriptor = TypeDescriptor::of::<FieldRecord>();
    let field = descriptor.field_at(0).expect("field index resolves");
    let record = FieldRecord { value: 17 };
    assert_eq!(field.index(), 0);
    assert_eq!(field.rust_name(), Some("value"));
    assert_eq!(field.query_name(), Some("value"));
    assert_eq!(field.variant_index(), None);
    assert_eq!(*field.get::<_, u32>(&record).expect("matching type"), 17);

    let error = field
        .get::<_, String>(&record)
        .expect_err("incorrect requested view rejected");
    let FieldAccessError::FieldTypeMismatch { mismatch, .. } = error else {
        panic!("typed access must preserve the field type mismatch category");
    };
    assert_eq!(mismatch.expected(), TypeId::of::<u32>());
    assert_eq!(mismatch.actual(), TypeId::of::<String>());
    assert_eq!(field.declaring_type().type_id(), TypeId::of::<FieldRecord>());
    assert!(matches!(
        field.field_type(),
        qubit_reflect::descriptor::TypeRef::Resolved(_)
    ));
    assert!(matches!(field.visibility(), FieldVisibility::Declared(_)));
    assert_eq!(
        field.visibility().as_declared(),
        Some(&qubit_reflect::identity::Visibility::Public)
    );
    assert_eq!(field.access_policy(), FieldAccessPolicy::ReadWrite);

    let payload = TypeDescriptor::of::<Message>()
        .variant("Data")
        .unwrap()
        .field("id")
        .unwrap();
    assert_eq!(payload.declaring_type().type_id(), TypeId::of::<Message>());
    assert_eq!(payload.variant_index(), Some(1));
    assert_eq!(payload.variant_rust_name(), Some("Data"));
    assert!(matches!(payload.visibility(), FieldVisibility::VariantInherited));
}

#[test]
fn dynamic_field_operations_preserve_borrows_and_classify_target_and_value_errors() {
    let field = TypeDescriptor::of::<FieldRecord>().field("value").unwrap();
    let mut record = FieldRecord { value: 3 };

    let value = field.get_dynamic(DynamicRef::<Local>::new(&record)).unwrap();
    assert_eq!(value.downcast_ref::<u32>(), Some(&3));
    {
        let mut value = field.get_mut_dynamic(DynamicMut::<Local>::new(&mut record)).unwrap();
        *value.downcast_mut::<u32>().unwrap() = 4;
    }
    assert_eq!(record.value, 4);
    field
        .set_dynamic(DynamicMut::<Local>::new(&mut record), DynamicOwned::<Local>::new(5_u32))
        .unwrap();
    assert_eq!(record.value, 5);

    let error = match field.get_dynamic(DynamicRef::<Local>::new(&17_u32)) {
        Err(error) => error,
        Ok(_) => panic!("dynamic read checks exact root type"),
    };
    assert!(matches!(error, FieldAccessError::TargetTypeMismatch { .. }));
    let failure = field
        .set_dynamic(
            DynamicMut::<Local>::new(&mut record),
            DynamicOwned::<Local>::new(String::from("wrong")),
        )
        .expect_err("dynamic set checks exact value type");
    assert!(matches!(failure.error(), FieldAccessError::ValueTypeMismatch { .. }));
    let recovered = failure.into_recovery().unwrap().into_value();
    match recovered.downcast::<String>() {
        Ok(value) => assert_eq!(value, "wrong"),
        Err(_) => panic!("recovery keeps the original replacement type"),
    }
    assert_eq!(record.value, 5);
}

#[test]
fn reflected_root_getters_track_structural_kind_and_lookup_boundaries() {
    use std::collections::HashMap;
    use std::collections::HashSet;

    let primitive = TypeDescriptor::of::<u32>();
    assert!(primitive.as_primitive().is_some());
    assert_eq!(
        primitive.kind(),
        qubit_reflect::descriptor::TypeKind::Primitive(qubit_reflect::descriptor::PrimitiveKind::U32)
    );
    assert!(primitive.as_text().is_none());
    assert!(primitive.as_enum().is_none());
    assert!(primitive.struct_construction().is_none());
    assert_eq!(primitive.type_id(), TypeId::of::<u32>());
    assert!(primitive.type_name().contains("u32"));
    assert!(!primitive.query_name().is_empty());

    let text = TypeDescriptor::of::<String>();
    assert!(text.as_text().is_some());
    assert_eq!(
        text.kind(),
        qubit_reflect::descriptor::TypeKind::Text(text.as_text().unwrap().kind())
    );
    let named = TypeDescriptor::of::<FieldRecord>();
    assert!(named.as_struct().is_some());
    assert!(named.struct_construction().is_some());
    assert_eq!(named.fields().len(), 1);
    assert_eq!(named.field("value").unwrap().index(), 0);
    assert!(named.field_at(1).is_none());
    assert!(named.variants().is_empty());
    assert!(named.type_definition().is_none());
    assert_eq!(named.definition_id(), None);
    assert!(named.concrete_generic().is_none());
    assert!(named.generic_arguments().is_none());

    let enumeration = TypeDescriptor::of::<Code>();
    assert!(enumeration.as_enum().is_some());
    assert_eq!(enumeration.fields().len(), 0);
    assert_eq!(enumeration.variants().len(), 2);
    let first = enumeration.variant("First").expect("first variant exists");
    assert!(std::ptr::eq(first.declaring_type(), enumeration));
    assert_eq!(first.index(), 0);
    assert_eq!(first.rust_name(), "First");
    assert_eq!(first.query_name(), "First");
    assert_eq!(first.kind(), qubit_reflect::descriptor::VariantKind::Unit);
    assert_eq!(
        first.discriminant_origin(),
        qubit_reflect::descriptor::DiscriminantOrigin::Explicit
    );
    assert_eq!(
        first.numeric_discriminant(),
        Some(qubit_reflect::descriptor::NumericDiscriminant::U8(2))
    );
    assert!(first.fields().is_empty());
    assert!(first.field("value").is_none());
    assert!(first.field_at(0).is_none());
    assert!(enumeration.variant_at(2).is_none());
    assert!(enumeration.variant("absent").is_none());
    assert_eq!(
        enumeration
            .variant_by_discriminant(qubit_reflect::descriptor::NumericDiscriminant::U8(9))
            .unwrap()
            .rust_name(),
        "Second"
    );
    assert!(
        enumeration
            .variant_by_discriminant(qubit_reflect::descriptor::NumericDiscriminant::U16(9))
            .is_none()
    );

    let tuple = TypeDescriptor::of::<(u8, String)>();
    assert!(tuple.as_tuple().is_some());
    let array = TypeDescriptor::of::<[u8; 2]>();
    assert!(array.as_array().is_some());
    let optional = TypeDescriptor::of::<Option<u8>>();
    assert!(optional.as_optional().is_some());
    let sequence = TypeDescriptor::of::<Vec<u8>>();
    assert!(sequence.as_sequence().is_some());
    let set = TypeDescriptor::of::<HashSet<u8>>();
    assert!(set.as_set().is_some());
    let map = TypeDescriptor::of::<HashMap<String, u8>>();
    assert!(map.as_map().is_some());
    let smart_pointer = TypeDescriptor::of::<Box<u8>>();
    assert!(smart_pointer.as_smart_pointer().is_some());
    let reference = TypeDescriptor::of::<&'static u8>();
    assert!(reference.as_reference().is_some());
    let slice = TypeDescriptor::of::<[u8]>();
    assert!(slice.as_slice().is_some());
    let pointer = TypeDescriptor::of::<*const u8>();
    assert!(pointer.as_raw_pointer().is_some());
    let function = TypeDescriptor::of::<fn(u8) -> u8>();
    assert!(function.as_function().is_some());
    let trait_object = TypeDescriptor::of::<dyn BindingTrait>();
    assert!(trait_object.as_trait_object().is_some());
    let opaque = TypeDescriptor::of::<HiddenRoot>();
    assert!(opaque.as_opaque().is_some());
    assert!(
        TypeDescriptor::of::<&'static u8>()
            .as_reference()
            .unwrap()
            .target_type()
            .as_resolved()
            .is_some()
    );
}

#[test]
fn dynamic_type_and_variant_constructors_cover_each_supported_shape() {
    let tuple = TypeDescriptor::of::<DynamicTuple>()
        .construct_tuple_any(TupleConstructionInput::new().value(8_u32).value(String::from("tuple")))
        .expect("tuple root supports dynamic construction")
        .downcast::<DynamicTuple>()
        .unwrap_or_else(|_| panic!("dynamic tuple result retains its root type"));
    assert_eq!(tuple.0, 8);
    assert_eq!(tuple.1, "tuple");

    let unit = TypeDescriptor::of::<DynamicUnit>()
        .construct_unit_any()
        .expect("unit root supports dynamic construction")
        .downcast::<DynamicUnit>()
        .unwrap_or_else(|_| panic!("dynamic unit result retains its root type"));
    let _ = unit;

    let variants = TypeDescriptor::of::<DynamicVariants>();
    let empty = variants
        .variant("Empty")
        .unwrap()
        .construct_unit_any()
        .expect("unit variant supports dynamic construction")
        .downcast::<DynamicVariants>()
        .unwrap_or_else(|_| panic!("unit variant result retains the enum root"));
    assert_eq!(*empty, DynamicVariants::Empty);

    let named = variants
        .variant("Named")
        .unwrap()
        .construct_struct_any(NamedConstructionInput::new().field("value", 13_u32))
        .expect("named variant supports dynamic construction")
        .downcast::<DynamicVariants>()
        .unwrap_or_else(|_| panic!("named variant result retains the enum root"));
    assert_eq!(*named, DynamicVariants::Named { value: 13 });

    let tuple = variants
        .variant("Tuple")
        .unwrap()
        .construct_tuple_any(
            TupleConstructionInput::new()
                .value(21_u32)
                .value(String::from("variant")),
        )
        .expect("tuple variant supports dynamic construction")
        .downcast::<DynamicVariants>()
        .unwrap_or_else(|_| panic!("tuple variant result retains the enum root"));
    assert_eq!(*tuple, DynamicVariants::Tuple(21, String::from("variant")));
}

#[test]
fn typed_type_construction_and_update_reject_wrong_roots_before_mutation() {
    let descriptor = TypeDescriptor::of::<FieldRecord>();
    let construction = descriptor
        .construct_struct::<String>(NamedConstructionInput::new().field("value", 7_u32))
        .expect_err("typed constructor requires its exact reflected root");
    assert!(matches!(
        construction.error(),
        ConstructionError::TargetTypeMismatch { .. }
    ));
    let (_, recovered) = construction.into_any_parts();
    assert_eq!(recovered.len(), 1);
    assert!(matches!(
        recovered[0],
        qubit_reflect::construct::RecoveredAnyValue::Named { .. }
    ));

    let update = descriptor
        .update_struct::<String>(StructUpdateInput::new(
            FieldRecord { value: 10 },
            NamedConstructionInput::new().field("value", 11_u32),
        ))
        .expect_err("typed updater requires its exact reflected root");
    assert!(matches!(update.error(), ConstructionError::TargetTypeMismatch { .. }));
    let (_, recovered) = update.into_any_parts();
    assert_eq!(recovered.len(), 2);
    assert!(matches!(
        recovered[0],
        qubit_reflect::construct::RecoveredAnyValue::Base(_)
    ));
}

#[test]
fn unavailable_type_and_variant_constructors_report_target_unavailable() {
    let descriptor = TypeDescriptor::of::<SkippedConstruction>();
    assert!(descriptor.struct_construction().is_none());
    let construction = descriptor
        .construct_struct::<SkippedConstruction>(NamedConstructionInput::new().field("visible", 2_u32))
        .expect_err("skipped Rust field disables whole-root construction");
    assert!(matches!(construction.error(), ConstructionError::TargetUnavailable));

    let update = descriptor
        .update_struct::<SkippedConstruction>(StructUpdateInput::new(
            SkippedConstruction {
                visible: 3,
                hidden: NotReflected,
            },
            NamedConstructionInput::new().field("visible", 4_u32),
        ))
        .expect_err("skipped Rust field disables whole-root updating");
    assert!(matches!(update.error(), ConstructionError::TargetUnavailable));

    let variant = TypeDescriptor::of::<SkippedVariantConstruction>()
        .variant("MissingPayload")
        .expect("variant structure remains reflectable");
    assert!(variant.construction().is_none());
    let construction = variant
        .construct_struct_any(NamedConstructionInput::new().field("value", 9_u32))
        .expect_err("skipped payload disables variant construction");
    assert!(matches!(construction.error(), ConstructionError::TargetUnavailable));
}

fn assert_target_unavailable<T>(
    result: Result<T, qubit_reflect::construct::ConstructionRecovery<Local>>,
) -> Box<[qubit_reflect::construct::RecoveredAnyValue]> {
    let recovery = match result {
        Ok(_) => panic!("missing constructor must report TargetUnavailable"),
        Err(recovery) => recovery,
    };
    let (error, recovered) = recovery.into_any_parts();
    assert!(matches!(error, ConstructionError::TargetUnavailable));
    recovered
}

fn assert_named_recovery(values: &[qubit_reflect::construct::RecoveredAnyValue], first_name: &str) {
    use qubit_reflect::construct::RecoveredAnyValue::Named;

    assert_eq!(values.len(), 2);
    assert!(matches!(
        &values[0],
        Named { name, value }
            if name.as_ref() == first_name
                && value.downcast_ref::<u32>() == Some(&2)
    ));
    assert!(matches!(
        &values[1],
        Named { name, value }
            if name.as_ref() == "extra"
                && value.downcast_ref::<String>().map(String::as_str) == Some("keep")
    ));
}

fn assert_positional_recovery(values: &[qubit_reflect::construct::RecoveredAnyValue]) {
    use qubit_reflect::construct::RecoveredAnyValue::Positional;

    assert_eq!(values.len(), 2);
    assert!(matches!(
        &values[0],
        Positional { index: 0, value } if value.downcast_ref::<u32>() == Some(&2)
    ));
    assert!(matches!(
        &values[1],
        Positional { index: 1, value }
            if value.downcast_ref::<String>().map(String::as_str) == Some("keep")
    ));
}

#[test]
fn unavailable_type_and_variant_constructors_recover_all_eight_safe_inputs() {
    let named_type = TypeDescriptor::of::<SkippedConstruction>();
    let tuple_type = TypeDescriptor::of::<SkippedTupleConstruction>();
    let named_variant = TypeDescriptor::of::<SkippedVariantConstruction>()
        .variant("MissingPayload")
        .expect("named skipped variant remains reflectable");
    let tuple_variant = TypeDescriptor::of::<SkippedTupleVariantConstruction>()
        .variant("MissingPayload")
        .expect("tuple skipped variant remains reflectable");
    assert!(named_type.struct_construction().is_none());
    assert!(tuple_type.struct_construction().is_none());
    assert!(named_variant.construction().is_none());
    assert!(tuple_variant.construction().is_none());

    let values = [
        assert_target_unavailable(
            named_type.construct_struct::<SkippedConstruction>(
                NamedConstructionInput::new()
                    .field("visible", 2_u32)
                    .field("extra", String::from("keep")),
            ),
        ),
        assert_target_unavailable(
            named_type.construct_struct_any(
                NamedConstructionInput::new()
                    .field("visible", 2_u32)
                    .field("extra", String::from("keep")),
            ),
        ),
        assert_target_unavailable(tuple_type.construct_tuple::<SkippedTupleConstruction>(
            TupleConstructionInput::new().value(2_u32).value(String::from("keep")),
        )),
        assert_target_unavailable(
            tuple_type.construct_tuple_any(TupleConstructionInput::new().value(2_u32).value(String::from("keep"))),
        ),
        assert_target_unavailable(
            named_variant.construct_struct::<SkippedVariantConstruction>(
                NamedConstructionInput::new()
                    .field("value", 2_u32)
                    .field("extra", String::from("keep")),
            ),
        ),
        assert_target_unavailable(
            named_variant.construct_struct_any(
                NamedConstructionInput::new()
                    .field("value", 2_u32)
                    .field("extra", String::from("keep")),
            ),
        ),
        assert_target_unavailable(tuple_variant.construct_tuple::<SkippedTupleVariantConstruction>(
            TupleConstructionInput::new().value(2_u32).value(String::from("keep")),
        )),
        assert_target_unavailable(
            tuple_variant.construct_tuple_any(TupleConstructionInput::new().value(2_u32).value(String::from("keep"))),
        ),
    ];
    assert_named_recovery(&values[0], "visible");
    assert_named_recovery(&values[1], "visible");
    assert_positional_recovery(&values[2]);
    assert_positional_recovery(&values[3]);
    assert_named_recovery(&values[4], "value");
    assert_named_recovery(&values[5], "value");
    assert_positional_recovery(&values[6]);
    assert_positional_recovery(&values[7]);
}
