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
    /// Type ID getter used to match the registration to reflected values.
    type_id: fn() -> TypeId,
    /// Static generated metadata describing the registered type's shape.
    metadata: &'static TypeKindMetadata,
}

impl TypeShapeRegistration {
    /// Associates static shape metadata with a reflected concrete type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete type described by `metadata`.
    ///
    /// # Parameters
    ///
    /// * `metadata` — generated kind and member metadata for `T`.
    pub const fn new<T: super::Reflect>(metadata: &'static TypeKindMetadata) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            metadata,
        }
    }

    /// Returns the concrete type ID for which this shape was registered.
    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the static metadata describing this registered type's shape.
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
    ///
    /// # Parameters
    ///
    /// * `name` — source field name to look up.
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
    ///
    /// # Parameters
    ///
    /// * `name` — source variant name to look up.
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
    /// Type whose fields are exposed by this view.
    declaring_type: TypeDescriptor<'ctx>,
    /// Field metadata in declaration order.
    metadata: &'static [FieldMetadata],
    /// Variant that owns these fields, when they belong to an enum variant.
    declaring_variant: Option<VariantDescriptor<'ctx>>,
}

impl<'ctx> Fields<'ctx> {
    /// Creates a field view with its owning type and optional variant context.
    ///
    /// # Parameters
    ///
    /// * `declaring_type` — type descriptor that owns the fields.
    /// * `metadata` — source ordered field metadata.
    /// * `declaring_variant` — enum variant owning the fields, if applicable.
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
    ///
    /// # Parameters
    ///
    /// * `name` — exact source field name.
    pub fn get(&self, name: &str) -> Option<FieldDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|field| field.name == name)
            .map(|field| self.descriptor(field))
    }

    /// Returns the field at its ordinal position in this view.
    ///
    /// # Parameters
    ///
    /// * `index` — zero-based position in source declaration order.
    pub fn get_at(&self, index: usize) -> Option<FieldDescriptor<'ctx>> {
        self.metadata.get(index).map(|field| self.descriptor(field))
    }

    /// Iterates over fields in source order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = FieldDescriptor<'ctx>> + '_ {
        self.metadata.iter().map(|field| self.descriptor(field))
    }

    /// Requires a field by name and preserves the failed query in lookup
    /// errors.
    ///
    /// # Parameters
    ///
    /// * `name` — exact source field name to require.
    pub(crate) fn require(&self, name: &str) -> Result<FieldDescriptor<'ctx>, LookupError> {
        self.get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::Field,
            query: LookupQuery::Name(name.into()),
        })
    }

    /// Binds one field metadata entry to this view's type and variant.
    ///
    /// # Parameters
    ///
    /// * `metadata` — static field metadata to expose.
    fn descriptor(&self, metadata: &'static FieldMetadata) -> FieldDescriptor<'ctx> {
        FieldDescriptor::new(metadata, self.declaring_type, self.declaring_variant)
    }
}

/// A lightweight view over source ordered enum variants.
#[derive(Clone, Copy)]
pub struct Variants<'ctx> {
    /// Type descriptor whose enum variants are exposed.
    declaring_type: TypeDescriptor<'ctx>,
    /// Variant metadata in source declaration order.
    metadata: &'static [VariantMetadata],
}

impl<'ctx> Variants<'ctx> {
    /// Creates a variant view for one reflected enum type.
    ///
    /// # Parameters
    ///
    /// * `declaring_type` — descriptor for the enum that owns the variants.
    /// * `metadata` — generated variant metadata in source order.
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
    ///
    /// # Parameters
    ///
    /// * `name` — exact source variant name.
    pub fn get(&self, name: &str) -> Option<VariantDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|variant| variant.name == name)
            .map(|variant| self.descriptor(variant))
    }

    /// Finds a variant by its original Rust discriminant index.
    ///
    /// # Parameters
    ///
    /// * `index` — discriminant index recorded from source declaration order.
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

    /// Requires a variant by name and preserves the failed query in lookup
    /// errors.
    ///
    /// # Parameters
    ///
    /// * `name` — exact source variant name to require.
    pub(crate) fn require(&self, name: &str) -> Result<VariantDescriptor<'ctx>, LookupError> {
        self.get(name).ok_or_else(|| LookupError::Missing {
            subject: LookupSubject::Variant,
            query: LookupQuery::Name(name.into()),
        })
    }

    /// Binds one variant metadata entry to the type represented by this view.
    ///
    /// # Parameters
    ///
    /// * `metadata` — static variant metadata to expose.
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
    fn test_field_collection_preserves_order_and_source_indices() {
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
    fn test_variant_collection_uses_source_indices_and_source_order() {
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
    fn test_absent_field_and_variant_return_structured_lookup_errors() {
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
    fn test_non_struct_and_non_enum_collections_are_empty() {
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
