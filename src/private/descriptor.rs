//! Hidden factories for immutable static descriptor data.

use crate::descriptor::FieldDescriptor;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::MapKind;
use crate::descriptor::Mutability;
use crate::descriptor::OpaqueTypeDescriptor;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::ReferenceKind;
use crate::descriptor::SequenceKind;
use crate::descriptor::SetKind;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::StructKind;
use crate::descriptor::TextKind;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::descriptor::TypeRef;
use crate::descriptor::VariantDescriptor;
use crate::descriptor::VariantKind;
use crate::expression::FunctionAbi;
use crate::identity::Visibility;

/// Creates a primitive root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn primitive<T: ?Sized + 'static>(query_name: &'static str, kind: PrimitiveKind) -> TypeDescriptor {
    TypeDescriptor::new_primitive::<T>(query_name, kind)
}

/// Creates a text root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn text<T: ?Sized + 'static>(query_name: &'static str, kind: TextKind) -> TypeDescriptor {
    TypeDescriptor::new_text::<T>(query_name, kind)
}

/// Creates a struct root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn struct_type<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: StructKind,
    fields: &'static [FieldDescriptor],
) -> TypeDescriptor {
    TypeDescriptor::new_struct::<T>(query_name, kind, fields)
}

/// Creates an enum root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn enum_type<T: ?Sized + 'static>(
    query_name: &'static str,
    variants: &'static [VariantDescriptor],
) -> TypeDescriptor {
    TypeDescriptor::new_enum::<T>(query_name, variants)
}

/// Creates a tuple root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn tuple<T: ?Sized + 'static>(query_name: &'static str, elements: &'static [TypeRef]) -> TypeDescriptor {
    TypeDescriptor::new_tuple::<T>(query_name, elements)
}

/// Creates an array root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn array<T: ?Sized + 'static>(
    query_name: &'static str,
    element: &'static TypeRef,
    length: usize,
) -> TypeDescriptor {
    TypeDescriptor::new_array::<T>(query_name, element, length)
}

/// Creates an optional root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn optional<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> TypeDescriptor {
    TypeDescriptor::new_optional::<T>(query_name, element)
}

/// Creates a sequence root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn sequence<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: SequenceKind,
    element: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_sequence::<T>(query_name, kind, element)
}

/// Creates a set root descriptor for `T` with a generated diagnostic type name.
#[doc(hidden)]
pub const fn set<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: SetKind,
    element: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_set::<T>(query_name, kind, element)
}

/// Creates a map root descriptor for `T` with a generated diagnostic type name.
#[doc(hidden)]
pub const fn map<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: MapKind,
    key: &'static TypeRef,
    value: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_map::<T>(query_name, kind, key, value)
}

/// Creates a smart-pointer root descriptor for `T` with a generated diagnostic
/// type name.
#[doc(hidden)]
pub const fn smart_pointer<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: SmartPointerKind,
    pointee: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_smart_pointer::<T>(query_name, kind, pointee)
}

/// Creates a reference root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn reference<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: ReferenceKind,
    target: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_reference::<T>(query_name, kind, target)
}

/// Creates a slice root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
pub const fn slice<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> TypeDescriptor {
    TypeDescriptor::new_slice::<T>(query_name, element)
}

/// Creates a raw-pointer root descriptor for `T` with a generated diagnostic
/// type name.
#[doc(hidden)]
pub const fn raw_pointer<T: ?Sized + 'static>(
    query_name: &'static str,
    mutability: Mutability,
    pointee: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_raw_pointer::<T>(query_name, mutability, pointee)
}

/// Creates a function-pointer root descriptor for `T` with a generated
/// diagnostic type name.
#[doc(hidden)]
pub const fn function<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: FunctionPointerKind,
    abi: &'static FunctionAbi,
    variadic: bool,
    parameters: &'static [TypeRef],
    return_type: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_function::<T>(query_name, kind, abi, variadic, parameters, return_type)
}

/// Creates a trait-object root descriptor for `T` with a generated diagnostic
/// type name.
#[doc(hidden)]
pub const fn trait_object<T: ?Sized + 'static>(query_name: &'static str) -> TypeDescriptor {
    TypeDescriptor::new_trait_object::<T>(query_name)
}

/// Creates an intentionally opaque root descriptor for `T` with a generated
/// diagnostic type name.
#[doc(hidden)]
pub const fn opaque_root<T: ?Sized + 'static>(query_name: &'static str) -> TypeDescriptor {
    TypeDescriptor::new_opaque::<T>(query_name)
}

/// Creates an explicit opaque member descriptor whose diagnostic name is
/// derived from `T`.
#[doc(hidden)]
pub const fn opaque_member<T: ?Sized + 'static>() -> OpaqueTypeDescriptor {
    OpaqueTypeDescriptor::new::<T>()
}

/// Creates an immutable field descriptor for generated descriptor data.
#[doc(hidden)]
pub const fn field(
    declaring_type: TypeDescriptorResolver,
    index: usize,
    rust_name: Option<&'static str>,
    query_name: Option<&'static str>,
    field_type: &'static TypeRef,
    visibility: Visibility,
) -> FieldDescriptor {
    FieldDescriptor::new(declaring_type, index, rust_name, query_name, field_type, visibility)
}

/// Creates an immutable enum variant descriptor for generated descriptor data.
#[doc(hidden)]
pub const fn variant(
    declaring_type: TypeDescriptorResolver,
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    kind: VariantKind,
    fields: &'static [FieldDescriptor],
) -> VariantDescriptor {
    VariantDescriptor::new(declaring_type, index, rust_name, query_name, kind, fields)
}
