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
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let primitive = TypeDescriptor::of::<u32>().as_primitive().expect("primitive type");
/// assert_eq!(primitive.kind(), qubit_reflect::descriptor::PrimitiveKind::U32);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveTypeDescriptor {
    kind: PrimitiveKind,
}

impl PrimitiveTypeDescriptor {
    /// Creates a primitive view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Primitive category represented by the root.
    ///
    /// # Returns
    ///
    /// Returns the typed view for `kind`.
    pub(crate) const fn new(kind: PrimitiveKind) -> Self {
        Self { kind }
    }

    /// Returns the exact primitive represented by this view.
    ///
    /// # Returns
    ///
    /// Returns the represented primitive category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> PrimitiveKind {
        self.kind
    }
}

/// The typed view of an owned or borrowed text descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let text = TypeDescriptor::of::<String>().as_text().expect("text type");
/// assert_eq!(text.kind(), qubit_reflect::descriptor::TextKind::String);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TextTypeDescriptor {
    kind: TextKind,
}

impl TextTypeDescriptor {
    /// Creates a text view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Owned or borrowed UTF-8 text representation.
    ///
    /// # Returns
    ///
    /// Returns the typed view for `kind`.
    pub(crate) const fn new(kind: TextKind) -> Self {
        Self { kind }
    }

    /// Returns the exact text representation.
    ///
    /// # Returns
    ///
    /// Returns the owned or borrowed text category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> TextKind {
        self.kind
    }
}

/// The typed view of a declared struct.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Record { value: u8 }
/// let record = TypeDescriptor::of::<Record>().as_struct().expect("struct type");
/// assert_eq!(record.kind(), qubit_reflect::descriptor::StructKind::Named);
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StructTypeDescriptor {
    kind: StructKind,
}

impl StructTypeDescriptor {
    /// Creates a struct view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Declared struct shape.
    ///
    /// # Returns
    ///
    /// Returns the typed view for `kind`.
    pub(crate) const fn new(kind: StructKind) -> Self {
        Self { kind }
    }

    /// Returns whether the struct is named, tuple-shaped, a newtype, or
    /// unit-shaped.
    ///
    /// # Returns
    ///
    /// Returns the source-level struct shape.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> StructKind {
        self.kind
    }
}

/// One normalized component of an enum's explicit `repr(...)` declarations.
///
/// Values are structural metadata rather than diagnostic strings. The enum
/// view exposes components in a stable canonical order, independent of their
/// source order.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::EnumRepr;
/// let repr = EnumRepr::C;
/// assert_eq!(repr, EnumRepr::C);
/// ```
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
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// enum State { Ready }
/// let state = TypeDescriptor::of::<State>().as_enum().expect("enum type");
/// assert!(state.representations().is_empty());
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EnumTypeDescriptor {
    representations: &'static [EnumRepr],
}

impl EnumTypeDescriptor {
    /// Creates an enum view from normalized explicit representation metadata.
    ///
    /// # Parameters
    ///
    /// - `representations`: Explicit components in canonical order.
    ///
    /// # Returns
    ///
    /// Returns an enum view retaining those representation components.
    pub(crate) const fn new(representations: &'static [EnumRepr]) -> Self {
        Self { representations }
    }

    /// Returns normalized explicit `repr(...)` components.
    ///
    /// An empty slice means the enum has no explicit representation
    /// declaration. Components use canonical order and never contain
    /// diagnostic text.
    ///
    /// # Returns
    ///
    /// Returns the explicit representation components, or an empty slice
    /// when no `repr` attribute was declared.
    #[must_use]
    #[inline]
    pub const fn representations(&self) -> &'static [EnumRepr] {
        self.representations
    }
}

/// The typed view of a tuple descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let tuple = TypeDescriptor::of::<(u8, bool)>().as_tuple().expect("tuple type");
/// assert_eq!(tuple.arity(), 2);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct TupleTypeDescriptor {
    elements: TypeRefListSource,
}

impl TupleTypeDescriptor {
    /// Creates a tuple view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `elements`: Eager tuple element type references in declaration order.
    ///
    /// # Returns
    ///
    /// Returns a tuple view backed by the supplied static slice.
    pub(crate) const fn new(elements: &'static [TypeRef]) -> Self {
        Self {
            elements: TypeRefListSource::Eager(elements),
        }
    }

    /// Creates a tuple view whose element list resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `elements`: Lazy source for tuple element references.
    ///
    /// # Returns
    ///
    /// Returns a tuple view backed by the lazy element list.
    pub(crate) const fn new_lazy(elements: &'static LazyTypeRefList) -> Self {
        Self {
            elements: TypeRefListSource::Lazy(elements),
        }
    }

    /// Returns the tuple element types in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the resolved static element list, initializing it on first
    /// access.
    #[must_use]
    #[inline]
    pub fn elements(&self) -> &'static [TypeRef] {
        self.elements.get()
    }

    /// Returns the tuple arity. The unit type `()` therefore has arity zero.
    ///
    /// # Returns
    ///
    /// Returns the number of tuple elements.
    #[must_use]
    #[inline]
    pub const fn arity(&self) -> usize {
        match self.elements {
            TypeRefListSource::Eager(elements) => elements.len(),
            TypeRefListSource::Lazy(elements) => elements.len(),
        }
    }
}

/// The typed view of a fixed-length array descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let array = TypeDescriptor::of::<[u8; 4]>().as_array().expect("array type");
/// assert_eq!(array.length(), 4);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ArrayTypeDescriptor {
    element: TypeRefSource,
    length: usize,
}

impl ArrayTypeDescriptor {
    /// Creates an array view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `element`: Eager element type reference.
    /// - `length`: Fixed array length.
    ///
    /// # Returns
    ///
    /// Returns an array view backed by the supplied type reference.
    pub(crate) const fn new(element: &'static TypeRef, length: usize) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
            length,
        }
    }

    /// Creates an array view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `element`: Lazy source for the element type.
    /// - `length`: Fixed array length.
    ///
    /// # Returns
    ///
    /// Returns an array view backed by the lazy type reference.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef, length: usize) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
            length,
        }
    }

    /// Returns the repeated element type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic element type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }

    /// Returns the compile-time array length.
    ///
    /// # Returns
    ///
    /// Returns the fixed number of elements.
    #[must_use]
    #[inline]
    pub const fn length(&self) -> usize {
        self.length
    }
}

/// The typed view of an optional descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let optional = TypeDescriptor::of::<Option<u8>>().as_optional().expect("optional type");
/// assert!(optional.element_type().as_resolved().is_some());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct OptionalTypeDescriptor {
    element: TypeRefSource,
}

impl OptionalTypeDescriptor {
    /// Creates an optional view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `element`: Eager optional element type reference.
    ///
    /// # Returns
    ///
    /// Returns an optional view backed by the supplied type reference.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates an optional view whose element is resolved on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `element`: Lazy source for the optional element type.
    ///
    /// # Returns
    ///
    /// Returns an optional view backed by the lazy type reference.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the optional element type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic element type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// A standard ordered-sequence family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::SequenceKind;
/// assert_eq!(SequenceKind::Vec, SequenceKind::Vec);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SequenceKind {
    /// [`Vec<T>`].
    Vec,
}

/// The typed view of an ordered sequence descriptor.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let sequence = TypeDescriptor::of::<Vec<u8>>().as_sequence().expect("sequence type");
/// assert_eq!(sequence.kind(), qubit_reflect::descriptor::SequenceKind::Vec);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SequenceTypeDescriptor {
    kind: SequenceKind,
    element: TypeRefSource,
}

impl SequenceTypeDescriptor {
    /// Creates a sequence view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library sequence family.
    /// - `element`: Eager element type reference.
    ///
    /// # Returns
    ///
    /// Returns a sequence view backed by the supplied type reference.
    pub(crate) const fn new(kind: SequenceKind, element: &'static TypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a sequence view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library sequence family.
    /// - `element`: Lazy source for the element type.
    ///
    /// # Returns
    ///
    /// Returns a sequence view backed by the lazy type reference.
    pub(crate) const fn new_lazy(kind: SequenceKind, element: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the concrete standard-library sequence family.
    ///
    /// # Returns
    ///
    /// Returns the sequence family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> SequenceKind {
        self.kind
    }

    /// Returns the sequence element type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic element type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// A standard set family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::SetKind;
/// assert_eq!(SetKind::HashSet, SetKind::HashSet);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetKind {
    /// `HashSet<T>`.
    HashSet,
    /// `BTreeSet<T>`.
    BTreeSet,
}

/// The typed view of a set descriptor.
///
/// # Examples
///
/// ```
/// use std::collections::HashSet;
/// use qubit_reflect::TypeDescriptor;
/// let set = TypeDescriptor::of::<HashSet<u8>>().as_set().expect("set type");
/// assert_eq!(set.kind(), qubit_reflect::descriptor::SetKind::HashSet);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SetTypeDescriptor {
    kind: SetKind,
    element: TypeRefSource,
}

impl SetTypeDescriptor {
    /// Creates a set view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library set family.
    /// - `element`: Eager element type reference.
    ///
    /// # Returns
    ///
    /// Returns a set view backed by the supplied type reference.
    pub(crate) const fn new(kind: SetKind, element: &'static TypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a set view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library set family.
    /// - `element`: Lazy source for the element type.
    ///
    /// # Returns
    ///
    /// Returns a set view backed by the lazy type reference.
    pub(crate) const fn new_lazy(kind: SetKind, element: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the concrete standard-library set family.
    ///
    /// # Returns
    ///
    /// Returns the set family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> SetKind {
        self.kind
    }

    /// Returns the set element type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic element type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// A standard map family.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::MapKind;
/// assert_eq!(MapKind::BTreeMap, MapKind::BTreeMap);
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MapKind {
    /// `HashMap<K, V>`.
    HashMap,
    /// `BTreeMap<K, V>`.
    BTreeMap,
}

/// The typed view of a key-value map descriptor.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use qubit_reflect::TypeDescriptor;
/// let map = TypeDescriptor::of::<BTreeMap<u8, bool>>().as_map().expect("map type");
/// assert_eq!(map.kind(), qubit_reflect::descriptor::MapKind::BTreeMap);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MapTypeDescriptor {
    kind: MapKind,
    key: TypeRefSource,
    value: TypeRefSource,
}

impl MapTypeDescriptor {
    /// Creates a map view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library map family.
    /// - `key`: Eager key type reference.
    /// - `value`: Eager value type reference.
    ///
    /// # Returns
    ///
    /// Returns a map view backed by the supplied type references.
    pub(crate) const fn new(kind: MapKind, key: &'static TypeRef, value: &'static TypeRef) -> Self {
        Self {
            kind,
            key: TypeRefSource::Eager(key),
            value: TypeRefSource::Eager(value),
        }
    }

    /// Creates a map view whose key and value resolve independently on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library map family.
    /// - `key`: Lazy key type source.
    /// - `value`: Lazy value type source.
    ///
    /// # Returns
    ///
    /// Returns a map view backed by the lazy type sources.
    pub(crate) const fn new_lazy(kind: MapKind, key: &'static LazyTypeRef, value: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            key: TypeRefSource::Lazy(key),
            value: TypeRefSource::Lazy(value),
        }
    }

    /// Returns the concrete standard-library map family.
    ///
    /// # Returns
    ///
    /// Returns the map family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> MapKind {
        self.kind
    }

    /// Returns the map key type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic key type, initializing a lazy
    /// reference on first access.
    #[must_use]
    #[inline]
    pub fn key_type(&self) -> &'static TypeRef {
        self.key.get()
    }

    /// Returns the map value type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic value type, initializing a lazy
    /// reference on first access.
    #[must_use]
    #[inline]
    pub fn value_type(&self) -> &'static TypeRef {
        self.value.get()
    }
}

/// The typed view of a standard smart pointer.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let pointer = TypeDescriptor::of::<Box<u8>>().as_smart_pointer().expect("smart pointer type");
/// assert_eq!(pointer.kind(), qubit_reflect::descriptor::SmartPointerKind::Box);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SmartPointerTypeDescriptor {
    kind: SmartPointerKind,
    pointee: TypeRefSource,
}

impl SmartPointerTypeDescriptor {
    /// Creates a smart-pointer view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard smart-pointer family.
    /// - `pointee`: Eager pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a smart-pointer view backed by the supplied type reference.
    pub(crate) const fn new(kind: SmartPointerKind, pointee: &'static TypeRef) -> Self {
        Self {
            kind,
            pointee: TypeRefSource::Eager(pointee),
        }
    }

    /// Creates a smart-pointer view whose pointee is resolved on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard smart-pointer family.
    /// - `pointee`: Lazy pointee type source.
    ///
    /// # Returns
    ///
    /// Returns a smart-pointer view backed by the lazy type source.
    pub(crate) const fn new_lazy(kind: SmartPointerKind, pointee: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            pointee: TypeRefSource::Lazy(pointee),
        }
    }

    /// Returns the concrete smart-pointer family.
    ///
    /// # Returns
    ///
    /// Returns the family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> SmartPointerKind {
        self.kind
    }

    /// Returns the pointee type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic pointee type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn pointee_type(&self) -> &'static TypeRef {
        self.pointee.get()
    }
}

/// The typed view of a Rust reference.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let reference = TypeDescriptor::of::<&'static u8>().as_reference().expect("reference type");
/// assert_eq!(reference.kind(), qubit_reflect::descriptor::ReferenceKind::Shared);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ReferenceTypeDescriptor {
    kind: ReferenceKind,
    target: TypeRefSource,
}

impl ReferenceTypeDescriptor {
    /// Creates a reference view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Shared or mutable reference category.
    /// - `target`: Eager referenced type.
    ///
    /// # Returns
    ///
    /// Returns a reference view backed by the supplied type.
    pub(crate) const fn new(kind: ReferenceKind, target: &'static TypeRef) -> Self {
        Self {
            kind,
            target: TypeRefSource::Eager(target),
        }
    }

    /// Creates a reference view whose target resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Shared or mutable reference category.
    /// - `target`: Lazy referenced type source.
    ///
    /// # Returns
    ///
    /// Returns a reference view backed by the lazy type source.
    pub(crate) const fn new_lazy(kind: ReferenceKind, target: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            target: TypeRefSource::Lazy(target),
        }
    }

    /// Returns whether the reference is shared or mutable.
    ///
    /// # Returns
    ///
    /// Returns the reference borrowing category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic target type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn target_type(&self) -> &'static TypeRef {
        self.target.get()
    }
}

/// The typed view of an unsized slice.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let slice = TypeDescriptor::of::<[u8]>().as_slice().expect("slice type");
/// assert!(slice.element_type().as_resolved().is_some());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SliceTypeDescriptor {
    element: TypeRefSource,
}

impl SliceTypeDescriptor {
    /// Creates a slice view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `element`: Eager slice element type reference.
    ///
    /// # Returns
    ///
    /// Returns a slice view backed by the supplied type reference.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self {
            element: TypeRefSource::Eager(element),
        }
    }

    /// Creates a slice view whose element resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `element`: Lazy slice element type source.
    ///
    /// # Returns
    ///
    /// Returns a slice view backed by the lazy type source.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
        }
    }

    /// Returns the slice element type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic element type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn element_type(&self) -> &'static TypeRef {
        self.element.get()
    }
}

/// The typed view of a raw pointer.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let pointer = TypeDescriptor::of::<*const u8>().as_raw_pointer().expect("raw pointer type");
/// assert_eq!(pointer.mutability(), qubit_reflect::descriptor::Mutability::Const);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct RawPointerTypeDescriptor {
    mutability: Mutability,
    pointee: TypeRefSource,
}

impl RawPointerTypeDescriptor {
    /// Creates a raw-pointer view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `mutability`: Const or mutable pointer category.
    /// - `pointee`: Eager pointee type reference.
    ///
    /// # Returns
    ///
    /// Returns a raw-pointer view backed by the supplied type.
    pub(crate) const fn new(mutability: Mutability, pointee: &'static TypeRef) -> Self {
        Self {
            mutability,
            pointee: TypeRefSource::Eager(pointee),
        }
    }

    /// Creates a raw-pointer view whose pointee resolves on first navigation.
    ///
    /// # Parameters
    ///
    /// - `mutability`: Const or mutable pointer category.
    /// - `pointee`: Lazy pointee type source.
    ///
    /// # Returns
    ///
    /// Returns a raw-pointer view backed by the lazy type source.
    pub(crate) const fn new_lazy(mutability: Mutability, pointee: &'static LazyTypeRef) -> Self {
        Self {
            mutability,
            pointee: TypeRefSource::Lazy(pointee),
        }
    }

    /// Returns whether the pointer is const or mutable.
    ///
    /// # Returns
    ///
    /// Returns the pointer's mutability category.
    #[must_use]
    #[inline]
    pub const fn mutability(&self) -> Mutability {
        self.mutability
    }

    /// Returns the pointee type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic pointee type, initializing a
    /// lazy reference on first access.
    #[must_use]
    #[inline]
    pub fn pointee_type(&self) -> &'static TypeRef {
        self.pointee.get()
    }
}

/// The typed view of a function pointer signature.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let function = TypeDescriptor::of::<fn(u8) -> bool>().as_function().expect("function pointer type");
/// assert_eq!(function.parameters().len(), 1);
/// ```
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
    ///
    /// # Parameters
    ///
    /// - `kind`: Safe or unsafe function-pointer category.
    /// - `abi`: Calling convention.
    /// - `variadic`: Whether the signature accepts a variadic tail.
    /// - `parameters`: Eager parameter types in declaration order.
    /// - `return_type`: Eager return type.
    ///
    /// # Returns
    ///
    /// Returns a function signature view backed by the supplied types.
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
    ///
    /// # Parameters
    ///
    /// - `kind`: Safe or unsafe function-pointer category.
    /// - `abi`: Calling convention.
    /// - `variadic`: Whether the signature accepts a variadic tail.
    /// - `parameters`: Lazy parameter type list in declaration order.
    /// - `return_type`: Lazy return type source.
    ///
    /// # Returns
    ///
    /// Returns a function signature view backed by the lazy type sources.
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
    ///
    /// # Returns
    ///
    /// Returns the function-pointer safety category.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> FunctionPointerKind {
        self.kind
    }

    /// Returns the declared calling convention.
    ///
    /// # Returns
    ///
    /// Returns the static ABI descriptor.
    #[must_use]
    #[inline]
    pub const fn abi(&self) -> &'static FunctionAbi {
        self.abi
    }

    /// Returns whether the function pointer accepts a C-style variadic tail.
    ///
    /// # Returns
    ///
    /// Returns `true` when the signature is variadic.
    #[must_use]
    #[inline]
    pub const fn is_variadic(&self) -> bool {
        self.variadic
    }

    /// Returns parameter types in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the resolved static parameter list, initializing lazy entries
    /// on first access.
    #[must_use]
    #[inline]
    pub fn parameters(&self) -> &'static [TypeRef] {
        self.parameters.get()
    }

    /// Returns the function return type.
    ///
    /// # Returns
    ///
    /// Returns the resolved static return type, initializing it on first
    /// access.
    #[must_use]
    #[inline]
    pub fn return_type(&self) -> &'static TypeRef {
        self.return_type.get()
    }
}

/// The typed view of a dyn-compatible trait object.
///
/// # Examples
///
/// ```
/// use qubit_reflect::TypeDescriptor;
/// let object = TypeDescriptor::of::<dyn std::fmt::Debug>()
///     .as_trait_object()
///     .expect("dyn Debug type");
/// assert!(object.trait_descriptor().rust_path().ends_with("Debug"));
/// ```
#[derive(Clone, Copy)]
pub struct TraitObjectTypeDescriptor {
    trait_descriptor: fn() -> &'static TraitDescriptor,
}

impl TraitObjectTypeDescriptor {
    /// Creates a trait-object view backed by a lazy applied-trait resolver.
    ///
    /// # Parameters
    ///
    /// - `trait_descriptor`: Resolver for the applied trait descriptor.
    ///
    /// # Returns
    ///
    /// Returns a trait-object view backed by that resolver.
    pub(crate) const fn new(trait_descriptor: fn() -> &'static TraitDescriptor) -> Self {
        Self { trait_descriptor }
    }

    /// Returns the applied trait declaration represented by this object type.
    ///
    /// # Returns
    ///
    /// Returns the process-lifetime applied trait descriptor, initializing it
    /// on first access.
    #[must_use]
    pub fn trait_descriptor(&self) -> &'static TraitDescriptor {
        (self.trait_descriptor)()
    }
}

impl std::fmt::Debug for TraitObjectTypeDescriptor {
    /// Formats the linked trait identity without expanding its full graph.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the trait object's local identity.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the identity, or the formatter error.
    ///
    /// # Errors
    ///
    /// Returns an error reported by `formatter`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TraitObjectTypeDescriptor")
            .field("trait", &self.trait_descriptor().rust_path())
            .finish()
    }
}

/// The typed view of an intentionally opaque root descriptor.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect, opaque)]
/// struct Hidden;
/// let opaque = TypeDescriptor::of::<Hidden>().as_opaque().expect("opaque root");
/// assert_eq!(std::mem::size_of_val(opaque), 0);
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct OpaqueTypeView;
