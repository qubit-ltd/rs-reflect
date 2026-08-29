// qubit-style: allow explicit-imports
//! Integration tests for `Reflect` struct derives.
use qubit_reflect as reflect;
use reflect::Reflect;
use reflect::TypeDescriptor;
use reflect::descriptor::StructKind;
use reflect::descriptor::TypeKind;
use reflect::registry::ReflectRegistry;
use reflect::value::ReflectedMut;
use reflect::value::ReflectedOwned;
use reflect::value::ReflectedRef;

#[derive(Reflect)]
struct DerivedNamed {
    value: u8,
}

#[derive(Reflect)]
struct DerivedTuple(u8, String);

#[derive(Reflect)]
struct DerivedNewtype(u16);

#[derive(Reflect)]
struct DerivedUnit;

#[derive(Reflect)]
#[reflect(opaque)]
struct DerivedOpaque<T> {
    value: T,
}

#[derive(Reflect)]
struct DerivedGeneric<T> {
    value: T,
}

#[derive(Reflect)]
struct DerivedOpaqueMember<T> {
    #[reflect(opaque)]
    value: T,
}

#[derive(Reflect)]
#[reflect(rename = "renamed-record")]
struct RenamedFields {
    #[reflect(rename = "payload", read_only)]
    pub(crate) value: u8,
    #[reflect(skip)]
    skipped: String,
}

/// Verifies the derive creates a root descriptor with direct named fields.
#[test]
fn test_derive_reflect_builds_named_struct_descriptor() {
    let descriptor = TypeDescriptor::of::<DerivedNamed>();

    assert_eq!(
        descriptor
            .as_struct()
            .expect("derived descriptor must expose a struct view")
            .kind(),
        StructKind::Named
    );
    let field = descriptor
        .field("value")
        .expect("derived named field must be discoverable");
    assert_eq!(field.index(), 0);
    assert_eq!(field.rust_name(), Some("value"));
}

/// Verifies a derived private field uses the safe generated access adapters.
#[test]
fn test_derive_reflect_accesses_private_named_field() {
    let field = TypeDescriptor::of::<DerivedNamed>()
        .field("value")
        .expect("derived field must be discoverable");
    let mut value = DerivedNamed { value: 7 };

    assert_eq!(
        field
            .get(ReflectedRef::new(&value))
            .expect("private field read must succeed")
            .downcast_ref::<u8>(),
        Some(&7)
    );
    field
        .set(ReflectedMut::new(&mut value), ReflectedOwned::new(9_u8))
        .expect("private field replacement must succeed");
    assert_eq!(value.value, 9);
}

/// Verifies tuple, newtype, and unit shapes retain their distinct descriptor
/// views.
#[test]
fn test_derive_reflect_preserves_non_named_struct_shapes() {
    let tuple = TypeDescriptor::of::<DerivedTuple>();
    let newtype = TypeDescriptor::of::<DerivedNewtype>();
    let unit = TypeDescriptor::of::<DerivedUnit>();

    assert_eq!(tuple.as_struct().expect("tuple view").kind(), StructKind::Tuple);
    assert_eq!(newtype.as_struct().expect("newtype view").kind(), StructKind::Newtype);
    assert_eq!(unit.as_struct().expect("unit view").kind(), StructKind::Unit);
    assert_eq!(tuple.field_at(0).expect("tuple field").rust_name(), None);
    assert_eq!(tuple.field_at(1).expect("tuple field").query_name(), None);
}

/// Verifies type-level opacity does not impose reflection bounds on its fields.
#[test]
fn test_derive_reflect_type_level_opaque_supports_unreflectable_generic_members() {
    let descriptor = TypeDescriptor::of::<DerivedOpaque<std::rc::Rc<()>>>();
    let opaque = DerivedOpaque {
        value: std::rc::Rc::new(()),
    };

    assert_eq!(descriptor.kind(), TypeKind::Opaque);
    assert!(descriptor.fields().is_empty());
    assert_eq!(std::rc::Rc::strong_count(&opaque.value), 1);
}

/// Verifies generic specializations receive distinct interned descriptors.
#[test]
fn test_derive_reflect_generic_struct_resolves_field_type_per_specialization() {
    let u8_descriptor = TypeDescriptor::of::<DerivedGeneric<u8>>();
    let u16_descriptor = TypeDescriptor::of::<DerivedGeneric<u16>>();

    assert_ne!(u8_descriptor.type_id(), u16_descriptor.type_id());
    assert!(std::ptr::eq(
        u8_descriptor
            .field_at(0)
            .expect("generic field")
            .field_type()
            .as_resolved()
            .expect("resolved field"),
        TypeDescriptor::of::<u8>(),
    ));
    assert!(std::ptr::eq(
        u16_descriptor
            .field_at(0)
            .expect("generic field")
            .field_type()
            .as_resolved()
            .expect("resolved field"),
        TypeDescriptor::of::<u16>(),
    ));
}

/// Verifies an opaque field does not require its generic argument to implement
/// `Reflect`.
#[test]
fn test_derive_reflect_opaque_generic_field_does_not_require_reflect_bound() {
    let descriptor = TypeDescriptor::of::<DerivedOpaqueMember<std::rc::Rc<()>>>();

    assert!(matches!(
        descriptor.field_at(0).expect("opaque field").field_type(),
        reflect::descriptor::TypeRef::Opaque(_)
    ));
}

/// Verifies a concrete derived root contributes a linker-discovered type
/// fragment.
#[test]
fn test_derive_reflect_registers_concrete_struct() {
    let registry = ReflectRegistry::initialize().expect("derived type fragments must validate");

    assert!(std::ptr::eq(
        registry
            .get(TypeDescriptor::of::<DerivedNamed>().type_id())
            .expect("derived struct must be registered"),
        TypeDescriptor::of::<DerivedNamed>(),
    ));
}

/// Verifies field policies retain source visibility and explicit query names.
#[test]
fn test_derive_reflect_honors_field_names_visibility_and_access_policies() {
    let descriptor = TypeDescriptor::of::<RenamedFields>();
    let payload = descriptor.field("payload").expect("renamed field");
    let skipped = descriptor.field_at(1).expect("skipped field");
    let values = RenamedFields {
        value: 1,
        skipped: "hidden".into(),
    };

    assert_eq!(descriptor.query_name(), "renamed-record");
    assert_eq!(payload.rust_name(), Some("value"));
    assert_eq!(payload.query_name(), Some("payload"));
    assert_eq!(
        payload.visibility().as_declared(),
        Some(&reflect::identity::Visibility::Crate)
    );
    assert_eq!(payload.access_policy(), reflect::access::FieldAccessPolicy::ReadOnly);
    assert_eq!(skipped.access_policy(), reflect::access::FieldAccessPolicy::Skipped);
    assert_eq!(values.skipped, "hidden");
}
