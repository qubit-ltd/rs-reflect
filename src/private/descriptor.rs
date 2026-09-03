// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow multiple-public-types
//! Hidden factories for immutable static descriptor data.

use crate::__private::LazyTypeRef;
use crate::__private::LazyTypeRefList;
use crate::access::VariantActiveAdapter;
use crate::capability::TypeCapabilities;
use crate::construct::StructConstructionDescriptor;
use crate::descriptor::AssociatedConstReader;
use crate::descriptor::ConcreteGenericDescriptor;
use crate::descriptor::EnumRepr;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::MapKind;
use crate::descriptor::Mutability;
use crate::descriptor::OpaqueTypeDescriptor;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::ReferenceKind;
use crate::descriptor::Reflect;
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
use crate::expression::ConstExpression;
use crate::expression::FunctionAbi;
use crate::identity::Visibility;
use crate::value::ReflectedOwned;

/// A zero-sized method-resolution probe for determining whether the generic
/// environment semantically proves `T: Reflect`.
#[doc(hidden)]
pub struct ReflectArgumentProbe<T: ?Sized>(std::marker::PhantomData<fn() -> T>);

impl<T: ?Sized> ReflectArgumentProbe<T> {
    /// Creates a probe without evaluating the target type's descriptor.
    #[doc(hidden)]
    #[must_use]
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

/// Selects a lazy concrete-argument resolver when the surrounding generic
/// bounds semantically provide the runtime reflection contract.
#[doc(hidden)]
pub trait ResolveReflectArgument {
    /// Returns a lazy resolver when the probed type implements `Reflect`.
    fn resolve_reflect_argument(self) -> Option<&'static LazyTypeRef>;
}

impl<T: ?Sized + 'static> ResolveReflectArgument for &&ReflectArgumentProbe<T> {
    fn resolve_reflect_argument(self) -> Option<&'static LazyTypeRef> {
        None
    }
}

impl<T: Reflect + ?Sized> ResolveReflectArgument for &ReflectArgumentProbe<T> {
    fn resolve_reflect_argument(self) -> Option<&'static LazyTypeRef> {
        Some(lazy_type_ref::<T>())
    }
}

/// Selects an exact descriptor resolver only when rustc proves `T: Reflect`
/// in the generic environment where method resolution occurs.
#[doc(hidden)]
pub trait ResolveReflectTypeDescriptor {
    /// Returns the proven resolver, or `None` without inspecting a concrete
    /// implementation that was not constrained by the declaration.
    fn resolve_reflect_type_descriptor(self) -> Option<TypeDescriptorResolver>;
}

impl<T: ?Sized> ResolveReflectTypeDescriptor for &&ReflectArgumentProbe<T> {
    fn resolve_reflect_type_descriptor(self) -> Option<TypeDescriptorResolver> {
        None
    }
}

impl<T: Reflect + ?Sized> ResolveReflectTypeDescriptor for &ReflectArgumentProbe<T> {
    fn resolve_reflect_type_descriptor(self) -> Option<TypeDescriptorResolver> {
        Some(T::type_descriptor)
    }
}

/// Creates an associated-constant reader only after rustc proves the exact
/// declared value type satisfies the sized `'static` owned boundary.
#[doc(hidden)]
pub fn associated_const_reader<T: 'static>(getter: fn() -> T) -> &'static AssociatedConstReader {
    Box::leak(Box::new(AssociatedConstReader::from_getter(getter)))
}

/// Delays access to an associated constant until its value type is proven to
/// satisfy the owned-value boundary.
#[doc(hidden)]
pub trait AssociatedConstProvider {
    /// The exact declared associated-constant type.
    type Value: ?Sized;

    /// Reads the value only in a context where rustc has proven it is sized.
    fn get() -> Self::Value
    where
        Self::Value: Sized;
}

/// Semantic probe for an associated-constant provider.
#[doc(hidden)]
pub struct AssociatedConstProbe<P: AssociatedConstProvider> {
    marker: std::marker::PhantomData<fn() -> P>,
}

impl<P: AssociatedConstProvider> AssociatedConstProbe<P> {
    /// Creates a zero-sized semantic probe.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<P: AssociatedConstProvider> Default for AssociatedConstProbe<P> {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves an owned reader only when the declaration's generic environment
/// proves the exact value type is both sized and `'static`.
#[doc(hidden)]
pub trait ResolveAssociatedConstReader {
    /// Returns the proven reader, or `None` without evaluating the constant.
    fn resolve_associated_const_reader(self) -> Option<&'static AssociatedConstReader>;
}

impl<P: AssociatedConstProvider> ResolveAssociatedConstReader for &&AssociatedConstProbe<P> {
    fn resolve_associated_const_reader(self) -> Option<&'static AssociatedConstReader> {
        None
    }
}

impl<P> ResolveAssociatedConstReader for &AssociatedConstProbe<P>
where
    P: AssociatedConstProvider,
    P::Value: Sized + 'static,
{
    fn resolve_associated_const_reader(self) -> Option<&'static AssociatedConstReader> {
        Some(associated_const_reader::<P::Value>(P::get))
    }
}

/// Converts stable primitive const-parameter values into structural runtime
/// metadata without requiring the derive macro to resolve Rust type aliases.
#[doc(hidden)]
pub trait ConstArgumentValue: Copy + 'static {
    /// Converts this value into its structural const expression.
    fn expression(self) -> ConstExpression;

    /// Produces the normalized diagnostic representation of this value.
    fn diagnostic(self) -> Box<str>;
}

macro_rules! impl_integer_const_argument {
    ($variant:ident, $cast:ty; $($type:ty),+ $(,)?) => {
        $(
            impl ConstArgumentValue for $type {
                fn expression(self) -> ConstExpression {
                    ConstExpression::$variant(self as $cast)
                }

                fn diagnostic(self) -> Box<str> {
                    self.to_string().into_boxed_str()
                }
            }
        )+
    };
}

impl_integer_const_argument!(SignedInteger, i128; i8, i16, i32, i64, i128, isize);
impl_integer_const_argument!(UnsignedInteger, u128; u8, u16, u32, u64, u128, usize);

impl ConstArgumentValue for bool {
    fn expression(self) -> ConstExpression {
        ConstExpression::Boolean(self)
    }

    fn diagnostic(self) -> Box<str> {
        self.to_string().into_boxed_str()
    }
}

impl ConstArgumentValue for char {
    fn expression(self) -> ConstExpression {
        ConstExpression::Character(self)
    }

    fn diagnostic(self) -> Box<str> {
        format!("{self:?}").into_boxed_str()
    }
}

/// Converts one primitive const argument into its structural expression.
#[doc(hidden)]
pub fn const_argument_expression<T: ConstArgumentValue>(value: T) -> ConstExpression {
    value.expression()
}

/// Returns normalized diagnostic text for one primitive const argument.
#[doc(hidden)]
pub fn const_argument_diagnostic<T: ConstArgumentValue>(value: T) -> Box<str> {
    value.diagnostic()
}

/// Wraps one concrete const argument in the local owned dynamic boundary.
#[doc(hidden)]
pub fn const_argument_owned<T: 'static>(value: T) -> ReflectedOwned {
    ReflectedOwned::new(value)
}

/// Interns a runtime-created descriptor for one concrete type specialization.
///
/// Generated implementations use this for generic types, whose descriptor
/// cannot be stored in a single local static without conflating distinct
/// substitutions.
#[doc(hidden)]
pub fn intern_type<T: ?Sized + 'static>(build: fn() -> TypeDescriptor) -> &'static TypeDescriptor {
    crate::builtin::interner::intern::<T>(build)
}

/// Creates a primitive root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
pub const fn primitive<T: ?Sized + 'static>(query_name: &'static str, kind: PrimitiveKind) -> TypeDescriptor {
    TypeDescriptor::new_primitive::<T>(query_name, kind)
}

/// Creates a primitive root descriptor with a descriptor-owned capability
/// resolver.
#[doc(hidden)]
pub const fn primitive_with_capabilities<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: PrimitiveKind,
    capabilities: fn() -> &'static TypeCapabilities,
) -> TypeDescriptor {
    TypeDescriptor::new_primitive_with_capabilities::<T>(query_name, kind, capabilities)
}

/// Creates a text root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
pub const fn text<T: ?Sized + 'static>(query_name: &'static str, kind: TextKind) -> TypeDescriptor {
    TypeDescriptor::new_text::<T>(query_name, kind)
}

/// Creates a text root descriptor with a descriptor-owned capability resolver.
#[doc(hidden)]
pub const fn text_with_capabilities<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: TextKind,
    capabilities: fn() -> &'static TypeCapabilities,
) -> TypeDescriptor {
    TypeDescriptor::new_text_with_capabilities::<T>(query_name, kind, capabilities)
}

/// Creates a struct root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
pub const fn struct_type<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: StructKind,
    fields: &'static [FieldDescriptor],
) -> TypeDescriptor {
    TypeDescriptor::new_struct::<T>(query_name, kind, fields)
}

/// Creates a reflected struct root with generated construction entry points.
#[doc(hidden)]
pub fn struct_type_with_construction<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: StructKind,
    fields: &'static [FieldDescriptor],
    construction: StructConstructionDescriptor,
) -> TypeDescriptor {
    TypeDescriptor::new_struct::<T>(query_name, kind, fields).with_struct_construction(construction)
}

/// Attaches generic declaration and concrete-instance facts to a root.
#[doc(hidden)]
pub const fn with_concrete_generic(
    descriptor: TypeDescriptor,
    generic: &'static ConcreteGenericDescriptor,
) -> TypeDescriptor {
    descriptor.with_concrete_generic(generic)
}

/// Attaches a generated capability resolver to one descriptor root before it
/// is interned.
#[doc(hidden)]
pub const fn with_capabilities(
    descriptor: TypeDescriptor,
    capabilities: fn() -> &'static TypeCapabilities,
) -> TypeDescriptor {
    descriptor.with_capabilities(capabilities)
}

/// Creates an enum root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
pub const fn enum_type<T: ?Sized + 'static>(
    query_name: &'static str,
    variants: &'static [VariantDescriptor],
) -> TypeDescriptor {
    TypeDescriptor::new_enum::<T>(query_name, variants)
}

/// Creates an enum root with normalized explicit representation metadata.
#[doc(hidden)]
#[must_use]
pub const fn enum_type_with_repr<T: ?Sized + 'static>(
    query_name: &'static str,
    variants: &'static [VariantDescriptor],
    representations: &'static [EnumRepr],
) -> TypeDescriptor {
    TypeDescriptor::new_enum_with_repr::<T>(query_name, variants, representations)
}

/// Creates a tuple root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
pub const fn tuple<T: ?Sized + 'static>(query_name: &'static str, elements: &'static [TypeRef]) -> TypeDescriptor {
    TypeDescriptor::new_tuple::<T>(query_name, elements)
}

/// Creates an array root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
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
#[must_use]
pub const fn optional<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> TypeDescriptor {
    TypeDescriptor::new_optional::<T>(query_name, element)
}

/// Creates a sequence root descriptor for `T` with a generated diagnostic type
/// name.
#[doc(hidden)]
#[must_use]
pub const fn sequence<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: SequenceKind,
    element: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_sequence::<T>(query_name, kind, element)
}

/// Creates a set root descriptor for `T` with a generated diagnostic type name.
#[doc(hidden)]
#[must_use]
pub const fn set<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: SetKind,
    element: &'static TypeRef,
) -> TypeDescriptor {
    TypeDescriptor::new_set::<T>(query_name, kind, element)
}

/// Creates a map root descriptor for `T` with a generated diagnostic type name.
#[doc(hidden)]
#[must_use]
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
#[must_use]
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
#[must_use]
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
#[must_use]
pub const fn slice<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> TypeDescriptor {
    TypeDescriptor::new_slice::<T>(query_name, element)
}

/// Creates a raw-pointer root descriptor for `T` with a generated diagnostic
/// type name.
#[doc(hidden)]
#[must_use]
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
#[must_use]
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
pub const fn trait_object<T: ?Sized + 'static>(
    query_name: &'static str,
    trait_descriptor: fn() -> &'static crate::descriptor::TraitDescriptor,
) -> TypeDescriptor {
    TypeDescriptor::new_trait_object::<T>(query_name, trait_descriptor)
}

/// Creates an intentionally opaque root descriptor for `T` with a generated
/// diagnostic type name.
#[doc(hidden)]
#[must_use]
pub const fn opaque_root<T: ?Sized + 'static>(query_name: &'static str) -> TypeDescriptor {
    TypeDescriptor::new_opaque::<T>(query_name)
}

/// Creates an opaque root descriptor with a descriptor-owned capability
/// resolver.
#[doc(hidden)]
pub const fn opaque_root_with_capabilities<T: ?Sized + 'static>(
    query_name: &'static str,
    capabilities: fn() -> &'static TypeCapabilities,
) -> TypeDescriptor {
    TypeDescriptor::new_opaque_with_capabilities::<T>(query_name, capabilities)
}

/// Creates an explicit opaque member descriptor whose diagnostic name is
/// derived from `T`.
#[doc(hidden)]
#[must_use]
pub const fn opaque_member<T: ?Sized + 'static>() -> OpaqueTypeDescriptor {
    OpaqueTypeDescriptor::new::<T>()
}

/// Allocates a process-lifetime relationship that resolves `T` only when the
/// relationship is navigated.
#[doc(hidden)]
#[must_use]
pub fn lazy_type_ref<T: Reflect + ?Sized>() -> &'static LazyTypeRef {
    Box::leak(Box::new(LazyTypeRef::resolved::<T>()))
}

/// Allocates a process-lifetime list of relationships that resolve only when
/// the list is navigated.
#[doc(hidden)]
pub(crate) fn lazy_type_ref_list(references: Vec<LazyTypeRef>) -> &'static LazyTypeRefList {
    let references = Box::leak(references.into_boxed_slice());
    Box::leak(Box::new(LazyTypeRefList::new(references)))
}

/// Creates an immutable field descriptor for generated descriptor data.
#[doc(hidden)]
#[must_use]
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

/// Creates an immutable field whose concrete type relationship is resolved on
/// first navigation.
#[doc(hidden)]
pub const fn lazy_field(
    declaring_type: TypeDescriptorResolver,
    index: usize,
    rust_name: Option<&'static str>,
    query_name: Option<&'static str>,
    field_type: &'static LazyTypeRef,
    visibility: Visibility,
) -> FieldDescriptor {
    FieldDescriptor::new_lazy(declaring_type, index, rust_name, query_name, field_type, visibility)
}

/// Creates an immutable enum variant descriptor for generated descriptor data.
#[doc(hidden)]
#[must_use]
pub const fn variant(
    declaring_type: TypeDescriptorResolver,
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    kind: VariantKind,
    fields: &'static [FieldDescriptor],
    active_test: VariantActiveAdapter,
) -> VariantDescriptor {
    VariantDescriptor::new(declaring_type, index, rust_name, query_name, kind, fields, active_test)
}

#[cfg(test)]
mod tests {
    use super::ReflectArgumentProbe;
    use super::ResolveReflectTypeDescriptor as _;
    use crate::descriptor::Reflect;
    use crate::descriptor::TypeDescriptorResolver;

    fn unresolved_descriptor<T: 'static>() -> Option<TypeDescriptorResolver> {
        let probe = ReflectArgumentProbe::<T>::new();
        (&probe).resolve_reflect_type_descriptor()
    }

    fn proven_descriptor<T: Reflect>() -> Option<TypeDescriptorResolver> {
        let probe = ReflectArgumentProbe::<T>::new();
        (&probe).resolve_reflect_type_descriptor()
    }

    #[test]
    fn semantic_probes_use_generic_environment_bounds_without_concrete_inspection() {
        assert!(unresolved_descriptor::<u8>().is_none());
        assert!(proven_descriptor::<u8>().is_some());
    }
}
