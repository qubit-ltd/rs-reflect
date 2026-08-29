//! Resolved, opaque, and symbolic references between descriptors.

use std::any::TypeId;
use std::fmt;

use crate::descriptor::TypeDescriptor;
use crate::expression::TypeExpression;

/// Returns the process-local Rust identity of `T`.
pub(crate) fn type_id_of<T: ?Sized + 'static>() -> TypeId {
    TypeId::of::<T>()
}

/// Returns the compiler-provided diagnostic name for `T`.
pub(crate) fn type_name_of<T: ?Sized + 'static>() -> &'static str {
    std::any::type_name::<T>()
}

/// A type used by a reflected declaration or member.
#[derive(Clone)]
pub enum TypeRef {
    /// A concrete reflected type with a unique root descriptor.
    Resolved(&'static TypeDescriptor),
    /// A concrete member type whose internal structure is intentionally hidden.
    Opaque(&'static OpaqueTypeDescriptor),
    /// A declaration-level type that is not concrete yet.
    Symbolic(TypeExpression),
}

impl TypeRef {
    /// Returns the root descriptor for a resolved reference.
    ///
    /// `None` means this reference is explicitly opaque or still symbolic.
    pub const fn as_resolved(&self) -> Option<&'static TypeDescriptor> {
        match self {
            Self::Resolved(descriptor) => Some(descriptor),
            Self::Opaque(_) | Self::Symbolic(_) => None,
        }
    }

    /// Returns the member-local opaque descriptor for an opaque reference.
    ///
    /// `None` means this reference is resolved or still symbolic.
    pub const fn as_opaque(&self) -> Option<&'static OpaqueTypeDescriptor> {
        match self {
            Self::Opaque(descriptor) => Some(descriptor),
            Self::Resolved(_) | Self::Symbolic(_) => None,
        }
    }

    /// Returns the structural expression for a symbolic reference.
    ///
    /// `None` means this reference already denotes a concrete resolved or
    /// opaque type.
    pub const fn as_symbolic(&self) -> Option<&TypeExpression> {
        match self {
            Self::Symbolic(expression) => Some(expression),
            Self::Resolved(_) | Self::Opaque(_) => None,
        }
    }
}

impl fmt::Debug for TypeRef {
    /// Formats resolved relationships by name so recursive descriptor graphs
    /// remain bounded.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Resolved(descriptor) => formatter
                .debug_tuple("Resolved")
                .field(&descriptor.type_name())
                .finish(),
            Self::Opaque(descriptor) => formatter.debug_tuple("Opaque").field(descriptor).finish(),
            Self::Symbolic(expression) => formatter.debug_tuple("Symbolic").field(expression).finish(),
        }
    }
}

/// A concrete member type whose internal structure was explicitly hidden.
///
/// This object is a member-local view, not a second root [`TypeDescriptor`]. It
/// retains only exact process-local identity and diagnostic naming until safe
/// whole-value adapters are added.
pub struct OpaqueTypeDescriptor {
    type_id: fn() -> TypeId,
    type_name: fn() -> &'static str,
}

impl OpaqueTypeDescriptor {
    /// Creates an immutable opaque member descriptor for `T`.
    ///
    /// Its diagnostic name is resolved from `T` only when queried, so static
    /// generated descriptor data cannot substitute a different type name.
    #[doc(hidden)]
    pub(crate) const fn new<T: ?Sized + 'static>() -> Self {
        Self {
            type_id: type_id_of::<T>,
            type_name: type_name_of::<T>,
        }
    }

    /// Returns the exact process-local Rust type identity.
    pub fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the diagnostic Rust type name.
    pub fn type_name(&self) -> &'static str {
        (self.type_name)()
    }
}

impl fmt::Debug for OpaqueTypeDescriptor {
    /// Formats the diagnostic identity without attempting root-descriptor
    /// navigation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpaqueTypeDescriptor")
            .field("type_id", &self.type_id())
            .field("type_name", &self.type_name())
            .finish()
    }
}
