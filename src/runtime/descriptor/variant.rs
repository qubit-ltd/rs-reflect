#![allow(unsafe_code)]

use std::any::TypeId;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::{DynamicRef, Local, TypeMismatch, ValueForRef};

use super::collections::Fields;
use super::field::{FieldMetadata, Visibility};

/// The source shape of an enum variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VariantShape {
    Unit,
    Tuple,
    Struct,
}

/// Whether a discriminant came from an explicit source expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DiscriminantOrigin {
    Implicit,
    Explicit,
}

/// A numeric discriminant with its declared representation preserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NumericDiscriminant {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
}

/// The source origin and optional numeric value of a variant discriminant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct VariantDiscriminant {
    pub(crate) origin: DiscriminantOrigin,
    pub(crate) numeric: Option<NumericDiscriminant>,
}

/// Stable identity of one variant within its declaring enum.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct VariantIdentity {
    pub(crate) owner: TypeId,
    pub(crate) index: usize,
    pub(crate) name: &'static str,
}

/// Typed variant activity adapter.
pub(crate) trait VariantAdapter: Sync {
    fn owner(&self) -> TypeIdentity;
    /// # Safety
    /// `value` must point to the exact owner type returned by `owner()`.
    unsafe fn is_active(&self, value: *const ()) -> bool;
    fn is_active_dynamic(&self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch>;
}

pub(crate) struct TypedVariantAdapter<Owner: 'static> {
    is_active: for<'a> fn(&'a Owner) -> bool,
}

impl<Owner: 'static> TypedVariantAdapter<Owner> {
    pub(crate) const fn new(is_active: for<'a> fn(&'a Owner) -> bool) -> Self {
        Self { is_active }
    }
}

impl<Owner: Reflect + ValueForRef<Local> + 'static> VariantAdapter for TypedVariantAdapter<Owner> {
    fn owner(&self) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<Owner>(),
            rust_name: std::any::type_name::<Owner>(),
        }
    }

    unsafe fn is_active(&self, value: *const ()) -> bool {
        // SAFETY: the caller checks the owner TypeId before invoking the adapter.
        (self.is_active)(unsafe { &*value.cast::<Owner>() })
    }

    fn is_active_dynamic(&self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch> {
        let owner = value
            .downcast::<Owner>()
            .map_err(|failure| failure.into_parts().0)?;
        Ok((self.is_active)(owner))
    }
}

/// Static source facts for one enum variant.
pub(crate) struct VariantMetadata {
    pub(crate) index: usize,
    pub(crate) name: &'static str,
    pub(crate) shape: VariantShape,
    pub(crate) visibility: Visibility,
    pub(crate) discriminant: VariantDiscriminant,
    pub(crate) fields: &'static [FieldMetadata],
    pub(crate) adapter: &'static dyn VariantAdapter,
}

impl VariantMetadata {
    pub(crate) const fn new(
        index: usize,
        name: &'static str,
        shape: VariantShape,
        visibility: Visibility,
        discriminant: VariantDiscriminant,
        fields: &'static [FieldMetadata],
        adapter: &'static dyn VariantAdapter,
    ) -> Self {
        Self {
            index,
            name,
            shape,
            visibility,
            discriminant,
            fields,
            adapter,
        }
    }

    pub(crate) fn identity(&self) -> VariantIdentity {
        VariantIdentity {
            owner: self.adapter.owner().type_id,
            index: self.index,
            name: self.name,
        }
    }
}

/// A context-bound view of one enum variant.
#[derive(Clone, Copy)]
pub(crate) struct VariantDescriptor<'ctx> {
    pub(crate) metadata: &'static VariantMetadata,
    declaring_type: TypeDescriptor<'ctx>,
}

impl<'ctx> VariantDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static VariantMetadata,
        declaring_type: TypeDescriptor<'ctx>,
    ) -> Self {
        Self {
            metadata,
            declaring_type,
        }
    }

    pub(crate) fn context(self) -> &'ctx ReflectionContext {
        self.declaring_type.context()
    }

    pub(crate) fn declaring_type(self) -> TypeDescriptor<'ctx> {
        self.declaring_type
    }

    pub(crate) fn index(self) -> usize {
        self.metadata.index
    }

    pub(crate) fn name(self) -> &'static str {
        self.metadata.name
    }

    pub(crate) fn shape(self) -> VariantShape {
        self.metadata.shape
    }

    pub(crate) fn visibility(self) -> Visibility {
        self.metadata.visibility
    }

    pub(crate) fn fields(self) -> Fields<'ctx> {
        Fields::new(self.declaring_type, self.metadata.fields, Some(self))
    }

    pub(crate) fn field(
        self,
        name: &str,
    ) -> Result<super::field::FieldDescriptor<'ctx>, crate::runtime::error::LookupError> {
        self.fields().require(name)
    }

    pub(crate) fn discriminant(self) -> VariantDiscriminant {
        self.metadata.discriminant
    }

    pub(crate) fn identity(self) -> VariantIdentity {
        self.metadata.identity()
    }

    pub(crate) fn is_active<T: 'static>(self, value: &T) -> Result<bool, TypeMismatch> {
        let actual = TypeIdentity {
            type_id: TypeId::of::<T>(),
            rust_name: std::any::type_name::<T>(),
        };
        let expected = self.metadata.adapter.owner();
        if actual.type_id != expected.type_id {
            return Err(TypeMismatch::new(expected, actual));
        }
        // SAFETY: exact owner TypeId was checked before the adapter call.
        Ok(unsafe { self.metadata.adapter.is_active((value as *const T).cast()) })
    }

    pub(crate) fn is_active_dynamic(self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch> {
        self.metadata.adapter.is_active_dynamic(value)
    }
}
