// qubit-style: allow explicit-imports
//! Integration tests for built-in descriptor families and interning.
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;
use std::sync::Arc;

use qubit_reflect as reflect;
use reflect::descriptor::FunctionPointerKind;
use reflect::descriptor::MapKind;
use reflect::descriptor::Mutability;
use reflect::descriptor::PrimitiveKind;
use reflect::descriptor::ReferenceKind;
use reflect::descriptor::SequenceKind;
use reflect::descriptor::SetKind;
use reflect::descriptor::SmartPointerKind;
use reflect::descriptor::TextKind;
use reflect::descriptor::TypeDescriptor;
use reflect::descriptor::TypeKind;

/// Verifies scalar and text built-ins retain their exact public category.
#[test]
fn test_builtin_primitive_and_text_descriptors_have_exact_kinds() {
    let boolean = TypeDescriptor::of::<bool>();
    let character = TypeDescriptor::of::<char>();
    let unsigned = TypeDescriptor::of::<u64>();
    let string = TypeDescriptor::of::<String>();
    let text = TypeDescriptor::of::<str>();

    assert_eq!(boolean.kind(), TypeKind::Primitive(PrimitiveKind::Bool));
    assert_eq!(character.kind(), TypeKind::Primitive(PrimitiveKind::Char));
    assert_eq!(unsigned.kind(), TypeKind::Primitive(PrimitiveKind::U64));
    assert_eq!(string.kind(), TypeKind::Text(TextKind::String));
    assert_eq!(text.kind(), TypeKind::Text(TextKind::Str));
}

/// Verifies generic built-ins preserve their family and resolved type
/// arguments.
#[test]
fn test_builtin_container_descriptors_preserve_family_and_arguments() {
    let optional = TypeDescriptor::of::<Option<u32>>();
    let sequence = TypeDescriptor::of::<Vec<String>>();
    let set = TypeDescriptor::of::<HashSet<u16>>();
    let map = TypeDescriptor::of::<BTreeMap<u8, String>>();

    assert!(std::ptr::eq(
        optional
            .as_optional()
            .expect("Option should expose the optional typed view")
            .element_type()
            .as_resolved()
            .expect("Option element should resolve"),
        TypeDescriptor::of::<u32>(),
    ));
    assert_eq!(
        sequence
            .as_sequence()
            .expect("Vec should expose the sequence typed view")
            .kind(),
        SequenceKind::Vec
    );
    assert!(std::ptr::eq(
        sequence
            .as_sequence()
            .expect("Vec should expose the sequence typed view")
            .element_type()
            .as_resolved()
            .expect("Vec element should resolve"),
        TypeDescriptor::of::<String>(),
    ));
    assert_eq!(
        set.as_set().expect("HashSet should expose the set typed view").kind(),
        SetKind::HashSet
    );
    assert_eq!(
        map.as_map().expect("BTreeMap should expose the map typed view").kind(),
        MapKind::BTreeMap
    );
    assert!(std::ptr::eq(
        map.as_map()
            .expect("BTreeMap should expose the map typed view")
            .key_type()
            .as_resolved()
            .expect("BTreeMap key should resolve"),
        TypeDescriptor::of::<u8>(),
    ));
    assert!(std::ptr::eq(
        map.as_map()
            .expect("BTreeMap should expose the map typed view")
            .value_type()
            .as_resolved()
            .expect("BTreeMap value should resolve"),
        TypeDescriptor::of::<String>(),
    ));
}

/// Verifies pointer-like built-ins preserve pointee type and pointer mode.
#[test]
fn test_builtin_pointer_descriptors_preserve_pointee_and_mode() {
    let smart_pointer = TypeDescriptor::of::<Arc<u32>>();
    let shared_reference = TypeDescriptor::of::<&'static u32>();
    let raw_pointer = TypeDescriptor::of::<*mut u32>();
    let slice = TypeDescriptor::of::<[u32]>();

    assert_eq!(
        smart_pointer
            .as_smart_pointer()
            .expect("Arc should expose the smart-pointer typed view")
            .kind(),
        SmartPointerKind::Arc
    );
    assert_eq!(
        shared_reference
            .as_reference()
            .expect("shared reference should expose the reference typed view")
            .kind(),
        ReferenceKind::Shared
    );
    assert_eq!(
        raw_pointer
            .as_raw_pointer()
            .expect("raw pointer should expose the raw-pointer typed view")
            .mutability(),
        Mutability::Mutable
    );
    assert!(std::ptr::eq(
        slice
            .as_slice()
            .expect("slice should expose the slice typed view")
            .element_type()
            .as_resolved()
            .expect("slice element should resolve"),
        TypeDescriptor::of::<u32>(),
    ));
}

/// Verifies tuple, array, and function pointer built-ins expose their precise
/// typed views.
#[test]
fn test_builtin_tuple_array_and_function_descriptors_expose_typed_views() {
    let tuple = TypeDescriptor::of::<(u8, String)>();
    let array = TypeDescriptor::of::<[bool; 3]>();
    let function = TypeDescriptor::of::<fn(u8) -> String>();

    assert_eq!(tuple.as_tuple().expect("tuple typed view should exist").arity(), 2);
    assert_eq!(array.as_array().expect("array typed view should exist").length(), 3);
    assert_eq!(
        function.as_function().expect("function typed view should exist").kind(),
        FunctionPointerKind::Safe
    );
    assert_eq!(
        function
            .as_function()
            .expect("function typed view should exist")
            .parameters()
            .len(),
        1
    );
}

/// Verifies interning gives every concrete type one shared root even when
/// first queried concurrently, without confusing generic arguments.
#[test]
fn test_builtin_interner_is_concurrent_and_distinguishes_generic_arguments() {
    let threads: Vec<_> = (0..16)
        .map(|_| std::thread::spawn(TypeDescriptor::of::<Vec<u32>>))
        .collect();
    let first = TypeDescriptor::of::<Vec<u32>>();

    for thread in threads {
        let descriptor = thread.join().expect("descriptor lookup thread should not panic");
        assert!(std::ptr::eq(first, descriptor));
    }

    assert!(!std::ptr::eq(first, TypeDescriptor::of::<Vec<String>>()));
}

/// Verifies built-in map descriptors cover non-default hasher specializations.
#[test]
fn test_builtin_hash_map_descriptor_supports_custom_hasher() {
    type CustomHashMap = HashMap<u8, String, BuildHasherDefault<DefaultHasher>>;

    let descriptor = TypeDescriptor::of::<CustomHashMap>();

    assert_eq!(
        descriptor
            .as_map()
            .expect("HashMap should expose the map typed view")
            .kind(),
        MapKind::HashMap
    );
}

/// Verifies builtin function descriptors preserve C-variadic signature facts.
#[test]
fn test_builtin_function_descriptor_supports_c_variadic_signatures() {
    type CVariadic = unsafe extern "C" fn(i32, ...) -> i32;

    let variadic = TypeDescriptor::of::<CVariadic>();

    assert!(
        variadic
            .as_function()
            .expect("function typed view should exist")
            .is_variadic()
    );
}
