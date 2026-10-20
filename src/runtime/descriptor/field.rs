// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![allow(unsafe_code)]

use std::any::TypeId;
use std::fmt;

use super::variant::VariantDescriptor;
use super::variant::VariantIdentity;
use super::variant::VariantMetadata;
use crate::runtime::access::DynamicFieldAdapter;
use crate::runtime::access::DynamicFieldMode;
use crate::runtime::access::FieldAdapter;
use crate::runtime::access::TypedFieldAdapter;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::expression::TypeExpression;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::DynamicMut;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::TypeMismatch;

/// Source visibility retained for a reflected field or variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    /// The item is public from all modules.
    Public,
    /// The item is visible within its defining crate.
    Crate,
    /// The item is visible within the named module subtree.
    Restricted(&'static str),
    /// The item is private to its defining module.
    Private,
}

/// Safe access capabilities available for one field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldAccess {
    /// Whether a safe read adapter is available.
    pub readable: bool,
    /// Whether a safe mutable borrow adapter is available.
    pub mutable: bool,
    /// Whether a safe replacement adapter is available.
    pub replaceable: bool,
    /// Whether the field can be initialized through dynamic construction.
    pub constructible: bool,
}

impl FieldAccess {
    /// Creates a set of field access capabilities.
    pub const fn new(readable: bool, mutable: bool, replaceable: bool, constructible: bool) -> Self {
        Self {
            readable,
            mutable,
            replaceable,
            constructible,
        }
    }
}

/// Why a reflected field adapter cannot be used dynamically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FieldAccessUnavailableReason {
    /// The field's value type has no concrete reflection adapter.
    FieldTypeNotReflectable,
    /// The field shape cannot be represented by the available adapter.
    UnsupportedFieldShape,
    /// No adapter was generated for this access operation.
    AdapterUnavailable,
}

/// A failure to read or mutate a field.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FieldAccessError {
    /// The supplied owner value does not match the field's declaring type.
    TargetTypeMismatch(TypeMismatch),
    /// The requested field value type does not match the declared field type.
    ValueTypeMismatch(TypeMismatch),
    /// The requested access operation has no usable adapter.
    Unavailable(FieldAccessUnavailableReason),
    /// The field is not readable.
    NotReadable,
    /// The field is not mutably accessible.
    NotMutable,
    /// Safe access is denied by source visibility.
    NotPublic,
    /// The field belongs to an enum variant that is not active.
    InactiveVariant {
        /// The variant that declares the requested field.
        expected: VariantIdentity,
        /// The variant currently active in the value.
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
pub enum FieldSetError {
    /// The supplied owner value does not match the field's declaring type.
    TargetTypeMismatch(TypeMismatch),
    /// The replacement value type does not match the declared field type.
    ValueTypeMismatch(TypeMismatch),
    /// No replacement adapter is available.
    Unavailable(FieldAccessUnavailableReason),
    /// The field cannot be replaced.
    NotReplaceable,
    /// Safe replacement is denied by source visibility.
    NotPublic,
    /// The field belongs to an enum variant that is not active.
    InactiveVariant {
        /// The variant that declares the requested field.
        expected: VariantIdentity,
        /// The variant currently active in the value.
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
pub struct FieldSetFailure<V> {
    error: FieldSetError,
    replacement: V,
}

impl<V> FieldSetFailure<V> {
    pub(crate) fn new(error: FieldSetError, replacement: V) -> Self {
        Self { error, replacement }
    }

    /// Returns the reason the replacement failed.
    pub fn error(&self) -> &FieldSetError {
        &self.error
    }

    /// Returns the replacement value retained after a failed assignment.
    pub fn replacement(&self) -> &V {
        &self.replacement
    }

    /// Consumes the failure and returns its error together with the replacement
    /// value.
    pub fn into_parts(self) -> (FieldSetError, V) {
        (self.error, self.replacement)
    }
}

impl<V> RecoverableFailure for FieldSetFailure<V> {
    type Error = FieldSetError;
    type Recovery = V;

    fn error(&self) -> &Self::Error {
        &self.error
    }

    fn recovery(&self) -> &Self::Recovery {
        &self.replacement
    }

    fn into_parts(self) -> (Self::Error, Self::Recovery) {
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
pub struct FieldMetadata {
    pub(crate) index: usize,
    pub(crate) name: &'static str,
    pub(crate) visibility: Visibility,
    pub(crate) type_expression: &'static crate::runtime::expression::ExpressionData,
    pub(crate) adapter: &'static dyn FieldAdapter,
    pub(crate) variant: Option<&'static VariantMetadata>,
    pub(crate) variant_source: Option<fn() -> &'static VariantMetadata>,
    pub(crate) active_variant: Option<unsafe fn(*const ()) -> VariantIdentity>,
    pub(crate) dynamic_local: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::Local>>,
    pub(crate) dynamic_thread_safe: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::ThreadSafe>>,
    pub(crate) dynamic_unavailable: FieldAccessUnavailableReason,
    pub(crate) access: FieldAccess,
}

impl FieldMetadata {
    /// Creates metadata for a statically typed field adapter.
    pub const fn typed<Owner: 'static, Value: 'static>(
        index: usize,
        name: &'static str,
        visibility: Visibility,
        type_expression: &'static crate::runtime::expression::ExpressionData,
        adapter: &'static crate::runtime::access::TypedFieldAdapter<Owner, Value>,
        access: FieldAccess,
    ) -> Self {
        Self {
            index,
            name,
            visibility,
            type_expression,
            adapter,
            variant: None,
            variant_source: None,
            active_variant: None,
            dynamic_local: None,
            dynamic_thread_safe: None,
            dynamic_unavailable: FieldAccessUnavailableReason::AdapterUnavailable,
            access,
        }
    }

    /// Creates metadata for a typed field declared inside an enum variant.
    pub const fn typed_variant<Owner: 'static, Value: 'static>(
        index: usize,
        name: &'static str,
        visibility: Visibility,
        type_expression: &'static crate::runtime::expression::ExpressionData,
        adapter: &'static TypedFieldAdapter<Owner, Value>,
        variant: fn() -> &'static VariantMetadata,
        access: FieldAccess,
    ) -> Self {
        Self {
            index,
            name,
            visibility,
            type_expression,
            adapter,
            variant: None,
            variant_source: Some(variant),
            active_variant: None,
            dynamic_local: None,
            dynamic_thread_safe: None,
            dynamic_unavailable: FieldAccessUnavailableReason::AdapterUnavailable,
            access,
        }
    }

    /// Enables local dynamic access when both field types support reflection.
    pub const fn with_local_dynamic<Owner, Value>(mut self, adapter: &'static TypedFieldAdapter<Owner, Value>) -> Self
    where
        Owner: crate::runtime::descriptor::Reflect
            + crate::runtime::value::ValueForRef<crate::runtime::value::Local>
            + crate::runtime::value::ValueForMut<crate::runtime::value::Local>
            + 'static,
        Value: crate::runtime::descriptor::Reflect
            + crate::runtime::value::ValueForRef<crate::runtime::value::Local>
            + crate::runtime::value::ValueForMut<crate::runtime::value::Local>
            + crate::runtime::value::ValueFor<crate::runtime::value::Local>
            + 'static,
    {
        self.dynamic_local = Some(adapter.local_dynamic());
        self
    }

    /// Enables thread-safe dynamic access when both field types are `Send +
    /// Sync`.
    pub const fn with_thread_safe_dynamic<Owner, Value>(
        mut self,
        adapter: &'static TypedFieldAdapter<Owner, Value>,
    ) -> Self
    where
        Owner: crate::runtime::descriptor::Reflect
            + crate::runtime::value::ValueForRef<crate::runtime::value::ThreadSafe>
            + crate::runtime::value::ValueForMut<crate::runtime::value::ThreadSafe>
            + 'static,
        Value: crate::runtime::descriptor::Reflect
            + crate::runtime::value::ValueForRef<crate::runtime::value::ThreadSafe>
            + crate::runtime::value::ValueForMut<crate::runtime::value::ThreadSafe>
            + crate::runtime::value::ValueFor<crate::runtime::value::ThreadSafe>
            + 'static,
    {
        self.dynamic_thread_safe = Some(adapter.thread_safe_dynamic());
        self
    }

    #[allow(clippy::too_many_arguments)]
    /// Creates field metadata with its typed adapter and source facts.
    pub const fn new(
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
            variant_source: None,
            active_variant,
            dynamic_local: None,
            dynamic_thread_safe: None,
            dynamic_unavailable: FieldAccessUnavailableReason::AdapterUnavailable,
            access,
        }
    }

    pub(crate) fn variant_metadata(&self) -> Option<&'static VariantMetadata> {
        self.variant.or_else(|| self.variant_source.map(|source| source()))
    }

    /// Adds adapters for dynamically typed reads and writes.
    pub const fn with_dynamic_adapters(
        mut self,
        local: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::Local>>,
        thread_safe: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::ThreadSafe>>,
    ) -> Self {
        self.dynamic_local = local;
        self.dynamic_thread_safe = thread_safe;
        self
    }

    /// Records why dynamic field access cannot be provided.
    pub const fn with_dynamic_unavailable_reason(mut self, reason: FieldAccessUnavailableReason) -> Self {
        self.dynamic_unavailable = reason;
        self
    }
}

/// A context-bound view of one reflected field.
#[derive(Clone, Copy)]
pub struct FieldDescriptor<'ctx> {
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

    /// Returns the context that owns this descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.declaring_type.context()
    }

    /// Returns the type that declares this field.
    pub fn declaring_type(self) -> TypeDescriptor<'ctx> {
        self.declaring_type
    }

    /// Returns the enum variant declaring this field, if any.
    pub fn declaring_variant(self) -> Option<VariantDescriptor<'ctx>> {
        self.declaring_variant
    }

    /// Returns the field's original source position within its declaration.
    pub fn index(self) -> usize {
        self.metadata.index
    }

    /// Returns the field's source name, or its generated tuple-field name.
    pub fn name(self) -> &'static str {
        self.metadata.name
    }

    /// Returns the field type expression in this descriptor's context.
    pub fn type_expression(self) -> TypeExpression<'ctx> {
        TypeExpression::new(self.metadata.type_expression, self.context())
    }

    /// Looks up the concrete field type when it is registered in this context.
    pub fn find_concrete_type(self) -> Option<TypeDescriptor<'ctx>> {
        self.context()
            .find_type_by_id(self.metadata.adapter.value().type_id)
            .ok()
            .flatten()
    }

    /// Resolves the concrete field type or returns the context/lookup failure.
    pub fn concrete_type(self) -> Result<TypeDescriptor<'ctx>, crate::runtime::error::ReflectionError> {
        self.context()
            .find_type_by_id(self.metadata.adapter.value().type_id)
            .map_err(crate::runtime::error::ReflectionError::Context)?
            .ok_or_else(|| {
                crate::runtime::error::ReflectionError::Lookup(crate::runtime::error::LookupError::Missing {
                    subject: crate::runtime::error::LookupSubject::Type,
                    query: crate::runtime::error::LookupQuery::TypeId(self.metadata.adapter.value().type_id),
                })
            })
    }

    /// Returns source visibility retained for this field.
    pub fn visibility(self) -> Visibility {
        self.metadata.visibility
    }

    /// Returns the safe dynamic access operations supported by this field.
    pub fn access(self) -> FieldAccess {
        let local = self.metadata.dynamic_local;
        let thread_safe = self.metadata.dynamic_thread_safe;
        let can_get =
            local.is_some_and(|adapter| adapter.can_get()) || thread_safe.is_some_and(|adapter| adapter.can_get());
        let can_get_mut = local.is_some_and(|adapter| adapter.can_get_mut())
            || thread_safe.is_some_and(|adapter| adapter.can_get_mut());
        let can_set =
            local.is_some_and(|adapter| adapter.can_set()) || thread_safe.is_some_and(|adapter| adapter.can_set());

        FieldAccess {
            readable: self.metadata.access.readable && can_get,
            mutable: self.metadata.access.mutable && can_get_mut,
            replaceable: self.metadata.access.replaceable && can_set,
            constructible: self.metadata.access.constructible && can_set,
        }
    }

    /// Reads a public field after checking owner and value types.
    pub fn get<T: 'static, V: 'static>(self, target: &T) -> Result<&V, FieldAccessError> {
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
        let Some(value) = (unsafe { self.metadata.adapter.ref_ptr((target as *const T).cast()) }) else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the adapter guarantees this pointer refers to a live field V
        // borrowed from `target` for `'a`.
        Ok(unsafe { &*value.cast::<V>() })
    }

    /// Reads a public field through a dynamically typed value.
    pub fn get_dynamic<'a, M: DynamicFieldMode>(
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

    /// Mutably borrows a public field through a dynamically typed value.
    pub fn get_dynamic_mut<'a, M: DynamicFieldMode>(
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

    /// Replaces a public field dynamically, retaining the replacement on
    /// failure.
    pub fn set_dynamic<M: DynamicFieldMode>(
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
    /// The caller must uphold the access policy that makes this private field
    /// readable.
    pub unsafe fn get_unsafe<T: 'static, V: 'static>(self, target: &T) -> Result<&V, FieldAccessError> {
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
        // SAFETY: the owner and field types were checked above; visibility is the
        // caller's responsibility.
        let Some(value) = (unsafe { self.metadata.adapter.ref_ptr((target as *const T).cast()) }) else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the typed adapter returns a pointer into `target` for `'a`.
        Ok(unsafe { &*value.cast::<V>() })
    }

    /// Mutably borrows the field while bypassing its source visibility
    /// restriction.
    ///
    /// # Safety
    /// The caller must uphold the access policy that makes this private field
    /// mutable.
    pub unsafe fn get_mut_unsafe<T: 'static, V: 'static>(self, target: &mut T) -> Result<&mut V, FieldAccessError> {
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
        // SAFETY: the owner and field types were checked above; visibility is the
        // caller's responsibility.
        let Some(value) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) }) else {
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
    /// The caller must uphold the access policy that permits replacing this
    /// private field.
    pub unsafe fn set_unsafe<T: 'static, V: 'static>(self, target: &mut T, value: V) -> Result<(), FieldSetFailure<V>> {
        if let Err(FieldAccessError::TargetTypeMismatch(mismatch)) = self.check_target::<T>() {
            return Err(FieldSetFailure::new(FieldSetError::TargetTypeMismatch(mismatch), value));
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
                FieldSetError::ValueTypeMismatch(TypeMismatch::new(self.metadata.adapter.value(), identity::<V>())),
                value,
            ));
        }
        // SAFETY: the owner and field types were checked above; visibility is the
        // caller's responsibility.
        let Some(field) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) }) else {
            return Err(FieldSetFailure::new(
                FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape),
                value,
            ));
        };
        // SAFETY: the typed adapter returns a valid aligned field pointer.
        drop(unsafe { field.cast::<V>().replace(value) });
        Ok(())
    }

    /// Mutably borrows a public field after checking owner and value types.
    pub fn get_mut<T: 'static, V: 'static>(self, target: &mut T) -> Result<&mut V, FieldAccessError> {
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
        let Some(value) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) }) else {
            return Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape,
            ));
        };
        // SAFETY: the adapter guarantees this points to the unique mutable V
        // field borrowed from `target` for `'a`.
        Ok(unsafe { &mut *value.cast::<V>() })
    }

    /// Replaces a public field; on error, the failure contains the unconsumed
    /// value.
    pub fn set<T: 'static, V: 'static>(self, target: &mut T, value: V) -> Result<(), FieldSetFailure<V>> {
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
                FieldSetError::ValueTypeMismatch(TypeMismatch::new(self.metadata.adapter.value(), identity::<V>())),
                value,
            ));
        }
        // SAFETY: both exact IDs were checked and the adapter points at a live
        // field of type V. `replace` moves the prior value out and drops it
        // exactly once after writing the replacement.
        let Some(field) = (unsafe { self.metadata.adapter.mut_ptr((target as *mut T).cast()) }) else {
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
                FieldAccessError::TargetTypeMismatch(TypeMismatch::new(self.metadata.adapter.owner(), actual))
            })
    }

    fn check_variant(self, target: *const ()) -> Result<(), FieldAccessError> {
        let Some(expected) = self.metadata.variant_metadata() else {
            return Ok(());
        };
        let actual = if let Some(active_variant) = self.metadata.active_variant {
            // SAFETY: the enclosing access operation checked the owner TypeId.
            unsafe { active_variant(target) }
        } else {
            // SAFETY: the enclosing access operation checked the owner TypeId.
            unsafe { self.metadata.adapter.active_variant(target) }.ok_or(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::AdapterUnavailable,
            ))?
        };
        (actual == expected.identity())
            .then_some(())
            .ok_or(FieldAccessError::InactiveVariant {
                expected: expected.identity(),
                actual,
            })
    }
}

#[cfg(all(test, feature = "derive"))]
mod tests {
    use std::any::TypeId;

    use super::FieldAccess;
    use super::FieldAccessError;
    use super::FieldAccessUnavailableReason;
    use super::FieldDescriptor;
    use super::FieldMetadata;
    use super::FieldSetError;
    use super::Visibility;
    use crate::runtime::access::FieldAdapter;
    use crate::runtime::descriptor::TypeDescriptor;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::identity::TypeIdentity;

    #[crate::Reflect]
    struct Owner {
        value: u32,
    }

    struct MissingPointerAdapter;

    impl FieldAdapter for MissingPointerAdapter {
        fn owner(&self) -> TypeIdentity {
            TypeIdentity {
                type_id: TypeId::of::<Owner>(),
                rust_name: std::any::type_name::<Owner>(),
            }
        }

        fn value(&self) -> TypeIdentity {
            TypeIdentity {
                type_id: TypeId::of::<u32>(),
                rust_name: std::any::type_name::<u32>(),
            }
        }

        unsafe fn ref_ptr(&self, _owner: *const ()) -> Option<*const ()> {
            None
        }

        unsafe fn mut_ptr(&self, _owner: *mut ()) -> Option<*mut ()> {
            None
        }
    }

    static ADAPTER: MissingPointerAdapter = MissingPointerAdapter;
    static EXPRESSION: ExpressionData = ExpressionData::Never;
    static METADATA: FieldMetadata = FieldMetadata::new(
        3,
        "value",
        Visibility::Public,
        &EXPRESSION,
        &ADAPTER,
        None,
        None,
        FieldAccess::new(true, true, true, true),
    );

    fn descriptor() -> FieldDescriptor<'static> {
        FieldDescriptor::new(&METADATA, TypeDescriptor::of::<Owner>(), None)
    }

    #[test]
    fn typed_access_reports_unavailable_when_adapter_cannot_return_field_pointer() {
        let field = descriptor();
        let mut owner = Owner { value: 7 };

        assert!(matches!(
            field.get::<_, u32>(&owner),
            Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape
            ))
        ));
        assert!(matches!(
            field.get_mut::<_, u32>(&mut owner),
            Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape
            ))
        ));
        assert_eq!(owner.value, 7);

        let failure = field
            .set(&mut owner, 11_u32)
            .expect_err("replacement requires an available field pointer");
        assert!(matches!(
            failure.error(),
            FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape)
        ));
        assert_eq!(*failure.replacement(), 11);
        assert_eq!(owner.value, 7);

        // SAFETY: this adapter intentionally returns no pointer, so the access
        // reports an error without dereferencing the owner.
        assert!(matches!(
            unsafe { field.get_unsafe::<_, u32>(&owner) },
            Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape
            ))
        ));
        // SAFETY: this adapter intentionally returns no pointer, so the access
        // reports an error without dereferencing the owner.
        assert!(matches!(
            unsafe { field.get_mut_unsafe::<_, u32>(&mut owner) },
            Err(FieldAccessError::Unavailable(
                FieldAccessUnavailableReason::UnsupportedFieldShape
            ))
        ));
        // SAFETY: this adapter intentionally returns no pointer, so replacement
        // is rejected before the provided value can be written.
        let failure = unsafe { field.set_unsafe(&mut owner, 13_u32) }
            .expect_err("unsafe replacement also requires an available pointer");
        let (error, value) = failure.into_parts();
        assert!(matches!(
            error,
            FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape)
        ));
        assert_eq!(value, 13);
        assert_eq!(owner.value, 7);
    }
}
