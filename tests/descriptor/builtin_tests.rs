// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow explicit-imports
//! Integration tests for built-in descriptor families and interning.
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::BuildHasherDefault;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_reflect as reflect;
use qubit_reflect::descriptor::FunctionPointerKind;
use qubit_reflect::descriptor::MapKind;
use qubit_reflect::descriptor::Mutability;
use qubit_reflect::descriptor::PrimitiveKind;
use qubit_reflect::descriptor::ReferenceKind;
use qubit_reflect::descriptor::Reflect;
use qubit_reflect::descriptor::SequenceKind;
use qubit_reflect::descriptor::SetKind;
use qubit_reflect::descriptor::SmartPointerKind;
use qubit_reflect::descriptor::StructKind;
use qubit_reflect::descriptor::TextKind;
use qubit_reflect::descriptor::TypeDescriptor;
use qubit_reflect::descriptor::TypeKind;
use qubit_reflect::expression::FunctionAbi;

struct LazyOptionalElement;

static LAZY_OPTIONAL_ELEMENT_INITIALIZATIONS: AtomicUsize = AtomicUsize::new(0);
static LAZY_OPTIONAL_ELEMENT_DESCRIPTOR: TypeDescriptor = reflect::__private::codegen_v2::descriptor::struct_type::<
    LazyOptionalElement,
>("lazy_optional_element", StructKind::Unit, &[]);

impl Reflect for LazyOptionalElement {
    /// Counts and returns the stable descriptor used to observe lazy relation
    /// resolution.
    fn type_descriptor() -> &'static TypeDescriptor {
        LAZY_OPTIONAL_ELEMENT_INITIALIZATIONS.fetch_add(1, Ordering::SeqCst);
        &LAZY_OPTIONAL_ELEMENT_DESCRIPTOR
    }
}

struct LazyBoxPointee;

static LAZY_BOX_POINTEE_INITIALIZATIONS: AtomicUsize = AtomicUsize::new(0);
static LAZY_BOX_POINTEE_DESCRIPTOR: TypeDescriptor = reflect::__private::codegen_v2::descriptor::struct_type::<
    LazyBoxPointee,
>("lazy_box_pointee", StructKind::Unit, &[]);

impl Reflect for LazyBoxPointee {
    /// Counts and returns the stable descriptor used to observe lazy relation
    /// resolution.
    fn type_descriptor() -> &'static TypeDescriptor {
        LAZY_BOX_POINTEE_INITIALIZATIONS.fetch_add(1, Ordering::SeqCst);
        &LAZY_BOX_POINTEE_DESCRIPTOR
    }
}

macro_rules! define_lazy_probe {
    ($type_name:ident, $counter:ident, $descriptor:ident, $query_name:literal) => {
        struct $type_name;

        static $counter: AtomicUsize = AtomicUsize::new(0);
        static $descriptor: TypeDescriptor =
            reflect::__private::codegen_v2::descriptor::struct_type::<$type_name>($query_name, StructKind::Unit, &[]);

        impl Reflect for $type_name {
            /// Counts and returns one stable descriptor used to observe lazy
            /// relation resolution.
            fn type_descriptor() -> &'static TypeDescriptor {
                $counter.fetch_add(1, Ordering::SeqCst);
                &$descriptor
            }
        }
    };
}

define_lazy_probe!(
    LazySequenceElement,
    LAZY_SEQUENCE_CALLS,
    LAZY_SEQUENCE_DESCRIPTOR,
    "lazy_sequence_element"
);
define_lazy_probe!(LazySetElement, LAZY_SET_CALLS, LAZY_SET_DESCRIPTOR, "lazy_set_element");
define_lazy_probe!(LazyMapKey, LAZY_MAP_KEY_CALLS, LAZY_MAP_KEY_DESCRIPTOR, "lazy_map_key");
define_lazy_probe!(
    LazyMapValue,
    LAZY_MAP_VALUE_CALLS,
    LAZY_MAP_VALUE_DESCRIPTOR,
    "lazy_map_value"
);
define_lazy_probe!(
    LazyArrayElement,
    LAZY_ARRAY_CALLS,
    LAZY_ARRAY_DESCRIPTOR,
    "lazy_array_element"
);
define_lazy_probe!(
    LazyTupleFirst,
    LAZY_TUPLE_FIRST_CALLS,
    LAZY_TUPLE_FIRST_DESCRIPTOR,
    "lazy_tuple_first"
);
define_lazy_probe!(
    LazyTupleSecond,
    LAZY_TUPLE_SECOND_CALLS,
    LAZY_TUPLE_SECOND_DESCRIPTOR,
    "lazy_tuple_second"
);
define_lazy_probe!(
    LazyReferenceTarget,
    LAZY_REFERENCE_CALLS,
    LAZY_REFERENCE_DESCRIPTOR,
    "lazy_reference_target"
);
define_lazy_probe!(LazyRawPointee, LAZY_RAW_CALLS, LAZY_RAW_DESCRIPTOR, "lazy_raw_pointee");
define_lazy_probe!(
    LazySliceElement,
    LAZY_SLICE_CALLS,
    LAZY_SLICE_DESCRIPTOR,
    "lazy_slice_element"
);
define_lazy_probe!(
    LazyFunctionParameter,
    LAZY_FUNCTION_PARAMETER_CALLS,
    LAZY_FUNCTION_PARAMETER_DESCRIPTOR,
    "lazy_function_parameter"
);
define_lazy_probe!(
    LazyFunctionReturn,
    LAZY_FUNCTION_RETURN_CALLS,
    LAZY_FUNCTION_RETURN_DESCRIPTOR,
    "lazy_function_return"
);
define_lazy_probe!(
    ConcurrentElementA,
    CONCURRENT_A_CALLS,
    CONCURRENT_A_DESCRIPTOR,
    "concurrent_a"
);
define_lazy_probe!(
    ConcurrentElementB,
    CONCURRENT_B_CALLS,
    CONCURRENT_B_DESCRIPTOR,
    "concurrent_b"
);
define_lazy_probe!(
    ConcurrentMapKey,
    CONCURRENT_KEY_CALLS,
    CONCURRENT_KEY_DESCRIPTOR,
    "concurrent_key"
);
define_lazy_probe!(
    ConcurrentMapValue,
    CONCURRENT_VALUE_CALLS,
    CONCURRENT_VALUE_DESCRIPTOR,
    "concurrent_value"
);

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

/// Verifies the built-in `dyn Debug` root links to an incomplete external
/// trait declaration rather than exposing an unlinked marker view.
#[test]
fn test_builtin_debug_trait_object_links_to_trait_descriptor() {
    let descriptor = TypeDescriptor::of::<dyn std::fmt::Debug>();
    let trait_descriptor = descriptor
        .as_trait_object()
        .expect("dyn Debug must expose a trait-object typed view")
        .trait_descriptor();

    assert_eq!(trait_descriptor.rust_name(), "Debug");
    assert_eq!(trait_descriptor.rust_path(), "std::fmt::Debug");
    assert_eq!(
        trait_descriptor.completeness(),
        reflect::descriptor::TraitCompleteness::ExternalIncomplete
    );
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

/// Verifies `Option<T>` and `Box<T>` publish complete root shapes before their
/// child relationships are resolved.
#[test]
fn test_builtin_optional_and_box_relations_resolve_lazily() {
    assert_eq!(LAZY_OPTIONAL_ELEMENT_INITIALIZATIONS.load(Ordering::SeqCst), 0);
    let optional = TypeDescriptor::of::<Option<LazyOptionalElement>>();
    assert_eq!(LAZY_OPTIONAL_ELEMENT_INITIALIZATIONS.load(Ordering::SeqCst), 0);
    let optional_element = optional
        .as_optional()
        .expect("Option should expose the optional typed view")
        .element_type()
        .as_resolved()
        .expect("Option element should resolve when navigated");
    assert!(std::ptr::eq(optional_element, &LAZY_OPTIONAL_ELEMENT_DESCRIPTOR));
    assert_eq!(LAZY_OPTIONAL_ELEMENT_INITIALIZATIONS.load(Ordering::SeqCst), 1);

    assert_eq!(LAZY_BOX_POINTEE_INITIALIZATIONS.load(Ordering::SeqCst), 0);
    let boxed = TypeDescriptor::of::<Box<LazyBoxPointee>>();
    assert_eq!(LAZY_BOX_POINTEE_INITIALIZATIONS.load(Ordering::SeqCst), 0);
    let pointee = boxed
        .as_smart_pointer()
        .expect("Box should expose the smart-pointer typed view")
        .pointee_type()
        .as_resolved()
        .expect("Box pointee should resolve when navigated");
    assert!(std::ptr::eq(pointee, &LAZY_BOX_POINTEE_DESCRIPTOR));
    assert_eq!(LAZY_BOX_POINTEE_INITIALIZATIONS.load(Ordering::SeqCst), 1);
}

/// Verifies collection roots do not resolve their element, key, or value
/// descriptors before the corresponding typed-view relationship is queried.
#[test]
fn test_builtin_collection_relations_resolve_lazily() {
    let sequence = TypeDescriptor::of::<Vec<LazySequenceElement>>();
    let set = TypeDescriptor::of::<HashSet<LazySetElement>>();
    let map = TypeDescriptor::of::<BTreeMap<LazyMapKey, LazyMapValue>>();
    assert_eq!(LAZY_SEQUENCE_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_SET_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_MAP_KEY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_MAP_VALUE_CALLS.load(Ordering::SeqCst), 0);

    let sequence_element = sequence
        .as_sequence()
        .expect("Vec should expose the sequence view")
        .element_type()
        .as_resolved()
        .expect("Vec element should resolve when navigated");
    let set_element = set
        .as_set()
        .expect("HashSet should expose the set view")
        .element_type()
        .as_resolved()
        .expect("set element should resolve when navigated");
    let map_view = map.as_map().expect("BTreeMap should expose the map view");
    let map_key = map_view
        .key_type()
        .as_resolved()
        .expect("map key should resolve when navigated");
    let map_value = map_view
        .value_type()
        .as_resolved()
        .expect("map value should resolve when navigated");

    assert!(std::ptr::eq(sequence_element, &LAZY_SEQUENCE_DESCRIPTOR));
    assert!(std::ptr::eq(set_element, &LAZY_SET_DESCRIPTOR));
    assert!(std::ptr::eq(map_key, &LAZY_MAP_KEY_DESCRIPTOR));
    assert!(std::ptr::eq(map_value, &LAZY_MAP_VALUE_DESCRIPTOR));
}

/// Verifies array and tuple roots publish before their child descriptors and
/// cache stable resolved slices when navigation begins.
#[test]
fn test_builtin_array_and_tuple_relations_resolve_lazily() {
    let array = TypeDescriptor::of::<[LazyArrayElement; 3]>();
    let tuple = TypeDescriptor::of::<(LazyTupleFirst, LazyTupleSecond)>();
    assert_eq!(LAZY_ARRAY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_TUPLE_FIRST_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_TUPLE_SECOND_CALLS.load(Ordering::SeqCst), 0);

    let array_element = array
        .as_array()
        .expect("array should expose its typed view")
        .element_type()
        .as_resolved()
        .expect("array element should resolve when navigated");
    let elements = tuple.as_tuple().expect("tuple should expose its typed view").elements();
    assert!(std::ptr::eq(array_element, &LAZY_ARRAY_DESCRIPTOR));
    assert!(std::ptr::eq(
        elements[0].as_resolved().expect("first tuple element should resolve"),
        &LAZY_TUPLE_FIRST_DESCRIPTOR,
    ));
    assert!(std::ptr::eq(
        elements[1].as_resolved().expect("second tuple element should resolve"),
        &LAZY_TUPLE_SECOND_DESCRIPTOR,
    ));
}

/// Verifies reference, raw-pointer, and slice roots defer target resolution
/// until their typed relationships are navigated.
#[test]
fn test_builtin_reference_pointer_and_slice_relations_resolve_lazily() {
    let reference = TypeDescriptor::of::<&'static LazyReferenceTarget>();
    let raw = TypeDescriptor::of::<*const LazyRawPointee>();
    let slice = TypeDescriptor::of::<[LazySliceElement]>();
    assert_eq!(LAZY_REFERENCE_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_RAW_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_SLICE_CALLS.load(Ordering::SeqCst), 0);

    let reference_target = reference
        .as_reference()
        .expect("reference should expose its typed view")
        .target_type()
        .as_resolved()
        .expect("reference target should resolve when navigated");
    let raw_pointee = raw
        .as_raw_pointer()
        .expect("raw pointer should expose its typed view")
        .pointee_type()
        .as_resolved()
        .expect("raw pointee should resolve when navigated");
    let slice_element = slice
        .as_slice()
        .expect("slice should expose its typed view")
        .element_type()
        .as_resolved()
        .expect("slice element should resolve when navigated");

    assert!(std::ptr::eq(reference_target, &LAZY_REFERENCE_DESCRIPTOR));
    assert!(std::ptr::eq(raw_pointee, &LAZY_RAW_DESCRIPTOR));
    assert!(std::ptr::eq(slice_element, &LAZY_SLICE_DESCRIPTOR));
}

/// Verifies function roots defer both parameter and return descriptor
/// resolution until signature navigation.
#[test]
fn test_builtin_function_relations_resolve_lazily() {
    type LazyFunction = fn(LazyFunctionParameter) -> LazyFunctionReturn;

    let function = TypeDescriptor::of::<LazyFunction>();
    assert_eq!(LAZY_FUNCTION_PARAMETER_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(LAZY_FUNCTION_RETURN_CALLS.load(Ordering::SeqCst), 0);

    let view = function.as_function().expect("function should expose its typed view");
    let parameter = view.parameters()[0]
        .as_resolved()
        .expect("function parameter should resolve when navigated");
    let return_type = view
        .return_type()
        .as_resolved()
        .expect("function return should resolve when navigated");
    assert!(std::ptr::eq(parameter, &LAZY_FUNCTION_PARAMETER_DESCRIPTOR));
    assert!(std::ptr::eq(return_type, &LAZY_FUNCTION_RETURN_DESCRIPTOR));
}

/// Verifies pointer-like built-ins preserve pointee type and pointer mode.
#[test]
fn test_builtin_pointer_descriptors_preserve_pointee_and_mode() {
    let smart_pointer = TypeDescriptor::of::<Arc<u32>>();
    let shared_reference = TypeDescriptor::of::<&'static u32>();
    let mutable_reference = TypeDescriptor::of::<&'static mut u32>();
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
        mutable_reference
            .as_reference()
            .expect("mutable reference should expose the reference typed view")
            .kind(),
        ReferenceKind::Mutable
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

/// Verifies several distinct composite specializations can be initialized at
/// the same instant without cross-contaminating their identities or roots.
#[test]
fn test_builtin_interner_concurrently_initializes_multiple_composite_types() {
    use std::sync::Barrier;

    type SequenceA = Vec<ConcurrentElementA>;
    type SequenceB = Vec<ConcurrentElementB>;
    type OptionalA = Option<Box<ConcurrentElementA>>;
    type Map = BTreeMap<ConcurrentMapKey, ConcurrentMapValue>;

    let barrier = Arc::new(Barrier::new(33));
    let workers: Vec<_> = (0..32)
        .map(|index| {
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let (group, descriptor) = match index % 4 {
                    0 => (0, TypeDescriptor::of::<SequenceA>()),
                    1 => (1, TypeDescriptor::of::<SequenceB>()),
                    2 => (2, TypeDescriptor::of::<OptionalA>()),
                    _ => (3, TypeDescriptor::of::<Map>()),
                };
                (
                    group,
                    descriptor as *const TypeDescriptor as usize,
                    descriptor.type_id(),
                )
            })
        })
        .collect();
    barrier.wait();

    let mut roots = [None; 4];
    let mut identities = [None; 4];
    for worker in workers {
        let (group, address, identity) = worker.join().expect("composite lookup worker should complete");
        assert_eq!(*roots[group].get_or_insert(address), address);
        assert_eq!(*identities[group].get_or_insert(identity), identity);
    }
    for left in 0..identities.len() {
        for right in left + 1..identities.len() {
            assert_ne!(identities[left], identities[right]);
            assert_ne!(roots[left], roots[right]);
        }
    }
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

/// Verifies both standard set implementations retain their concrete family.
#[test]
fn test_builtin_set_descriptors_preserve_collection_family() {
    let hash_set = TypeDescriptor::of::<HashSet<u8>>();
    let btree_set = TypeDescriptor::of::<BTreeSet<u8>>();

    assert_eq!(hash_set.as_set().expect("HashSet typed view").kind(), SetKind::HashSet,);
    assert_eq!(
        btree_set.as_set().expect("BTreeSet typed view").kind(),
        SetKind::BTreeSet,
    );
}

/// Verifies all supported fixed-arity calling conventions retain safety and
/// ABI metadata.
#[test]
fn test_builtin_function_descriptors_cover_supported_calling_conventions() {
    type SafeRust = fn(u8) -> u16;
    type UnsafeRust = unsafe fn(u8) -> u16;
    type SafeC = extern "C" fn(u8) -> u16;
    type UnsafeC = unsafe extern "C" fn(u8) -> u16;
    type SafeCUnwind = extern "C-unwind" fn(u8) -> u16;
    type UnsafeCUnwind = unsafe extern "C-unwind" fn(u8) -> u16;
    type SafeSystem = extern "system" fn(u8) -> u16;
    type UnsafeSystem = unsafe extern "system" fn(u8) -> u16;
    type SafeSystemUnwind = extern "system-unwind" fn(u8) -> u16;
    type UnsafeSystemUnwind = unsafe extern "system-unwind" fn(u8) -> u16;

    let cases = [
        (
            TypeDescriptor::of::<SafeRust>(),
            FunctionPointerKind::Safe,
            FunctionAbi::Rust,
        ),
        (
            TypeDescriptor::of::<UnsafeRust>(),
            FunctionPointerKind::Unsafe,
            FunctionAbi::Rust,
        ),
        (TypeDescriptor::of::<SafeC>(), FunctionPointerKind::Safe, FunctionAbi::C),
        (
            TypeDescriptor::of::<UnsafeC>(),
            FunctionPointerKind::Unsafe,
            FunctionAbi::C,
        ),
        (
            TypeDescriptor::of::<SafeCUnwind>(),
            FunctionPointerKind::Safe,
            FunctionAbi::Other(String::from("C-unwind").into()),
        ),
        (
            TypeDescriptor::of::<UnsafeCUnwind>(),
            FunctionPointerKind::Unsafe,
            FunctionAbi::Other(String::from("C-unwind").into()),
        ),
        (
            TypeDescriptor::of::<SafeSystem>(),
            FunctionPointerKind::Safe,
            FunctionAbi::System,
        ),
        (
            TypeDescriptor::of::<UnsafeSystem>(),
            FunctionPointerKind::Unsafe,
            FunctionAbi::System,
        ),
        (
            TypeDescriptor::of::<SafeSystemUnwind>(),
            FunctionPointerKind::Safe,
            FunctionAbi::Other(String::from("system-unwind").into()),
        ),
        (
            TypeDescriptor::of::<UnsafeSystemUnwind>(),
            FunctionPointerKind::Unsafe,
            FunctionAbi::Other(String::from("system-unwind").into()),
        ),
    ];

    for (descriptor, expected_kind, expected_abi) in cases {
        let function = descriptor.as_function().expect("function typed view");
        assert_eq!(function.kind(), expected_kind);
        assert_eq!(*function.abi(), expected_abi);
        assert!(!function.is_variadic());
    }
}

/// Verifies builtin function descriptors preserve C-variadic signature facts.
#[test]
fn test_builtin_function_descriptor_supports_c_variadic_signatures() {
    type SafeCVariadic = extern "C" fn(i32, ...) -> i32;
    type UnsafeCVariadic = unsafe extern "C" fn(i32, ...) -> i32;

    let safe_variadic = TypeDescriptor::of::<SafeCVariadic>();
    let unsafe_variadic = TypeDescriptor::of::<UnsafeCVariadic>();

    assert!(safe_variadic.as_function().expect("safe variadic view").is_variadic());
    assert_eq!(
        safe_variadic.as_function().expect("safe variadic view").kind(),
        FunctionPointerKind::Safe,
    );
    assert!(
        unsafe_variadic
            .as_function()
            .expect("unsafe variadic view")
            .is_variadic()
    );
    assert_eq!(
        unsafe_variadic.as_function().expect("unsafe variadic view").kind(),
        FunctionPointerKind::Unsafe,
    );
}

type Tuple31 = (
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
);
type Tuple32 = (
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
);
type Function31 = fn(
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
) -> u8;
type Function32 = fn(
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
    u8,
) -> u8;

/// Verifies the documented maximum arity is implemented at both boundary
/// values for tuples and portable function pointers.
#[test]
fn test_builtin_tuple_and_function_arity_boundaries() {
    for (descriptor, expected) in [
        (TypeDescriptor::of::<Tuple31>(), 31),
        (TypeDescriptor::of::<Tuple32>(), 32),
    ] {
        assert_eq!(descriptor.as_tuple().expect("tuple descriptor").arity(), expected);
    }
    for (descriptor, expected) in [
        (TypeDescriptor::of::<Function31>(), 31),
        (TypeDescriptor::of::<Function32>(), 32),
    ] {
        assert_eq!(
            descriptor
                .as_function()
                .expect("function descriptor")
                .parameters()
                .len(),
            expected
        );
    }
}
