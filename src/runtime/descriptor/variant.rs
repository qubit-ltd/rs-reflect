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
    I8(
        /// Signed 8-bit discriminant value.
        i8,
    ),
    /// A signed 16-bit discriminant.
    I16(
        /// Signed 16-bit discriminant value.
        i16,
    ),
    /// A signed 32-bit discriminant.
    I32(
        /// Signed 32-bit discriminant value.
        i32,
    ),
    /// A signed 64-bit discriminant.
    I64(
        /// Signed 64-bit discriminant value.
        i64,
    ),
    /// A signed 128-bit discriminant.
    I128(
        /// Signed 128-bit discriminant value.
        i128,
    ),
    /// A pointer-width signed discriminant.
    Isize(
        /// Pointer-width signed discriminant value.
        isize,
    ),
    /// An unsigned 8-bit discriminant.
    U8(
        /// Unsigned 8-bit discriminant value.
        u8,
    ),
    /// An unsigned 16-bit discriminant.
    U16(
        /// Unsigned 16-bit discriminant value.
        u16,
    ),
    /// An unsigned 32-bit discriminant.
    U32(
        /// Unsigned 32-bit discriminant value.
        u32,
    ),
    /// An unsigned 64-bit discriminant.
    U64(
        /// Unsigned 64-bit discriminant value.
        u64,
    ),
    /// An unsigned 128-bit discriminant.
    U128(
        /// Unsigned 128-bit discriminant value.
        u128,
    ),
    /// A pointer-width unsigned discriminant.
    Usize(
        /// Pointer-width unsigned discriminant value.
        usize,
    ),
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
    ///
    /// # Parameters
    ///
    /// - `origin`: Whether Rust inferred the value or source specified it.
    /// - `numeric`: Represented numeric value, when available.
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
    ///
    /// # Parameters
    ///
    /// - `self`: Adapter whose owner type is reported.
    ///
    /// # Returns
    ///
    /// Returns the runtime identity of the enum type.
    fn owner(&self) -> TypeIdentity;
    /// # Safety
    /// `value` must point to the exact owner type returned by `owner()`.
    ///
    /// # Parameters
    ///
    /// - `self`: Typed activity predicate adapter.
    /// - `value`: Non-null pointer to a value of the exact owner type.
    ///
    /// # Returns
    ///
    /// Returns whether the value has this variant active.
    unsafe fn is_active(&self, value: *const ()) -> bool;
    /// Tests variant activity for a dynamically typed value.
    ///
    /// # Parameters
    ///
    /// - `self`: Adapter used to test the variant.
    /// - `value`: Dynamically typed value to downcast and inspect.
    ///
    /// # Returns
    ///
    /// Returns activity state, or a type mismatch if the owner differs.
    fn is_active_dynamic(&self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch>;
}

/// A variant activity adapter backed by a typed predicate.
///
/// The predicate receives a shared reference, so checking the active variant
/// does not mutate or consume the enum value.
///
/// # Type Parameters
///
/// - `Owner`: Enum type whose variants are checked; it must be `'static`.
pub struct TypedVariantAdapter<Owner: 'static> {
    /// Match predicate generated for the owner enum and this variant.
    is_active: for<'a> fn(&'a Owner) -> bool,
}

impl<Owner: 'static> TypedVariantAdapter<Owner> {
    /// Creates a typed variant adapter from a safe match predicate.
    ///
    /// # Type Parameters
    ///
    /// - `Owner`: Enum type whose variants the predicate distinguishes.
    ///
    /// # Parameters
    ///
    /// - `is_active`: Predicate that returns true for this variant.
    pub const fn new(is_active: for<'a> fn(&'a Owner) -> bool) -> Self {
        Self { is_active }
    }
}

impl<Owner: Reflect + ValueForRef<Local> + 'static> VariantAdapter for TypedVariantAdapter<Owner> {
    /// Returns the identity of `Owner`, which controls pointer validation.
    fn owner(&self) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<Owner>(),
            rust_name: std::any::type_name::<Owner>(),
        }
    }

    /// Calls the predicate after the caller has verified the owner `TypeId`.
    ///
    /// # Safety
    ///
    /// `value` must point to a valid `Owner` value for the duration of the
    /// call.
    ///
    /// # Parameters
    ///
    /// - `self`: Adapter containing the typed predicate.
    /// - `value`: Erased pointer to the matching owner value.
    ///
    /// # Returns
    ///
    /// Returns true when the owner value matches this variant.
    unsafe fn is_active(&self, value: *const ()) -> bool {
        // SAFETY: the caller checks the owner TypeId before invoking the adapter.
        (self.is_active)(unsafe { &*value.cast::<Owner>() })
    }

    /// Downcasts a dynamic value to `Owner` before calling the predicate.
    ///
    /// # Parameters
    ///
    /// - `self`: Adapter containing the typed predicate.
    /// - `value`: Dynamic value to check.
    ///
    /// # Returns
    ///
    /// Returns activity state, or a type mismatch when the value is not
    /// `Owner`.
    fn is_active_dynamic(&self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch> {
        let owner = value.downcast::<Owner>().map_err(|failure| failure.into_parts().0)?;
        Ok((self.is_active)(owner))
    }
}

/// Static source facts for one enum variant.
pub struct VariantMetadata {
    /// Zero-based position of the variant in the enum declaration.
    pub(crate) index: usize,
    /// Source identifier of the variant.
    pub(crate) name: &'static str,
    /// Unit, tuple, or named-field source shape.
    pub(crate) shape: VariantShape,
    /// Visibility retained from source syntax.
    pub(crate) visibility: Visibility,
    /// Explicitness and available numeric value of the discriminant.
    pub(crate) discriminant: VariantDiscriminant,
    /// Fields declared by the variant in source order.
    pub(crate) fields: &'static [FieldMetadata],
    /// Typed predicate that checks whether the variant is active.
    pub(crate) adapter: &'static dyn VariantAdapter,
}

impl VariantMetadata {
    /// Creates static metadata for one enum variant.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based declaration position.
    /// - `name`: Source variant identifier.
    /// - `shape`: Unit, tuple, or named-field shape.
    /// - `visibility`: Visibility retained from source.
    /// - `discriminant`: Discriminant origin and represented numeric value.
    /// - `fields`: Variant fields in source order.
    /// - `adapter`: Typed predicate used for activity checks.
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

    /// Builds the stable owner-and-index identity for this variant.
    ///
    /// # Parameters
    ///
    /// - `self`: Static metadata whose identity is constructed.
    ///
    /// # Returns
    ///
    /// Returns the declaring enum's type ID, source index, and variant name.
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
    /// Static source metadata for this variant.
    pub(crate) metadata: &'static VariantMetadata,
    /// Descriptor for the enum that declares the variant.
    declaring_type: TypeDescriptor<'ctx>,
}

impl<'ctx> VariantDescriptor<'ctx> {
    /// Binds static variant metadata to its declaring enum descriptor.
    ///
    /// # Parameters
    ///
    /// - `metadata`: Static source facts for this enum variant.
    /// - `declaring_type`: Context-bound descriptor for the owner enum.
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
    ///
    /// # Parameters
    ///
    /// - `name`: Source field name to locate.
    ///
    /// # Errors
    ///
    /// Returns `LookupError` when the variant has no field with that name.
    pub fn field(self, name: &str) -> Result<super::field::FieldDescriptor<'ctx>, crate::runtime::error::LookupError> {
        self.fields().require(name)
    }

    /// Returns the explicitness and available numeric value of the
    /// discriminant.
    pub fn discriminant(self) -> VariantDiscriminant {
        self.metadata.discriminant
    }

    /// Reports whether this variant is active in a value of its declaring type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete type of the value; it must be the declaring enum type.
    ///
    /// # Parameters
    ///
    /// - `value`: Shared reference to the value whose active variant is
    ///   checked.
    ///
    /// # Errors
    ///
    /// Returns `TypeMismatch` when `T` is not the declaring enum type.
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
    ///
    /// # Parameters
    ///
    /// - `value`: Dynamically typed value to check.
    ///
    /// # Errors
    ///
    /// Returns `TypeMismatch` when the dynamic value has a different owner
    /// type.
    pub fn is_active_dynamic(self, value: DynamicRef<'_>) -> Result<bool, TypeMismatch> {
        self.metadata.adapter.is_active_dynamic(value)
    }
}
