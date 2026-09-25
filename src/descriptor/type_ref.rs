// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Resolved, opaque, and symbolic references between descriptors.

use std::any::TypeId;
use std::fmt;

use super::opaque_type_descriptor::OpaqueTypeDescriptor;
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::TypeRef;
/// use qubit_reflect::TypeDescriptor;
///
/// let reference = TypeRef::Resolved(TypeDescriptor::of::<u32>());
/// assert_eq!(reference.as_resolved().map(TypeDescriptor::type_name), Some("u32"));
/// ```
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
    /// Returns the exact identity for a concrete reference.
    #[must_use]
    pub fn concrete_type_id(&self) -> Option<TypeId> {
        match self {
            Self::Resolved(value) => Some(value.type_id()),
            Self::Opaque(value) => Some(value.type_id()),
            Self::Symbolic(_) => None,
        }
    }
    /// Returns the root descriptor for a resolved reference.
    ///
    /// `None` means this reference is explicitly opaque or still symbolic.
    #[must_use]
    #[inline]
    pub const fn as_resolved(&self) -> Option<&'static TypeDescriptor> {
        match self {
            Self::Resolved(descriptor) => Some(descriptor),
            Self::Opaque(_) | Self::Symbolic(_) => None,
        }
    }

    /// Returns the member-local opaque descriptor for an opaque reference.
    ///
    /// `None` means this reference is resolved or still symbolic.
    #[must_use]
    #[inline]
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
    #[must_use]
    #[inline]
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
