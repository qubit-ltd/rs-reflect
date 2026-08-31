// qubit-style: allow explicit-imports
//! Integration tests for `Reflect` enum derives.
use std::cell::Cell;
use std::rc::Rc;

use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;
use reflect::access::FieldAccessError;
use reflect::descriptor::DiscriminantOrigin;
use reflect::descriptor::EnumRepr;
use reflect::descriptor::NumericDiscriminant;
use reflect::descriptor::TypeKind;
use reflect::descriptor::VariantKind;
use reflect::registry::ReflectRegistry;
use reflect::value::ReflectedMut;
use reflect::value::ReflectedOwned;
use reflect::value::ReflectedRef;

/// Counts destruction of a replacement submitted to an inactive variant.
struct VariantDropProbe(Rc<Cell<usize>>);

impl Drop for VariantDropProbe {
    /// Records when the caller-owned replacement is finally destroyed.
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

#[derive(Reflect)]
enum RecoverableEvent {
    Probe(#[reflect(opaque)] VariantDropProbe),
    Inactive,
}

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
#[repr(C)]
enum CReprEvent {
    First,
    Second,
}

#[derive(Reflect)]
#[repr(u8, C)]
enum CombinedReprEvent {
    Unit = 5,
    Payload(u16),
}

#[derive(Reflect)]
#[repr(Rust)]
enum RustReprEvent {
    Unit,
    Payload(u8),
}

#[derive(Reflect)]
#[repr(transparent)]
enum TransparentReprEvent {
    Payload(u32),
}

#[derive(Reflect)]
#[repr(align(16))]
enum AlignedReprEvent {
    Unit,
    Payload(u8),
}

#[derive(Reflect)]
#[allow(clippy::duplicated_attributes)]
#[repr(u16)]
#[repr(C)]
#[repr(align(8))]
#[repr(C)]
#[repr(align(8))]
enum MultiAttributeReprEvent {
    Unit,
    Payload(u8),
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
    assert_eq!(
        u8_descriptor
            .concrete_generic()
            .expect("generic enum instances expose substitutions")
            .arguments()
            .len(),
        1
    );
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

/// Verifies inactive-variant validation returns the untouched replacement
/// before the generated set adapter consumes it.
#[test]
fn test_derive_reflect_enum_field_set_recovers_inactive_variant_replacement() {
    let field = TypeDescriptor::of::<RecoverableEvent>()
        .variant("Probe")
        .expect("probe variant")
        .field_at(0)
        .expect("probe field");
    let mut target = RecoverableEvent::Inactive;
    let drops = Rc::new(Cell::new(0));

    let failure = field
        .set(
            ReflectedMut::new(&mut target),
            ReflectedOwned::new(VariantDropProbe(Rc::clone(&drops))),
        )
        .expect_err("an inactive variant must reject field replacement");

    assert!(matches!(failure.error(), FieldAccessError::InactiveVariant { .. }));
    assert!(matches!(target, RecoverableEvent::Inactive));
    assert_eq!(drops.get(), 0, "validation must not destroy the replacement");
    let replacement = failure
        .into_recovery()
        .unwrap_or_else(|_| panic!("inactive-variant failure must contain recovery"))
        .into_value_at(0)
        .unwrap_or_else(|_| panic!("replacement must be recoverable by field index"))
        .downcast::<VariantDropProbe>()
        .unwrap_or_else(|_| panic!("recovery must preserve the replacement type"));
    assert_eq!(drops.get(), 0, "taking recovery must preserve ownership");
    drop(replacement);
    assert_eq!(drops.get(), 1, "the caller controls final destruction");
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

/// Verifies enum views expose normalized, structured `repr` declarations.
#[test]
fn test_derive_reflect_records_normalized_enum_representations() {
    assert_eq!(
        TypeDescriptor::of::<ReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::U8],
    );
    assert_eq!(
        TypeDescriptor::of::<CReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::C],
    );
    assert_eq!(
        TypeDescriptor::of::<CombinedReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::C, EnumRepr::U8],
    );
    assert_eq!(
        TypeDescriptor::of::<RustReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::Rust],
    );
    assert_eq!(
        TypeDescriptor::of::<TransparentReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::Transparent],
    );
    assert_eq!(
        TypeDescriptor::of::<AlignedReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::Align(16)],
    );
    assert_eq!(
        TypeDescriptor::of::<MultiAttributeReprEvent>()
            .as_enum()
            .expect("enum view")
            .representations(),
        &[EnumRepr::C, EnumRepr::U16, EnumRepr::Align(8)],
    );
}

/// Verifies non-integer and data-carrying enums do not manufacture numeric
/// discriminant metadata.
#[test]
fn test_derive_reflect_limits_numeric_discriminants_to_fieldless_integer_repr_enums() {
    let c_descriptor = TypeDescriptor::of::<CReprEvent>();
    assert!(
        c_descriptor
            .variants()
            .iter()
            .all(|variant| variant.numeric_discriminant().is_none())
    );
    assert!(
        c_descriptor
            .variant_by_discriminant(NumericDiscriminant::I32(0))
            .is_none()
    );

    let combined_descriptor = TypeDescriptor::of::<CombinedReprEvent>();
    assert!(
        combined_descriptor
            .variants()
            .iter()
            .all(|variant| variant.numeric_discriminant().is_none())
    );
    assert!(
        combined_descriptor
            .variant_by_discriminant(NumericDiscriminant::U8(5))
            .is_none()
    );

    let CombinedReprEvent::Payload(payload) = CombinedReprEvent::Payload(13) else {
        panic!("payload variant must remain constructible");
    };
    assert_eq!(payload, 13);
}
