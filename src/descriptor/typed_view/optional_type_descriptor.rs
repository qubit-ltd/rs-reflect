// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;
use std::any::TypeId;

use super::OptionalProjectionError;
use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::Reflect;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::value::DynamicMut;
use crate::value::DynamicRef;
use crate::value::Local;

/// Projects a borrowed optional value into its contained element, when present.
pub(crate) type OptionalRefProjector =
    for<'a> fn(DynamicRef<'a, Local>) -> Result<Option<DynamicRef<'a, Local>>, TypeMismatch>;
/// Projects a mutable optional borrow while preserving its exclusive lifetime.
pub(crate) type OptionalMutProjector =
    for<'a> fn(DynamicMut<'a, Local>) -> Result<Option<DynamicMut<'a, Local>>, TypeMismatch>;

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
    /// Optional function that mutably projects a borrowed optional value.
    mut_projector: Option<OptionalMutProjector>,
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
            mut_projector: None,
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
    pub(crate) const fn new_lazy<T: Reflect>(element: &'static LazyTypeRef, projector: OptionalRefProjector) -> Self {
        Self {
            element: TypeRefSource::Lazy(element),
            projector: Some(projector),
            mut_projector: Some(project_option_mut::<T>),
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
    pub fn project_ref<'a, T: Any>(&self, value: &'a Option<T>) -> Result<Option<&'a T>, OptionalProjectionError> {
        let projected = self.projector.ok_or(OptionalProjectionError::Unavailable)?(DynamicRef::<Local>::new(value))
            .map_err(OptionalProjectionError::TypeMismatch)?;
        projected
            .map(|element| {
                let actual = element.value_type_id();
                element
                    .downcast::<T>()
                    .map_err(|_| OptionalProjectionError::TypeMismatch(TypeMismatch::new(TypeId::of::<T>(), actual)))
            })
            .transpose()
    }

    /// Projects a checked dynamic shared borrow to its optional element.
    ///
    /// This operation checks only the exact optional type and whether it is
    /// `Some`; it does not bypass member visibility or other access policies.
    pub fn project_dynamic_ref<'a>(
        &self,
        value: DynamicRef<'a, Local>,
    ) -> Result<Option<DynamicRef<'a, Local>>, OptionalProjectionError> {
        self.projector.ok_or(OptionalProjectionError::Unavailable)?(value)
            .map_err(OptionalProjectionError::TypeMismatch)
    }

    /// Projects a checked dynamic exclusive borrow to its optional element.
    ///
    /// This operation checks only the exact optional type and whether it is
    /// `Some`; it does not bypass member visibility or other access policies.
    pub fn project_dynamic_mut<'a>(
        &self,
        value: DynamicMut<'a, Local>,
    ) -> Result<Option<DynamicMut<'a, Local>>, OptionalProjectionError> {
        self.mut_projector.ok_or(OptionalProjectionError::Unavailable)?(value)
            .map_err(OptionalProjectionError::TypeMismatch)
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
    value: DynamicRef<'a, Local>,
) -> Result<Option<DynamicRef<'a, Local>>, TypeMismatch> {
    let actual = value.value_type_id();
    let option = value
        .downcast::<Option<T>>()
        .map_err(|_| TypeMismatch::new(TypeId::of::<Option<T>>(), actual))?;
    Ok(option.as_ref().map(DynamicRef::<Local>::new))
}

/// Projects a built-in `Option<T>` value through an exclusive borrow.
pub(crate) fn project_option_mut<'a, T: Reflect>(
    value: DynamicMut<'a, Local>,
) -> Result<Option<DynamicMut<'a, Local>>, TypeMismatch> {
    let actual = value.value_type_id();
    let option = value
        .downcast::<Option<T>>()
        .map_err(|_| TypeMismatch::new(TypeId::of::<Option<T>>(), actual))?;
    Ok(option.as_mut().map(DynamicMut::<Local>::new))
}
