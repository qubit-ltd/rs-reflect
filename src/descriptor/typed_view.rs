// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Kind-specific, immutable views of root type descriptors.

use crate::__private::LazyTypeRef;
use crate::__private::LazyTypeRefList;
use crate::__private::TypeRefListSource;
use crate::__private::TypeRefSource;
use crate::descriptor::FunctionPointerKind;
use crate::descriptor::Mutability;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::ReferenceKind;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::StructKind;
use crate::descriptor::TextKind;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TypeRef;
use crate::expression::FunctionAbi;

/// The typed view of a primitive descriptor.
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveTypeDescriptor {
    kind: PrimitiveKind,
}

impl PrimitiveTypeDescriptor {
    /// Creates a primitive view for internal descriptor construction.
    pub(crate) const fn new(kind: PrimitiveKind) -> Self {
        Self { kind }
    }

    /// Returns the exact primitive represented by this view.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> PrimitiveKind {
        self.kind
    }
}

/// The typed view of an owned or borrowed text descriptor.
#[derive(Clone, Copy, Debug)]
pub struct TextTypeDescriptor {
    kind: TextKind,
}

impl TextTypeDescriptor {
    /// Creates a text view for internal descriptor construction.
    pub(crate) const fn new(kind: TextKind) -> Self {
        Self { kind }
    }

    /// Returns the exact text representation.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> TextKind {
        self.kind
    }
}

/// The typed view of a declared struct.
#[derive(Clone, Copy, Debug)]
pub struct StructTypeDescriptor {
    kind: StructKind,
}

impl StructTypeDescriptor {
    /// Creates a struct view for internal descriptor construction.
    pub(crate) const fn new(kind: StructKind) -> Self {
        Self { kind }
    }

    /// Returns whether the struct is named, tuple-shaped, a newtype, or
    /// unit-shaped.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> StructKind {
        self.kind
    }
}

/// One normalized component of an enum's explicit `repr(...)` declarations.
///
/// Values are structural metadata rather than diagnostic strings. The enum
/// view exposes components in a stable canonical order, independent of their
/// source order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EnumRepr {
    /// Rust's native representation was requested explicitly.
    Rust,
    /// The C-compatible representation was requested.
    C,
    /// The transparent representation was requested.
    Transparent,
    /// An `i8` discriminant representation.
    I8,
    /// An `i16` discriminant representation.
    I16,
    /// An `i32` discriminant representation.
    I32,
    /// An `i64` discriminant representation.
    I64,
    /// An `i128` discriminant representation.
    I128,
    /// An `isize` discriminant representation.
    Isize,
    /// A `u8` discriminant representation.
    U8,
    /// A `u16` discriminant representation.
    U16,
    /// A `u32` discriminant representation.
    U32,
    /// A `u64` discriminant representation.
    U64,
    /// A `u128` discriminant representation.
    U128,
    /// A `usize` discriminant representation.
    Usize,
    /// An explicit minimum alignment in bytes.
    Align(usize),
}

/// The typed view of a declared enum.
#[derive(Clone, Copy, Debug)]
pub struct EnumTypeDescriptor {
    representations: &'static [EnumRepr],
}

impl EnumTypeDescriptor {
    /// Creates an enum view from normalized explicit representation metadata.
    pub(crate) const fn new(representations: &'static [EnumRepr]) -> Self {
        Self { representations }
    }

    /// Returns normalized explicit `repr(...)` components.
    ///
    /// An empty slice means the enum has no explicit representation
    /// declaration. Components use canonical order and never contain
    /// diagnostic text.
    #[must_use]
    #[inline(always)]
    pub const fn representations(&self) -> &'static [EnumRepr] {
        self.representations
    }
}

/// The typed view of a tuple descriptor.
#[derive(Clone, Copy, Debug)]
pub struct TupleTypeDescriptor {
    elements: TypeRefListSource,
}

impl TupleTypeDescriptor {
    /// Creates a tuple view for internal descriptor construction.
    pub(crate) const fn new(elements: &'static [TypeRef]) -> Self {
        Self {
            elements: TypeRefListSource::Eager(elements),
        }
    }

    /// Creates a tuple view whose element list resolves on first navigation.
    pub(crate) const fn new_lazy(elements: &'static LazyTypeRefList) -> Self {
        Self {
            elements: TypeRefListSource::Lazy(elements),
        }
    }

    /// Returns the tuple element types in declaration order.
    #[must_use]
    #[inline(always)]
    pub fn elements(&self) -> &'static [TypeRef] {
        self.elements.get()
    }

    /// Returns the tuple arity. The unit type `()` therefore has arity zero.
    #[must_use]
    #[inline(always)]
    pub const fn arity(&self) -> usize {
        match self.elements {
            TypeRefListSource::Eager(elements) => elements.len(),
            TypeRefListSource::Lazy(elements) => elements.len(),
        }
    }
}

/// The typed view of a fixed-length array descriptor.
#[derive(Clone, Copy, Debug)]
pub struct ArrayTypeDescriptor {
    element: TypeRefSource,
    length: usize,
}

impl ArrayTypeDescriptor {
    /// Creates an array view for internal descriptor construction.
    pub(crate) const fn new(element: &'static TypeRef, length: usize) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
            length,
        }
    }

    /// Creates an array view whose element resolves on first navigation.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef, length: usize) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
            length,
        }
    }

    /// Returns the repeated element type.
    #[must_use]
    #[inline(always)]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }

    /// Returns the compile-time array length.
    #[must_use]
    #[inline(always)]
    pub const fn length(&self) -> usize {
        self.length
    }
}

/// The typed view of an optional descriptor.
#[derive(Clone, Copy, Debug)]
pub struct OptionalTypeDescriptor {
    element: TypeRefSource,
}

impl OptionalTypeDescriptor {
    /// Creates an optional view for internal descriptor construction.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates an optional view whose element is resolved on first
    /// navigation.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the optional element type.
    #[must_use]
    #[inline(always)]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// A standard ordered-sequence family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SequenceKind {
    /// [`Vec<T>`].
    Vec,
}

/// The typed view of an ordered sequence descriptor.
#[derive(Clone, Copy, Debug)]
pub struct SequenceTypeDescriptor {
    kind: SequenceKind,
    element: TypeRefSource,
}

impl SequenceTypeDescriptor {
    /// Creates a sequence view for internal descriptor construction.
    pub(crate) const fn new(kind: SequenceKind, element: &'static TypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a sequence view whose element resolves on first navigation.
    pub(crate) const fn new_lazy(kind: SequenceKind, element: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the concrete standard-library sequence family.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> SequenceKind {
        self.kind
    }

    /// Returns the sequence element type.
    #[must_use]
    #[inline(always)]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// A standard set family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetKind {
    /// `HashSet<T>`.
    HashSet,
    /// `BTreeSet<T>`.
    BTreeSet,
}

/// The typed view of a set descriptor.
#[derive(Clone, Copy, Debug)]
pub struct SetTypeDescriptor {
    kind: SetKind,
    element: TypeRefSource,
}

impl SetTypeDescriptor {
    /// Creates a set view for internal descriptor construction.
    pub(crate) const fn new(kind: SetKind, element: &'static TypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a set view whose element resolves on first navigation.
    pub(crate) const fn new_lazy(kind: SetKind, element: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the concrete standard-library set family.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> SetKind {
        self.kind
    }

    /// Returns the set element type.
    #[must_use]
    #[inline(always)]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// A standard map family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MapKind {
    /// `HashMap<K, V>`.
    HashMap,
    /// `BTreeMap<K, V>`.
    BTreeMap,
}

/// The typed view of a key-value map descriptor.
#[derive(Clone, Copy, Debug)]
pub struct MapTypeDescriptor {
    kind: MapKind,
    key: TypeRefSource,
    value: TypeRefSource,
}

impl MapTypeDescriptor {
    /// Creates a map view for internal descriptor construction.
    pub(crate) const fn new(kind: MapKind, key: &'static TypeRef, value: &'static TypeRef) -> Self {
        Self {
            kind,
            key: TypeRefSource::Eager(key),
            value: TypeRefSource::Eager(value),
        }
    }

    /// Creates a map view whose key and value resolve independently on first
    /// navigation.
    pub(crate) const fn new_lazy(kind: MapKind, key: &'static LazyTypeRef, value: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            key: TypeRefSource::Lazy(key),
            value: TypeRefSource::Lazy(value),
        }
    }

    /// Returns the concrete standard-library map family.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> MapKind {
        self.kind
    }

    /// Returns the map key type.
    #[must_use]
    #[inline(always)]
    pub fn key_type(&self) -> &'static TypeRef {
        self.key.get()
    }

    /// Returns the map value type.
    #[must_use]
    #[inline(always)]
    pub fn value_type(&self) -> &'static TypeRef {
        self.value.get()
    }
}

/// The typed view of a standard smart pointer.
#[derive(Clone, Copy, Debug)]
pub struct SmartPointerTypeDescriptor {
    kind: SmartPointerKind,
    pointee: TypeRefSource,
}

impl SmartPointerTypeDescriptor {
    /// Creates a smart-pointer view for internal descriptor construction.
    pub(crate) const fn new(kind: SmartPointerKind, pointee: &'static TypeRef) -> Self {
        Self {
            kind,
            pointee: TypeRefSource::Eager(pointee),
        }
    }

    /// Creates a smart-pointer view whose pointee is resolved on first
    /// navigation.
    pub(crate) const fn new_lazy(kind: SmartPointerKind, pointee: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            pointee: TypeRefSource::Lazy(pointee),
        }
    }

    /// Returns the concrete smart-pointer family.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> SmartPointerKind {
        self.kind
    }

    /// Returns the pointee type.
    #[must_use]
    #[inline(always)]
    pub fn pointee_type(&self) -> &'static TypeRef {
        self.pointee.get()
    }
}

/// The typed view of a Rust reference.
#[derive(Clone, Copy, Debug)]
pub struct ReferenceTypeDescriptor {
    kind: ReferenceKind,
    target: TypeRefSource,
}

impl ReferenceTypeDescriptor {
    /// Creates a reference view for internal descriptor construction.
    pub(crate) const fn new(kind: ReferenceKind, target: &'static TypeRef) -> Self {
        Self {
            kind,
            target: TypeRefSource::Eager(target),
        }
    }

    /// Creates a reference view whose target resolves on first navigation.
    pub(crate) const fn new_lazy(kind: ReferenceKind, target: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            target: TypeRefSource::Lazy(target),
        }
    }

    /// Returns whether the reference is shared or mutable.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced type.
    #[must_use]
    #[inline(always)]
    pub fn target_type(&self) -> &'static TypeRef {
        self.target.get()
    }
}

/// The typed view of an unsized slice.
#[derive(Clone, Copy, Debug)]
pub struct SliceTypeDescriptor {
    element: TypeRefSource,
}

impl SliceTypeDescriptor {
    /// Creates a slice view for internal descriptor construction.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a slice view whose element resolves on first navigation.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the slice element type.
    #[must_use]
    #[inline(always)]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// The typed view of a raw pointer.
#[derive(Clone, Copy, Debug)]
pub struct RawPointerTypeDescriptor {
    mutability: Mutability,
    pointee: TypeRefSource,
}

impl RawPointerTypeDescriptor {
    /// Creates a raw-pointer view for internal descriptor construction.
    pub(crate) const fn new(mutability: Mutability, pointee: &'static TypeRef) -> Self {
        Self {
            mutability,
            pointee: TypeRefSource::Eager(pointee),
        }
    }

    /// Creates a raw-pointer view whose pointee resolves on first navigation.
    pub(crate) const fn new_lazy(mutability: Mutability, pointee: &'static LazyTypeRef) -> Self {
        Self {
            mutability,
            pointee: TypeRefSource::Lazy(pointee),
        }
    }

    /// Returns whether the pointer is const or mutable.
    #[must_use]
    #[inline(always)]
    pub const fn mutability(&self) -> Mutability {
        self.mutability
    }

    /// Returns the pointee type.
    #[must_use]
    #[inline(always)]
    pub fn pointee_type(&self) -> &'static TypeRef {
        self.pointee.get()
    }
}

/// The typed view of a function pointer signature.
#[derive(Clone, Copy, Debug)]
pub struct FunctionTypeDescriptor {
    kind: FunctionPointerKind,
    abi: &'static FunctionAbi,
    variadic: bool,
    parameters: TypeRefListSource,
    return_type: TypeRefSource,
}

impl FunctionTypeDescriptor {
    /// Creates a function-pointer view for internal descriptor construction.
    pub(crate) const fn new(
        kind: FunctionPointerKind,
        abi: &'static FunctionAbi,
        variadic: bool,
        parameters: &'static [TypeRef],
        return_type: &'static TypeRef,
    ) -> Self {
        Self {
            kind,
            abi,
            variadic,
            parameters: TypeRefListSource::Eager(parameters),
            return_type: TypeRefSource::Eager(return_type),
        }
    }

    /// Creates a function view whose signature relationships resolve on first
    /// navigation.
    pub(crate) const fn new_lazy(
        kind: FunctionPointerKind,
        abi: &'static FunctionAbi,
        variadic: bool,
        parameters: &'static LazyTypeRefList,
        return_type: &'static LazyTypeRef,
    ) -> Self {
        Self {
            kind,
            abi,
            variadic,
            parameters: TypeRefListSource::Lazy(parameters),
            return_type: TypeRefSource::Lazy(return_type),
        }
    }

    /// Returns whether the function pointer is safe or unsafe.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> FunctionPointerKind {
        self.kind
    }

    /// Returns the declared calling convention.
    #[must_use]
    #[inline(always)]
    pub const fn abi(&self) -> &'static FunctionAbi {
        self.abi
    }

    /// Returns whether the function pointer accepts a C-style variadic tail.
    #[must_use]
    #[inline(always)]
    pub const fn is_variadic(&self) -> bool {
        self.variadic
    }

    /// Returns parameter types in declaration order.
    #[must_use]
    #[inline(always)]
    pub fn parameters(&self) -> &'static [TypeRef] {
        self.parameters.get()
    }

    /// Returns the function return type.
    #[must_use]
    #[inline(always)]
    pub fn return_type(&self) -> &'static TypeRef {
        self.return_type.get()
    }
}

/// The typed view of a dyn-compatible trait object.
#[derive(Clone, Copy)]
pub struct TraitObjectTypeDescriptor {
    trait_descriptor: fn() -> &'static TraitDescriptor,
}

impl TraitObjectTypeDescriptor {
    /// Creates a trait-object view backed by a lazy applied-trait resolver.
    pub(crate) const fn new(trait_descriptor: fn() -> &'static TraitDescriptor) -> Self {
        Self { trait_descriptor }
    }

    /// Returns the applied trait declaration represented by this object type.
    #[must_use]
    #[inline(always)]
    pub fn trait_descriptor(&self) -> &'static TraitDescriptor {
        (self.trait_descriptor)()
    }
}

impl std::fmt::Debug for TraitObjectTypeDescriptor {
    /// Formats the linked trait identity without expanding its full graph.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TraitObjectTypeDescriptor")
            .field("trait", &self.trait_descriptor().rust_path())
            .finish()
    }
}

/// The typed view of an intentionally opaque root descriptor.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpaqueTypeView;
