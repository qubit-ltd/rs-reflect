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
use crate::capability::TypeCapabilitiesResult;
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
///
/// # Type Parameters
///
/// - `T`: Candidate reflected type whose bound is checked by method resolution.
#[doc(hidden)]
pub struct ReflectArgumentProbe<T: ?Sized>(std::marker::PhantomData<fn() -> T>);

impl<T: ?Sized> ReflectArgumentProbe<T> {
    /// Creates a probe without evaluating the target type's descriptor.
    ///
    /// # Returns
    ///
    /// Returns a zero-sized probe for `T`.
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
    ///
    /// # Returns
    ///
    /// Returns the process-lifetime lazy resolver when the bound is proven,
    /// otherwise `None` without resolving the target.
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
    ///
    /// # Returns
    ///
    /// Returns the exact descriptor resolver when `T: Reflect` is proven,
    /// otherwise `None`.
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
///
/// # Type Parameters
///
/// - `T`: Associated-constant value type proven sized and `'static`.
///
/// # Parameters
///
/// - `getter`: Function that reads the associated constant.
///
/// # Returns
///
/// Returns a process-lifetime reader for the generated getter.
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
    ///
    /// # Returns
    ///
    /// Returns the associated constant value.
    fn get() -> Self::Value
    where
        Self::Value: Sized;
}

/// Semantic probe for an associated-constant provider.
///
/// # Type Parameters
///
/// - `P`: Provider whose associated value type is checked by method resolution.
#[doc(hidden)]
pub struct AssociatedConstProbe<P: AssociatedConstProvider> {
    marker: std::marker::PhantomData<fn() -> P>,
}

impl<P: AssociatedConstProvider> AssociatedConstProbe<P> {
    /// Creates a zero-sized semantic probe.
    ///
    /// # Returns
    ///
    /// Returns a probe for the provider `P`.
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
    ///
    /// # Returns
    ///
    /// Returns an owned-value reader only when the provider's exact value type
    /// is both sized and `'static`.
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
    ///
    /// # Returns
    ///
    /// Returns the type-independent structural expression for this value.
    fn expression(self) -> ConstExpression;

    /// Produces the normalized diagnostic representation of this value.
    ///
    /// # Returns
    ///
    /// Returns the stable source-like diagnostic text.
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
///
/// # Type Parameters
///
/// - `T`: Supported primitive const parameter type.
///
/// # Parameters
///
/// - `value`: Const value to represent structurally.
///
/// # Returns
///
/// Returns the corresponding structural const expression.
#[doc(hidden)]
pub fn const_argument_expression<T: ConstArgumentValue>(value: T) -> ConstExpression {
    value.expression()
}

/// Returns normalized diagnostic text for one primitive const argument.
///
/// # Type Parameters
///
/// - `T`: Supported primitive const parameter type.
///
/// # Parameters
///
/// - `value`: Const value to format for diagnostics.
///
/// # Returns
///
/// Returns normalized owned text for the value.
#[doc(hidden)]
pub fn const_argument_diagnostic<T: ConstArgumentValue>(value: T) -> Box<str> {
    value.diagnostic()
}

/// Wraps one concrete const argument in the local owned dynamic boundary.
///
/// # Type Parameters
///
/// - `T`: Sized owned type retained in the dynamic value.
///
/// # Parameters
///
/// - `value`: Concrete const argument value.
///
/// # Returns
///
/// Returns the value wrapped as a local reflected owned value.
#[doc(hidden)]
pub fn const_argument_owned<T: 'static>(value: T) -> ReflectedOwned {
    ReflectedOwned::new(value)
}

/// Interns a runtime-created descriptor for one concrete type specialization.
///
/// Generated implementations use this for generic types, whose descriptor
/// cannot be stored in a single local static without conflating distinct
/// substitutions.
///
/// # Type Parameters
///
/// - `T`: Concrete Rust specialization whose descriptor is interned.
///
/// # Parameters
///
/// - `build`: Factory that creates the immutable specialization descriptor.
///
/// # Returns
///
/// Returns the unique process-lifetime descriptor for `T`.
///
/// # Panics
///
/// Propagates a panic from `build`; the descriptor cell remains available for
/// a later retry.
#[doc(hidden)]
pub fn intern_type<T: ?Sized + 'static>(build: fn() -> TypeDescriptor) -> &'static TypeDescriptor {
    crate::builtin::interner::intern::<T>(build)
}

/// Creates a primitive root descriptor for `T` with a generated diagnostic type
/// name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Primitive category to expose.
///
/// # Returns
///
/// Returns an immutable primitive root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn primitive<T: ?Sized + 'static>(query_name: &'static str, kind: PrimitiveKind) -> TypeDescriptor {
    TypeDescriptor::new_primitive::<T>(query_name, kind)
}

/// Creates a primitive root descriptor with a descriptor-owned capability
/// resolver.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Primitive category to expose.
/// - `capabilities`: Provider for the descriptor's intrinsic capabilities.
///
/// # Returns
///
/// Returns an immutable primitive root descriptor with that provider.
#[doc(hidden)]
pub const fn primitive_with_capabilities<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: PrimitiveKind,
    capabilities: fn() -> TypeCapabilitiesResult,
) -> TypeDescriptor {
    TypeDescriptor::new_primitive_with_capabilities::<T>(query_name, kind, capabilities)
}

/// Creates a text root descriptor for `T` with a generated diagnostic type
/// name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Text category to expose.
///
/// # Returns
///
/// Returns an immutable text root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn text<T: ?Sized + 'static>(query_name: &'static str, kind: TextKind) -> TypeDescriptor {
    TypeDescriptor::new_text::<T>(query_name, kind)
}

/// Creates a text root descriptor with a descriptor-owned capability resolver.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Text category to expose.
/// - `capabilities`: Provider for the descriptor's intrinsic capabilities.
///
/// # Returns
///
/// Returns an immutable text root descriptor with that provider.
#[doc(hidden)]
pub const fn text_with_capabilities<T: ?Sized + 'static>(
    query_name: &'static str,
    kind: TextKind,
    capabilities: fn() -> TypeCapabilitiesResult,
) -> TypeDescriptor {
    TypeDescriptor::new_text_with_capabilities::<T>(query_name, kind, capabilities)
}

/// Creates a struct root descriptor for `T` with a generated diagnostic type
/// name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Struct category to expose.
/// - `fields`: Fields in declaration order.
///
/// # Returns
///
/// Returns an immutable struct root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Struct category to expose.
/// - `fields`: Fields in declaration order.
/// - `construction`: Generated construction and update entry points.
///
/// # Returns
///
/// Returns an immutable struct root descriptor with construction support.
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
///
/// # Parameters
///
/// - `descriptor`: Root descriptor to enrich.
/// - `generic`: Generic declaration and concrete substitution facts.
///
/// # Returns
///
/// Returns the descriptor carrying the concrete generic facts.
#[doc(hidden)]
pub const fn with_concrete_generic(
    descriptor: TypeDescriptor,
    generic: &'static ConcreteGenericDescriptor,
) -> TypeDescriptor {
    descriptor.with_concrete_generic(generic)
}

/// Links one concrete descriptor to its source-level generic declaration.
///
/// # Parameters
///
/// - `descriptor`: Concrete root descriptor to enrich.
/// - `definition`: Lazy resolver for its source generic declaration.
///
/// # Returns
///
/// Returns the descriptor linked to the generic definition.
#[doc(hidden)]
#[must_use]
pub const fn with_type_definition(
    descriptor: TypeDescriptor,
    definition: fn() -> &'static crate::descriptor::TypeDefinitionDescriptor,
) -> TypeDescriptor {
    descriptor.with_type_definition(definition)
}

/// Attaches a generated capability resolver to one descriptor root before it
/// is interned.
///
/// # Parameters
///
/// - `descriptor`: Root descriptor to enrich.
/// - `capabilities`: Provider for intrinsic capability facts.
///
/// # Returns
///
/// Returns the descriptor carrying the capability provider.
#[doc(hidden)]
pub const fn with_capabilities(
    descriptor: TypeDescriptor,
    capabilities: fn() -> TypeCapabilitiesResult,
) -> TypeDescriptor {
    descriptor.with_capabilities(capabilities)
}

/// Creates an enum root descriptor for `T` with a generated diagnostic type
/// name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `variants`: Variants in declaration order.
///
/// # Returns
///
/// Returns an immutable enum root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn enum_type<T: ?Sized + 'static>(
    query_name: &'static str,
    variants: &'static [VariantDescriptor],
) -> TypeDescriptor {
    TypeDescriptor::new_enum::<T>(query_name, variants)
}

/// Creates an enum root with normalized explicit representation metadata.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `variants`: Variants in declaration order.
/// - `representations`: Explicit representation facts in stable order.
///
/// # Returns
///
/// Returns an immutable enum root descriptor with representation metadata.
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
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `elements`: Tuple element types in source order.
///
/// # Returns
///
/// Returns an immutable tuple root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn tuple<T: ?Sized + 'static>(query_name: &'static str, elements: &'static [TypeRef]) -> TypeDescriptor {
    TypeDescriptor::new_tuple::<T>(query_name, elements)
}

/// Creates an array root descriptor for `T` with a generated diagnostic type
/// name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `element`: Element type relationship.
/// - `length`: Number of elements.
///
/// # Returns
///
/// Returns an immutable array root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `element`: Optional payload type relationship.
///
/// # Returns
///
/// Returns an immutable optional root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn optional<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> TypeDescriptor {
    TypeDescriptor::new_optional::<T>(query_name, element)
}

/// Creates a sequence root descriptor for `T` with a generated diagnostic type
/// name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Sequence category to expose.
/// - `element`: Sequence element type relationship.
///
/// # Returns
///
/// Returns an immutable sequence root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Set category to expose.
/// - `element`: Set element type relationship.
///
/// # Returns
///
/// Returns an immutable set root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Map category to expose.
/// - `key`: Map key type relationship.
/// - `value`: Map value type relationship.
///
/// # Returns
///
/// Returns an immutable map root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust pointer type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Smart-pointer category to expose.
/// - `pointee`: Referenced pointee type relationship.
///
/// # Returns
///
/// Returns an immutable smart-pointer root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust reference type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Shared or mutable reference category.
/// - `target`: Referenced type relationship.
///
/// # Returns
///
/// Returns an immutable reference root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Rust slice type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `element`: Slice element type relationship.
///
/// # Returns
///
/// Returns an immutable slice root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn slice<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> TypeDescriptor {
    TypeDescriptor::new_slice::<T>(query_name, element)
}

/// Creates a raw-pointer root descriptor for `T` with a generated diagnostic
/// type name.
///
/// # Type Parameters
///
/// - `T`: Raw-pointer type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `mutability`: Whether the pointer target is mutable.
/// - `pointee`: Pointed-to type relationship.
///
/// # Returns
///
/// Returns an immutable raw-pointer root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Function-pointer type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `kind`: Function-pointer calling and safety category.
/// - `abi`: Function ABI metadata.
/// - `variadic`: Whether the function accepts variadic arguments.
/// - `parameters`: Parameter type relationships in call order.
/// - `return_type`: Function return type relationship.
///
/// # Returns
///
/// Returns an immutable function-pointer root descriptor.
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
///
/// # Type Parameters
///
/// - `T`: Trait-object type represented by the descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `trait_descriptor`: Lazy resolver for the reflected trait descriptor.
///
/// # Returns
///
/// Returns an immutable trait-object root descriptor.
#[doc(hidden)]
pub const fn trait_object<T: ?Sized + 'static>(
    query_name: &'static str,
    trait_descriptor: fn() -> &'static crate::descriptor::TraitDescriptor,
) -> TypeDescriptor {
    TypeDescriptor::new_trait_object::<T>(query_name, trait_descriptor)
}

/// Creates an intentionally opaque root descriptor for `T` with a generated
/// diagnostic type name.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the opaque descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
///
/// # Returns
///
/// Returns an immutable opaque root descriptor.
#[doc(hidden)]
#[must_use]
pub const fn opaque_root<T: ?Sized + 'static>(query_name: &'static str) -> TypeDescriptor {
    TypeDescriptor::new_opaque::<T>(query_name)
}

/// Creates an opaque root descriptor with a descriptor-owned capability
/// resolver.
///
/// # Type Parameters
///
/// - `T`: Rust type represented by the opaque descriptor.
///
/// # Parameters
///
/// - `query_name`: Stable reflection query name.
/// - `capabilities`: Provider for intrinsic capability facts.
///
/// # Returns
///
/// Returns an opaque descriptor carrying that capability provider.
#[doc(hidden)]
pub const fn opaque_root_with_capabilities<T: ?Sized + 'static>(
    query_name: &'static str,
    capabilities: fn() -> TypeCapabilitiesResult,
) -> TypeDescriptor {
    TypeDescriptor::new_opaque_with_capabilities::<T>(query_name, capabilities)
}

/// Creates an explicit opaque member descriptor whose diagnostic name is
/// derived from `T`.
///
/// # Type Parameters
///
/// - `T`: Type represented by this opaque member.
///
/// # Returns
///
/// Returns an opaque member descriptor for `T`.
#[doc(hidden)]
#[must_use]
pub const fn opaque_member<T: ?Sized + 'static>() -> OpaqueTypeDescriptor {
    OpaqueTypeDescriptor::new::<T>()
}

/// Allocates a process-lifetime relationship that resolves `T` only when the
/// relationship is navigated.
///
/// # Type Parameters
///
/// - `T`: Reflected target type resolved on first navigation.
///
/// # Returns
///
/// Returns a process-lifetime lazy relationship resolver.
#[doc(hidden)]
#[must_use]
pub fn lazy_type_ref<T: Reflect + ?Sized>() -> &'static LazyTypeRef {
    Box::leak(Box::new(LazyTypeRef::resolved::<T>()))
}

/// Allocates a process-lifetime list of relationships that resolve only when
/// the list is navigated.
///
/// # Parameters
///
/// - `references`: Lazy relationships in declaration order.
///
/// # Returns
///
/// Returns a process-lifetime lazy list used by generated descriptors.
#[doc(hidden)]
pub(crate) fn lazy_type_ref_list(references: Vec<LazyTypeRef>) -> &'static LazyTypeRefList {
    let references = Box::leak(references.into_boxed_slice());
    Box::leak(Box::new(LazyTypeRefList::new(references)))
}

/// Creates an immutable field descriptor for generated descriptor data.
///
/// # Parameters
///
/// - `declaring_type`: Resolver for the field's declaring type.
/// - `index`: Source declaration index.
/// - `rust_name`: Rust field name, or `None` for tuple fields.
/// - `query_name`: Reflection query name, or `None` when unavailable.
/// - `field_type`: Field type relationship.
/// - `visibility`: Visibility retained for the field.
///
/// # Returns
///
/// Returns an immutable field descriptor.
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
///
/// # Parameters
///
/// - `declaring_type`: Resolver for the field's declaring type.
/// - `index`: Source declaration index.
/// - `rust_name`: Rust field name, or `None` for tuple fields.
/// - `query_name`: Reflection query name, or `None` when unavailable.
/// - `field_type`: Deferred resolver for the field type.
/// - `visibility`: Visibility retained for the field.
///
/// # Returns
///
/// Returns an immutable field descriptor with deferred type navigation.
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
///
/// # Parameters
///
/// - `declaring_type`: Resolver for the declaring enum.
/// - `index`: Variant source declaration index.
/// - `rust_name`: Variant's Rust name.
/// - `query_name`: Stable reflection query name.
/// - `kind`: Unit, tuple, or struct variant shape.
/// - `fields`: Variant fields in declaration order.
/// - `active_test`: Runtime probe for the active variant.
///
/// # Returns
///
/// Returns an immutable variant descriptor.
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
    fn test_semantic_probes_use_generic_environment_bounds_without_concrete_inspection() {
        assert!(unresolved_descriptor::<u8>().is_none());
        assert!(proven_descriptor::<u8>().is_some());
    }
}
