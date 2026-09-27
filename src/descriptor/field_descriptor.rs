// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Immutable structural facts about reflected fields.

use std::any::TypeId;
use std::fmt;

use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::access::FieldAccessError;
use crate::access::FieldAccessOperation;
use crate::access::FieldAccessPolicy;
use crate::access::FieldGetAdapter;
use crate::access::FieldGetMutAdapter;
use crate::access::FieldIdentity;
use crate::access::FieldSetAdapter;
use crate::access::FieldSetFailure;
use crate::access::FieldSetPreflightAdapter;
use crate::access::FieldVisibility;
use crate::access::ThreadSafeFieldGetAdapter;
use crate::access::ThreadSafeFieldGetMutAdapter;
use crate::access::ThreadSafeFieldSetAdapter;
use crate::access::ThreadSafeFieldSetPreflightAdapter;
use crate::access::field_adapter::dynamic_mut_type_id;
use crate::access::field_adapter::dynamic_owned_type_id;
use crate::access::field_adapter::dynamic_ref_type_id;
use crate::access::field_adapter::thread_safe_mut_type_id;
use crate::access::field_adapter::thread_safe_owned_type_id;
use crate::access::field_adapter::thread_safe_ref_type_id;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::identity::Visibility;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::ReflectedMut;
use crate::value::ReflectedOwned;
use crate::value::ReflectedRef;
use crate::value::ThreadSafe;

/// A function that resolves a field or variant's declaring root descriptor.
///
/// Resolver indirection permits immutable static descriptor graphs to contain
/// cycles.
pub type TypeDescriptorResolver = fn() -> &'static TypeDescriptor;

/// The immutable structural description of one reflected field.
#[cfg_attr(
    feature = "derive",
    doc = r#"
# Examples

```
#![allow(proc_macro_derive_resolution_fallback)]
use qubit_reflect::Reflect;
use qubit_reflect::TypeDescriptor;

#[derive(Reflect)]
#[reflect(crate = qubit_reflect)]
struct User {
    name: String,
}

let field = TypeDescriptor::of::<User>()
    .field("name")
    .expect("the derived field exists");
assert_eq!(field.rust_name(), Some("name"));
```
"#
)]
pub struct FieldDescriptor {
    /// Resolver for the exact root type that owns this field.
    declaring_type: TypeDescriptorResolver,
    /// Zero-based source position in the declaring type or variant.
    index: usize,
    /// Source Rust identifier, absent for positional fields.
    rust_name: Option<&'static str>,
    /// Reflected lookup identifier, absent for positional fields.
    query_name: Option<&'static str>,
    /// Eager or lazy source for the field's reflected type.
    field_type: TypeRefSource,
    /// Declared source visibility retained for structural inspection.
    visibility: Visibility,
    /// Containing enum variant index, absent for struct fields.
    variant_index: Option<usize>,
    /// Containing enum variant's Rust name, absent for struct fields.
    variant_rust_name: Option<&'static str>,
    /// Policy checked before any dynamic field operation.
    access_policy: FieldAccessPolicy,
    /// Generated local shared-borrow adapter, when supported.
    get: Option<FieldGetAdapter>,
    /// Generated local exclusive-borrow adapter, when supported.
    get_mut: Option<FieldGetMutAdapter>,
    /// Generated local replacement adapter, when supported.
    set: Option<FieldSetAdapter>,
    /// Optional local validation hook run before replacement consumes input.
    set_preflight: Option<FieldSetPreflightAdapter>,
    /// Generated thread-safe shared-borrow adapter, when supported.
    thread_safe_get: Option<ThreadSafeFieldGetAdapter>,
    /// Generated thread-safe exclusive-borrow adapter, when supported.
    thread_safe_get_mut: Option<ThreadSafeFieldGetMutAdapter>,
    /// Generated thread-safe replacement adapter, when supported.
    thread_safe_set: Option<ThreadSafeFieldSetAdapter>,
    /// Optional thread-safe hook run before replacement consumes input.
    thread_safe_set_preflight: Option<ThreadSafeFieldSetPreflightAdapter>,
}

impl FieldDescriptor {
    /// Creates a frozen field descriptor for generated or hand-written
    /// descriptor data.
    ///
    /// The resolver must return the root containing this field. Positional
    /// fields use `None` for both names. This constructor performs no
    /// allocation and never calls the resolver.
    ///
    /// # Parameters
    ///
    /// - `declaring_type`: Resolver for the root type that owns the field.
    /// - `index`: Zero-based position in the source declaration.
    /// - `rust_name`: Source identifier, or `None` for positional fields.
    /// - `query_name`: Lookup identifier, or `None` for positional fields.
    /// - `field_type`: The eager resolved, opaque, or symbolic field type.
    /// - `visibility`: The declared source visibility.
    ///
    /// # Returns
    ///
    /// Returns a structural descriptor with default read-write policy and no
    /// runtime access adapters.
    #[doc(hidden)]
    #[must_use]
    pub(crate) const fn new(
        declaring_type: TypeDescriptorResolver,
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
        field_type: &'static TypeRef,
        visibility: Visibility,
    ) -> Self {
        Self {
            declaring_type,
            index,
            rust_name,
            query_name,
            field_type: TypeRefSource::Eager(field_type),
            visibility,
            variant_index: None,
            variant_rust_name: None,
            access_policy: FieldAccessPolicy::ReadWrite,
            get: None,
            get_mut: None,
            set: None,
            set_preflight: None,
            thread_safe_get: None,
            thread_safe_get_mut: None,
            thread_safe_set: None,
            thread_safe_set_preflight: None,
        }
    }

    /// Creates a field whose resolved type is deferred until navigation.
    ///
    /// Generated descriptors use this constructor so recursive type graphs do
    /// not re-enter a root descriptor while its member list is being built.
    ///
    /// # Parameters
    ///
    /// - `declaring_type`: Resolver for the root type that owns the field.
    /// - `index`: Zero-based position in the source declaration.
    /// - `rust_name`: Source identifier, or `None` for positional fields.
    /// - `query_name`: Lookup identifier, or `None` for positional fields.
    /// - `field_type`: Lazy source for the resolved field type.
    /// - `visibility`: The declared source visibility.
    ///
    /// # Returns
    ///
    /// Returns a structural descriptor that resolves its type on navigation.
    #[doc(hidden)]
    #[must_use]
    pub(crate) const fn new_lazy(
        declaring_type: TypeDescriptorResolver,
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
        field_type: &'static LazyTypeRef,
        visibility: Visibility,
    ) -> Self {
        Self {
            declaring_type,
            index,
            rust_name,
            query_name,
            field_type: TypeRefSource::Lazy(field_type),
            visibility,
            variant_index: None,
            variant_rust_name: None,
            access_policy: FieldAccessPolicy::ReadWrite,
            get: None,
            get_mut: None,
            set: None,
            set_preflight: None,
            thread_safe_get: None,
            thread_safe_get_mut: None,
            thread_safe_set: None,
            thread_safe_set_preflight: None,
        }
    }

    /// Attaches safe generated adapters and their source access policy.
    ///
    /// Descriptors without adapters remain useful for structural navigation;
    /// their dynamic operations return [`FieldAccessError::Unavailable`]. The
    /// policy is enforced before any adapter is invoked.
    ///
    /// # Parameters
    ///
    /// - `access_policy`: Source policy checked before dynamic dispatch.
    /// - `get`: Optional local shared-borrow adapter.
    /// - `get_mut`: Optional local exclusive-borrow adapter.
    /// - `set`: Optional local replacement adapter.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with the local operation contract attached.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_access(
        mut self,
        access_policy: FieldAccessPolicy,
        get: Option<FieldGetAdapter>,
        get_mut: Option<FieldGetMutAdapter>,
        set: Option<FieldSetAdapter>,
    ) -> Self {
        self.access_policy = access_policy;
        self.get = get;
        self.get_mut = get_mut;
        self.set = set;
        self
    }

    /// Attaches a non-consuming validation hook that runs immediately before
    /// the set adapter.
    ///
    /// Generated enum fields use this hook to validate the active variant
    /// while the replacement remains recoverable by the descriptor.
    ///
    /// # Parameters
    ///
    /// - `set_preflight`: Optional non-consuming preflight adapter.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with the local preflight hook attached.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_set_preflight(mut self, set_preflight: Option<FieldSetPreflightAdapter>) -> Self {
        self.set_preflight = set_preflight;
        self
    }

    /// Attaches mode-preserving thread-safe field adapters.
    ///
    /// # Parameters
    ///
    /// - `get`: Optional thread-safe shared-borrow adapter.
    /// - `get_mut`: Optional thread-safe exclusive-borrow adapter.
    /// - `set`: Optional thread-safe replacement adapter.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with its thread-safe operations attached.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_thread_safe_access(
        mut self,
        get: Option<ThreadSafeFieldGetAdapter>,
        get_mut: Option<ThreadSafeFieldGetMutAdapter>,
        set: Option<ThreadSafeFieldSetAdapter>,
    ) -> Self {
        self.thread_safe_get = get;
        self.thread_safe_get_mut = get_mut;
        self.thread_safe_set = set;
        self
    }

    /// Attaches a thread-safe non-consuming set preflight hook.
    ///
    /// # Parameters
    ///
    /// - `set_preflight`: Optional non-consuming thread-safe preflight adapter.
    ///
    /// # Returns
    ///
    /// Returns this descriptor with the thread-safe preflight hook attached.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_thread_safe_set_preflight(
        mut self,
        set_preflight: Option<ThreadSafeFieldSetPreflightAdapter>,
    ) -> Self {
        self.thread_safe_set_preflight = set_preflight;
        self
    }

    /// Marks this field as belonging to one enum variant.
    ///
    /// The variant source index and Rust name become part of the field's
    /// runtime identity, so fields at equal positions in different variants
    /// remain distinct.
    ///
    /// # Parameters
    ///
    /// - `variant_index`: Zero-based source position of the enum variant.
    /// - `variant_rust_name`: Source identifier of the enum variant.
    ///
    /// # Returns
    ///
    /// Returns this descriptor associated with the supplied variant.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn with_variant(mut self, variant_index: usize, variant_rust_name: &'static str) -> Self {
        self.variant_index = Some(variant_index);
        self.variant_rust_name = Some(variant_rust_name);
        self
    }

    /// Returns the root type that contains this field.
    ///
    /// # Returns
    ///
    /// Returns the exact declaring root descriptor.
    #[must_use]
    #[inline]
    pub fn declaring_type(&self) -> &'static TypeDescriptor {
        (self.declaring_type)()
    }

    /// Returns the zero-based source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the field's source position in its struct or variant.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust field name, or `None` for tuple and newtype fields.
    ///
    /// # Returns
    ///
    /// Returns the source identifier, or `None` for positional fields.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the lookup name, or `None` for tuple and newtype fields.
    ///
    /// # Returns
    ///
    /// Returns the reflected lookup identifier, or `None` for positional
    /// fields.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the resolved, explicitly opaque, or symbolic field type.
    ///
    /// # Returns
    ///
    /// Returns the cached field type classification.
    #[must_use]
    pub fn field_type(&self) -> &'static TypeRef {
        self.field_type.get()
    }

    /// Returns declared struct-field visibility or the explicit fact that an
    /// enum-variant field inherits its enclosing access boundary.
    ///
    /// # Returns
    ///
    /// Returns declared visibility for structs or the inherited marker for
    /// enum-variant fields.
    #[must_use]
    #[inline]
    pub const fn visibility(&self) -> FieldVisibility<'_> {
        match self.variant_index {
            Some(_) => FieldVisibility::VariantInherited,
            None => FieldVisibility::Declared(&self.visibility),
        }
    }

    /// Returns the source policy controlling this field's dynamic adapters.
    ///
    /// # Returns
    ///
    /// Returns the policy checked before any operation adapter is invoked.
    #[must_use]
    #[inline]
    pub const fn access_policy(&self) -> FieldAccessPolicy {
        self.access_policy
    }

    /// Returns the containing variant's source index for an enum field.
    ///
    /// `None` identifies a direct struct field.
    ///
    /// # Returns
    ///
    /// Returns the containing variant index, or `None` for a struct field.
    #[must_use]
    #[inline]
    pub const fn variant_index(&self) -> Option<usize> {
        self.variant_index
    }

    /// Returns the containing variant's Rust name for an enum field.
    ///
    /// `None` identifies a direct struct field.
    ///
    /// # Returns
    ///
    /// Returns the containing variant name, or `None` for a struct field.
    #[must_use]
    #[inline]
    pub const fn variant_rust_name(&self) -> Option<&'static str> {
        self.variant_rust_name
    }

    /// Reads this field from an exact declaring-type target.
    ///
    /// Returns a target mismatch, policy, or unavailable-adapter error before
    /// invoking generated code. An enum-field adapter may additionally report
    /// [`FieldAccessError::InactiveVariant`]. The returned borrow cannot
    /// outlive `target`.
    ///
    /// # Parameters
    ///
    /// - `target`: The shared dynamic borrow of the exact declaring type.
    ///
    /// # Returns
    ///
    /// Returns a shared dynamic borrow of the field.
    ///
    /// # Errors
    ///
    /// Returns [`FieldAccessError`] if the target has the wrong type, access
    /// is disabled, no adapter is available, or the containing enum variant is
    /// inactive.
    pub fn get<'a>(&self, target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
        self.validate_shared_target(&target)?;
        self.validate_policy(FieldAccessOperation::Get)?;
        let adapter = self.get.ok_or_else(|| self.unavailable(FieldAccessOperation::Get))?;
        adapter(target)
    }

    /// Mutably borrows this field from an exact declaring-type target.
    ///
    /// Returns a target mismatch, read-only/skipped policy, or unavailable-
    /// adapter error before invoking generated code. The returned exclusive
    /// borrow cannot outlive `target`.
    ///
    /// # Parameters
    ///
    /// - `target`: The exclusive dynamic borrow of the exact declaring type.
    ///
    /// # Returns
    ///
    /// Returns an exclusive dynamic borrow of the field.
    ///
    /// # Errors
    ///
    /// Returns [`FieldAccessError`] if the target has the wrong type, the
    /// policy forbids mutation, no mutable adapter is available, or the
    /// containing enum variant is inactive.
    pub fn get_mut<'a>(&self, target: ReflectedMut<'a>) -> Result<ReflectedMut<'a>, FieldAccessError> {
        self.validate_mutable_target(&target)?;
        self.validate_policy(FieldAccessOperation::GetMut)?;
        let adapter = self
            .get_mut
            .ok_or_else(|| self.unavailable(FieldAccessOperation::GetMut))?;
        adapter(target)
    }

    /// Replaces this field using an exact declaring-type target and exact
    /// declared field value.
    ///
    /// All target, policy, value, and adapter-availability checks complete
    /// before generated code is invoked. These failures do not modify the
    /// target and return the untouched replacement through
    /// [`FieldSetFailure::recovery`]. A symbolic definition-level field has no
    /// exact runtime identity and therefore reports
    /// [`FieldAccessError::Unavailable`].
    ///
    /// # Parameters
    ///
    /// - `target`: The exclusive dynamic borrow of the exact declaring type.
    /// - `value`: The owned replacement with the field's expected type.
    ///
    /// # Returns
    ///
    /// Returns `()` after the adapter replaces the field.
    ///
    /// # Errors
    ///
    /// Returns a [`FieldSetFailure`] containing the error and, when validation
    /// failed before adapter execution, the original replacement value.
    pub fn set(&self, target: ReflectedMut<'_>, value: ReflectedOwned) -> Result<(), FieldSetFailure> {
        if let Err(error) = self.validate_mutable_target(&target) {
            return Err(self.set_failure(error, value));
        }
        if let Err(error) = self.validate_policy(FieldAccessOperation::Set) {
            return Err(self.set_failure(error, value));
        }
        let (expected, _) = match self.concrete_field_identity() {
            Some(identity) => identity,
            None => {
                return Err(self.set_failure(self.unavailable(FieldAccessOperation::Set), value));
            }
        };
        let actual = dynamic_owned_type_id(&value);
        if actual != expected {
            let error = FieldAccessError::ValueTypeMismatch {
                field: self.identity(),
                mismatch: Box::new(
                    TypeMismatch::new(expected, actual)
                        .with_expected_name(self.concrete_field_identity().expect("checked above").1),
                ),
            };
            return Err(self.set_failure(error, value));
        }
        let adapter = match self.set {
            Some(adapter) => adapter,
            None => {
                return Err(self.set_failure(self.unavailable(FieldAccessOperation::Set), value));
            }
        };
        if let Some(preflight) = self.set_preflight
            && let Err(error) = preflight(&target)
        {
            return Err(self.set_failure(error, value));
        }
        adapter(target, value).map_err(FieldSetFailure::after_execution)
    }

    /// Reads this field while preserving a thread-safe erased boundary.
    ///
    /// # Parameters
    ///
    /// - `target`: A thread-safe shared borrow of the exact declaring type.
    ///
    /// # Returns
    ///
    /// Returns a thread-safe shared borrow of the field.
    ///
    /// # Errors
    ///
    /// Returns an access error for a target mismatch, disallowed policy,
    /// unavailable adapter, or inactive enum variant.
    pub fn get_thread_safe<'a>(
        &self,
        target: DynamicRef<'a, ThreadSafe>,
    ) -> Result<DynamicRef<'a, ThreadSafe>, FieldAccessError> {
        self.validate_target_identity(self.declaring_type().type_id(), thread_safe_ref_type_id(&target))?;
        self.validate_policy(FieldAccessOperation::Get)?;
        self.thread_safe_get
            .ok_or_else(|| self.unavailable(FieldAccessOperation::Get))?(target)
    }

    /// Mutably reads this field while preserving a thread-safe erased boundary.
    ///
    /// # Parameters
    ///
    /// - `target`: A thread-safe exclusive borrow of the exact declaring type.
    ///
    /// # Returns
    ///
    /// Returns a thread-safe exclusive borrow of the field.
    ///
    /// # Errors
    ///
    /// Returns an access error for a target mismatch, disallowed policy,
    /// unavailable adapter, or inactive enum variant.
    pub fn get_mut_thread_safe<'a>(
        &self,
        target: DynamicMut<'a, ThreadSafe>,
    ) -> Result<DynamicMut<'a, ThreadSafe>, FieldAccessError> {
        self.validate_target_identity(self.declaring_type().type_id(), thread_safe_mut_type_id(&target))?;
        self.validate_policy(FieldAccessOperation::GetMut)?;
        self.thread_safe_get_mut
            .ok_or_else(|| self.unavailable(FieldAccessOperation::GetMut))?(target)
    }

    /// Replaces this field through thread-safe dynamic values.
    ///
    /// # Parameters
    ///
    /// - `target`: A thread-safe exclusive borrow of the exact declaring type.
    /// - `value`: The owned thread-safe replacement value.
    ///
    /// # Returns
    ///
    /// Returns `()` after replacement succeeds.
    ///
    /// # Errors
    ///
    /// Returns the structured error and retains the replacement when validation
    /// fails before adapter execution.
    pub fn set_thread_safe(
        &self,
        target: DynamicMut<'_, ThreadSafe>,
        value: DynamicOwned<ThreadSafe>,
    ) -> Result<(), FieldSetFailure<ThreadSafe>> {
        let field = self.identity();
        let failure = |error, value| FieldSetFailure::before_execution(error, field.clone(), self.query_name, value);
        if let Err(error) =
            self.validate_target_identity(self.declaring_type().type_id(), thread_safe_mut_type_id(&target))
        {
            return Err(failure(error, value));
        }
        if let Err(error) = self.validate_policy(FieldAccessOperation::Set) {
            return Err(failure(error, value));
        }
        let Some((expected, expected_name)) = self.concrete_field_identity() else {
            return Err(failure(self.unavailable(FieldAccessOperation::Set), value));
        };
        let actual = thread_safe_owned_type_id(&value);
        if actual != expected {
            return Err(failure(
                FieldAccessError::ValueTypeMismatch {
                    field: field.clone(),
                    mismatch: Box::new(TypeMismatch::new(expected, actual).with_expected_name(expected_name)),
                },
                value,
            ));
        }
        let Some(adapter) = self.thread_safe_set else {
            return Err(failure(self.unavailable(FieldAccessOperation::Set), value));
        };
        if let Some(preflight) = self.thread_safe_set_preflight
            && let Err(error) = preflight(&target)
        {
            return Err(failure(error, value));
        }
        adapter(target, value).map_err(FieldSetFailure::after_execution)
    }

    /// Validates a shared target without consuming or changing it.
    ///
    /// # Parameters
    ///
    /// - `target`: The shared borrow whose exact root type is checked.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the target matches this field's declaring type.
    ///
    /// # Errors
    ///
    /// Returns a target-type mismatch associated with this field.
    fn validate_shared_target(&self, target: &ReflectedRef<'_>) -> Result<(), FieldAccessError> {
        let expected = self.declaring_type().type_id();
        let actual = dynamic_ref_type_id(target);
        self.validate_target_identity(expected, actual)
    }

    /// Validates a mutable target without consuming or changing it.
    ///
    /// # Parameters
    ///
    /// - `target`: The mutable borrow whose exact root type is checked.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the target matches this field's declaring type.
    ///
    /// # Errors
    ///
    /// Returns a target-type mismatch associated with this field.
    fn validate_mutable_target(&self, target: &ReflectedMut<'_>) -> Result<(), FieldAccessError> {
        let expected = self.declaring_type().type_id();
        let actual = dynamic_mut_type_id(target);
        self.validate_target_identity(expected, actual)
    }

    /// Compares exact target identities and attaches this field's identity to
    /// a mismatch.
    ///
    /// # Parameters
    ///
    /// - `expected`: The declaring root's exact process-local type identity.
    /// - `actual`: The supplied target's exact process-local type identity.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the identities match.
    ///
    /// # Errors
    ///
    /// Returns a mismatch carrying this field's source identity otherwise.
    fn validate_target_identity(&self, expected: TypeId, actual: TypeId) -> Result<(), FieldAccessError> {
        if actual == expected {
            Ok(())
        } else {
            Err(FieldAccessError::TargetTypeMismatch {
                field: self.identity(),
                mismatch: Box::new(
                    TypeMismatch::new(expected, actual).with_expected_name(self.declaring_type().type_name()),
                ),
            })
        }
    }

    /// Enforces skip and read-only policy before dispatching an adapter.
    ///
    /// # Parameters
    ///
    /// - `operation`: The requested read or mutation operation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the field policy allows the operation.
    ///
    /// # Errors
    ///
    /// Returns a skipped or read-only error when policy forbids the operation.
    fn validate_policy(&self, operation: FieldAccessOperation) -> Result<(), FieldAccessError> {
        match (self.access_policy, operation) {
            (FieldAccessPolicy::Skipped, _) => Err(FieldAccessError::Skipped {
                field: self.identity(),
                operation,
            }),
            (FieldAccessPolicy::ReadOnly, FieldAccessOperation::GetMut | FieldAccessOperation::Set) => {
                Err(FieldAccessError::ReadOnly {
                    field: self.identity(),
                    operation,
                })
            }
            (FieldAccessPolicy::ReadWrite | FieldAccessPolicy::ReadOnly, _) => Ok(()),
        }
    }

    /// Returns the exact runtime identity and diagnostic name for a concrete
    /// resolved or explicitly opaque field.
    ///
    /// # Returns
    ///
    /// Returns the process-local type ID and diagnostic name, or `None` for a
    /// symbolic field type.
    fn concrete_field_identity(&self) -> Option<(TypeId, &'static str)> {
        match self.field_type() {
            TypeRef::Resolved(descriptor) => Some((descriptor.type_id(), descriptor.type_name())),
            TypeRef::Opaque(descriptor) => Some((descriptor.type_id(), descriptor.type_name())),
            TypeRef::Symbolic(_) => None,
        }
    }

    /// Builds the source identity included in every field access error.
    ///
    /// # Returns
    ///
    /// Returns this field's stable source identity, including variant context.
    ///
    /// # Panics
    ///
    /// Panics if variant index and variant name metadata are inconsistent.
    fn identity(&self) -> FieldIdentity {
        let declaring_type = self.declaring_type();
        match (self.variant_index, self.variant_rust_name) {
            (Some(variant_index), Some(variant_rust_name)) => FieldIdentity::new_variant_with_query_name(
                declaring_type.type_id(),
                declaring_type.type_name(),
                self.index,
                self.rust_name,
                self.query_name,
                variant_index,
                variant_rust_name,
            ),
            (None, None) => FieldIdentity::new_with_query_name(
                declaring_type.type_id(),
                declaring_type.type_name(),
                self.index,
                self.rust_name,
                self.query_name,
            ),
            _ => unreachable!("variant field identity is assigned atomically"),
        }
    }

    /// Builds an unavailable-adapter error for `operation`.
    ///
    /// # Parameters
    ///
    /// - `operation`: The operation for which no adapter is available.
    ///
    /// # Returns
    ///
    /// Returns an unavailable-adapter error associated with this field.
    fn unavailable(&self, operation: FieldAccessOperation) -> FieldAccessError {
        FieldAccessError::Unavailable {
            field: self.identity(),
            operation,
        }
    }

    /// Pairs a pre-execution set error with the untouched replacement value.
    ///
    /// # Parameters
    ///
    /// - `error`: The validation or dispatch error.
    /// - `value`: The original replacement value retained for recovery.
    ///
    /// # Returns
    ///
    /// Returns a failure that owns the error and replacement recovery.
    fn set_failure(&self, error: FieldAccessError, value: ReflectedOwned) -> FieldSetFailure {
        FieldSetFailure::before_execution(error, self.identity(), self.query_name, value)
    }
}

impl fmt::Debug for FieldDescriptor {
    /// Formats local facts without following declaring or field-type
    /// relationships recursively.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the local metadata.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldDescriptor")
            .field("declaring_type", &"<resolver>")
            .field("index", &self.index)
            .field("rust_name", &self.rust_name)
            .field("query_name", &self.query_name)
            .field("field_type", &self.field_type)
            .field("visibility", &self.visibility)
            .field("variant_index", &self.variant_index)
            .field("variant_rust_name", &self.variant_rust_name)
            .field("access_policy", &self.access_policy)
            .field("has_get", &self.get.is_some())
            .field("has_get_mut", &self.get_mut.is_some())
            .field("has_set", &self.set.is_some())
            .field("has_set_preflight", &self.set_preflight.is_some())
            .field("has_thread_safe_get", &self.thread_safe_get.is_some())
            .field("has_thread_safe_get_mut", &self.thread_safe_get_mut.is_some())
            .field("has_thread_safe_set", &self.thread_safe_set.is_some())
            .finish()
    }
}
