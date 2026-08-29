//! Kind-specific, immutable views of root type descriptors.

use crate::descriptor::FunctionPointerKind;
use crate::descriptor::Mutability;
use crate::descriptor::PrimitiveKind;
use crate::descriptor::ReferenceKind;
use crate::descriptor::SmartPointerKind;
use crate::descriptor::StructKind;
use crate::descriptor::TextKind;
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
    pub const fn kind(&self) -> StructKind {
        self.kind
    }
}

/// The typed view of a declared enum.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnumTypeDescriptor;

/// The typed view of a tuple descriptor.
#[derive(Clone, Copy, Debug)]
pub struct TupleTypeDescriptor {
    elements: &'static [TypeRef],
}

impl TupleTypeDescriptor {
    /// Creates a tuple view for internal descriptor construction.
    pub(crate) const fn new(elements: &'static [TypeRef]) -> Self {
        Self { elements }
    }

    /// Returns the tuple element types in declaration order.
    pub const fn elements(&self) -> &'static [TypeRef] {
        self.elements
    }

    /// Returns the tuple arity. The unit type `()` therefore has arity zero.
    pub const fn arity(&self) -> usize {
        self.elements.len()
    }
}

/// The typed view of a fixed-length array descriptor.
#[derive(Clone, Copy, Debug)]
pub struct ArrayTypeDescriptor {
    element: &'static TypeRef,
    length: usize,
}

impl ArrayTypeDescriptor {
    /// Creates an array view for internal descriptor construction.
    pub(crate) const fn new(element: &'static TypeRef, length: usize) -> Self {
        Self { element, length }
    }

    /// Returns the repeated element type.
    pub const fn element_type(&self) -> &'static TypeRef {
        self.element
    }

    /// Returns the compile-time array length.
    pub const fn length(&self) -> usize {
        self.length
    }
}

/// The typed view of an optional descriptor.
#[derive(Clone, Copy, Debug)]
pub struct OptionalTypeDescriptor {
    element: &'static TypeRef,
}

impl OptionalTypeDescriptor {
    /// Creates an optional view for internal descriptor construction.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self { element }
    }

    /// Returns the optional element type.
    pub const fn element_type(&self) -> &'static TypeRef {
        self.element
    }
}

/// A standard ordered-sequence family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SequenceKind {
    /// [`Vec<T>`](Vec).
    Vec,
}

/// The typed view of an ordered sequence descriptor.
#[derive(Clone, Copy, Debug)]
pub struct SequenceTypeDescriptor {
    kind: SequenceKind,
    element: &'static TypeRef,
}

impl SequenceTypeDescriptor {
    /// Creates a sequence view for internal descriptor construction.
    pub(crate) const fn new(kind: SequenceKind, element: &'static TypeRef) -> Self {
        Self { kind, element }
    }

    /// Returns the concrete standard-library sequence family.
    pub const fn kind(&self) -> SequenceKind {
        self.kind
    }

    /// Returns the sequence element type.
    pub const fn element_type(&self) -> &'static TypeRef {
        self.element
    }
}

/// A standard set family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SetKind {
    /// [`HashSet<T>`](std::collections::HashSet).
    HashSet,
    /// [`BTreeSet<T>`](std::collections::BTreeSet).
    BTreeSet,
}

/// The typed view of a set descriptor.
#[derive(Clone, Copy, Debug)]
pub struct SetTypeDescriptor {
    kind: SetKind,
    element: &'static TypeRef,
}

impl SetTypeDescriptor {
    /// Creates a set view for internal descriptor construction.
    pub(crate) const fn new(kind: SetKind, element: &'static TypeRef) -> Self {
        Self { kind, element }
    }

    /// Returns the concrete standard-library set family.
    pub const fn kind(&self) -> SetKind {
        self.kind
    }

    /// Returns the set element type.
    pub const fn element_type(&self) -> &'static TypeRef {
        self.element
    }
}

/// A standard map family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MapKind {
    /// [`HashMap<K, V>`](std::collections::HashMap).
    HashMap,
    /// [`BTreeMap<K, V>`](std::collections::BTreeMap).
    BTreeMap,
}

/// The typed view of a key-value map descriptor.
#[derive(Clone, Copy, Debug)]
pub struct MapTypeDescriptor {
    kind: MapKind,
    key: &'static TypeRef,
    value: &'static TypeRef,
}

impl MapTypeDescriptor {
    /// Creates a map view for internal descriptor construction.
    pub(crate) const fn new(kind: MapKind, key: &'static TypeRef, value: &'static TypeRef) -> Self {
        Self { kind, key, value }
    }

    /// Returns the concrete standard-library map family.
    pub const fn kind(&self) -> MapKind {
        self.kind
    }

    /// Returns the map key type.
    pub const fn key_type(&self) -> &'static TypeRef {
        self.key
    }

    /// Returns the map value type.
    pub const fn value_type(&self) -> &'static TypeRef {
        self.value
    }
}

/// The typed view of a standard smart pointer.
#[derive(Clone, Copy, Debug)]
pub struct SmartPointerTypeDescriptor {
    kind: SmartPointerKind,
    pointee: &'static TypeRef,
}

impl SmartPointerTypeDescriptor {
    /// Creates a smart-pointer view for internal descriptor construction.
    pub(crate) const fn new(kind: SmartPointerKind, pointee: &'static TypeRef) -> Self {
        Self { kind, pointee }
    }

    /// Returns the concrete smart-pointer family.
    pub const fn kind(&self) -> SmartPointerKind {
        self.kind
    }

    /// Returns the pointee type.
    pub const fn pointee_type(&self) -> &'static TypeRef {
        self.pointee
    }
}

/// The typed view of a Rust reference.
#[derive(Clone, Copy, Debug)]
pub struct ReferenceTypeDescriptor {
    kind: ReferenceKind,
    target: &'static TypeRef,
}

impl ReferenceTypeDescriptor {
    /// Creates a reference view for internal descriptor construction.
    pub(crate) const fn new(kind: ReferenceKind, target: &'static TypeRef) -> Self {
        Self { kind, target }
    }

    /// Returns whether the reference is shared or mutable.
    pub const fn kind(&self) -> ReferenceKind {
        self.kind
    }

    /// Returns the referenced type.
    pub const fn target_type(&self) -> &'static TypeRef {
        self.target
    }
}

/// The typed view of an unsized slice.
#[derive(Clone, Copy, Debug)]
pub struct SliceTypeDescriptor {
    element: &'static TypeRef,
}

impl SliceTypeDescriptor {
    /// Creates a slice view for internal descriptor construction.
    pub(crate) const fn new(element: &'static TypeRef) -> Self {
        Self { element }
    }

    /// Returns the slice element type.
    pub const fn element_type(&self) -> &'static TypeRef {
        self.element
    }
}

/// The typed view of a raw pointer.
#[derive(Clone, Copy, Debug)]
pub struct RawPointerTypeDescriptor {
    mutability: Mutability,
    pointee: &'static TypeRef,
}

impl RawPointerTypeDescriptor {
    /// Creates a raw-pointer view for internal descriptor construction.
    pub(crate) const fn new(mutability: Mutability, pointee: &'static TypeRef) -> Self {
        Self { mutability, pointee }
    }

    /// Returns whether the pointer is const or mutable.
    pub const fn mutability(&self) -> Mutability {
        self.mutability
    }

    /// Returns the pointee type.
    pub const fn pointee_type(&self) -> &'static TypeRef {
        self.pointee
    }
}

/// The typed view of a function pointer signature.
#[derive(Clone, Copy, Debug)]
pub struct FunctionTypeDescriptor {
    kind: FunctionPointerKind,
    abi: &'static FunctionAbi,
    variadic: bool,
    parameters: &'static [TypeRef],
    return_type: &'static TypeRef,
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
            parameters,
            return_type,
        }
    }

    /// Returns whether the function pointer is safe or unsafe.
    pub const fn kind(&self) -> FunctionPointerKind {
        self.kind
    }

    /// Returns the declared calling convention.
    pub const fn abi(&self) -> &'static FunctionAbi {
        self.abi
    }

    /// Returns whether the function pointer accepts a C-style variadic tail.
    pub const fn is_variadic(&self) -> bool {
        self.variadic
    }

    /// Returns parameter types in declaration order.
    pub const fn parameters(&self) -> &'static [TypeRef] {
        self.parameters
    }

    /// Returns the function return type.
    pub const fn return_type(&self) -> &'static TypeRef {
        self.return_type
    }
}

/// The typed view of a dyn-compatible trait object.
#[derive(Clone, Copy, Debug, Default)]
pub struct TraitObjectTypeDescriptor;

/// The typed view of an intentionally opaque root descriptor.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpaqueTypeView;
