// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use super::field::FieldDescriptor;
use super::field::FieldMetadata;
use super::type_kind::TypeKind;
use super::type_kind::TypeKindMetadata;
use super::variant::VariantDescriptor;
use super::variant::VariantMetadata;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;

/// Generated source metadata for fields, variants, and the concrete type kind.
pub struct TypeShapeRegistration {
    type_id: fn() -> TypeId,
    metadata: &'static TypeKindMetadata,
}

impl TypeShapeRegistration {
    /// Associates static shape metadata with a reflected concrete type.
    pub const fn new<T: super::Reflect>(metadata: &'static TypeKindMetadata) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            metadata,
        }
    }

    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn metadata(&self) -> &'static TypeKindMetadata {
        self.metadata
    }
}

inventory::collect!(TypeShapeRegistration);

const NO_FIELDS: &[FieldMetadata] = &[];
const NO_VARIANTS: &[VariantMetadata] = &[];

impl<'ctx> TypeDescriptor<'ctx> {
    /// Returns the source shape of this type, or `Opaque` when shape metadata
    /// is absent.
    pub fn kind(self) -> TypeKind<'ctx> {
        TypeKind::from_metadata(self, self.shape_metadata().copied().unwrap_or(TypeKindMetadata::Opaque))
    }

    /// Returns the fields when this type is a struct or tuple; otherwise
    /// returns an empty view.
    pub fn fields(self) -> Fields<'ctx> {
        match self.kind() {
            TypeKind::Struct(view) => view.fields(),
            TypeKind::Tuple(view) => view.fields(),
            _ => Fields::new(self, NO_FIELDS, None),
        }
    }

    /// Finds a field by name, returning a lookup error when it is absent.
    pub fn field(self, name: &str) -> Result<FieldDescriptor<'ctx>, LookupError> {
        self.fields().require(name)
    }

    /// Returns the variants when this type is an enum; otherwise returns an
    /// empty view.
    pub fn variants(self) -> Variants<'ctx> {
        match self.kind() {
            TypeKind::Enum(view) => view.variants(),
            _ => Variants::new(self, NO_VARIANTS),
        }
    }

    /// Finds an enum variant by name, returning a lookup error when it is
    /// absent.
    pub fn variant(self, name: &str) -> Result<VariantDescriptor<'ctx>, LookupError> {
        self.variants().require(name)
    }

    /// Returns the struct view when this type has struct shape.
    pub fn as_struct(self) -> Option<super::type_kind::StructType<'ctx>> {
        self.kind().as_struct()
    }

    /// Returns the enum view when this type has enum shape.
    pub fn as_enum(self) -> Option<super::type_kind::EnumType<'ctx>> {
        self.kind().as_enum()
    }

    /// Returns the sequence view when this type has sequence shape.
    pub fn as_sequence(self) -> Option<super::type_kind::SequenceType<'ctx>> {
        self.kind().as_sequence()
    }
}

/// A lightweight view over source ordered fields.
#[derive(Clone, Copy)]
pub struct Fields<'ctx> {
    declaring_type: TypeDescriptor<'ctx>,
    metadata: &'static [FieldMetadata],
    declaring_variant: Option<VariantDescriptor<'ctx>>,
}

impl<'ctx> Fields<'ctx> {
    pub(crate) const fn new(
        declaring_type: TypeDescriptor<'ctx>,
        metadata: &'static [FieldMetadata],
        declaring_variant: Option<VariantDescriptor<'ctx>>,
    ) -> Self {
        Self {
            declaring_type,
            metadata,
            declaring_variant,
        }
    }

    /// Returns the number of reflected fields.
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// Returns whether this view contains no fields.
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Finds a field by its source name.
    pub fn get(&self, name: &str) -> Option<FieldDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|field| field.name == name)
            .map(|field| self.descriptor(field))
    }

    /// Returns the field at its ordinal position in this view.
    pub fn get_at(&self, index: usize) -> Option<FieldDescriptor<'ctx>> {
        self.metadata.get(index).map(|field| self.descriptor(field))
    }

    /// Iterates over fields in source order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = FieldDescriptor<'ctx>> + '_ {
        self.metadata.iter().map(|field| self.descriptor(field))
    }

    pub(crate) fn require(&self, name: &str) -> Result<FieldDescriptor<'ctx>, LookupError> {
        self.get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::Field,
            query: LookupQuery::Name(name.into()),
        })
    }

    fn descriptor(&self, metadata: &'static FieldMetadata) -> FieldDescriptor<'ctx> {
        FieldDescriptor::new(metadata, self.declaring_type, self.declaring_variant)
    }
}

/// A lightweight view over source ordered enum variants.
#[derive(Clone, Copy)]
pub struct Variants<'ctx> {
    declaring_type: TypeDescriptor<'ctx>,
    metadata: &'static [VariantMetadata],
}

impl<'ctx> Variants<'ctx> {
    pub(crate) const fn new(declaring_type: TypeDescriptor<'ctx>, metadata: &'static [VariantMetadata]) -> Self {
        Self {
            declaring_type,
            metadata,
        }
    }

    /// Returns the number of reflected variants.
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// Returns whether this view contains no variants.
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Finds a variant by its source name.
    pub fn get(&self, name: &str) -> Option<VariantDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|variant| variant.name == name)
            .map(|variant| self.descriptor(variant))
    }

    /// Finds a variant by its original Rust discriminant index.
    pub fn get_at(&self, index: usize) -> Option<VariantDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|variant| variant.index == index)
            .map(|variant| self.descriptor(variant))
    }

    /// Iterates over reflected variants in source order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = VariantDescriptor<'ctx>> + '_ {
        self.metadata.iter().map(|variant| self.descriptor(variant))
    }

    pub(crate) fn require(&self, name: &str) -> Result<VariantDescriptor<'ctx>, LookupError> {
        self.get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::Variant,
            query: LookupQuery::Name(name.into()),
        })
    }

    fn descriptor(&self, metadata: &'static VariantMetadata) -> VariantDescriptor<'ctx> {
        VariantDescriptor::new(metadata, self.declaring_type)
    }
}

#[cfg(all(test, feature = "derive"))]
mod tests {
    use crate::runtime::descriptor::TypeDescriptor;
    use crate::runtime::error::LookupError;
    use crate::runtime::error::LookupQuery;
    use crate::runtime::error::LookupSubject;

    #[crate::Reflect]
    struct Record {
        first: u8,
        #[reflect(skip)]
        omitted: bool,
        last: String,
    }

    #[crate::Reflect]
    enum Event {
        Start,
        #[expect(dead_code, reason = "the test inspects skipped source metadata")]
        #[reflect(skip)]
        Omitted,
        Finish(u8),
    }

    #[crate::Reflect]
    struct Opaque;

    #[test]
    fn field_collection_preserves_order_and_source_indices() {
        let descriptor = TypeDescriptor::of::<Record>();
        let fields = descriptor.fields();

        let record = Record {
            first: 1,
            omitted: true,
            last: String::from("done"),
        };
        assert_eq!(record.first, 1);
        assert!(record.omitted);
        assert_eq!(record.last, "done");
        assert_eq!(fields.len(), 2);
        assert!(!fields.is_empty());
        assert_eq!(fields.get("first").expect("first field exists").index(), 0);
        assert_eq!(fields.get("last").expect("last field exists").index(), 2);
        assert!(fields.get("omitted").is_none());
        assert_eq!(fields.get_at(0).expect("first position exists").name(), "first");
        assert_eq!(
            fields.get_at(1).expect("second reflected position exists").name(),
            "last"
        );
        assert!(fields.get_at(2).is_none());

        let mut iter = fields.iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next().expect("first field").name(), "first");
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next().expect("last field").name(), "last");
        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn variant_collection_uses_source_indices_and_source_order() {
        let variants = TypeDescriptor::of::<Event>().variants();
        assert!(matches!(Event::Start, Event::Start));
        assert!(matches!(Event::Finish(5), Event::Finish(5)));

        assert_eq!(variants.len(), 2);
        assert!(!variants.is_empty());
        assert_eq!(variants.get("Start").expect("start exists").index(), 0);
        assert_eq!(variants.get("Finish").expect("finish exists").index(), 2);
        assert!(variants.get("Omitted").is_none());
        assert_eq!(variants.get_at(0).expect("first source index exists").name(), "Start");
        assert!(variants.get_at(1).is_none());
        assert_eq!(variants.get_at(2).expect("last source index exists").name(), "Finish");
        assert!(variants.get_at(3).is_none());

        let mut iter = variants.iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next().expect("start variant").name(), "Start");
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next().expect("finish variant").name(), "Finish");
        assert_eq!(iter.len(), 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn absent_field_and_variant_return_structured_lookup_errors() {
        let descriptor = TypeDescriptor::of::<Record>();
        let field_error = match descriptor.field("missing") {
            Ok(_) => panic!("unknown field should be rejected"),
            Err(error) => error,
        };
        assert_eq!(
            field_error,
            LookupError::Missing {
                subject: LookupSubject::Field,
                query: LookupQuery::Name("missing".into()),
            }
        );

        let descriptor = TypeDescriptor::of::<Event>();
        let variant_error = match descriptor.variant("missing") {
            Ok(_) => panic!("unknown variant should be rejected"),
            Err(error) => error,
        };
        assert_eq!(
            variant_error,
            LookupError::Missing {
                subject: LookupSubject::Variant,
                query: LookupQuery::Name("missing".into()),
            }
        );
    }

    #[test]
    fn non_struct_and_non_enum_collections_are_empty() {
        let descriptor = TypeDescriptor::of::<Opaque>();
        let fields = descriptor.fields();
        let variants = descriptor.variants();

        assert!(fields.is_empty());
        assert_eq!(fields.len(), 0);
        assert!(fields.iter().next().is_none());
        assert!(fields.get("anything").is_none());
        assert!(fields.get_at(0).is_none());
        assert!(variants.is_empty());
        assert_eq!(variants.len(), 0);
        assert!(variants.iter().next().is_none());
        assert!(variants.get("anything").is_none());
        assert!(variants.get_at(0).is_none());
    }
}
