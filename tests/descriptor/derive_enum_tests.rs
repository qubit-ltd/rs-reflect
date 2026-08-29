// qubit-style: allow explicit-imports
//! Integration tests for `Reflect` enum derives.
use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;
use reflect::descriptor::DiscriminantOrigin;
use reflect::descriptor::NumericDiscriminant;
use reflect::descriptor::TypeKind;
use reflect::descriptor::VariantKind;
use reflect::registry::ReflectRegistry;
use reflect::value::ReflectedRef;

#[derive(Reflect)]
enum DerivedEvent {
    Ready,
    Number(u8),
    Named { value: String },
}

#[derive(Reflect)]
#[repr(u8)]
enum ReprEvent {
    First = 3,
    Second,
}

#[derive(Reflect)]
enum GenericEvent<T> {
    Value(T),
    Empty,
}

#[test]
fn test_derive_reflect_generic_enum_interns_concrete_instances() {
    let _ = GenericEvent::Value(1_u8);
    let _ = GenericEvent::<String>::Empty;
    let u8_descriptor = TypeDescriptor::of::<GenericEvent<u8>>();
    let text_descriptor = TypeDescriptor::of::<GenericEvent<String>>();
    assert!(!std::ptr::eq(u8_descriptor, text_descriptor));
    assert!(std::ptr::eq(u8_descriptor, TypeDescriptor::of::<GenericEvent<u8>>()));
    assert!(std::ptr::eq(
        u8_descriptor
            .variant("Value")
            .expect("value variant")
            .field_at(0)
            .expect("payload")
            .field_type()
            .as_resolved()
            .expect("resolved payload"),
        TypeDescriptor::of::<u8>(),
    ));
}

/// Verifies unit, tuple, and named variants retain their source shapes.
#[test]
fn test_derive_reflect_builds_enum_variant_descriptors() {
    let descriptor = TypeDescriptor::of::<DerivedEvent>();

    assert_eq!(descriptor.kind(), TypeKind::Enum);
    assert_eq!(
        descriptor.variant("Ready").expect("unit variant").kind(),
        VariantKind::Unit
    );
    assert_eq!(
        descriptor.variant("Number").expect("tuple variant").kind(),
        VariantKind::Tuple
    );
    assert_eq!(
        descriptor.variant("Named").expect("named variant").kind(),
        VariantKind::Struct
    );
    let named = DerivedEvent::Named {
        value: "payload".into(),
    };
    let DerivedEvent::Named { value } = named else {
        panic!("constructed variant must retain payload");
    };
    assert_eq!(value, "payload");
}

/// Verifies generated enum field adapters only expose the active payload.
#[test]
fn test_derive_reflect_enum_field_access_checks_active_variant() {
    let number = TypeDescriptor::of::<DerivedEvent>()
        .variant("Number")
        .expect("tuple variant");
    let value = DerivedEvent::Number(7);

    assert!(number.is_active(ReflectedRef::new(&value)).expect("enum target"));
    assert_eq!(
        number
            .field_at(0)
            .expect("tuple field")
            .get(ReflectedRef::new(&value))
            .expect("active payload")
            .downcast_ref::<u8>(),
        Some(&7)
    );
    assert!(
        number
            .field_at(0)
            .expect("tuple field")
            .get(ReflectedRef::new(&DerivedEvent::Ready))
            .is_err()
    );
}

/// Verifies a concrete derived enum is discoverable through static
/// registration.
#[test]
fn test_derive_reflect_registers_concrete_enum() {
    let registry = ReflectRegistry::initialize().expect("derived enum fragments must validate");

    assert!(std::ptr::eq(
        registry
            .get(TypeDescriptor::of::<DerivedEvent>().type_id())
            .expect("derived enum must be registered"),
        TypeDescriptor::of::<DerivedEvent>(),
    ));
}

/// Verifies explicit and implicit integer-repr discriminants remain exact.
#[test]
fn test_derive_reflect_records_and_reverse_looks_up_integer_discriminants() {
    let descriptor = TypeDescriptor::of::<ReprEvent>();
    let first = descriptor.variant("First").expect("first variant");
    let second = descriptor.variant("Second").expect("second variant");

    assert_eq!(first.discriminant_origin(), DiscriminantOrigin::Explicit);
    assert_eq!(second.discriminant_origin(), DiscriminantOrigin::Implicit);
    assert_eq!(first.numeric_discriminant(), Some(NumericDiscriminant::U8(3)));
    assert_eq!(second.numeric_discriminant(), Some(NumericDiscriminant::U8(4)));
    assert_eq!(
        descriptor
            .variant_by_discriminant(NumericDiscriminant::U8(4))
            .expect("reverse lookup")
            .rust_name(),
        "Second"
    );
}
