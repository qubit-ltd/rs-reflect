use crate::runtime::descriptor::TypeDescriptor;

use super::collections::{Fields, Variants};
use super::field::FieldMetadata;
use super::variant::VariantMetadata;

/// Static kind facts supplied by the type registration.
#[derive(Clone, Copy)]
pub enum TypeKindMetadata {
    Primitive,
    Text,
    Struct(&'static [FieldMetadata]),
    Enum(&'static [VariantMetadata]),
    Tuple(&'static [FieldMetadata]),
    Array,
    Optional,
    Sequence,
    Set,
    Map,
    SmartPointer,
    Reference,
    Slice,
    RawPointer,
    Function,
    TraitObject,
    Opaque,
}

/// A type category paired with its context-bound descriptor.
pub(crate) enum TypeKind<'ctx> {
    Primitive(PrimitiveType<'ctx>),
    Text(TextType<'ctx>),
    Struct(StructType<'ctx>),
    Enum(EnumType<'ctx>),
    Tuple(TupleType<'ctx>),
    Array(ArrayType<'ctx>),
    Optional(OptionalType<'ctx>),
    Sequence(SequenceType<'ctx>),
    Set(SetType<'ctx>),
    Map(MapType<'ctx>),
    SmartPointer(SmartPointerType<'ctx>),
    Reference(ReferenceType<'ctx>),
    Slice(SliceType<'ctx>),
    RawPointer(RawPointerType<'ctx>),
    Function(FunctionType<'ctx>),
    TraitObject(TraitObjectType<'ctx>),
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

    pub(crate) fn as_struct(&self) -> Option<StructType<'ctx>> {
        match self {
            Self::Struct(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_enum(&self) -> Option<EnumType<'ctx>> {
        match self {
            Self::Enum(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_sequence(&self) -> Option<SequenceType<'ctx>> {
        match self {
            Self::Sequence(value) => Some(*value),
            _ => None,
        }
    }
}

/// A reflected struct with its source ordered fields.
#[derive(Clone, Copy)]
pub(crate) struct StructType<'ctx> {
    ty: TypeDescriptor<'ctx>,
    fields: Fields<'ctx>,
}

impl<'ctx> StructType<'ctx> {
    pub(crate) fn descriptor(self) -> TypeDescriptor<'ctx> {
        self.ty
    }
    pub(crate) fn fields(self) -> Fields<'ctx> {
        self.fields
    }
}

/// A reflected enum with its source ordered variants.
#[derive(Clone, Copy)]
pub(crate) struct EnumType<'ctx> {
    ty: TypeDescriptor<'ctx>,
    variants: Variants<'ctx>,
}

impl<'ctx> EnumType<'ctx> {
    pub(crate) fn descriptor(self) -> TypeDescriptor<'ctx> {
        self.ty
    }
    pub(crate) fn variants(self) -> Variants<'ctx> {
        self.variants
    }
}

/// A tuple struct or tuple type with source ordered fields.
#[derive(Clone, Copy)]
pub(crate) struct TupleType<'ctx> {
    ty: TypeDescriptor<'ctx>,
    fields: Fields<'ctx>,
}

impl<'ctx> TupleType<'ctx> {
    pub(crate) fn descriptor(self) -> TypeDescriptor<'ctx> {
        self.ty
    }
    pub(crate) fn fields(self) -> Fields<'ctx> {
        self.fields
    }
}

macro_rules! descriptor_view {
    ($name:ident) => {
        #[derive(Clone, Copy)]
        pub(crate) struct $name<'ctx>(TypeDescriptor<'ctx>);
        impl<'ctx> $name<'ctx> {
            pub(crate) fn descriptor(self) -> TypeDescriptor<'ctx> {
                self.0
            }
        }
    };
}

descriptor_view!(PrimitiveType);
descriptor_view!(TextType);
descriptor_view!(ArrayType);
descriptor_view!(OptionalType);
descriptor_view!(SequenceType);
descriptor_view!(SetType);
descriptor_view!(MapType);
descriptor_view!(SmartPointerType);
descriptor_view!(ReferenceType);
descriptor_view!(SliceType);
descriptor_view!(RawPointerType);
descriptor_view!(FunctionType);
descriptor_view!(TraitObjectType);
descriptor_view!(OpaqueType);
