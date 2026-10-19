use std::any::TypeId;

use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::error::{LookupError, LookupQuery, LookupSubject};

use super::field::{FieldDescriptor, FieldMetadata};
use super::type_kind::{TypeKind, TypeKindMetadata};
use super::variant::{VariantDescriptor, VariantMetadata};

/// Generated source metadata for fields, variants, and the concrete type kind.
pub struct TypeShapeRegistration {
    type_id: fn() -> TypeId,
    metadata: &'static TypeKindMetadata,
}

impl TypeShapeRegistration {
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
    pub(crate) fn kind(self) -> Result<TypeKind<'ctx>, crate::runtime::error::ContextError> {
        let metadata = self
            .context()
            .shape_for(self.type_id())?
            .copied()
            .unwrap_or(TypeKindMetadata::Opaque);
        Ok(TypeKind::from_metadata(self, metadata))
    }

    pub(crate) fn fields(self) -> Result<Fields<'ctx>, crate::runtime::error::ContextError> {
        Ok(match self.kind()? {
            TypeKind::Struct(view) => view.fields(),
            TypeKind::Tuple(view) => view.fields(),
            _ => Fields::new(self, NO_FIELDS, None),
        })
    }

    pub fn field(self, name: &str) -> Result<FieldDescriptor<'ctx>, LookupError> {
        self.fields().map_err(LookupError::Context)?.require(name)
    }

    pub(crate) fn variants(self) -> Result<Variants<'ctx>, crate::runtime::error::ContextError> {
        Ok(match self.kind()? {
            TypeKind::Enum(view) => view.variants(),
            _ => Variants::new(self, NO_VARIANTS),
        })
    }

    pub(crate) fn variant(self, name: &str) -> Result<VariantDescriptor<'ctx>, LookupError> {
        self.variants().map_err(LookupError::Context)?.require(name)
    }
}

/// A lightweight view over source ordered fields.
#[derive(Clone, Copy)]
pub(crate) struct Fields<'ctx> {
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

    pub(crate) fn len(&self) -> usize {
        self.metadata.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    pub(crate) fn get(&self, name: &str) -> Option<FieldDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|field| field.name == name)
            .map(|field| self.descriptor(field))
    }

    pub(crate) fn get_at(&self, index: usize) -> Option<FieldDescriptor<'ctx>> {
        self.metadata.get(index).map(|field| self.descriptor(field))
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = FieldDescriptor<'ctx>> + '_ {
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
pub(crate) struct Variants<'ctx> {
    declaring_type: TypeDescriptor<'ctx>,
    metadata: &'static [VariantMetadata],
}

impl<'ctx> Variants<'ctx> {
    pub(crate) const fn new(
        declaring_type: TypeDescriptor<'ctx>,
        metadata: &'static [VariantMetadata],
    ) -> Self {
        Self {
            declaring_type,
            metadata,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.metadata.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    pub(crate) fn get(&self, name: &str) -> Option<VariantDescriptor<'ctx>> {
        self.metadata
            .iter()
            .find(|variant| variant.name == name)
            .map(|variant| self.descriptor(variant))
    }

    pub(crate) fn get_at(&self, index: usize) -> Option<VariantDescriptor<'ctx>> {
        self.metadata
            .get(index)
            .map(|variant| self.descriptor(variant))
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = VariantDescriptor<'ctx>> + '_ {
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
