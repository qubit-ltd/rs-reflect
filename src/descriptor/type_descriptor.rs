// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! The unique root descriptor and [`Reflect`] query trait.

use std::any::TypeId;
use std::fmt;

use crate::__private::LazyTypeRef;
use crate::__private::LazyTypeRefList;
use crate::capability::TypeCapabilitiesResult;
use crate::construct::ConstructionError;
use crate::construct::ConstructionRecovery;
use crate::construct::NamedConstructionInput;
use crate::construct::StructConstructionDescriptor;
use crate::construct::TupleConstructionInput;
use crate::descriptor::ArrayTypeDescriptor;
use crate::descriptor::ConcreteGenericDescriptor;
use crate::descriptor::EnumRepr;
use crate::descriptor::EnumTypeDescriptor;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::FunctionTypeDescriptor;
use crate::descriptor::ImplDescriptor;
use crate::descriptor::MapKind;
use crate::descriptor::MapTypeDescriptor;
use crate::descriptor::MethodInstanceDescriptor;
use crate::descriptor::MethodLookup;
use crate::descriptor::MethodQualifier;
use crate::descriptor::Mutability;
use crate::descriptor::OpaqueTypeView;
use crate::descriptor::OptionalRefProjector;
use crate::descriptor::OptionalTypeDescriptor;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::PrimitiveTypeDescriptor;
use crate::descriptor::RawPointerTypeDescriptor;
use crate::descriptor::ReferenceKind;
use crate::descriptor::ReferenceTypeDescriptor;
use crate::descriptor::Reflect;
use crate::descriptor::SequenceKind;
use crate::descriptor::SequenceTypeDescriptor;
use crate::descriptor::SetKind;
use crate::descriptor::SetTypeDescriptor;
use crate::descriptor::SliceTypeDescriptor;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::SmartPointerTypeDescriptor;
use crate::descriptor::StructKind;
use crate::descriptor::StructTypeDescriptor;
use crate::descriptor::TextKind;
use crate::descriptor::TextTypeDescriptor;
use crate::descriptor::TraitObjectTypeDescriptor;
use crate::descriptor::TupleTypeDescriptor;
use crate::descriptor::TypeDefinitionDescriptor;
use crate::descriptor::TypeDefinitionId;
use crate::descriptor::TypeKind;
use crate::descriptor::TypeRef;
use crate::descriptor::VariantDescriptor;
use crate::descriptor::type_ref::type_id_of;
use crate::descriptor::type_ref::type_name_of;
use crate::error::RegistryError;
use crate::expression::FunctionAbi;
use crate::registry::ReflectRegistry;
use crate::value::ReflectedOwned;

/// Kind-specific data retained by a root descriptor.
enum TypeDescriptorData {
    Primitive(PrimitiveTypeDescriptor),
    Text(TextTypeDescriptor),
    Struct(StructTypeDescriptor),
    Enum(EnumTypeDescriptor),
    Tuple(TupleTypeDescriptor),
    Array(ArrayTypeDescriptor),
    Optional(OptionalTypeDescriptor),
    Sequence(SequenceTypeDescriptor),
    Set(SetTypeDescriptor),
    Map(MapTypeDescriptor),
    SmartPointer(SmartPointerTypeDescriptor),
    Reference(ReferenceTypeDescriptor),
    Slice(SliceTypeDescriptor),
    RawPointer(RawPointerTypeDescriptor),
    Function(FunctionTypeDescriptor),
    TraitObject(TraitObjectTypeDescriptor),
    Opaque(OpaqueTypeView),
}

/// The immutable root description of one concrete Rust type.
///
/// Relationships use shared static references. Construction is reserved for
/// generated and built-in descriptor data; ordinary callers navigate a
/// descriptor returned by [`Reflect`].
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
///
/// let descriptor = TypeDescriptor::of::<u32>();
/// assert_eq!(descriptor.type_name(), "u32");
/// assert!(descriptor.as_primitive().is_some());
/// ```
pub struct TypeDescriptor {
    /// Resolver for the represented type's process-local identity.
    type_id: fn() -> TypeId,
    /// Resolver for the represented type's diagnostic name.
    type_name: fn() -> &'static str,
    /// Stable lookup name used by reflection queries.
    query_name: &'static str,
    /// Type-kind-specific metadata for this root.
    data: TypeDescriptorData,
    /// Direct fields retained in source declaration order.
    fields: &'static [FieldDescriptor],
    /// Enum variants retained in source declaration order.
    variants: &'static [VariantDescriptor],
    /// Resolver for capabilities declared by this root.
    capabilities: fn() -> TypeCapabilitiesResult,
    /// Optional generated struct construction entry points.
    construction: Option<StructConstructionDescriptor>,
    /// Optional concrete generic arguments for this root.
    generic: Option<&'static ConcreteGenericDescriptor>,
    /// Optional resolver for the source generic declaration.
    definition: Option<fn() -> &'static TypeDefinitionDescriptor>,
}

impl TypeDescriptor {
    /// Returns the unique root descriptor supplied by `T`'s [`Reflect`]
    /// implementation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Reflected concrete type whose root descriptor is requested.
    ///
    /// # Returns
    ///
    /// Returns the static root descriptor provided by `T`.
    ///
    /// # Panics
    ///
    /// Panics if the descriptor returned by `T` reports a different
    /// [`TypeId`] than `T`. This indicates an invalid `Reflect` implementation.
    #[must_use]
    pub fn of<T: Reflect + ?Sized>() -> &'static Self {
        let descriptor = T::type_descriptor();
        assert_eq!(
            descriptor.type_id(),
            TypeId::of::<T>(),
            "Reflect descriptor type mismatch for {}",
            std::any::type_name::<T>(),
        );
        descriptor
    }

    /// Returns generated construction and owned-update entry points for a
    /// reflected struct root, if that root exposes them.
    ///
    /// # Returns
    ///
    /// Returns the construction descriptor, or `None` when construction is
    /// unavailable.
    #[must_use]
    #[inline]
    pub const fn struct_construction(&self) -> Option<&StructConstructionDescriptor> {
        self.construction.as_ref()
    }

    /// Returns the declaration and concrete substitution facts for a generic
    /// root instance, if this root was derived from generic source.
    ///
    /// # Returns
    ///
    /// Returns the concrete generic metadata, or `None` for non-generic roots.
    #[must_use]
    #[inline]
    pub const fn concrete_generic(&self) -> Option<&'static ConcreteGenericDescriptor> {
        self.generic
    }

    /// Returns the declaration and concrete substitutions for this generic
    /// instance.
    ///
    /// # Returns
    ///
    /// Returns the concrete arguments and their source definition, or `None`
    /// when this root has no generic metadata.
    #[must_use]
    #[inline]
    pub const fn generic_arguments(&self) -> Option<&'static ConcreteGenericDescriptor> {
        self.generic
    }

    /// Returns the generic declaration shared by this concrete instance.
    ///
    /// `None` means this descriptor does not originate from a registered
    /// generic type declaration.
    ///
    /// # Returns
    ///
    /// Returns the source generic declaration, or `None` when unavailable.
    #[must_use]
    #[inline]
    pub fn type_definition(&self) -> Option<&'static TypeDefinitionDescriptor> {
        self.definition.map(|definition| definition())
    }

    /// Returns the generic declaration identity associated with this instance.
    ///
    /// `None` means this descriptor does not originate from a generic
    /// declaration.
    ///
    /// # Returns
    ///
    /// Returns the process-local declaration identity, or `None` when this
    /// root is not linked to a generic declaration.
    #[must_use]
    #[inline]
    pub fn definition_id(&self) -> Option<TypeDefinitionId> {
        self.type_definition().map(TypeDefinitionDescriptor::id)
    }

    /// Constructs a named struct through its generated local adapter.
    ///
    /// # Parameters
    ///
    /// - `input`: Named field values supplied for the reflected struct.
    ///
    /// # Returns
    ///
    /// Returns the constructed value with local dynamic ownership.
    ///
    /// # Errors
    ///
    /// Returns the construction error and all recoverable caller values if
    /// validation fails or this descriptor has no constructor.
    pub fn construct_struct(
        &self,
        input: NamedConstructionInput<crate::value::Local>,
    ) -> Result<ReflectedOwned, ConstructionRecovery<crate::value::Local>> {
        match self.struct_construction() {
            Some(construction) => construction.local_constructor().construct_named(input),
            None => Err(input.into_recovery(ConstructionError::TargetUnavailable)),
        }
    }

    /// Constructs a tuple or newtype struct through its generated local
    /// adapter.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional field values supplied for the reflected struct.
    ///
    /// # Returns
    ///
    /// Returns the constructed value with local dynamic ownership.
    ///
    /// # Errors
    ///
    /// Returns the construction error and all recoverable caller values if
    /// validation fails or this descriptor has no constructor.
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<crate::value::Local>,
    ) -> Result<ReflectedOwned, ConstructionRecovery<crate::value::Local>> {
        match self.struct_construction() {
            Some(construction) => construction.local_constructor().construct_tuple(input),
            None => Err(input.into_recovery(ConstructionError::TargetUnavailable)),
        }
    }

    /// Constructs a unit struct through its generated local adapter.
    ///
    /// # Returns
    ///
    /// Returns the constructed value with local dynamic ownership.
    ///
    /// # Errors
    ///
    /// Returns the construction error if validation fails or this descriptor
    /// has no constructor.
    pub fn construct_unit(&self) -> Result<ReflectedOwned, ConstructionRecovery<crate::value::Local>> {
        match self.struct_construction() {
            Some(construction) => construction.local_constructor().construct_unit(),
            None => Err(ConstructionRecovery::new(
                ConstructionError::TargetUnavailable,
                Vec::new(),
            )),
        }
    }

    /// Creates a primitive root for generated or built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the new root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name for the root.
    /// - `kind`: Primitive category represented by the root.
    ///
    /// # Returns
    ///
    /// Returns a primitive root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_primitive<T: ?Sized + 'static>(query_name: &'static str, kind: PrimitiveKind) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Primitive(PrimitiveTypeDescriptor::new(kind)),
            &[],
            &[],
        )
    }

    /// Creates a text root for generated or built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the new root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name for the root.
    /// - `kind`: Owned or borrowed UTF-8 text category.
    ///
    /// # Returns
    ///
    /// Returns a text root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_text<T: ?Sized + 'static>(query_name: &'static str, kind: TextKind) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Text(TextTypeDescriptor::new(kind)),
            &[],
            &[],
        )
    }

    /// Creates a struct root for generated descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the new root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name for the root.
    /// - `kind`: Named, tuple, newtype, or unit struct shape.
    /// - `fields`: Direct field descriptors in source order.
    ///
    /// # Returns
    ///
    /// Returns a struct root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_struct<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: StructKind,
        fields: &'static [FieldDescriptor],
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Struct(StructTypeDescriptor::new(kind)),
            fields,
            &[],
        )
    }

    /// Attaches generated struct construction entry points to this root.
    ///
    /// # Parameters
    ///
    /// - `construction`: Generated local and optional thread-safe constructor
    ///   metadata.
    ///
    /// # Returns
    ///
    /// Returns the root with its struct construction entry points attached.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub fn with_struct_construction(mut self, construction: StructConstructionDescriptor) -> Self {
        self.construction = Some(construction);
        self
    }

    /// Attaches generic declaration and concrete-instance facts to this root.
    ///
    /// # Parameters
    ///
    /// - `generic`: Concrete generic metadata for this root instance.
    ///
    /// # Returns
    ///
    /// Returns the root linked to its concrete generic metadata.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_concrete_generic(mut self, generic: &'static ConcreteGenericDescriptor) -> Self {
        self.generic = Some(generic);
        self
    }

    /// Links this concrete descriptor to its source-level generic declaration.
    ///
    /// # Parameters
    ///
    /// - `definition`: Resolver for the static source-level declaration.
    ///
    /// # Returns
    ///
    /// Returns the root linked to that generic declaration.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_type_definition(mut self, definition: fn() -> &'static TypeDefinitionDescriptor) -> Self {
        self.definition = Some(definition);
        self
    }

    /// Creates an enum root for generated descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust enum type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `variants`: Visible variants in source order.
    ///
    /// # Returns
    ///
    /// Returns an enum root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_enum<T: ?Sized + 'static>(
        query_name: &'static str,
        variants: &'static [VariantDescriptor],
    ) -> Self {
        Self::new_enum_with_repr::<T>(query_name, variants, &[])
    }

    /// Creates an enum root with normalized explicit representation metadata.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust enum type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `variants`: Visible variants in source order.
    /// - `representations`: Normalized `repr` hints retained from the source.
    ///
    /// # Returns
    ///
    /// Returns an enum root descriptor with representation metadata.
    #[doc(hidden)]
    pub(crate) const fn new_enum_with_repr<T: ?Sized + 'static>(
        query_name: &'static str,
        variants: &'static [VariantDescriptor],
        representations: &'static [EnumRepr],
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Enum(EnumTypeDescriptor::new(representations)),
            &[],
            variants,
        )
    }

    /// Creates a tuple root, including the zero-arity unit tuple, for built-in
    /// data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust tuple type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `elements`: Element type references in tuple order.
    ///
    /// # Returns
    ///
    /// Returns a tuple root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_tuple<T: ?Sized + 'static>(query_name: &'static str, elements: &'static [TypeRef]) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Tuple(TupleTypeDescriptor::new(elements)),
            &[],
            &[],
        )
    }

    /// Creates a tuple root whose element relationships resolve on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust tuple type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `elements`: Lazy element references in tuple order.
    ///
    /// # Returns
    ///
    /// Returns a tuple root with lazily resolved element types.
    #[doc(hidden)]
    pub(crate) const fn new_tuple_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        elements: &'static LazyTypeRefList,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Tuple(TupleTypeDescriptor::new_lazy(elements)),
            &[],
            &[],
        )
    }

    /// Creates an array root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust array type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `element`: Element type reference.
    /// - `length`: Fixed array length.
    ///
    /// # Returns
    ///
    /// Returns an array root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_array<T: ?Sized + 'static>(
        query_name: &'static str,
        element: &'static TypeRef,
        length: usize,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Array(ArrayTypeDescriptor::new(element, length)),
            &[],
            &[],
        )
    }

    /// Creates an array root whose element relationship resolves on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust array type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `element`: Lazy element type reference.
    /// - `length`: Fixed array length.
    ///
    /// # Returns
    ///
    /// Returns an array root with a lazily resolved element type.
    #[doc(hidden)]
    pub(crate) const fn new_array_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        element: &'static LazyTypeRef,
        length: usize,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Array(ArrayTypeDescriptor::new_lazy(element, length)),
            &[],
            &[],
        )
    }

    /// Creates an optional root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust optional type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `element`: Optional element type reference.
    ///
    /// # Returns
    ///
    /// Returns an optional root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_optional<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Optional(OptionalTypeDescriptor::new(element)),
            &[],
            &[],
        )
    }

    #[doc(hidden)]
    pub(crate) const fn new_optional_lazy<T: Reflect>(
        query_name: &'static str,
        element: &'static LazyTypeRef,
        projector: OptionalRefProjector,
    ) -> Self {
        Self::new::<Option<T>>(
            query_name,
            TypeDescriptorData::Optional(OptionalTypeDescriptor::new_lazy(element, projector)),
            &[],
            &[],
        )
    }

    /// Creates a sequence root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust sequence type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Sequence family, such as vector or deque.
    /// - `element`: Element type reference.
    ///
    /// # Returns
    ///
    /// Returns a sequence root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_sequence<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: SequenceKind,
        element: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Sequence(SequenceTypeDescriptor::new(kind, element)),
            &[],
            &[],
        )
    }

    /// Creates a sequence root whose element relationship resolves on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust sequence type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Sequence family, such as vector or deque.
    /// - `element`: Lazy element type reference.
    ///
    /// # Returns
    ///
    /// Returns a sequence root with a lazily resolved element type.
    #[doc(hidden)]
    pub(crate) const fn new_sequence_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: SequenceKind,
        element: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Sequence(SequenceTypeDescriptor::new_lazy(kind, element)),
            &[],
            &[],
        )
    }

    /// Creates a set root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust set type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Set family, such as ordered or hashed.
    /// - `element`: Element type reference.
    ///
    /// # Returns
    ///
    /// Returns a set root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_set<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: SetKind,
        element: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Set(SetTypeDescriptor::new(kind, element)),
            &[],
            &[],
        )
    }

    /// Creates a set root whose element relationship resolves on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust set type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Set family, such as ordered or hashed.
    /// - `element`: Lazy element type reference.
    ///
    /// # Returns
    ///
    /// Returns a set root with a lazily resolved element type.
    #[doc(hidden)]
    pub(crate) const fn new_set_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: SetKind,
        element: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Set(SetTypeDescriptor::new_lazy(kind, element)),
            &[],
            &[],
        )
    }

    /// Creates a map root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust map type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Ordered or hashed map family.
    /// - `key`: Key type reference.
    /// - `value`: Value type reference.
    ///
    /// # Returns
    ///
    /// Returns a map root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_map<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: MapKind,
        key: &'static TypeRef,
        value: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Map(MapTypeDescriptor::new(kind, key, value)),
            &[],
            &[],
        )
    }

    /// Creates a map root whose key and value relationships resolve on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust map type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Ordered or hashed map family.
    /// - `key`: Lazy key type reference.
    /// - `value`: Lazy value type reference.
    ///
    /// # Returns
    ///
    /// Returns a map root with lazily resolved key and value types.
    #[doc(hidden)]
    pub(crate) const fn new_map_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: MapKind,
        key: &'static LazyTypeRef,
        value: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Map(MapTypeDescriptor::new_lazy(kind, key, value)),
            &[],
            &[],
        )
    }

    /// Creates a smart-pointer root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust smart-pointer type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Smart-pointer family.
    /// - `pointee`: Pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a smart-pointer root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_smart_pointer<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: SmartPointerKind,
        pointee: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::SmartPointer(SmartPointerTypeDescriptor::new(kind, pointee)),
            &[],
            &[],
        )
    }

    /// Creates a smart-pointer root whose pointee relationship is resolved on
    /// first navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust smart-pointer type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Smart-pointer family.
    /// - `pointee`: Lazy pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a smart-pointer root with a lazily resolved pointee type.
    #[doc(hidden)]
    pub(crate) const fn new_smart_pointer_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: SmartPointerKind,
        pointee: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::SmartPointer(SmartPointerTypeDescriptor::new_lazy(kind, pointee)),
            &[],
            &[],
        )
    }

    /// Creates a reference root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust reference type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Shared or mutable reference category.
    /// - `target`: Referenced type reference.
    ///
    /// # Returns
    ///
    /// Returns a reference root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_reference<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: ReferenceKind,
        target: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Reference(ReferenceTypeDescriptor::new(kind, target)),
            &[],
            &[],
        )
    }

    /// Creates a reference root whose target relationship resolves on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust reference type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Shared or mutable reference category.
    /// - `target`: Lazy referenced type reference.
    ///
    /// # Returns
    ///
    /// Returns a reference root with a lazily resolved target type.
    #[doc(hidden)]
    pub(crate) const fn new_reference_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: ReferenceKind,
        target: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Reference(ReferenceTypeDescriptor::new_lazy(kind, target)),
            &[],
            &[],
        )
    }

    /// Creates a slice root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust slice type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `element`: Slice element type reference.
    ///
    /// # Returns
    ///
    /// Returns a slice root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_slice<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Slice(SliceTypeDescriptor::new(element)),
            &[],
            &[],
        )
    }

    /// Creates a slice root whose element relationship resolves on first
    /// navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust slice type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `element`: Lazy slice element reference.
    ///
    /// # Returns
    ///
    /// Returns a slice root with a lazily resolved element type.
    #[doc(hidden)]
    pub(crate) const fn new_slice_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        element: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Slice(SliceTypeDescriptor::new_lazy(element)),
            &[],
            &[],
        )
    }

    /// Creates a raw-pointer root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust raw-pointer type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `mutability`: Const or mutable pointer category.
    /// - `pointee`: Pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a raw-pointer root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_raw_pointer<T: ?Sized + 'static>(
        query_name: &'static str,
        mutability: Mutability,
        pointee: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::RawPointer(RawPointerTypeDescriptor::new(mutability, pointee)),
            &[],
            &[],
        )
    }

    /// Creates a raw-pointer root whose pointee relationship resolves on
    /// first navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust raw-pointer type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `mutability`: Const or mutable pointer category.
    /// - `pointee`: Lazy pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a raw-pointer root with a lazily resolved pointee type.
    #[doc(hidden)]
    pub(crate) const fn new_raw_pointer_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        mutability: Mutability,
        pointee: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::RawPointer(RawPointerTypeDescriptor::new_lazy(mutability, pointee)),
            &[],
            &[],
        )
    }

    /// Creates a function-pointer root for built-in descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust function-pointer type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Safe or unsafe function-pointer category.
    /// - `abi`: Calling convention retained by the descriptor.
    /// - `variadic`: Whether the signature accepts variadic arguments.
    /// - `parameters`: Parameter types in source order.
    /// - `return_type`: Return type reference.
    ///
    /// # Returns
    ///
    /// Returns a function-pointer root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_function<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: FunctionPointerKind,
        abi: &'static FunctionAbi,
        variadic: bool,
        parameters: &'static [TypeRef],
        return_type: &'static TypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Function(FunctionTypeDescriptor::new(
                kind,
                abi,
                variadic,
                parameters,
                return_type,
            )),
            &[],
            &[],
        )
    }

    /// Creates a function-pointer root whose signature relationships resolve
    /// on first navigation.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust function-pointer type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Safe or unsafe function-pointer category.
    /// - `abi`: Calling convention retained by the descriptor.
    /// - `variadic`: Whether the signature accepts variadic arguments.
    /// - `parameters`: Lazy parameter type references in source order.
    /// - `return_type`: Lazy return type reference.
    ///
    /// # Returns
    ///
    /// Returns a function-pointer root with lazily resolved signature types.
    #[doc(hidden)]
    pub(crate) const fn new_function_lazy<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: FunctionPointerKind,
        abi: &'static FunctionAbi,
        variadic: bool,
        parameters: &'static LazyTypeRefList,
        return_type: &'static LazyTypeRef,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Function(FunctionTypeDescriptor::new_lazy(
                kind,
                abi,
                variadic,
                parameters,
                return_type,
            )),
            &[],
            &[],
        )
    }

    /// Creates a trait-object root for generated descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust trait-object type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `trait_descriptor`: Resolver for the reflected trait root.
    ///
    /// # Returns
    ///
    /// Returns a trait-object root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_trait_object<T: ?Sized + 'static>(
        query_name: &'static str,
        trait_descriptor: fn() -> &'static crate::descriptor::TraitDescriptor,
    ) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::TraitObject(TraitObjectTypeDescriptor::new(trait_descriptor)),
            &[],
            &[],
        )
    }

    /// Creates an intentionally opaque root for generated descriptor data.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the opaque root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    ///
    /// # Returns
    ///
    /// Returns an opaque root descriptor for `T`.
    #[doc(hidden)]
    pub(crate) const fn new_opaque<T: ?Sized + 'static>(query_name: &'static str) -> Self {
        Self::new::<T>(query_name, TypeDescriptorData::Opaque(OpaqueTypeView), &[], &[])
    }

    /// Creates an opaque root with an explicit static capability resolver.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the opaque root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `capabilities`: Resolver for capabilities intrinsic to this root.
    ///
    /// # Returns
    ///
    /// Returns an opaque root descriptor with the supplied capability source.
    #[doc(hidden)]
    pub(crate) const fn new_opaque_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        capabilities: fn() -> TypeCapabilitiesResult,
    ) -> Self {
        Self::new_with_capabilities::<T>(
            query_name,
            TypeDescriptorData::Opaque(OpaqueTypeView),
            &[],
            &[],
            capabilities,
        )
    }

    /// Creates a primitive root with an explicit static capability resolver.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust primitive type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Primitive category represented by the root.
    /// - `capabilities`: Resolver for capabilities intrinsic to this root.
    ///
    /// # Returns
    ///
    /// Returns a primitive root descriptor with the supplied capability source.
    #[doc(hidden)]
    pub(crate) const fn new_primitive_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: PrimitiveKind,
        capabilities: fn() -> TypeCapabilitiesResult,
    ) -> Self {
        Self::new_with_capabilities::<T>(
            query_name,
            TypeDescriptorData::Primitive(PrimitiveTypeDescriptor::new(kind)),
            &[],
            &[],
            capabilities,
        )
    }

    /// Creates a text root with an explicit static capability resolver.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust text type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `kind`: Owned or borrowed UTF-8 text category.
    /// - `capabilities`: Resolver for capabilities intrinsic to this root.
    ///
    /// # Returns
    ///
    /// Returns a text root descriptor with the supplied capability source.
    #[doc(hidden)]
    pub(crate) const fn new_text_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: TextKind,
        capabilities: fn() -> TypeCapabilitiesResult,
    ) -> Self {
        Self::new_with_capabilities::<T>(
            query_name,
            TypeDescriptorData::Text(TextTypeDescriptor::new(kind)),
            &[],
            &[],
            capabilities,
        )
    }

    /// Builds common immutable root state without exposing an independently
    /// mutable builder.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `data`: Kind-specific root metadata.
    /// - `fields`: Direct fields in source order.
    /// - `variants`: Enum variants in source order.
    ///
    /// # Returns
    ///
    /// Returns a root with empty intrinsic capabilities and no optional
    /// construction or generic metadata.
    const fn new<T: ?Sized + 'static>(
        query_name: &'static str,
        data: TypeDescriptorData,
        fields: &'static [FieldDescriptor],
        variants: &'static [VariantDescriptor],
    ) -> Self {
        Self::new_with_capabilities::<T>(query_name, data, fields, variants, || {
            Ok(crate::capability::empty_capabilities())
        })
    }

    /// Builds common immutable root state with a descriptor-owned capability
    /// resolver.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Rust type represented by the root.
    ///
    /// # Parameters
    ///
    /// - `query_name`: Reflection lookup name.
    /// - `data`: Kind-specific root metadata.
    /// - `fields`: Direct fields in source order.
    /// - `variants`: Enum variants in source order.
    /// - `capabilities`: Resolver for the root's intrinsic capabilities.
    ///
    /// # Returns
    ///
    /// Returns a root initialized with the supplied identity, members, and
    /// capability resolver.
    const fn new_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        data: TypeDescriptorData,
        fields: &'static [FieldDescriptor],
        variants: &'static [VariantDescriptor],
        capabilities: fn() -> TypeCapabilitiesResult,
    ) -> Self {
        Self {
            type_id: type_id_of::<T>,
            type_name: type_name_of::<T>,
            query_name,
            data,
            fields,
            variants,
            capabilities,
            construction: None,
            generic: None,
            definition: None,
        }
    }

    /// Replaces this descriptor's immutable capability resolver.
    ///
    /// Generated descriptor roots call this before the root is interned, so
    /// every query observes one stable capability set for the concrete type.
    ///
    /// # Parameters
    ///
    /// - `capabilities`: Resolver for capabilities intrinsic to this root.
    ///
    /// # Returns
    ///
    /// Returns the root with its capability resolver replaced.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_capabilities(mut self, capabilities: fn() -> TypeCapabilitiesResult) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Returns the process-local Rust type identity.
    ///
    /// # Returns
    ///
    /// Returns the exact `TypeId` represented by this root.
    #[must_use]
    #[inline]
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the diagnostic Rust type name.
    ///
    /// # Returns
    ///
    /// Returns the compiler-provided Rust type name.
    #[must_use]
    #[inline]
    pub fn type_name(&self) -> &'static str {
        (self.type_name)()
    }

    /// Returns the immutable lookup name, which may differ from
    /// [`Self::type_name`].
    ///
    /// # Returns
    ///
    /// Returns the immutable reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns capabilities declared directly by this descriptor.
    ///
    /// # Returns
    ///
    /// Returns the intrinsic capabilities for this concrete root.
    pub(crate) fn declared_capabilities(&self) -> TypeCapabilitiesResult {
        (self.capabilities)()
    }

    /// Returns the stable hierarchical type category.
    ///
    /// # Returns
    ///
    /// Returns the category and any primitive, shape, pointer, or mutability
    /// qualifier attached to it.
    #[must_use]
    pub const fn kind(&self) -> TypeKind {
        match &self.data {
            TypeDescriptorData::Primitive(view) => TypeKind::Primitive(view.kind()),
            TypeDescriptorData::Text(view) => TypeKind::Text(view.kind()),
            TypeDescriptorData::Struct(view) => TypeKind::Struct(view.kind()),
            TypeDescriptorData::Enum(_) => TypeKind::Enum,
            TypeDescriptorData::Tuple(_) => TypeKind::Tuple,
            TypeDescriptorData::Array(_) => TypeKind::Array,
            TypeDescriptorData::Optional(_) => TypeKind::Optional,
            TypeDescriptorData::Sequence(_) => TypeKind::Sequence,
            TypeDescriptorData::Set(_) => TypeKind::Set,
            TypeDescriptorData::Map(_) => TypeKind::Map,
            TypeDescriptorData::SmartPointer(view) => TypeKind::SmartPointer(view.kind()),
            TypeDescriptorData::Reference(view) => TypeKind::Reference(view.kind()),
            TypeDescriptorData::Slice(_) => TypeKind::Slice,
            TypeDescriptorData::RawPointer(view) => TypeKind::RawPointer(view.mutability()),
            TypeDescriptorData::Function(view) => TypeKind::FunctionPointer(view.kind()),
            TypeDescriptorData::TraitObject(_) => TypeKind::TraitObject,
            TypeDescriptorData::Opaque(_) => TypeKind::Opaque,
        }
    }

    /// Returns the primitive view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns primitive metadata, or `None` when this root is not primitive.
    #[must_use]
    #[inline]
    pub const fn as_primitive(&self) -> Option<&PrimitiveTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Primitive(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the text view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns text metadata, or `None` when this root is not a text type.
    #[must_use]
    #[inline]
    pub const fn as_text(&self) -> Option<&TextTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Text(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the struct view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns struct shape metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_struct(&self) -> Option<&StructTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Struct(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the enum view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns enum representation metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_enum(&self) -> Option<&EnumTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Enum(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the tuple view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns tuple element metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_tuple(&self) -> Option<&TupleTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Tuple(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the array view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns array element and length metadata, or `None` for other kinds.
    #[must_use]
    #[inline]
    pub const fn as_array(&self) -> Option<&ArrayTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Array(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the optional view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns optional element metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_optional(&self) -> Option<&OptionalTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Optional(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the sequence view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns sequence kind and element metadata, or `None` for other kinds.
    #[must_use]
    #[inline]
    pub const fn as_sequence(&self) -> Option<&SequenceTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Sequence(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the set view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns set kind and element metadata, or `None` for other kinds.
    #[must_use]
    #[inline]
    pub const fn as_set(&self) -> Option<&SetTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Set(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the map view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns map kind, key, and value metadata, or `None` for other kinds.
    #[must_use]
    #[inline]
    pub const fn as_map(&self) -> Option<&MapTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Map(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the smart-pointer view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns smart-pointer kind and pointee metadata, or `None` otherwise.
    #[must_use]
    #[inline]
    pub const fn as_smart_pointer(&self) -> Option<&SmartPointerTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::SmartPointer(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the reference view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns reference kind and target metadata, or `None` otherwise.
    #[must_use]
    #[inline]
    pub const fn as_reference(&self) -> Option<&ReferenceTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Reference(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the slice view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns slice element metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_slice(&self) -> Option<&SliceTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Slice(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the raw-pointer view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns raw-pointer mutability and pointee metadata, or `None`
    /// otherwise.
    #[must_use]
    #[inline]
    pub const fn as_raw_pointer(&self) -> Option<&RawPointerTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::RawPointer(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the function-pointer view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns function signature metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_function(&self) -> Option<&FunctionTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Function(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the trait-object view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns trait-object metadata, or `None` for other root kinds.
    #[must_use]
    #[inline]
    pub const fn as_trait_object(&self) -> Option<&TraitObjectTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::TraitObject(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the opaque-root view, or `None` for every other kind.
    ///
    /// # Returns
    ///
    /// Returns the opaque marker, or `None` when this root exposes another
    /// kind.
    #[must_use]
    #[inline]
    pub const fn as_opaque(&self) -> Option<&OpaqueTypeView> {
        match &self.data {
            TypeDescriptorData::Opaque(view) => Some(view),
            _ => None,
        }
    }

    /// Returns direct struct fields in source declaration order.
    ///
    /// Non-struct roots, including enum roots, return an empty slice.
    ///
    /// # Returns
    ///
    /// Returns direct fields, or an empty slice for roots without direct
    /// fields.
    #[must_use]
    #[inline]
    pub const fn fields(&self) -> &'static [FieldDescriptor] {
        self.fields
    }

    /// Finds a direct named field by query name.
    ///
    /// `None` means the root has no direct field with that lookup name.
    ///
    /// # Parameters
    ///
    /// - `name`: Field lookup name to match.
    ///
    /// # Returns
    ///
    /// Returns the matching direct field, or `None` when no field has that
    /// name.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor> {
        self.fields.iter().find(|field| field.query_name() == Some(name))
    }

    /// Returns a direct field by source index.
    ///
    /// `None` means the index is outside the direct field range.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based position in the direct field slice.
    ///
    /// # Returns
    ///
    /// Returns the field at that index, or `None` when out of range.
    #[must_use]
    pub fn field_at(&self, index: usize) -> Option<&FieldDescriptor> {
        self.fields.get(index)
    }

    /// Returns enum variants in source declaration order.
    ///
    /// Non-enum roots return an empty slice.
    ///
    /// # Returns
    ///
    /// Returns variants in source order, or an empty slice for non-enum roots.
    #[must_use]
    #[inline]
    pub const fn variants(&self) -> &'static [VariantDescriptor] {
        self.variants
    }

    /// Finds a variant by query name.
    ///
    /// `None` means the root has no variant with that lookup name.
    ///
    /// # Parameters
    ///
    /// - `name`: Variant lookup name to match.
    ///
    /// # Returns
    ///
    /// Returns the matching variant, or `None` when absent.
    #[must_use]
    pub fn variant(&self, name: &str) -> Option<&VariantDescriptor> {
        self.variants.iter().find(|variant| variant.query_name() == name)
    }

    /// Returns a variant by source index.
    ///
    /// `None` means no visible variant has the source declaration index.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source variant index.
    ///
    /// # Returns
    ///
    /// Returns the visible variant with that source index, or `None` when
    /// absent.
    #[must_use]
    pub fn variant_at(&self, index: usize) -> Option<&VariantDescriptor> {
        self.variants.iter().find(|variant| variant.index() == index)
    }

    /// Finds the fieldless integer-`repr` variant with the exact numeric value.
    ///
    /// # Parameters
    ///
    /// - `discriminant`: Signed or unsigned integer value to match.
    ///
    /// # Returns
    ///
    /// Returns the matching fieldless variant, or `None` when no variant has
    /// that exact numeric discriminant.
    #[must_use]
    pub fn variant_by_discriminant(
        &self,
        discriminant: crate::descriptor::NumericDiscriminant,
    ) -> Option<&VariantDescriptor> {
        self.variants
            .iter()
            .find(|variant| variant.numeric_discriminant() == Some(discriminant))
    }

    /// Returns every linked reflected implementation targeting this exact
    /// root in deterministic registry order.
    ///
    /// Both the returned slice and its descriptors belong to the immutable
    /// process-wide registry. A cached [`RegistryError`] is returned when
    /// distributed registration could not be aggregated.
    ///
    /// # Returns
    ///
    /// Returns implementations targeting this type in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns the cached registry initialization error if registration could
    /// not be aggregated.
    #[must_use = "inspect the reflected implementations or handle the registry error"]
    pub fn impls(&self) -> Result<&'static [&'static ImplDescriptor], RegistryError> {
        let registry = ReflectRegistry::initialize()?;
        Ok(self.impls_in(registry))
    }

    /// Returns implementations from an explicitly supplied immutable registry
    /// snapshot without consulting process-wide initialization.
    ///
    /// # Parameters
    ///
    /// - `registry`: Snapshot from which this root's implementations are read.
    ///
    /// # Returns
    ///
    /// Returns implementations borrowed from the supplied snapshot in its
    /// deterministic order.
    #[must_use]
    #[inline]
    pub fn impls_in<'registry>(&self, registry: &'registry ReflectRegistry) -> &'registry [&'static ImplDescriptor] {
        registry.implementations(self.type_id())
    }

    /// Returns the frozen effective method instances for this exact root.
    ///
    /// The view includes inherent methods and concrete trait methods, with
    /// defaulted trait methods replaced by their effective override where
    /// applicable. The slice is empty when no reflected impl targets this
    /// root. A cached [`RegistryError`] is returned when aggregation failed.
    ///
    /// # Returns
    ///
    /// Returns the frozen effective method instances for this type.
    ///
    /// # Errors
    ///
    /// Returns the cached registry initialization error if registration could
    /// not be aggregated.
    #[must_use = "inspect the effective methods or handle the registry error"]
    pub fn methods(&self) -> Result<&'static [&'static MethodInstanceDescriptor], RegistryError> {
        let registry = ReflectRegistry::initialize()?;
        Ok(self.methods_in(registry))
    }

    /// Returns effective methods from an explicitly supplied immutable
    /// registry snapshot.
    ///
    /// # Parameters
    ///
    /// - `registry`: Snapshot whose effective method view is queried.
    ///
    /// # Returns
    ///
    /// Returns effective method instances borrowed from the supplied snapshot.
    #[must_use]
    #[inline]
    pub fn methods_in<'registry>(
        &self,
        registry: &'registry ReflectRegistry,
    ) -> &'registry [&'static MethodInstanceDescriptor] {
        registry.effective_view(self.type_id()).methods()
    }

    /// Looks up an effective method by query name across all namespaces.
    ///
    /// [`MethodLookup::Missing`] means no method matched, while
    /// [`MethodLookup::Ambiguous`] means multiple inherent or trait namespaces
    /// use the name. Callers that need a qualified namespace can use
    /// [`crate::registry::EffectiveTypeView::lookup_method`] on a registry
    /// view. A cached [`RegistryError`] is returned when aggregation failed.
    ///
    /// # Parameters
    ///
    /// - `name`: Method query name to match.
    ///
    /// # Returns
    ///
    /// Returns the missing, unique, or ambiguous lookup result.
    ///
    /// # Errors
    ///
    /// Returns the cached registry initialization error if registration could
    /// not be aggregated.
    #[must_use = "inspect the method lookup result or handle the registry error"]
    pub fn methods_named(&self, name: &str) -> Result<MethodLookup<'static>, RegistryError> {
        let registry = ReflectRegistry::initialize()?;
        Ok(self.methods_named_in(registry, name))
    }

    /// Looks up an effective method in an explicitly supplied immutable
    /// registry snapshot.
    ///
    /// # Parameters
    ///
    /// - `registry`: Snapshot whose effective method view is queried.
    /// - `name`: Method query name to match.
    ///
    /// # Returns
    ///
    /// Returns the missing, unique, or ambiguous lookup result borrowing the
    /// supplied registry snapshot.
    #[must_use]
    #[inline]
    pub fn methods_named_in<'registry>(
        &self,
        registry: &'registry ReflectRegistry,
        name: &str,
    ) -> MethodLookup<'registry> {
        registry
            .effective_view(self.type_id())
            .lookup_method(MethodQualifier::Any, name)
    }
}

impl fmt::Debug for TypeDescriptor {
    /// Formats root-local facts and collection sizes without recursively
    /// expanding relationships or evaluating intrinsic capability providers.
    /// Logging a descriptor from its own provider cannot re-enter
    /// initialization.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination for the root's local facts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the local representation.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TypeDescriptor")
            .field("type_id", &self.type_id())
            .field("type_name", &self.type_name())
            .field("query_name", &self.query_name)
            .field("kind", &self.kind())
            .field("field_count", &self.fields.len())
            .field("variant_count", &self.variants.len())
            .finish()
    }
}
