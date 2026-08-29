// qubit-style: allow public-type-layout
//! Stable, hierarchical categories for reflected Rust types.

/// A Rust primitive represented by a root descriptor.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PrimitiveKind {
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

/// A UTF-8 text representation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TextKind {
    /// An owned [`String`].
    String,
    /// A borrowed `str` slice.
    Str,
}

/// The declared shape of a Rust struct.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructKind {
    /// A struct with named fields.
    Named,
    /// A struct with two or more positional fields.
    Tuple,
    /// A tuple struct with exactly one field.
    Newtype,
    /// A struct with no fields.
    Unit,
}

/// A standard smart-pointer family.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SmartPointerKind {
    /// [`Box<T>`](Box).
    Box,
    /// [`Rc<T>`](std::rc::Rc).
    Rc,
    /// [`Arc<T>`](std::sync::Arc).
    Arc,
}

/// The borrowing mode of a reference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReferenceKind {
    /// A shared reference.
    Shared,
    /// An exclusive mutable reference.
    Mutable,
}

/// The mutability of a raw pointer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mutability {
    /// A const raw pointer.
    Const,
    /// A mutable raw pointer.
    Mutable,
}

/// The safety qualifier of a function pointer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FunctionPointerKind {
    /// A safe function pointer.
    Safe,
    /// An unsafe function pointer.
    Unsafe,
}

/// The stable top-level category of a reflected Rust type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TypeKind {
    /// A scalar primitive.
    Primitive(PrimitiveKind),
    /// An owned or borrowed UTF-8 text type.
    Text(TextKind),
    /// A declared struct and its precise shape.
    Struct(StructKind),
    /// A declared enum.
    Enum,
    /// A Rust tuple, including `()`.
    Tuple,
    /// A fixed-length array.
    Array,
    /// An optional value.
    Optional,
    /// An ordered sequence.
    Sequence,
    /// A set.
    Set,
    /// A key-value map.
    Map,
    /// A standard smart pointer.
    SmartPointer(SmartPointerKind),
    /// A shared or mutable reference.
    Reference(ReferenceKind),
    /// An unsized slice.
    Slice,
    /// A raw pointer.
    RawPointer(Mutability),
    /// A function pointer.
    FunctionPointer(FunctionPointerKind),
    /// A dyn-compatible trait object.
    TraitObject,
    /// A root type whose internal shape is intentionally hidden.
    Opaque,
}
