// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Immutable structural facts about reflected fields.

use std::any::Any;
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
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::identity::Visibility;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::Local;

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

mod example {
    use qubit_reflect::Reflect;
    #[Reflect]
    pub struct User { pub name: String }
}

let field = TypeDescriptor::of::<example::User>()
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

    /// Reads the field as `V` from an exact, publicly accessible target.
    ///
    /// # Parameters
    ///
    /// - `target`: Shared reference to the declaring value.
    ///
    /// # Returns
    ///
    /// A shared borrow of the field, tied to `target`.
    ///
    /// # Errors
    ///
    /// Returns an error if the target or field type differs, the field is not
    /// public, the adapter is unavailable, or its enum variant is inactive.
    pub fn get<'a, T: 'static, V: 'static>(&self, target: &'a T) -> Result<&'a V, FieldAccessError> {
        self.get_impl(target, false)
    }

    /// Reads the field while bypassing only its source visibility check.
    ///
    /// # Parameters
    ///
    /// - `target`: Shared reference to the declaring value.
    ///
    /// # Returns
    ///
    /// A shared borrow of the field, tied to `target`.
    ///
    /// # Safety
    ///
    /// The caller must ensure access is permitted by the program's privacy
    /// and encapsulation invariants.
    #[allow(unsafe_code)]
    pub unsafe fn get_unsafe<'a, T: 'static, V: 'static>(&self, target: &'a T) -> Result<&'a V, FieldAccessError> {
        self.get_impl(target, true)
    }

    fn get_impl<'a, T: 'static, V: 'static>(
        &self,
        target: &'a T,
        bypass_visibility: bool,
    ) -> Result<&'a V, FieldAccessError> {
        let target = DynamicRef::<Local>::new(target);
        self.validate_target_identity(self.declaring_type().type_id(), target.value_type_id())?;
        self.validate_policy(FieldAccessOperation::Get)?;
        self.validate_visibility(FieldAccessOperation::Get, bypass_visibility)?;
        if self.concrete_field_identity().is_none() {
            return Err(self.unavailable(FieldAccessOperation::Get));
        }
        let adapter = self.get.ok_or_else(|| self.unavailable(FieldAccessOperation::Get))?;
        adapter(target)?
            .downcast::<V>()
            .map_err(|_| self.field_type_mismatch::<V>())
    }

    /// Reads a field through the local dynamic borrow boundary.
    pub fn get_dynamic<'a>(&self, target: DynamicRef<'a, Local>) -> Result<DynamicRef<'a, Local>, FieldAccessError> {
        self.get_dynamic_impl(target, false)
    }

    /// Reads a field through the local dynamic borrow boundary while bypassing
    /// only source visibility.
    ///
    /// # Safety
    ///
    /// The caller must ensure access is permitted by the program's privacy
    /// and encapsulation invariants.
    #[allow(unsafe_code)]
    pub unsafe fn get_dynamic_unsafe<'a>(
        &self,
        target: DynamicRef<'a, Local>,
    ) -> Result<DynamicRef<'a, Local>, FieldAccessError> {
        self.get_dynamic_impl(target, true)
    }

    fn get_dynamic_impl<'a>(
        &self,
        target: DynamicRef<'a, Local>,
        bypass_visibility: bool,
    ) -> Result<DynamicRef<'a, Local>, FieldAccessError> {
        self.validate_target_identity(self.declaring_type().type_id(), target.value_type_id())?;
        self.validate_policy(FieldAccessOperation::Get)?;
        self.validate_visibility(FieldAccessOperation::Get, bypass_visibility)?;
        let (expected, expected_name) = self
            .concrete_field_identity()
            .ok_or_else(|| self.unavailable(FieldAccessOperation::Get))?;
        let adapter = self.get.ok_or_else(|| self.unavailable(FieldAccessOperation::Get))?;
        let value = adapter(target)?;
        let actual = value.value_type_id();
        if actual == expected {
            Ok(value)
        } else {
            Err(self.field_type_mismatch_for(expected, actual, expected_name))
        }
    }

    /// Mutably borrows the field as `V` from an exact, publicly accessible
    /// target.
    ///
    /// # Parameters
    ///
    /// - `target`: Exclusive reference to the declaring value.
    ///
    /// # Returns
    ///
    /// An exclusive borrow of the field, tied to `target`.
    ///
    /// # Errors
    ///
    /// Returns an error if the target or field type differs, the field is not
    /// public, mutation is unavailable, or its enum variant is inactive.
    pub fn get_mut<'a, T: 'static, V: 'static>(&self, target: &'a mut T) -> Result<&'a mut V, FieldAccessError> {
        self.get_mut_impl(target, false)
    }

    /// Mutably borrows the field while bypassing only source visibility.
    ///
    /// # Parameters
    ///
    /// - `target`: Exclusive reference to the declaring value.
    ///
    /// # Returns
    ///
    /// An exclusive borrow of the field, tied to `target`.
    ///
    /// # Safety
    ///
    /// The caller must ensure access is permitted by the program's privacy
    /// and encapsulation invariants.
    #[allow(unsafe_code)]
    pub unsafe fn get_mut_unsafe<'a, T: 'static, V: 'static>(
        &self,
        target: &'a mut T,
    ) -> Result<&'a mut V, FieldAccessError> {
        self.get_mut_impl(target, true)
    }

    fn get_mut_impl<'a, T: 'static, V: 'static>(
        &self,
        target: &'a mut T,
        bypass_visibility: bool,
    ) -> Result<&'a mut V, FieldAccessError> {
        let target = DynamicMut::<Local>::new(target);
        self.validate_target_identity(self.declaring_type().type_id(), target.value_type_id())?;
        self.validate_policy(FieldAccessOperation::GetMut)?;
        self.validate_visibility(FieldAccessOperation::GetMut, bypass_visibility)?;
        if self.concrete_field_identity().is_none() {
            return Err(self.unavailable(FieldAccessOperation::GetMut));
        }
        let adapter = self
            .get_mut
            .ok_or_else(|| self.unavailable(FieldAccessOperation::GetMut))?;
        adapter(target)?
            .downcast::<V>()
            .map_err(|_| self.field_type_mismatch::<V>())
    }

    /// Mutably borrows a field through the local dynamic borrow boundary.
    pub fn get_mut_dynamic<'a>(
        &self,
        target: DynamicMut<'a, Local>,
    ) -> Result<DynamicMut<'a, Local>, FieldAccessError> {
        self.get_mut_dynamic_impl(target, false)
    }

    /// Mutably borrows a field through the local dynamic borrow boundary while
    /// bypassing only source visibility.
    ///
    /// # Safety
    ///
    /// The caller must ensure access is permitted by the program's privacy
    /// and encapsulation invariants.
    #[allow(unsafe_code)]
    pub unsafe fn get_mut_dynamic_unsafe<'a>(
        &self,
        target: DynamicMut<'a, Local>,
    ) -> Result<DynamicMut<'a, Local>, FieldAccessError> {
        self.get_mut_dynamic_impl(target, true)
    }

    fn get_mut_dynamic_impl<'a>(
        &self,
        target: DynamicMut<'a, Local>,
        bypass_visibility: bool,
    ) -> Result<DynamicMut<'a, Local>, FieldAccessError> {
        self.validate_target_identity(self.declaring_type().type_id(), target.value_type_id())?;
        self.validate_policy(FieldAccessOperation::GetMut)?;
        self.validate_visibility(FieldAccessOperation::GetMut, bypass_visibility)?;
        let (expected, expected_name) = self
            .concrete_field_identity()
            .ok_or_else(|| self.unavailable(FieldAccessOperation::GetMut))?;
        let adapter = self
            .get_mut
            .ok_or_else(|| self.unavailable(FieldAccessOperation::GetMut))?;
        let value = adapter(target)?;
        let actual = value.value_type_id();
        if actual == expected {
            Ok(value)
        } else {
            Err(self.field_type_mismatch_for(expected, actual, expected_name))
        }
    }

    /// Replaces a public field with an ordinary value.
    ///
    /// On validation failure, the exact input value is retained in the
    /// returned failure.
    ///
    /// # Parameters
    ///
    /// - `target`: Exclusive reference to the declaring value.
    /// - `value`: Exact replacement value for the field.
    ///
    /// # Returns
    ///
    /// `Ok(())` after replacement, or an error retaining `value` when
    /// validation fails before adapter execution.
    pub fn set<T: 'static, V: 'static>(&self, target: &mut T, value: V) -> Result<(), FieldSetFailure<V>> {
        self.set_impl(target, value, false)
    }

    /// Replaces a field while bypassing only source visibility.
    ///
    /// # Parameters
    ///
    /// - `target`: Exclusive reference to the declaring value.
    /// - `value`: Exact replacement value for the field.
    ///
    /// # Returns
    ///
    /// `Ok(())` after replacement, or an error retaining `value` when
    /// validation fails before adapter execution.
    ///
    /// # Safety
    ///
    /// The caller must ensure mutation is permitted by the program's privacy
    /// and encapsulation invariants.
    #[allow(unsafe_code)]
    pub unsafe fn set_unsafe<T: 'static, V: 'static>(
        &self,
        target: &mut T,
        value: V,
    ) -> Result<(), FieldSetFailure<V>> {
        self.set_impl(target, value, true)
    }

    fn set_impl<T: 'static, V: 'static>(
        &self,
        target: &mut T,
        value: V,
        bypass_visibility: bool,
    ) -> Result<(), FieldSetFailure<V>> {
        let target = DynamicMut::<Local>::new(target);
        if let Err(error) = self.validate_target_identity(self.declaring_type().type_id(), target.value_type_id()) {
            return Err(self.set_failure(error, value));
        }
        if let Err(error) = self.validate_policy(FieldAccessOperation::Set) {
            return Err(self.set_failure(error, value));
        }
        if let Err(error) = self.validate_visibility(FieldAccessOperation::Set, bypass_visibility) {
            return Err(self.set_failure(error, value));
        }
        let Some((expected, expected_name)) = self.concrete_field_identity() else {
            return Err(self.set_failure(self.unavailable(FieldAccessOperation::Set), value));
        };
        let actual = field_value_type_id(&value, expected);
        if actual != expected {
            let error = FieldAccessError::ValueTypeMismatch {
                field: self.identity(),
                mismatch: Box::new(TypeMismatch::new(expected, actual).with_expected_name(expected_name)),
            };
            return Err(self.set_failure(error, value));
        }
        let Some(adapter) = self.set else {
            return Err(self.set_failure(self.unavailable(FieldAccessOperation::Set), value));
        };
        if let Some(preflight) = self.set_preflight
            && let Err(error) = preflight(&target)
        {
            return Err(self.set_failure(error, value));
        }
        adapter(target, erase_field_value(value, expected)).map_err(FieldSetFailure::after_execution)
    }

    /// Replaces a field through the local dynamic value boundary.
    pub fn set_dynamic(
        &self,
        target: DynamicMut<'_, Local>,
        value: DynamicOwned<Local>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<Local>>> {
        self.set_dynamic_impl(target, value, false)
    }

    /// Replaces a field through the local dynamic value boundary while
    /// bypassing only source visibility.
    ///
    /// # Safety
    ///
    /// The caller must ensure mutation is permitted by the program's privacy
    /// and encapsulation invariants.
    #[allow(unsafe_code)]
    pub unsafe fn set_dynamic_unsafe(
        &self,
        target: DynamicMut<'_, Local>,
        value: DynamicOwned<Local>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<Local>>> {
        self.set_dynamic_impl(target, value, true)
    }

    fn set_dynamic_impl(
        &self,
        target: DynamicMut<'_, Local>,
        value: DynamicOwned<Local>,
        bypass_visibility: bool,
    ) -> Result<(), FieldSetFailure<DynamicOwned<Local>>> {
        if let Err(error) = self.validate_target_identity(self.declaring_type().type_id(), target.value_type_id()) {
            return Err(self.set_failure(error, value));
        }
        if let Err(error) = self.validate_policy(FieldAccessOperation::Set) {
            return Err(self.set_failure(error, value));
        }
        if let Err(error) = self.validate_visibility(FieldAccessOperation::Set, bypass_visibility) {
            return Err(self.set_failure(error, value));
        }
        let Some((expected, expected_name)) = self.concrete_field_identity() else {
            return Err(self.set_failure(self.unavailable(FieldAccessOperation::Set), value));
        };
        let actual = dynamic_field_value_type_id(&value, expected);
        if actual != expected {
            let error = FieldAccessError::ValueTypeMismatch {
                field: self.identity(),
                mismatch: Box::new(TypeMismatch::new(expected, actual).with_expected_name(expected_name)),
            };
            return Err(self.set_failure(error, value));
        }
        let Some(adapter) = self.set else {
            return Err(self.set_failure(self.unavailable(FieldAccessOperation::Set), value));
        };
        if let Some(preflight) = self.set_preflight
            && let Err(error) = preflight(&target)
        {
            return Err(self.set_failure(error, value));
        }
        adapter(target, normalize_dynamic_field_value(value, expected)).map_err(FieldSetFailure::after_execution)
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

    fn validate_visibility(
        &self,
        operation: FieldAccessOperation,
        bypass_visibility: bool,
    ) -> Result<(), FieldAccessError> {
        if bypass_visibility || self.visibility == Visibility::Public {
            Ok(())
        } else {
            Err(FieldAccessError::NotPublic {
                field: self.identity(),
                operation,
            })
        }
    }

    fn field_type_mismatch<V: 'static>(&self) -> FieldAccessError {
        let (expected, expected_name) = self
            .concrete_field_identity()
            .expect("typed access requires a concrete field type");
        let actual = TypeId::of::<V>();
        self.field_type_mismatch_for(expected, actual, expected_name)
    }

    fn field_type_mismatch_for(
        &self,
        expected: TypeId,
        actual: TypeId,
        expected_name: &'static str,
    ) -> FieldAccessError {
        FieldAccessError::FieldTypeMismatch {
            field: self.identity(),
            mismatch: Box::new(TypeMismatch::new(expected, actual).with_expected_name(expected_name)),
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
            TypeRef::Resolved(descriptor) => Some((TypeDescriptor::type_id(descriptor), descriptor.type_name())),
            TypeRef::Opaque(descriptor) => Some((
                crate::descriptor::OpaqueTypeDescriptor::type_id(descriptor),
                descriptor.type_name(),
            )),
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
    fn set_failure<V>(&self, error: FieldAccessError, value: V) -> FieldSetFailure<V> {
        FieldSetFailure::before_execution(error, self.identity(), self.query_name, value)
    }
}

fn field_value_type_id<V: 'static>(value: &V, expected: TypeId) -> TypeId {
    let is_dynamic_box = TypeId::of::<V>() == TypeId::of::<Box<dyn Any>>();
    let expected_is_box = expected == TypeId::of::<Box<dyn Any>>();
    if is_dynamic_box && !expected_is_box {
        let value = value as &dyn Any;
        value
            .downcast_ref::<Box<dyn Any>>()
            .expect("the input type ID identifies Box<dyn Any>")
            .as_ref()
            .type_id()
    } else {
        TypeId::of::<V>()
    }
}

fn dynamic_field_value_type_id(value: &DynamicOwned<Local>, expected: TypeId) -> TypeId {
    let actual = value.value_type_id();
    if actual == TypeId::of::<Box<dyn Any>>() && expected != TypeId::of::<Box<dyn Any>>() {
        value
            .downcast_ref::<Box<dyn Any>>()
            .expect("the dynamic owned value has the boxed Any type")
            .as_ref()
            .type_id()
    } else {
        actual
    }
}

fn normalize_dynamic_field_value(value: DynamicOwned<Local>, expected: TypeId) -> DynamicOwned<Local> {
    if value.value_type_id() == TypeId::of::<Box<dyn Any>>() && expected != TypeId::of::<Box<dyn Any>>() {
        let boxed = value
            .downcast::<Box<dyn Any>>()
            .unwrap_or_else(|_| unreachable!("the dynamic boxed Any type was checked"));
        DynamicOwned::<Local>::from_box(boxed)
    } else {
        value
    }
}

fn erase_field_value<V: 'static>(value: V, expected: TypeId) -> DynamicOwned<Local> {
    let is_dynamic_box = TypeId::of::<V>() == TypeId::of::<Box<dyn Any>>();
    let expected_is_box = expected == TypeId::of::<Box<dyn Any>>();
    let erased: Box<dyn Any> = Box::new(value);
    if is_dynamic_box && !expected_is_box {
        let boxed = erased
            .downcast::<Box<dyn Any>>()
            .expect("the input type ID identifies Box<dyn Any>");
        DynamicOwned::<Local>::from_box(*boxed)
    } else {
        DynamicOwned::<Local>::from_box(erased)
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
