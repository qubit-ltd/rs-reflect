// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::collections::Fields;
use super::collections::Variants;
use super::field::FieldMetadata;
use super::variant::VariantMetadata;
use crate::runtime::descriptor::TypeDescriptor;

/// Static kind facts supplied by the type registration.
#[derive(Clone, Copy)]
pub enum TypeKindMetadata {
    /// A built-in scalar or other primitive value.
    Primitive,
    /// An owned or borrowed text type.
    Text,
    /// A struct with its reflected fields.
    Struct(&'static [FieldMetadata]),
    /// An enum with its reflected variants.
    Enum(&'static [VariantMetadata]),
    /// A tuple type with its reflected fields.
    Tuple(&'static [FieldMetadata]),
    /// A fixed-size array.
    Array,
    /// An optional value such as `Option<T>`.
    Optional,
    /// A sequential collection.
    Sequence,
    /// An unordered collection of distinct values.
    Set,
    /// A key-value collection.
    Map,
    /// A type that provides shared or owned indirection.
    SmartPointer,
    /// A reference type.
    Reference,
    /// A dynamically sized slice.
    Slice,
    /// A raw pointer type.
    RawPointer,
    /// A function or function pointer type.
    Function,
    /// A trait object type.
    TraitObject,
    /// A type without a more specific reflected shape.
    Opaque,
}

/// A type category paired with its context-bound descriptor.
pub enum TypeKind<'ctx> {
    /// A primitive type.
    Primitive(PrimitiveType<'ctx>),
    /// A text type.
    Text(TextType<'ctx>),
    /// A struct type.
    Struct(StructType<'ctx>),
    /// An enum type.
    Enum(EnumType<'ctx>),
    /// A tuple type.
    Tuple(TupleType<'ctx>),
    /// An array type.
    Array(ArrayType<'ctx>),
    /// An optional type.
    Optional(OptionalType<'ctx>),
    /// A sequence collection.
    Sequence(SequenceType<'ctx>),
    /// A set collection.
    Set(SetType<'ctx>),
    /// A map collection.
    Map(MapType<'ctx>),
    /// A smart pointer type.
    SmartPointer(SmartPointerType<'ctx>),
    /// A reference type.
    Reference(ReferenceType<'ctx>),
    /// A slice type.
    Slice(SliceType<'ctx>),
    /// A raw pointer type.
    RawPointer(RawPointerType<'ctx>),
    /// A function type.
    Function(FunctionType<'ctx>),
    /// A trait object type.
    TraitObject(TraitObjectType<'ctx>),
    /// An opaque type.
    Opaque(OpaqueType<'ctx>),
}

impl<'ctx> TypeKind<'ctx> {
    pub(crate) fn from_metadata(ty: TypeDescriptor<'ctx>, metadata: TypeKindMetadata) -> Self {
        match metadata {
            TypeKindMetadata::Primitive => Self::Primitive(PrimitiveType(ty)),
            TypeKindMetadata::Text => Self::Text(TextType(ty)),
            TypeKindMetadata::Struct(fields) => Self::Struct(StructType {
                ty,
                fields: Fields::new(ty, fields, None),
            }),
            TypeKindMetadata::Enum(variants) => Self::Enum(EnumType {
                ty,
                variants: Variants::new(ty, variants),
            }),
            TypeKindMetadata::Tuple(fields) => Self::Tuple(TupleType {
                ty,
                fields: Fields::new(ty, fields, None),
            }),
            TypeKindMetadata::Array => Self::Array(ArrayType(ty)),
            TypeKindMetadata::Optional => Self::Optional(OptionalType(ty)),
            TypeKindMetadata::Sequence => Self::Sequence(SequenceType(ty)),
            TypeKindMetadata::Set => Self::Set(SetType(ty)),
            TypeKindMetadata::Map => Self::Map(MapType(ty)),
            TypeKindMetadata::SmartPointer => Self::SmartPointer(SmartPointerType(ty)),
            TypeKindMetadata::Reference => Self::Reference(ReferenceType(ty)),
            TypeKindMetadata::Slice => Self::Slice(SliceType(ty)),
            TypeKindMetadata::RawPointer => Self::RawPointer(RawPointerType(ty)),
            TypeKindMetadata::Function => Self::Function(FunctionType(ty)),
            TypeKindMetadata::TraitObject => Self::TraitObject(TraitObjectType(ty)),
            TypeKindMetadata::Opaque => Self::Opaque(OpaqueType(ty)),
        }
    }

    /// Returns the struct view, or `None` for other type kinds.
    pub fn as_struct(&self) -> Option<StructType<'ctx>> {
        match self {
            Self::Struct(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the enum view, or `None` for other type kinds.
    pub fn as_enum(&self) -> Option<EnumType<'ctx>> {
        match self {
            Self::Enum(value) => Some(*value),
            _ => None,
        }
    }

    /// Returns the sequence view, or `None` for other type kinds.
    pub fn as_sequence(&self) -> Option<SequenceType<'ctx>> {
        match self {
            Self::Sequence(value) => Some(*value),
            _ => None,
        }
    }
}

/// A reflected struct with its source ordered fields.
#[derive(Clone, Copy)]
pub struct StructType<'ctx> {
    ty: TypeDescriptor<'ctx>,
    fields: Fields<'ctx>,
}

impl<'ctx> StructType<'ctx> {
    /// Returns the descriptor for the underlying type.
    pub fn descriptor(self) -> TypeDescriptor<'ctx> {
        self.ty
    }
    /// Returns the struct's fields in source order.
    pub fn fields(self) -> Fields<'ctx> {
        self.fields
    }
}

/// A reflected enum with its source ordered variants.
#[derive(Clone, Copy)]
pub struct EnumType<'ctx> {
    ty: TypeDescriptor<'ctx>,
    variants: Variants<'ctx>,
}

impl<'ctx> EnumType<'ctx> {
    /// Returns the descriptor for the underlying type.
    pub fn descriptor(self) -> TypeDescriptor<'ctx> {
        self.ty
    }
    /// Returns the enum's variants in source order.
    pub fn variants(self) -> Variants<'ctx> {
        self.variants
    }
}

/// A tuple struct or tuple type with source ordered fields.
#[derive(Clone, Copy)]
pub struct TupleType<'ctx> {
    ty: TypeDescriptor<'ctx>,
    fields: Fields<'ctx>,
}

impl<'ctx> TupleType<'ctx> {
    /// Returns the descriptor for the underlying type.
    pub fn descriptor(self) -> TypeDescriptor<'ctx> {
        self.ty
    }
    /// Returns the tuple's fields in source order.
    pub fn fields(self) -> Fields<'ctx> {
        self.fields
    }
}

macro_rules! descriptor_view {
    ($name:ident, $description:literal) => {
        #[derive(Clone, Copy)]
        #[doc = $description]
        pub struct $name<'ctx>(TypeDescriptor<'ctx>);
        impl<'ctx> $name<'ctx> {
            /// Returns the descriptor for the underlying type.
            pub fn descriptor(self) -> TypeDescriptor<'ctx> {
                self.0
            }
        }
    };
}

descriptor_view!(PrimitiveType, "A primitive type.");
descriptor_view!(TextType, "A text type.");
descriptor_view!(ArrayType, "A fixed-size array type.");
descriptor_view!(OptionalType, "An optional type.");
descriptor_view!(SequenceType, "A sequential collection type.");
descriptor_view!(SetType, "A set collection type.");
descriptor_view!(MapType, "A key-value collection type.");
descriptor_view!(SmartPointerType, "A smart pointer type.");
descriptor_view!(ReferenceType, "A reference type.");
descriptor_view!(SliceType, "A dynamically sized slice type.");
descriptor_view!(RawPointerType, "A raw pointer type.");
descriptor_view!(FunctionType, "A function type.");
descriptor_view!(TraitObjectType, "A trait object type.");
descriptor_view!(OpaqueType, "An opaque reflected type.");

#[cfg(test)]
mod tests {
    use super::TypeKind;
    use super::TypeKindMetadata;
    use crate::runtime::descriptor::TypeDescriptor;

    #[test]
    fn metadata_maps_to_each_public_kind_view() {
        let ty = TypeDescriptor::of::<u8>();

        macro_rules! assert_kind {
            ($metadata:expr, $variant:ident) => {{
                let kind = TypeKind::from_metadata(ty, $metadata);
                assert!(matches!(
                    &kind,
                    TypeKind::$variant(view) if view.descriptor().type_id() == ty.type_id()
                ));
            }};
        }

        assert_kind!(TypeKindMetadata::Primitive, Primitive);
        assert_kind!(TypeKindMetadata::Text, Text);
        assert_kind!(TypeKindMetadata::Struct(&[]), Struct);
        assert_kind!(TypeKindMetadata::Enum(&[]), Enum);
        assert_kind!(TypeKindMetadata::Tuple(&[]), Tuple);
        assert_kind!(TypeKindMetadata::Array, Array);
        assert_kind!(TypeKindMetadata::Optional, Optional);
        assert_kind!(TypeKindMetadata::Sequence, Sequence);
        assert_kind!(TypeKindMetadata::Set, Set);
        assert_kind!(TypeKindMetadata::Map, Map);
        assert_kind!(TypeKindMetadata::SmartPointer, SmartPointer);
        assert_kind!(TypeKindMetadata::Reference, Reference);
        assert_kind!(TypeKindMetadata::Slice, Slice);
        assert_kind!(TypeKindMetadata::RawPointer, RawPointer);
        assert_kind!(TypeKindMetadata::Function, Function);
        assert_kind!(TypeKindMetadata::TraitObject, TraitObject);
        assert_kind!(TypeKindMetadata::Opaque, Opaque);

        let struct_kind = TypeKind::from_metadata(ty, TypeKindMetadata::Struct(&[]));
        let TypeKind::Struct(struct_view) = struct_kind else {
            panic!("struct metadata should produce the struct view");
        };
        assert!(struct_view.fields().is_empty());

        let enum_kind = TypeKind::from_metadata(ty, TypeKindMetadata::Enum(&[]));
        let TypeKind::Enum(enum_view) = enum_kind else {
            panic!("enum metadata should produce the enum view");
        };
        assert!(enum_view.variants().is_empty());

        let tuple_kind = TypeKind::from_metadata(ty, TypeKindMetadata::Tuple(&[]));
        let TypeKind::Tuple(tuple_view) = tuple_kind else {
            panic!("tuple metadata should produce the tuple view");
        };
        assert!(tuple_view.fields().is_empty());
    }

    #[test]
    fn kind_downcasts_return_none_for_unmatched_categories() {
        let ty = TypeDescriptor::of::<u8>();
        let metadata = [
            TypeKindMetadata::Primitive,
            TypeKindMetadata::Text,
            TypeKindMetadata::Tuple(&[]),
            TypeKindMetadata::Array,
            TypeKindMetadata::Optional,
            TypeKindMetadata::Set,
            TypeKindMetadata::Map,
            TypeKindMetadata::SmartPointer,
            TypeKindMetadata::Reference,
            TypeKindMetadata::Slice,
            TypeKindMetadata::RawPointer,
            TypeKindMetadata::Function,
            TypeKindMetadata::TraitObject,
            TypeKindMetadata::Opaque,
        ];

        for metadata in metadata {
            let kind = TypeKind::from_metadata(ty, metadata);
            assert!(kind.as_struct().is_none());
            assert!(kind.as_enum().is_none());
            assert!(kind.as_sequence().is_none());
        }
    }

    #[test]
    fn kind_downcasts_return_the_matching_category() {
        let ty = TypeDescriptor::of::<u8>();

        let struct_kind = TypeKind::from_metadata(ty, TypeKindMetadata::Struct(&[]));
        assert_eq!(struct_kind.as_struct().unwrap().descriptor().type_id(), ty.type_id());
        assert!(struct_kind.as_enum().is_none());
        assert!(struct_kind.as_sequence().is_none());

        let enum_kind = TypeKind::from_metadata(ty, TypeKindMetadata::Enum(&[]));
        assert_eq!(enum_kind.as_enum().unwrap().descriptor().type_id(), ty.type_id());
        assert!(enum_kind.as_struct().is_none());
        assert!(enum_kind.as_sequence().is_none());

        let sequence_kind = TypeKind::from_metadata(ty, TypeKindMetadata::Sequence);
        assert_eq!(
            sequence_kind.as_sequence().unwrap().descriptor().type_id(),
            ty.type_id()
        );
        assert!(sequence_kind.as_struct().is_none());
        assert!(sequence_kind.as_enum().is_none());
    }
}
