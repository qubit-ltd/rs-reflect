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
    Restricted(
        /// Module path that bounds the restricted visibility.
        &'static str,
    ),
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
    ///
    /// # Parameters
    /// - `readable`: whether safe field reads are available.
    /// - `mutable`: whether safe mutable borrows are available.
    /// - `replaceable`: whether safe replacement is available.
    /// - `constructible`: whether construction may initialize the field.
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
    TargetTypeMismatch(
        /// Expected and actual owner type identities.
        TypeMismatch,
    ),
    /// The requested field value type does not match the declared field type.
    ValueTypeMismatch(
        /// Expected field value and supplied value type identities.
        TypeMismatch,
    ),
    /// The requested access operation has no usable adapter.
    Unavailable(
        /// Reason the requested adapter could not be used.
        FieldAccessUnavailableReason,
    ),
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
    /// Formats the access failure using its diagnostic representation.
    ///
    /// # Parameters
    /// - `formatter`: destination for the error text.
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
    TargetTypeMismatch(
        /// Expected and actual owner type identities.
        TypeMismatch,
    ),
    /// The replacement value type does not match the declared field type.
    ValueTypeMismatch(
        /// Expected field value and replacement value type identities.
        TypeMismatch,
    ),
    /// No replacement adapter is available.
    Unavailable(
        /// Reason the replacement adapter could not be used.
        FieldAccessUnavailableReason,
    ),
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
    /// Formats the replacement failure using its diagnostic representation.
    ///
    /// # Parameters
    /// - `formatter`: destination for the error text.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FieldSetError {}

/// A failed replacement that returns the original replacement value.
///
/// # Type Parameters
/// - `V`: replacement value type retained for recovery.
#[must_use]
pub struct FieldSetFailure<V> {
    /// Diagnostic reason the field assignment failed.
    error: FieldSetError,
    /// Value returned to the caller without being consumed.
    replacement: V,
}

impl<V> FieldSetFailure<V> {
    /// Creates a field replacement failure while retaining the rejected value.
    ///
    /// # Type Parameters
    /// - `V`: replacement value type retained for recovery.
    ///
    /// # Parameters
    /// - `error`: reason the replacement could not be applied.
    /// - `replacement`: original value returned to the caller.
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
    /// Diagnostic type associated with this recoverable failure.
    type Error = FieldSetError;
    /// Original replacement value retained after the failed assignment.
    type Recovery = V;

    /// Returns the field assignment error without consuming the failure.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Borrows the original replacement value for recovery.
    fn recovery(&self) -> &Self::Recovery {
        &self.replacement
    }

    /// Returns the error and replacement value as owned parts.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        (self.error, self.replacement)
    }
}

impl<V> fmt::Debug for FieldSetFailure<V> {
    /// Formats the error and the retained replacement value's type name.
    ///
    /// # Parameters
    /// - `formatter`: destination for the debug representation.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldSetFailure")
            .field("error", &self.error)
            .field("replacement", &std::any::type_name::<V>())
            .finish()
    }
}

/// Creates the runtime identity for a concrete Rust type.
///
/// # Type Parameters
/// - `T`: type whose identity is requested.
fn identity<T: 'static>() -> TypeIdentity {
    TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

/// Creates a descriptor for a reflected field value in the supplied context.
///
/// # Type Parameters
/// - `Value`: reflected value type of the field.
///
/// # Parameters
/// - `context`: context in which the value descriptor is bound.
fn concrete_type_of<'ctx, Value: crate::runtime::descriptor::Reflect>(
    context: &'ctx ReflectionContext,
) -> TypeDescriptor<'ctx> {
    TypeDescriptor::of_in::<Value>(context)
}

/// Static source facts for a field, consumed when binding a descriptor.
pub struct FieldMetadata {
    /// Zero-based source position of the field.
    pub(crate) index: usize,
    /// Source name or generated tuple-field name.
    pub(crate) name: &'static str,
    /// Source visibility retained for safe access checks.
    pub(crate) visibility: Visibility,
    /// Source expression describing the field's declared type.
    pub(crate) type_expression: &'static crate::runtime::expression::ExpressionData,
    /// Optional factory for the concrete reflected field type.
    pub(crate) concrete_type: Option<for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>>,
    /// Typed adapter for reading or writing the field in its owner.
    pub(crate) adapter: &'static dyn FieldAdapter,
    /// Variant metadata when the field belongs to an enum variant.
    pub(crate) variant: Option<&'static VariantMetadata>,
    /// Deferred metadata factory for a variant field.
    pub(crate) variant_source: Option<fn() -> &'static VariantMetadata>,
    /// Adapter that identifies the active enum variant for safe access.
    pub(crate) active_variant: Option<unsafe fn(*const ()) -> VariantIdentity>,
    /// Local-mode adapter for dynamic field operations, when generated.
    pub(crate) dynamic_local: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::Local>>,
    /// Thread-safe adapter for dynamic field operations, when generated.
    pub(crate) dynamic_thread_safe: Option<&'static dyn DynamicFieldAdapter<crate::runtime::value::ThreadSafe>>,
    /// Reason dynamic operations are unavailable when no adapter exists.
    pub(crate) dynamic_unavailable: FieldAccessUnavailableReason,
    /// Safe access operations supported by the source field.
    pub(crate) access: FieldAccess,
}

impl FieldMetadata {
    /// Creates metadata for a statically typed field adapter.
    ///
    /// # Type Parameters
    /// - `Owner`: Rust type containing the field.
    /// - `Value`: Rust type stored in the field.
    ///
    /// # Parameters
    /// - `index`: zero-based source position of the field.
    /// - `name`: source name or generated tuple-field name.
    /// - `visibility`: source visibility of the field.
    /// - `type_expression`: source expression for the field type.
    /// - `adapter`: typed owner/value access adapter.
    /// - `access`: safe operations available for the field.
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
            concrete_type: None,
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
    ///
    /// # Type Parameters
    /// - `Owner`: enum type containing the variant field.
    /// - `Value`: Rust type stored in the field.
    ///
    /// # Parameters
    /// - `index`: zero-based source position of the field.
    /// - `name`: source name or generated tuple-field name.
    /// - `visibility`: source visibility of the field.
    /// - `type_expression`: source expression for the field type.
    /// - `adapter`: typed owner/value access adapter.
    /// - `variant`: deferred metadata for the declaring variant.
    /// - `access`: safe operations available for the field.
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
            concrete_type: None,
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

    /// Associates a context-bound descriptor factory for a concrete reflected
    /// field type.
    ///
    /// # Type Parameters
    /// - `Value`: concrete reflected field type.
    #[doc(hidden)]
    pub const fn with_concrete_type<Value: crate::runtime::descriptor::Reflect>(mut self) -> Self {
        self.concrete_type = Some(concrete_type_of::<Value>);
        self
    }

    /// Enables local dynamic access when both field types support reflection.
    ///
    /// # Type Parameters
    /// - `Owner`: owner type containing the field.
    /// - `Value`: concrete type stored in the field.
    ///
    /// # Parameters
    /// - `adapter`: generated typed adapter used for local dynamic access.
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
    ///
    /// # Type Parameters
    /// - `Owner`: owner type containing the field.
    /// - `Value`: thread-safe concrete type stored in the field.
    ///
    /// # Parameters
    /// - `adapter`: generated typed adapter used for thread-safe access.
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
            + Sync
            + 'static,
    {
        self.dynamic_thread_safe = Some(adapter.thread_safe_dynamic());
        self
    }

    /// Creates field metadata with its typed adapter and source facts.
    ///
    /// # Parameters
    /// - `index`: zero-based source position of the field.
    /// - `name`: source name or generated tuple-field name.
    /// - `visibility`: source visibility of the field.
    /// - `type_expression`: source expression for the field type.
    /// - `adapter`: typed adapter for field access.
    /// - `variant`: optional declaring variant metadata.
    /// - `active_variant`: optional adapter that identifies the active variant.
    /// - `access`: safe operations available for the field.
    #[allow(clippy::too_many_arguments)]
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
            concrete_type: None,
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

    /// Resolves the field's declaring variant from stored or deferred metadata.
    pub(crate) fn variant_metadata(&self) -> Option<&'static VariantMetadata> {
        self.variant.or_else(|| self.variant_source.map(|source| source()))
    }

    /// Adds adapters for dynamically typed reads and writes.
    ///
    /// # Parameters
    /// - `local`: optional adapter supporting local dynamic operations.
    /// - `thread_safe`: optional adapter supporting thread-safe operations.
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
    ///
    /// # Parameters
    /// - `reason`: reason returned when no dynamic adapter is available.
    pub const fn with_dynamic_unavailable_reason(mut self, reason: FieldAccessUnavailableReason) -> Self {
        self.dynamic_unavailable = reason;
        self
    }
}

/// A context-bound view of one reflected field.
#[derive(Clone, Copy)]
pub struct FieldDescriptor<'ctx> {
    /// Static metadata that defines this field's source and access rules.
    pub(crate) metadata: &'static FieldMetadata,
    /// Reflected owner type that declares the field.
    declaring_type: TypeDescriptor<'ctx>,
    /// Declaring enum variant, when the field is variant-specific.
    declaring_variant: Option<VariantDescriptor<'ctx>>,
}

impl<'ctx> FieldDescriptor<'ctx> {
    /// Creates a field descriptor bound to its owner and optional variant.
    ///
    /// # Parameters
    /// - `metadata`: static source metadata for the field.
    /// - `declaring_type`: reflected owner type descriptor.
    /// - `declaring_variant`: declaring enum variant, when applicable.
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
        self.metadata
            .concrete_type
            .map(|factory| factory(self.context()))
            .or_else(|| {
                self.context()
                    .find_type_by_id(self.metadata.adapter.value().type_id)
                    .ok()
                    .flatten()
            })
    }

    /// Resolves the concrete field type or returns the context/lookup failure.
    pub fn concrete_type(self) -> Result<TypeDescriptor<'ctx>, crate::runtime::error::ReflectionError> {
        if let Some(factory) = self.metadata.concrete_type {
            return Ok(factory(self.context()));
        }
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
    ///
    /// # Type Parameters
    /// - `T`: owner type of the target value.
    /// - `V`: expected field value type.
    ///
    /// # Parameters
    /// - `target`: owner value whose field is read.
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
    ///
    /// # Type Parameters
    /// - `M`: dynamic value mode of the target.
    ///
    /// # Parameters
    /// - `target`: dynamically typed owner value to read.
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

    /// Reads a field through a dynamically typed value while bypassing source
    /// visibility restrictions.
    ///
    /// Owner type, active variant, field type, and adapter availability are
    /// still checked by the generated dynamic adapter.
    ///
    /// # Safety
    /// The caller must ensure that reading this field is authorized even when
    /// its source visibility is private or restricted.
    ///
    /// # Type Parameters
    /// - `M`: dynamic value mode of the target.
    ///
    /// # Parameters
    /// - `target`: dynamically typed owner value to read.
    pub unsafe fn get_dynamic_unsafe<'a, M: DynamicFieldMode>(
        self,
        target: DynamicRef<'a, M>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError> {
        if !self.metadata.access.readable {
            return Err(FieldAccessError::NotReadable);
        }
        M::get(self.metadata, target)
    }

    /// Mutably borrows a public field through a dynamically typed value.
    ///
    /// # Type Parameters
    /// - `M`: dynamic value mode of the target.
    ///
    /// # Parameters
    /// - `target`: dynamically typed owner value to borrow mutably.
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

    /// Mutably borrows a field through a dynamically typed value while
    /// bypassing its source visibility restriction.
    ///
    /// Owner type, active variant, field type, and adapter availability are
    /// still checked by the generated dynamic adapter.
    ///
    /// # Safety
    /// The caller must ensure that mutating this field is authorized even when
    /// its source visibility is private or restricted.
    ///
    /// # Type Parameters
    /// - `M`: dynamic value mode of the target.
    ///
    /// # Parameters
    /// - `target`: dynamically typed owner value to borrow mutably.
    pub unsafe fn get_dynamic_mut_unsafe<'a, M: DynamicFieldMode>(
        self,
        target: DynamicMut<'a, M>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError> {
        if !self.metadata.access.mutable {
            return Err(FieldAccessError::NotMutable);
        }
        M::get_mut(self.metadata, target)
    }

    /// Replaces a public field dynamically, retaining the replacement on
    /// failure.
    ///
    /// # Type Parameters
    /// - `M`: dynamic value mode of the target and replacement.
    ///
    /// # Parameters
    /// - `target`: dynamically typed owner value to update.
    /// - `value`: replacement value retained if the assignment fails.
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

    /// Replaces a field dynamically while bypassing its source visibility
    /// restriction, retaining the replacement on failure.
    ///
    /// Owner type, active variant, field type, and adapter availability are
    /// still checked by the generated dynamic adapter.
    ///
    /// # Safety
    /// The caller must ensure that replacing this field is authorized even
    /// when its source visibility is private or restricted.
    ///
    /// # Type Parameters
    /// - `M`: dynamic value mode of the target and replacement.
    ///
    /// # Parameters
    /// - `target`: dynamically typed owner value to update.
    /// - `value`: replacement value retained if the assignment fails.
    pub unsafe fn set_dynamic_unsafe<M: DynamicFieldMode>(
        self,
        target: DynamicMut<'_, M>,
        value: DynamicOwned<M>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>> {
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
    ///
    /// # Type Parameters
    /// - `T`: owner type of the target value.
    /// - `V`: expected field value type.
    ///
    /// # Parameters
    /// - `target`: owner value whose field is read.
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
    ///
    /// # Type Parameters
    /// - `T`: owner type of the target value.
    /// - `V`: expected field value type.
    ///
    /// # Parameters
    /// - `target`: owner value whose field is borrowed mutably.
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
    ///
    /// # Type Parameters
    /// - `T`: owner type of the target value.
    /// - `V`: replacement field value type.
    ///
    /// # Parameters
    /// - `target`: owner value whose field is replaced.
    /// - `value`: replacement returned if assignment fails.
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
    ///
    /// # Type Parameters
    /// - `T`: owner type of the target value.
    /// - `V`: expected field value type.
    ///
    /// # Parameters
    /// - `target`: owner value whose field is borrowed mutably.
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
    ///
    /// # Type Parameters
    /// - `T`: owner type of the target value.
    /// - `V`: replacement field value type.
    ///
    /// # Parameters
    /// - `target`: owner value whose field is replaced.
    /// - `value`: replacement returned if assignment fails.
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

    /// Checks that the target owner type matches this field's adapter.
    ///
    /// # Type Parameters
    /// - `T`: owner type of the supplied target.
    fn check_target<T: 'static>(self) -> Result<(), FieldAccessError> {
        let actual = identity::<T>();
        (actual.type_id == self.metadata.adapter.owner().type_id)
            .then_some(())
            .ok_or_else(|| {
                FieldAccessError::TargetTypeMismatch(TypeMismatch::new(self.metadata.adapter.owner(), actual))
            })
    }

    /// Verifies that this field's enum variant is active in `target`.
    ///
    /// # Parameters
    /// - `target`: pointer to the owner whose active variant is checked.
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
    use crate::runtime::value::DynamicMut;
    use crate::runtime::value::DynamicOwned;
    use crate::runtime::value::ThreadSafe;

    #[crate::Reflect]
    struct Owner {
        value: u32,
    }

    #[crate::Reflect]
    struct PrivateOwner {
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
    fn test_typed_access_reports_unavailable_when_adapter_cannot_return_field_pointer() {
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

    #[test]
    fn test_unsafe_dynamic_access_can_mutate_a_private_field() {
        let descriptor = TypeDescriptor::of::<PrivateOwner>();
        let field = descriptor.field("value").expect("private reflected field");
        let mut owner = PrivateOwner { value: 7 };

        // SAFETY: this unit test owns the value and explicitly verifies the
        // caller's responsibility for accessing a private field.
        let mut value = unsafe { field.get_dynamic_mut_unsafe(DynamicMut::<ThreadSafe>::new(&mut owner)) }
            .expect("mutable dynamic adapter is available");
        *value.downcast_mut::<u32>().expect("field value is u32") = 11;
        drop(value);
        assert_eq!(owner.value, 11);

        // SAFETY: this unit test owns the value and explicitly verifies the
        // caller's responsibility for replacing a private field.
        unsafe {
            field
                .set_dynamic_unsafe(
                    DynamicMut::<ThreadSafe>::new(&mut owner),
                    DynamicOwned::<ThreadSafe>::new(13_u32),
                )
                .expect("dynamic replacement adapter is available");
        }
        assert_eq!(owner.value, 13);
    }
}
