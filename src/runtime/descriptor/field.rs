#![allow(unsafe_code)]

use std::any::TypeId;
use std::fmt;

use crate::runtime::access::FieldAdapter;
use crate::runtime::access::{DynamicFieldAdapter, DynamicFieldMode};
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::expression::TypeExpression;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::{DynamicMut, DynamicOwned, DynamicRef, TypeMismatch};

use super::variant::{VariantDescriptor, VariantIdentity, VariantMetadata};

/// Source visibility retained for a reflected field or variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Visibility {
    Public,
    Crate,
    Restricted(&'static str),
    Private,
}

/// Safe access capabilities available for one field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FieldAccess {
    pub(crate) readable: bool,
    pub(crate) mutable: bool,
    pub(crate) replaceable: bool,
    pub(crate) constructible: bool,
}

/// Why a reflected field adapter cannot be used dynamically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum FieldAccessUnavailableReason {
    FieldTypeNotReflectable,
    UnsupportedFieldShape,
    AdapterUnavailable,
}

/// A failure to read or mutate a field.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum FieldAccessError {
    TargetTypeMismatch(TypeMismatch),
    ValueTypeMismatch(TypeMismatch),
    Unavailable(FieldAccessUnavailableReason),
    NotReadable,
    NotMutable,
    NotPublic,
    InactiveVariant {
        expected: VariantIdentity,
        actual: VariantIdentity,
    },
}

impl fmt::Display for FieldAccessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FieldAccessError {}

/// The diagnostic part of a failed field replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum FieldSetError {
    TargetTypeMismatch(TypeMismatch),
    ValueTypeMismatch(TypeMismatch),
    Unavailable(FieldAccessUnavailableReason),
    NotReplaceable,
    NotPublic,
    InactiveVariant {
        expected: VariantIdentity,
        actual: VariantIdentity,
    },
}

impl fmt::Display for FieldSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FieldSetError {}

/// A failed replacement that returns the original replacement value.
#[must_use]
pub(crate) struct FieldSetFailure<V> {
    error: FieldSetError,
    replacement: V,
}

impl<V> FieldSetFailure<V> {
    pub(crate) fn new(error: FieldSetError, replacement: V) -> Self {
        Self { error, replacement }
    }

    pub(crate) fn error(&self) -> &FieldSetError {
        &self.error
    }

    pub(crate) fn replacement(&self) -> &V {
        &self.replacement
    }

    pub(crate) fn into_parts(self) -> (FieldSetError, V) {
        (self.error, self.replacement)
    }
}

impl<V> fmt::Debug for FieldSetFailure<V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldSetFailure")
            .field("error", &self.error)
            .field("replacement", &std::any::type_name::<V>())
            .finish()
    }
}

fn identity<T: 'static>() -> TypeIdentity {
    TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

/// Static source facts for a field, consumed when binding a descriptor.
pub(crate) struct FieldMetadata {
    pub(crate) index: usize,
    pub(crate) name: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) type_expression: &'static crate::runtime::expression::ExpressionData,
    pub(crate) adapter: &'static dyn FieldAdapter,
    pub(crate) variant: Option<&'static VariantMetadata>,
    pub(crate) active_variant: Option<unsafe fn(*const ()) -> VariantIdentity>,
    pub(crate) dynamic_local:
        Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::Local>>,
    pub(crate) dynamic_thread_safe:
        Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::ThreadSafe>>,
    pub(crate) dynamic_unavailable: FieldAccessUnavailableReason,
    pub(crate) access: FieldAccess,
}

impl FieldMetadata {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        index: usize,
        name: &'static str,
        visibility: Visibility,
        type_expression: &'static crate::runtime::expression::ExpressionData,
        adapter: &'static dyn FieldAdapter,
        variant: Option<&'static VariantMetadata>,
        active_variant: Option<unsafe fn(*const ()) -> VariantIdentity>,
        access: FieldAccess,
    ) -> Self {
        Self {
            index,
            name,
            visibility,
            type_expression,
            adapter,
            variant,
            active_variant,
            dynamic_local: None,
            dynamic_thread_safe: None,
            dynamic_unavailable: FieldAccessUnavailableReason::AdapterUnavailable,
            access,
        }
    }

    pub(crate) const fn with_dynamic_adapters(
        mut self,
        local: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::Local>>,
        thread_safe: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::ThreadSafe>>,
    ) -> Self {
        self.dynamic_local = local;
        self.dynamic_thread_safe = thread_safe;
        self
    }

    pub(crate) const fn with_dynamic_unavailable_reason(
        mut self,
        reason: FieldAccessUnavailableReason,
    ) -> Self {
        self.dynamic_unavailable = reason;
        self
    }
}

/// A context-bound view of one reflected field.
#[derive(Clone, Copy)]
pub(crate) struct FieldDescriptor<'ctx> {
    pub(crate) metadata: &'static FieldMetadata,
    declaring_type: TypeDescriptor<'ctx>,
    declaring_variant: Option<VariantDescriptor<'ctx>>,
}

impl<'ctx> FieldDescriptor<'ctx> {
    pub(crate) const fn new(
        metadata: &'static FieldMetadata,
        declaring_type: TypeDescriptor<'ctx>,
        declaring_variant: Option<VariantDescriptor<'ctx>>,
    ) -> Self {
        Self {
            metadata,
            declaring_type,
            declaring_variant,
        }
    }

    pub(crate) fn context(self) -> &'ctx ReflectionContext {
        self.declaring_type.context()
    }

    pub(crate) fn declaring_type(self) -> TypeDescriptor<'ctx> {
        self.declaring_type
    }

    pub(crate) fn declaring_variant(self) -> Option<VariantDescriptor<'ctx>> {
        self.declaring_variant
    }

    pub(crate) fn index(self) -> usize {
        self.metadata.index
    }

    pub(crate) fn name(self) -> &'static str {
        self.metadata.name
    }

    pub(crate) fn type_expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.type_expression, self.context())
    }

    pub(crate) fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.context()
            .find_type_by_id(self.metadata.adapter.value().type_id)
            .ok()
            .flatten()
    }

    pub(crate) fn concrete_type(
        self,
    ) -> Result<TypeDescriptor<'ctx>, crate::runtime::error::ReflectionError> {
        self.context()
            .find_type_by_id(self.metadata.adapter.value().type_id)
            .map_err(crate::runtime::error::ReflectionError::Context)?
            .ok_or_else(|| {
                crate::runtime::error::ReflectionError::Lookup(
                    crate::runtime::error::LookupError::Missing {
                        subject: crate::runtime::error::LookupSubject::Type,
                        query: crate::runtime::error::LookupQuery::TypeId(
                            self.metadata.adapter.value().type_id,
                        ),
                    },
                )
            })
    }

    pub(crate) fn visibility(self) -> Visibility {
        self.metadata.visibility
    }

    pub(crate) fn access(self) -> FieldAccess {
        let local = self.metadata.dynamic_local;
        let thread_safe = self.metadata.dynamic_thread_safe;
        let can_get = local.is_some_and(|adapter| adapter.can_get())
            || thread_safe.is_some_and(|adapter| adapter.can_get());
        let can_get_mut = local.is_some_and(|adapter| adapter.can_get_mut())
            || thread_safe.is_some_and(|adapter| adapter.can_get_mut());
        let can_set = local.is_some_and(|adapter| adapter.can_set())
            || thread_safe.is_some_and(|adapter| adapter.can_set());

        FieldAccess {
            readable: self.metadata.access.readable && can_get,
            mutable: self.metadata.access.mutable && can_get_mut,
            replaceable: self.metadata.access.replaceable && can_set,
            constructible: self.metadata.access.constructible && can_set,
        }
    }

    pub(crate) fn get<'a, T: 'static, V: 'static>(
        self,
        target: &'a T,
    ) -> Result<&'a V, FieldAccessError> {
        self.check_target::<T>()?;
        if self.metadata.visibility != Visibility::Public {
            return Err(FieldAccessError::NotPublic);
        }
        if !self.metadata.access.readable {
            return Err(FieldAccessError::NotReadable);
        }
        self.check_variant((target as *const T).cast())?;
        if TypeId::of::<V>() != self.metadata.adapter.value().type_id {
            return Err(FieldAccessError::ValueTypeMismatch(TypeMismatch::new(
                self.metadata.adapter.value(),
                identity::<V>(),
            )));
        }
        // SAFETY: owner and field IDs were checked, and the adapter was built
        // from references with the exact Owner and Value types.
        let Some(value) = (unsafe { self.metadata.adapter.ref_ptr((target as *const T).cast()) })
        else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the adapter guarantees this pointer refers to a live field V
        // borrowed from `target` for `'a`.
        Ok(unsafe { &*value.cast::<V>() })
    }

    pub(crate) fn get_dynamic<'a, M: DynamicFieldMode>(
        self,
        target: DynamicRef<'a, M>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError> {
        if self.metadata.visibility != Visibility::Public {
            return Err(FieldAccessError::NotPublic);
        }
        if !self.metadata.access.readable {
            return Err(FieldAccessError::NotReadable);
        }
        M::get(self.metadata, target)
    }

    pub(crate) fn get_dynamic_mut<'a, M: DynamicFieldMode>(
        self,
        target: DynamicMut<'a, M>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError> {
        if self.metadata.visibility != Visibility::Public {
            return Err(FieldAccessError::NotPublic);
        }
        if !self.metadata.access.mutable {
            return Err(FieldAccessError::NotMutable);
        }
        M::get_mut(self.metadata, target)
    }

    pub(crate) fn set_dynamic<M: DynamicFieldMode>(
        self,
        target: DynamicMut<'_, M>,
        value: DynamicOwned<M>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>> {
        if self.metadata.visibility != Visibility::Public {
            return Err(FieldSetFailure::new(FieldSetError::NotPublic, value));
        }
        if !self.metadata.access.replaceable {
            return Err(FieldSetFailure::new(FieldSetError::NotReplaceable, value));
        }
        M::set(self.metadata, target, value)
    }

    /// Reads the field while bypassing its source visibility restriction.
    ///
    /// # Safety
    /// The caller must uphold the access policy that makes this private field readable.
    pub(crate) unsafe fn get_unsafe<'a, T: 'static, V: 'static>(
        self,
        target: &'a T,
    ) -> Result<&'a V, FieldAccessError> {
        self.check_target::<T>()?;
        if !self.metadata.access.readable {
            return Err(FieldAccessError::NotReadable);
        }
        self.check_variant((target as *const T).cast())?;
        if TypeId::of::<V>() != self.metadata.adapter.value().type_id {
            return Err(FieldAccessError::ValueTypeMismatch(TypeMismatch::new(
                self.metadata.adapter.value(),
                identity::<V>(),
            )));
        }
        // SAFETY: the owner and field types were checked above; visibility is the caller's responsibility.
        let Some(value) = (unsafe { self.metadata.adapter.ref_ptr((target as *const T).cast()) })
        else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the typed adapter returns a pointer into `target` for `'a`.
        Ok(unsafe { &*value.cast::<V>() })
    }

    /// Mutably borrows the field while bypassing its source visibility restriction.
    ///
    /// # Safety
    /// The caller must uphold the access policy that makes this private field mutable.
    pub(crate) unsafe fn get_mut_unsafe<'a, T: 'static, V: 'static>(
        self,
        target: &'a mut T,
    ) -> Result<&'a mut V, FieldAccessError> {
        self.check_target::<T>()?;
        if !self.metadata.access.mutable {
            return Err(FieldAccessError::NotMutable);
        }
        self.check_variant((target as *mut T).cast())?;
        if TypeId::of::<V>() != self.metadata.adapter.value().type_id {
            return Err(FieldAccessError::ValueTypeMismatch(TypeMismatch::new(
                self.metadata.adapter.value(),
                identity::<V>(),
            )));
        }
        // SAFETY: the owner and field types were checked above; visibility is the caller's responsibility.
        let Some(value) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) })
        else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the typed adapter returns the unique field pointer for `'a`.
        Ok(unsafe { &mut *value.cast::<V>() })
    }

    /// Replaces the field while bypassing its source visibility restriction.
    ///
    /// # Safety
    /// The caller must uphold the access policy that permits replacing this private field.
    pub(crate) unsafe fn set_unsafe<T: 'static, V: 'static>(
        self,
        target: &mut T,
        value: V,
    ) -> Result<(), FieldSetFailure<V>> {
        if let Err(FieldAccessError::TargetTypeMismatch(mismatch)) = self.check_target::<T>() {
            return Err(FieldSetFailure::new(
                FieldSetError::TargetTypeMismatch(mismatch),
                value,
            ));
        }
        if !self.metadata.access.replaceable {
            return Err(FieldSetFailure::new(FieldSetError::NotReplaceable, value));
        }
        if let Err(error) = self.check_variant((target as *mut T).cast()) {
            let error = match error {
                FieldAccessError::InactiveVariant { expected, actual } => {
                    FieldSetError::InactiveVariant { expected, actual }
                }
                FieldAccessError::Unavailable(reason) => FieldSetError::Unavailable(reason),
                _ => unreachable!("variant validation returns only variant errors"),
            };
            return Err(FieldSetFailure::new(error, value));
        }
        if TypeId::of::<V>() != self.metadata.adapter.value().type_id {
            return Err(FieldSetFailure::new(
                FieldSetError::ValueTypeMismatch(TypeMismatch::new(
                    self.metadata.adapter.value(),
                    identity::<V>(),
                )),
                value,
            ));
        }
        // SAFETY: the owner and field types were checked above; visibility is the caller's responsibility.
        let Some(field) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) })
        else {
            return Err(FieldSetFailure::new(
                FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape),
                value,
            ));
        };
        // SAFETY: the typed adapter returns a valid aligned field pointer.
        drop(unsafe { field.cast::<V>().replace(value) });
        Ok(())
    }

    pub(crate) fn get_mut<'a, T: 'static, V: 'static>(
        self,
        target: &'a mut T,
    ) -> Result<&'a mut V, FieldAccessError> {
        self.check_target::<T>()?;
        if self.metadata.visibility != Visibility::Public {
            return Err(FieldAccessError::NotPublic);
        }
        if !self.metadata.access.mutable {
            return Err(FieldAccessError::NotMutable);
        }
        self.check_variant((target as *mut T).cast())?;
        if TypeId::of::<V>() != self.metadata.adapter.value().type_id {
            return Err(FieldAccessError::ValueTypeMismatch(TypeMismatch::new(
                self.metadata.adapter.value(),
                identity::<V>(),
            )));
        }
        // SAFETY: owner and field IDs were checked, and the adapter was built
        // from mutable references with the exact Owner and Value types.
        let Some(value) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) })
        else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the adapter guarantees this points to the unique mutable V
        // field borrowed from `target` for `'a`.
        Ok(unsafe { &mut *value.cast::<V>() })
    }

    pub(crate) fn set<T: 'static, V: 'static>(
        self,
        target: &mut T,
        value: V,
    ) -> Result<(), FieldSetFailure<V>> {
        if let Err(error) = self.check_target::<T>() {
            return Err(FieldSetFailure::new(
                FieldSetError::TargetTypeMismatch(match error {
                    FieldAccessError::TargetTypeMismatch(mismatch) => mismatch,
                    _ => unreachable!("target validation returns only a type mismatch"),
                }),
                value,
            ));
        }
        if self.metadata.visibility != Visibility::Public {
            return Err(FieldSetFailure::new(FieldSetError::NotPublic, value));
        }
        if !self.metadata.access.replaceable {
            return Err(FieldSetFailure::new(FieldSetError::NotReplaceable, value));
        }
        if let Err(error) = self.check_variant((target as *mut T).cast()) {
            let error = match error {
                FieldAccessError::InactiveVariant { expected, actual } => {
                    FieldSetError::InactiveVariant { expected, actual }
                }
                FieldAccessError::Unavailable(reason) => FieldSetError::Unavailable(reason),
                _ => unreachable!("variant validation returns only variant errors"),
            };
            return Err(FieldSetFailure::new(error, value));
        }
        if TypeId::of::<V>() != self.metadata.adapter.value().type_id {
            return Err(FieldSetFailure::new(
                FieldSetError::ValueTypeMismatch(TypeMismatch::new(
                    self.metadata.adapter.value(),
                    identity::<V>(),
                )),
                value,
            ));
        }
        // SAFETY: both exact IDs were checked and the adapter points at a live
        // field of type V. `replace` moves the prior value out and drops it
        // exactly once after writing the replacement.
        let Some(field) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) })
        else {
            return Err(FieldSetFailure::new(
                FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape),
                value,
            ));
        };
        // SAFETY: `field` is aligned and valid for one V as guaranteed by the adapter.
        drop(unsafe { field.cast::<V>().replace(value) });
        Ok(())
    }

    fn check_target<T: 'static>(self) -> Result<(), FieldAccessError> {
        let actual = identity::<T>();
        (actual.type_id == self.metadata.adapter.owner().type_id)
            .then_some(())
            .ok_or_else(|| {
                FieldAccessError::TargetTypeMismatch(TypeMismatch::new(
                    self.metadata.adapter.owner(),
                    actual,
                ))
            })
    }

    fn check_variant(self, target: *const ()) -> Result<(), FieldAccessError> {
        let Some(expected) = self.metadata.variant else {
            return Ok(());
        };
        let Some(active_variant) = self.metadata.active_variant else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::AdapterUnavailable,
            ));
        };
        // SAFETY: the enclosing access operation checked the owner TypeId.
        let actual = unsafe { active_variant(target) };
        (actual == expected.identity())
            .then_some(())
            .ok_or(FieldAccessError::InactiveVariant {
                expected: expected.identity(),
                actual,
            })
    }
}
