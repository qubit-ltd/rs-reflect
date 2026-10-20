// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]

use std::any::TypeId;

use super::collections::Fields;
use super::field::FieldMetadata;
use super::field::Visibility;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::Local;
use crate::runtime::value::TypeMismatch;
use crate::runtime::value::ValueForRef;

/// The source shape of an enum variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariantShape {
    /// The variant carries no fields.
    Unit,
    /// The variant carries positional fields.
    Tuple,
    /// The variant carries named fields.
    Struct,
}

/// Whether a discriminant came from an explicit source expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscriminantOrigin {
    /// Rust inferred this value from preceding discriminants.
    Implicit,
    /// The source declaration supplied an explicit discriminant expression.
    Explicit,
}

/// A numeric discriminant with its declared representation preserved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericDiscriminant {
    /// A signed 8-bit discriminant.
    I8(i8),
    /// A signed 16-bit discriminant.
    I16(i16),
    /// A signed 32-bit discriminant.
    I32(i32),
    /// A signed 64-bit discriminant.
    I64(i64),
    /// A signed 128-bit discriminant.
    I128(i128),
    /// A pointer-width signed discriminant.
    Isize(isize),
    /// An unsigned 8-bit discriminant.
    U8(u8),
    /// An unsigned 16-bit discriminant.
    U16(u16),
    /// An unsigned 32-bit discriminant.
    U32(u32),
    /// An unsigned 64-bit discriminant.
    U64(u64),
    /// An unsigned 128-bit discriminant.
    U128(u128),
    /// A pointer-width unsigned discriminant.
    Usize(usize),
}

/// The source origin and optional numeric value of a variant discriminant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VariantDiscriminant {
    /// Whether the source assigned the discriminant explicitly.
    pub origin: DiscriminantOrigin,
    /// The numeric value when it can be represented directly.
    pub numeric: Option<NumericDiscriminant>,
}

impl VariantDiscriminant {
    /// Creates source origin and optional explicit numeric discriminant
    /// metadata.
    pub const fn new(origin: DiscriminantOrigin, numeric: Option<NumericDiscriminant>) -> Self {
        Self { origin, numeric }
    }
}

/// Stable identity of one variant within its declaring enum.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct VariantIdentity {
    /// `TypeId` of the enum containing this variant.
    pub owner: TypeId,
    /// The variant's original zero-based source index.
    pub index: usize,
    /// The source name of the variant.
    pub name: &'static str,
}

/// Typed variant activity adapter.
pub trait VariantAdapter: Sync {
    /// Returns the concrete enum type accepted by this adapter.
    fn owner(&self) -> TypeIdentity;
    /// # Safety
    /// `value` must point to the exact owner type returned by `owner()`.
    unsafe fn is_active(&self, value: *const ()) -> bool;
    /// Tests variant activity for a dynamically typed value.
    fn is_active_dynamic(&self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch>;
}

/// A variant activity adapter backed by a typed predicate.
pub struct TypedVariantAdapter<Owner: 'static> {
    is_active: for<'a> fn(&'a Owner) -> bool,
}

impl<Owner: 'static> TypedVariantAdapter<Owner> {
    /// Creates a typed variant adapter from a safe match predicate.
    pub const fn new(is_active: for<'a> fn(&'a Owner) -> bool) -> Self {
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
        let owner = value.downcast::<Owner>().map_err(|failure| failure.into_parts().0)?;
        Ok((self.is_active)(owner))
    }
}

/// Static source facts for one enum variant.
pub struct VariantMetadata {
    pub(crate) index: usize,
    pub(crate) name: &'static str,
    pub(crate) shape: VariantShape,
    pub(crate) visibility: Visibility,
    pub(crate) discriminant: VariantDiscriminant,
    pub(crate) fields: &'static [FieldMetadata],
    pub(crate) adapter: &'static dyn VariantAdapter,
}

impl VariantMetadata {
    /// Creates static metadata for one enum variant.
    pub const fn new(
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
pub struct VariantDescriptor<'ctx> {
    pub(crate) metadata: &'static VariantMetadata,
    declaring_type: TypeDescriptor<'ctx>,
}

impl<'ctx> VariantDescriptor<'ctx> {
    pub(crate) const fn new(metadata: &'static VariantMetadata, declaring_type: TypeDescriptor<'ctx>) -> Self {
        Self {
            metadata,
            declaring_type,
        }
    }

    /// Returns the context that owns this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.declaring_type.context()
    }

    /// Returns the enum declaring this variant.
    pub fn declaring_type(self) -> TypeDescriptor<'ctx> {
        self.declaring_type
    }

    /// Returns the variant's original source index.
    pub fn index(self) -> usize {
        self.metadata.index
    }

    /// Returns the variant's source name.
    pub fn name(self) -> &'static str {
        self.metadata.name
    }

    /// Returns whether the variant is unit-like, tuple-like, or struct-like.
    pub fn shape(self) -> VariantShape {
        self.metadata.shape
    }

    /// Returns source visibility retained for this variant.
    pub fn visibility(self) -> Visibility {
        self.metadata.visibility
    }

    /// Returns the variant's fields in source order.
    pub fn fields(self) -> Fields<'ctx> {
        Fields::new(self.declaring_type, self.metadata.fields, Some(self))
    }

    /// Finds a field by name, returning a lookup error when it is absent.
    pub fn field(self, name: &str) -> Result<super::field::FieldDescriptor<'ctx>, crate::runtime::error::LookupError> {
        self.fields().require(name)
    }

    /// Returns the explicitness and available numeric value of the
    /// discriminant.
    pub fn discriminant(self) -> VariantDiscriminant {
        self.metadata.discriminant
    }

    /// Reports whether this variant is active in a value of its declaring type.
    pub fn is_active<T: 'static>(self, value: &T) -> Result<bool, TypeMismatch> {
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

    /// Reports whether this variant is active in a dynamically typed value.
    pub fn is_active_dynamic(self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch> {
        self.metadata.adapter.is_active_dynamic(value)
    }
}
