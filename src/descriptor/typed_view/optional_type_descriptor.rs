// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;

use super::OptionalProjectionError;
use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::Reflect;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::value::ReflectedRef;

/// Projects a borrowed optional value into its contained element, when present.
pub(crate) type OptionalRefProjector = for<'a> fn(ReflectedRef<'a>) -> Result<Option<ReflectedRef<'a>>, TypeMismatch>;

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
    /// Eager or lazily resolved optional element type.
    element: TypeRefSource,
    /// Optional function that inspects a borrowed value of the represented
    /// type.
    projector: Option<OptionalRefProjector>,
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
            projector: None,
        }
    }

    /// Creates an optional view backed by a lazily resolved element type.
    ///
    /// # Parameters
    ///
    /// - `element`: Static lazy reference to the optional element type.
    /// - `projector`: Function that checks and borrows the contained value.
    ///
    /// # Returns
    ///
    /// Returns an optional view that resolves its element type on first access
    /// and can project borrowed values.
    pub(crate) const fn new_lazy(element: &'static LazyTypeRef, projector: OptionalRefProjector) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
            projector: Some(projector),
        }
    }

    /// Returns whether this descriptor can inspect borrowed optional values.
    ///
    /// # Returns
    ///
    /// Returns `true` when a projector was registered during construction.
    #[must_use]
    pub const fn has_ref_projection(&self) -> bool {
        self.projector.is_some()
    }

    /// Projects a reflected `Some` value to its element, or reports `None`.
    ///
    /// # Parameters
    ///
    /// - `value`: Local shared borrow of the concrete optional value to
    ///   inspect.
    ///
    /// # Returns
    ///
    /// Returns a borrow of the contained element for `Some`, or `None` when
    /// the optional value is absent.
    ///
    /// # Errors
    ///
    /// Returns [`OptionalProjectionError::Unavailable`] when no projector was
    /// registered, or [`OptionalProjectionError::TypeMismatch`] for a value
    /// whose concrete type differs from the optional type represented here.
    pub fn project_ref<'a>(
        &self,
        value: ReflectedRef<'a>,
    ) -> Result<Option<ReflectedRef<'a>>, OptionalProjectionError> {
        self.projector.ok_or(OptionalProjectionError::Unavailable)?(value).map_err(Into::into)
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

/// Projects a built-in `Option<T>` value using its concrete `T` specialization.
///
/// The returned wrapper is borrowed directly from the consumed wrapper's
/// referent and retains the same lifetime. No element value is cloned.
///
/// # Parameters
///
/// - `value`: Local shared borrow whose concrete type must be `Option<T>`.
///
/// # Returns
///
/// Returns the contained value as a shared reflected borrow, or `None` for an
/// absent optional value.
///
/// # Errors
///
/// Returns a `TypeMismatch` with the expected `Option<T>` and actual value
/// type IDs when the supplied borrow has another concrete type.
pub(crate) fn project_option_ref<'a, T: Reflect>(
    value: ReflectedRef<'a>,
) -> Result<Option<ReflectedRef<'a>>, TypeMismatch> {
    let actual = value.value_type_id();
    let option = value
        .downcast::<Option<T>>()
        .map_err(|_| TypeMismatch::new(TypeId::of::<Option<T>>(), actual))?;
    Ok(option.as_ref().map(ReflectedRef::new))
}
