//! The unique root descriptor and [`Reflect`] query trait.

use std::any::TypeId;
use std::fmt;

use crate::capability::CapabilityKey;
use crate::capability::TypeCapabilities;
use crate::descriptor::ArrayTypeDescriptor;
use crate::descriptor::EnumTypeDescriptor;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::FunctionTypeDescriptor;
use crate::descriptor::MapKind;
use crate::descriptor::MapTypeDescriptor;
use crate::descriptor::Mutability;
use crate::descriptor::OpaqueTypeView;
use crate::descriptor::OptionalTypeDescriptor;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::PrimitiveTypeDescriptor;
use crate::descriptor::RawPointerTypeDescriptor;
use crate::descriptor::ReferenceKind;
use crate::descriptor::ReferenceTypeDescriptor;
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
use crate::descriptor::TypeKind;
use crate::descriptor::TypeRef;
use crate::descriptor::VariantDescriptor;
use crate::descriptor::type_ref::type_id_of;
use crate::descriptor::type_ref::type_name_of;
use crate::expression::FunctionAbi;

/// The sole public generic contract for types that provide a static reflection
/// descriptor.
pub trait Reflect: 'static {
    /// Returns the unique root descriptor for `Self`.
    fn type_descriptor() -> &'static TypeDescriptor;
}

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
pub struct TypeDescriptor {
    type_id: fn() -> TypeId,
    type_name: fn() -> &'static str,
    query_name: &'static str,
    data: TypeDescriptorData,
    fields: &'static [FieldDescriptor],
    variants: &'static [VariantDescriptor],
    capabilities: fn() -> &'static TypeCapabilities,
}

impl TypeDescriptor {
    /// Returns the unique root descriptor supplied by `T`'s [`Reflect`]
    /// implementation.
    pub fn of<T: Reflect + ?Sized>() -> &'static Self {
        T::type_descriptor()
    }

    /// Creates a primitive root for generated or built-in descriptor data.
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

    /// Creates an enum root for generated descriptor data.
    #[doc(hidden)]
    pub(crate) const fn new_enum<T: ?Sized + 'static>(
        query_name: &'static str,
        variants: &'static [VariantDescriptor],
    ) -> Self {
        Self::new::<T>(query_name, TypeDescriptorData::Enum(EnumTypeDescriptor), &[], variants)
    }

    /// Creates a tuple root, including the zero-arity unit tuple, for built-in
    /// data.
    #[doc(hidden)]
    pub(crate) const fn new_tuple<T: ?Sized + 'static>(query_name: &'static str, elements: &'static [TypeRef]) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Tuple(TupleTypeDescriptor::new(elements)),
            &[],
            &[],
        )
    }

    /// Creates an array root for built-in descriptor data.
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

    /// Creates an optional root for built-in descriptor data.
    #[doc(hidden)]
    pub(crate) const fn new_optional<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Optional(OptionalTypeDescriptor::new(element)),
            &[],
            &[],
        )
    }

    /// Creates a sequence root for built-in descriptor data.
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

    /// Creates a set root for built-in descriptor data.
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

    /// Creates a map root for built-in descriptor data.
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

    /// Creates a smart-pointer root for built-in descriptor data.
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

    /// Creates a reference root for built-in descriptor data.
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

    /// Creates a slice root for built-in descriptor data.
    #[doc(hidden)]
    pub(crate) const fn new_slice<T: ?Sized + 'static>(query_name: &'static str, element: &'static TypeRef) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::Slice(SliceTypeDescriptor::new(element)),
            &[],
            &[],
        )
    }

    /// Creates a raw-pointer root for built-in descriptor data.
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

    /// Creates a function-pointer root for built-in descriptor data.
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

    /// Creates a trait-object root for generated descriptor data.
    #[doc(hidden)]
    pub(crate) const fn new_trait_object<T: ?Sized + 'static>(query_name: &'static str) -> Self {
        Self::new::<T>(
            query_name,
            TypeDescriptorData::TraitObject(TraitObjectTypeDescriptor),
            &[],
            &[],
        )
    }

    /// Creates an intentionally opaque root for generated descriptor data.
    #[doc(hidden)]
    pub(crate) const fn new_opaque<T: ?Sized + 'static>(query_name: &'static str) -> Self {
        Self::new::<T>(query_name, TypeDescriptorData::Opaque(OpaqueTypeView), &[], &[])
    }

    /// Creates an opaque root with an explicit static capability resolver.
    #[doc(hidden)]
    pub(crate) const fn new_opaque_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        capabilities: fn() -> &'static TypeCapabilities,
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
    #[doc(hidden)]
    pub(crate) const fn new_primitive_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: PrimitiveKind,
        capabilities: fn() -> &'static TypeCapabilities,
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
    #[doc(hidden)]
    pub(crate) const fn new_text_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        kind: TextKind,
        capabilities: fn() -> &'static TypeCapabilities,
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
    const fn new<T: ?Sized + 'static>(
        query_name: &'static str,
        data: TypeDescriptorData,
        fields: &'static [FieldDescriptor],
        variants: &'static [VariantDescriptor],
    ) -> Self {
        Self::new_with_capabilities::<T>(
            query_name,
            data,
            fields,
            variants,
            crate::capability::empty_capabilities,
        )
    }

    /// Builds common immutable root state with a descriptor-owned capability
    /// resolver.
    const fn new_with_capabilities<T: ?Sized + 'static>(
        query_name: &'static str,
        data: TypeDescriptorData,
        fields: &'static [FieldDescriptor],
        variants: &'static [VariantDescriptor],
        capabilities: fn() -> &'static TypeCapabilities,
    ) -> Self {
        Self {
            type_id: type_id_of::<T>,
            type_name: type_name_of::<T>,
            query_name,
            data,
            fields,
            variants,
            capabilities,
        }
    }

    /// Returns the process-local Rust type identity.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the diagnostic Rust type name.
    pub fn type_name(&self) -> &'static str {
        (self.type_name)()
    }

    /// Returns the immutable lookup name, which may differ from
    /// [`Self::type_name`].
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns this descriptor's immutable registered capability set.
    pub fn capabilities(&self) -> &'static TypeCapabilities {
        (self.capabilities)()
    }

    /// Retrieves one executable capability adapter through its typed key.
    ///
    /// `None` means this descriptor did not register the key, registered a
    /// different contract, or records a fact without an operation adapter.
    pub fn get_capability<A: 'static>(&self, key: CapabilityKey<A>) -> Option<&A> {
        self.capabilities().get(key)
    }

    /// Returns the stable hierarchical type category.
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
    pub const fn as_primitive(&self) -> Option<&PrimitiveTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Primitive(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the text view, or `None` for every other kind.
    pub const fn as_text(&self) -> Option<&TextTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Text(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the struct view, or `None` for every other kind.
    pub const fn as_struct(&self) -> Option<&StructTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Struct(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the enum view, or `None` for every other kind.
    pub const fn as_enum(&self) -> Option<&EnumTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Enum(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the tuple view, or `None` for every other kind.
    pub const fn as_tuple(&self) -> Option<&TupleTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Tuple(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the array view, or `None` for every other kind.
    pub const fn as_array(&self) -> Option<&ArrayTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Array(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the optional view, or `None` for every other kind.
    pub const fn as_optional(&self) -> Option<&OptionalTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Optional(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the sequence view, or `None` for every other kind.
    pub const fn as_sequence(&self) -> Option<&SequenceTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Sequence(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the set view, or `None` for every other kind.
    pub const fn as_set(&self) -> Option<&SetTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Set(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the map view, or `None` for every other kind.
    pub const fn as_map(&self) -> Option<&MapTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Map(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the smart-pointer view, or `None` for every other kind.
    pub const fn as_smart_pointer(&self) -> Option<&SmartPointerTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::SmartPointer(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the reference view, or `None` for every other kind.
    pub const fn as_reference(&self) -> Option<&ReferenceTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Reference(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the slice view, or `None` for every other kind.
    pub const fn as_slice(&self) -> Option<&SliceTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Slice(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the raw-pointer view, or `None` for every other kind.
    pub const fn as_raw_pointer(&self) -> Option<&RawPointerTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::RawPointer(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the function-pointer view, or `None` for every other kind.
    pub const fn as_function(&self) -> Option<&FunctionTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::Function(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the trait-object view, or `None` for every other kind.
    pub const fn as_trait_object(&self) -> Option<&TraitObjectTypeDescriptor> {
        match &self.data {
            TypeDescriptorData::TraitObject(view) => Some(view),
            _ => None,
        }
    }

    /// Returns the opaque-root view, or `None` for every other kind.
    pub const fn as_opaque(&self) -> Option<&OpaqueTypeView> {
        match &self.data {
            TypeDescriptorData::Opaque(view) => Some(view),
            _ => None,
        }
    }

    /// Returns direct struct fields in source declaration order.
    ///
    /// Non-struct roots, including enum roots, return an empty slice.
    pub const fn fields(&self) -> &'static [FieldDescriptor] {
        self.fields
    }

    /// Finds a direct named field by query name.
    ///
    /// `None` means the root has no direct field with that lookup name.
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor> {
        self.fields.iter().find(|field| field.query_name() == Some(name))
    }

    /// Returns a direct field by source index.
    ///
    /// `None` means the index is outside the direct field range.
    pub fn field_at(&self, index: usize) -> Option<&FieldDescriptor> {
        self.fields.get(index)
    }

    /// Returns enum variants in source declaration order.
    ///
    /// Non-enum roots return an empty slice.
    pub const fn variants(&self) -> &'static [VariantDescriptor] {
        self.variants
    }

    /// Finds a variant by query name.
    ///
    /// `None` means the root has no variant with that lookup name.
    pub fn variant(&self, name: &str) -> Option<&VariantDescriptor> {
        self.variants.iter().find(|variant| variant.query_name() == name)
    }

    /// Returns a variant by source index.
    ///
    /// `None` means the index is outside the variant range.
    pub fn variant_at(&self, index: usize) -> Option<&VariantDescriptor> {
        self.variants.get(index)
    }

    /// Finds the fieldless integer-`repr` variant with the exact numeric value.
    pub fn variant_by_discriminant(
        &self,
        discriminant: crate::descriptor::NumericDiscriminant,
    ) -> Option<&VariantDescriptor> {
        self.variants
            .iter()
            .find(|variant| variant.numeric_discriminant() == Some(discriminant))
    }
}

impl fmt::Debug for TypeDescriptor {
    /// Formats root-local facts and collection sizes without recursively
    /// expanding relationships.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TypeDescriptor")
            .field("type_id", &self.type_id())
            .field("type_name", &self.type_name())
            .field("query_name", &self.query_name)
            .field("kind", &self.kind())
            .field("field_count", &self.fields.len())
            .field("variant_count", &self.variants.len())
            .field("capability_count", &self.capabilities().descriptors().len())
            .finish()
    }
}
