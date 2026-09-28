// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Immutable structural facts about reflected enum variants.

use std::fmt;

use crate::access::VariantActiveAdapter;
use crate::access::field_adapter::dynamic_ref_type_id;
use crate::construct::ConstructionError;
use crate::construct::ConstructionRecovery;
use crate::construct::NamedConstructionInput;
use crate::construct::TupleConstructionInput;
use crate::construct::VariantConstructionDescriptor;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::error::TypeMismatch;
use crate::value::ReflectedOwned;
use crate::value::ReflectedRef;

/// The declared shape of an enum variant.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::VariantKind;
/// assert_eq!(VariantKind::Unit, VariantKind::Unit);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VariantKind {
    /// A fieldless variant.
    Unit,
    /// A positional variant.
    Tuple,
    /// A variant with named fields.
    Struct,
}

/// Whether a variant's discriminant was written explicitly in Rust source.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::DiscriminantOrigin;
/// assert_eq!(DiscriminantOrigin::Implicit, DiscriminantOrigin::Implicit);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiscriminantOrigin {
    /// Rust assigned the value from declaration order and preceding values.
    Implicit,
    /// The variant declaration contains an explicit discriminant expression.
    Explicit,
}

/// The exact integer representation of an enum discriminant value.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::NumericDiscriminant;
/// assert_eq!(NumericDiscriminant::U8(3), NumericDiscriminant::U8(3));
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NumericDiscriminant {
    /// An `i8` discriminant.
    I8(i8),
    /// An `i16` discriminant.
    I16(i16),
    /// An `i32` discriminant.
    I32(i32),
    /// An `i64` discriminant.
    I64(i64),
    /// An `i128` discriminant.
    I128(i128),
    /// An `isize` discriminant.
    Isize(isize),
    /// A `u8` discriminant.
    U8(u8),
    /// A `u16` discriminant.
    U16(u16),
    /// A `u32` discriminant.
    U32(u32),
    /// A `u64` discriminant.
    U64(u64),
    /// A `u128` discriminant.
    U128(u128),
    /// A `usize` discriminant.
    Usize(usize),
}

/// The immutable structural description of one reflected enum variant.
#[cfg_attr(
    feature = "derive",
    doc = r#"
# Examples

```
#![allow(proc_macro_derive_resolution_fallback)]
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
enum State {
    Ready,
}

let variant = TypeDescriptor::of::<State>()
    .variant("Ready")
    .expect("the derived variant exists");
assert_eq!(variant.rust_name(), "Ready");
```
"#
)]
pub struct VariantDescriptor {
    /// Resolver for the enum root that contains this variant.
    declaring_type: TypeDescriptorResolver,
    /// Zero-based position in the source enum.
    index: usize,
    /// Source Rust identifier for the variant.
    rust_name: &'static str,
    /// Stable reflection lookup name for the variant.
    query_name: &'static str,
    /// Unit, tuple, or struct-like shape.
    kind: VariantKind,
    /// Variant fields retained in source order.
    fields: &'static [FieldDescriptor],
    /// Generated adapter that checks the active enum variant.
    active_test: VariantActiveAdapter,
    /// Whether source specified this variant's discriminant.
    discriminant_origin: DiscriminantOrigin,
    /// Exact numeric discriminant when supported by the enum representation.
    numeric_discriminant: Option<NumericDiscriminant>,
    /// Optional generated dynamic-construction entry point.
    construction: Option<VariantConstructionDescriptor>,
}

impl VariantDescriptor {
    /// Creates a frozen enum-variant descriptor for generated descriptor data.
    ///
    /// The resolver must return the enum root containing the variant. Fields
    /// must be ordered by source index. This constructor performs no
    /// allocation and never calls the resolver.
    #[doc(hidden)]
    pub(crate) const fn new(
        declaring_type: TypeDescriptorResolver,
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        kind: VariantKind,
        fields: &'static [FieldDescriptor],
        active_test: VariantActiveAdapter,
    ) -> Self {
        Self {
            declaring_type,
            index,
            rust_name,
            query_name,
            kind,
            fields,
            active_test,
            discriminant_origin: DiscriminantOrigin::Implicit,
            numeric_discriminant: None,
            construction: None,
        }
    }

    /// Records source discriminant facts supplied by generated enum metadata.
    ///
    /// # Parameters
    ///
    /// - `origin`: Whether the discriminant was explicit in source.
    /// - `numeric`: Exact numeric value when supported by the enum representation.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with its source discriminant facts recorded.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_discriminant(mut self, origin: DiscriminantOrigin, numeric: Option<NumericDiscriminant>) -> Self {
        self.discriminant_origin = origin;
        self.numeric_discriminant = numeric;
        self
    }

    /// Attaches a generated dynamic-construction entry point to this variant.
    ///
    /// # Parameters
    ///
    /// - `construction`: Generated constructor and construction policy.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with dynamic construction enabled.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub fn with_construction(mut self, construction: VariantConstructionDescriptor) -> Self {
        self.construction = Some(construction);
        self
    }

    /// Returns the generated from-zero construction entry point, if enabled
    /// for this variant.
    #[must_use]
    #[inline]
    pub const fn construction(&self) -> Option<&VariantConstructionDescriptor> {
        self.construction.as_ref()
    }

    /// Constructs a named variant through its generated local adapter.
    ///
    /// # Parameters
    ///
    /// - `input`: Named field values supplied for the variant.
    ///
    /// # Returns
    ///
    /// Returns the constructed enum value with local dynamic ownership.
    ///
    /// # Errors
    ///
    /// Returns the construction error and recoverable input values if
    /// validation fails or this variant has no constructor.
    pub fn construct_struct(
        &self,
        input: NamedConstructionInput<crate::value::Local>,
    ) -> Result<ReflectedOwned, ConstructionRecovery<crate::value::Local>> {
        match self.construction() {
            Some(construction) => construction.local_constructor().construct_named(input),
            None => Err(input.into_recovery(ConstructionError::TargetUnavailable)),
        }
    }

    /// Constructs a tuple variant through its generated local adapter.
    ///
    /// # Parameters
    ///
    /// - `input`: Positional field values supplied for the variant.
    ///
    /// # Returns
    ///
    /// Returns the constructed enum value with local dynamic ownership.
    ///
    /// # Errors
    ///
    /// Returns the construction error and recoverable input values if
    /// validation fails or this variant has no constructor.
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<crate::value::Local>,
    ) -> Result<ReflectedOwned, ConstructionRecovery<crate::value::Local>> {
        match self.construction() {
            Some(construction) => construction.local_constructor().construct_tuple(input),
            None => Err(input.into_recovery(ConstructionError::TargetUnavailable)),
        }
    }

    /// Constructs a unit variant through its generated local adapter.
    ///
    /// # Returns
    ///
    /// Returns the constructed enum value with local dynamic ownership.
    ///
    /// # Errors
    ///
    /// Returns the construction error if validation fails or this variant has
    /// no constructor.
    pub fn construct_unit(&self) -> Result<ReflectedOwned, ConstructionRecovery<crate::value::Local>> {
        match self.construction() {
            Some(construction) => construction.local_constructor().construct_unit(),
            None => Err(ConstructionRecovery::new(
                ConstructionError::TargetUnavailable,
                Vec::new(),
            )),
        }
    }

    /// Returns the enum root that contains this variant.
    ///
    /// # Returns
    ///
    /// Returns the static root descriptor for the declaring enum.
    #[must_use]
    #[inline]
    pub fn declaring_type(&self) -> &'static TypeDescriptor {
        (self.declaring_type)()
    }

    /// Returns the zero-based source declaration index.
    ///
    /// # Returns
    ///
    /// Returns this variant's position in the enum declaration.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the immutable Rust variant name.
    ///
    /// # Returns
    ///
    /// Returns the source Rust identifier.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the immutable lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns whether the variant is unit-, tuple-, or struct-shaped.
    ///
    /// # Returns
    ///
    /// Returns the source variant kind.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> VariantKind {
        self.kind
    }

    /// Returns whether the discriminant appeared explicitly in source.
    ///
    /// # Returns
    ///
    /// Returns the source discriminant origin.
    #[must_use]
    #[inline]
    pub const fn discriminant_origin(&self) -> DiscriminantOrigin {
        self.discriminant_origin
    }

    /// Returns a numeric discriminant only for fieldless integer-`repr` enums.
    ///
    /// # Returns
    ///
    /// Returns the exact numeric discriminant, or `None` when unavailable.
    #[must_use]
    #[inline]
    pub const fn numeric_discriminant(&self) -> Option<NumericDiscriminant> {
        self.numeric_discriminant
    }

    /// Returns fields in source declaration order.
    ///
    /// # Returns
    ///
    /// Returns this variant's fields in source order.
    #[must_use]
    #[inline]
    pub const fn fields(&self) -> &'static [FieldDescriptor] {
        self.fields
    }

    /// Finds a named field by query name.
    ///
    /// `None` means the variant has no field with that lookup name.
    ///
    /// # Parameters
    ///
    /// - `name`: Field lookup name to match.
    ///
    /// # Returns
    ///
    /// Returns the matching field, or `None` when absent.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&FieldDescriptor> {
        self.fields.iter().find(|field| field.query_name() == Some(name))
    }

    /// Returns a field by source index.
    ///
    /// `None` means the index is outside this variant's field range.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based position in this variant's field slice.
    ///
    /// # Returns
    ///
    /// Returns the field at that position, or `None` when out of range.
    #[must_use]
    pub fn field_at(&self, index: usize) -> Option<&FieldDescriptor> {
        self.fields.get(index)
    }

    /// Returns whether this variant is active for an exact declaring enum
    /// value.
    ///
    /// A target of another type returns [`TypeMismatch`] without invoking the
    /// generated adapter.
    ///
    /// # Returns
    ///
    /// Returns whether this variant is active for `value`.
    ///
    /// # Errors
    ///
    /// Returns [`TypeMismatch`] if `value` does not contain the declaring enum
    /// type.
    ///
    /// # Parameters
    ///
    /// - `value`: Dynamic reference expected to contain the declaring enum.
    pub fn is_active(&self, value: ReflectedRef<'_>) -> Result<bool, TypeMismatch> {
        let expected = self.declaring_type().type_id();
        let actual = dynamic_ref_type_id(&value);
        if actual != expected {
            return Err(TypeMismatch::new(expected, actual).with_expected_name(self.declaring_type().type_name()));
        }
        (self.active_test)(value)
    }
}

impl fmt::Debug for VariantDescriptor {
    /// Formats local facts without following declaring-type relationships
    /// recursively.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving this variant's local facts.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after writing the representation.
    ///
    /// # Errors
    ///
    /// Returns the error reported by the formatter.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VariantDescriptor")
            .field("declaring_type", &"<resolver>")
            .field("index", &self.index)
            .field("rust_name", &self.rust_name)
            .field("query_name", &self.query_name)
            .field("kind", &self.kind)
            .field("field_count", &self.fields.len())
            .field("has_active_test", &true)
            .finish()
    }
}
