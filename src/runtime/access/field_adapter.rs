// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;
use std::any::TypeId;

use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::field::FieldAccessError;
use crate::runtime::descriptor::field::FieldAccessUnavailableReason;
use crate::runtime::descriptor::field::FieldMetadata;
use crate::runtime::descriptor::field::FieldSetError;
use crate::runtime::descriptor::field::FieldSetFailure;
use crate::runtime::descriptor::variant::VariantIdentity;
use crate::runtime::descriptor::variant::VariantMetadata;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::DynamicMut;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::DynamicRef;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueForMut;
use crate::runtime::value::ValueForRef;
use crate::runtime::value::ValueMode;

/// Concrete typed callbacks retained by reflected field metadata.
pub trait FieldAdapter: Sync {
    /// Returns the exact owner type handled by this adapter.
    fn owner(&self) -> TypeIdentity;
    /// Returns the exact field value type handled by this adapter.
    fn value(&self) -> TypeIdentity;

    /// # Safety
    /// `owner` must point to the exact owner type returned by `owner()`.
    unsafe fn ref_ptr(&self, owner: *const ()) -> Option<*const ()>;

    /// # Safety
    /// `owner` must be uniquely borrowed and point to the exact owner type.
    unsafe fn mut_ptr(&self, owner: *mut ()) -> Option<*mut ()>;

    /// # Safety
    /// `owner` must point to the exact owner type returned by `owner()`.
    unsafe fn active_variant(&self, _owner: *const ()) -> Option<VariantIdentity> {
        None
    }
}

/// Performs dynamic access to a concrete reflected field in value mode `M`.
pub trait DynamicFieldAdapter<M: ValueMode>: Sync {
    /// Reports whether shared dynamic field access is supported.
    fn can_get(&self) -> bool {
        true
    }

    /// Reports whether mutable dynamic field access is supported.
    fn can_get_mut(&self) -> bool {
        true
    }

    /// Reports whether dynamic field assignment is supported.
    fn can_set(&self) -> bool {
        true
    }

    /// Gets the field value after checking the target type and optional
    /// variant.
    fn get<'a>(
        &self,
        target: DynamicRef<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError>;

    /// Gets the field value mutably after checking the target type and variant.
    fn get_mut<'a>(
        &self,
        target: DynamicMut<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError>;

    /// Sets the field value, returning ownership of the rejected value on
    /// error.
    // The failure owns the input value so callers can recover it after a
    // rejected field update.
    #[allow(clippy::result_large_err)]
    fn set<'a>(
        &self,
        target: DynamicMut<'a, M>,
        value: DynamicOwned<M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>>;
}

/// Dispatches field operations to metadata's adapter for one value mode.
pub trait DynamicFieldMode: ValueMode + Sized {
    /// Gets a field using metadata's adapter for this value mode.
    fn get<'a>(
        metadata: &'static FieldMetadata,
        target: DynamicRef<'a, Self>,
    ) -> Result<DynamicRef<'a, Self>, FieldAccessError>;

    /// Gets a field mutably using metadata's adapter for this value mode.
    fn get_mut<'a>(
        metadata: &'static FieldMetadata,
        target: DynamicMut<'a, Self>,
    ) -> Result<DynamicMut<'a, Self>, FieldAccessError>;

    /// Sets a field using metadata's adapter, preserving the value on failure.
    // The failure owns the input value so callers can recover it after a
    // rejected field update.
    #[allow(clippy::result_large_err)]
    fn set<'a>(
        metadata: &'static FieldMetadata,
        target: DynamicMut<'a, Self>,
        value: DynamicOwned<Self>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<Self>>>;
}

macro_rules! field_mode {
    ($mode:ty, $member:ident) => {
        impl DynamicFieldMode for $mode {
            fn get<'a>(
                metadata: &'static FieldMetadata,
                target: DynamicRef<'a, Self>,
            ) -> Result<DynamicRef<'a, Self>, FieldAccessError> {
                let Some(adapter) = metadata.$member else {
                    return Err(FieldAccessError::Unavailable(metadata.dynamic_unavailable));
                };
                adapter.get(
                    target,
                    metadata.variant_metadata().map(|variant| variant.identity()),
                )
            }

            fn get_mut<'a>(
                metadata: &'static FieldMetadata,
                target: DynamicMut<'a, Self>,
            ) -> Result<DynamicMut<'a, Self>, FieldAccessError> {
                let Some(adapter) = metadata.$member else {
                    return Err(FieldAccessError::Unavailable(metadata.dynamic_unavailable));
                };
                adapter.get_mut(
                    target,
                    metadata.variant_metadata().map(|variant| variant.identity()),
                )
            }

            fn set<'a>(
                metadata: &'static FieldMetadata,
                target: DynamicMut<'a, Self>,
                value: DynamicOwned<Self>,
            ) -> Result<(), FieldSetFailure<DynamicOwned<Self>>> {
                let Some(adapter) = metadata.$member else {
                    return Err(FieldSetFailure::new(
                        FieldSetError::Unavailable(metadata.dynamic_unavailable),
                        value,
                    ));
                };
                adapter.set(
                    target,
                    value,
                    metadata.variant_metadata().map(|variant| variant.identity()),
                )
            }
        }
    };
}

field_mode!(Local, dynamic_local);
field_mode!(ThreadSafe, dynamic_thread_safe);

/// An adapter generated from field accessors for a concrete owner and value.
pub struct TypedFieldAdapter<Owner: 'static, Value: 'static> {
    get_ref: for<'a> fn(&'a Owner) -> &'a Value,
    get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    active_variant: Option<for<'a> fn(&'a Owner) -> VariantIdentity>,
    active_variant_metadata: Option<for<'a> fn(&'a Owner) -> &'static VariantMetadata>,
}

trait TypedDynamicRef<M: ValueMode> {
    fn new_any<'a>(value: &'a Self) -> DynamicRef<'a, M>;
}

impl<T: ValueForRef<Local> + Any> TypedDynamicRef<Local> for T {
    fn new_any<'a>(value: &'a Self) -> DynamicRef<'a, Local> {
        DynamicRef::<Local>::new_any(value)
    }
}

impl<T: ValueForRef<ThreadSafe> + Any + Sync> TypedDynamicRef<ThreadSafe> for T {
    fn new_any<'a>(value: &'a Self) -> DynamicRef<'a, ThreadSafe> {
        DynamicRef::<ThreadSafe>::new_any(value)
    }
}

/// A dynamic only adapter for an unsized owner or field value.
#[cfg(test)]
pub(crate) struct DynamicOnlyFieldAdapter<Owner: ?Sized + 'static, Value: ?Sized + 'static> {
    get_ref: for<'a> fn(&'a Owner) -> &'a Value,
    get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    active_variant: Option<for<'a> fn(&'a Owner) -> VariantIdentity>,
    active_variant_metadata: Option<for<'a> fn(&'a Owner) -> &'static VariantMetadata>,
}

#[cfg(test)]
impl<Owner: ?Sized + 'static, Value: ?Sized + 'static> DynamicOnlyFieldAdapter<Owner, Value> {
    pub const fn new(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: None,
            active_variant_metadata: None,
        }
    }

    fn check_variant(
        &self,
        owner: &Owner,
        expected: Option<VariantIdentity>,
    ) -> Result<(), (VariantIdentity, VariantIdentity)> {
        let Some(expected) = expected else {
            return Ok(());
        };
        if let Some(active) = self.active_variant {
            let actual = active(owner);
            return (actual == expected).then_some(()).ok_or((expected, actual));
        }
        if let Some(active) = self.active_variant_metadata {
            let actual = active(owner).identity();
            return (actual == expected).then_some(()).ok_or((expected, actual));
        }
        Err((expected, expected))
    }
}

#[cfg(test)]
impl<Owner: ?Sized + 'static, Value: ?Sized + 'static> FieldAdapter for DynamicOnlyFieldAdapter<Owner, Value> {
    fn owner(&self) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<Owner>(),
            rust_name: std::any::type_name::<Owner>(),
        }
    }

    fn value(&self) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<Value>(),
            rust_name: std::any::type_name::<Value>(),
        }
    }

    unsafe fn ref_ptr(&self, _owner: *const ()) -> Option<*const ()> {
        None
    }

    unsafe fn mut_ptr(&self, _owner: *mut ()) -> Option<*mut ()> {
        None
    }
}

#[cfg(test)]
impl<Owner, Value, M> DynamicFieldAdapter<M> for DynamicOnlyFieldAdapter<Owner, Value>
where
    Owner: Reflect + ValueForRef<M> + ValueForMut<M> + ?Sized + 'static,
    Value: Reflect + ValueForRef<M> + ValueForMut<M> + ?Sized + 'static,
    M: ValueMode,
{
    fn can_set(&self) -> bool {
        false
    }

    fn get<'a>(
        &self,
        target: DynamicRef<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError> {
        let owner = target
            .downcast::<Owner>()
            .map_err(|failure| FieldAccessError::TargetTypeMismatch(failure.into_parts().0))?;
        if let Err((expected, actual)) = self.check_variant(owner, expected_variant) {
            return Err(FieldAccessError::InactiveVariant { expected, actual });
        }
        Ok(DynamicRef::new((self.get_ref)(owner)))
    }

    fn get_mut<'a>(
        &self,
        target: DynamicMut<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError> {
        let owner = target
            .downcast::<Owner>()
            .map_err(|failure| FieldAccessError::TargetTypeMismatch(failure.into_parts().0))?;
        if let Err((expected, actual)) = self.check_variant(owner, expected_variant) {
            return Err(FieldAccessError::InactiveVariant { expected, actual });
        }
        Ok(DynamicMut::new((self.get_mut)(owner)))
    }

    fn set<'a>(
        &self,
        _target: DynamicMut<'a, M>,
        value: DynamicOwned<M>,
        _expected_variant: Option<VariantIdentity>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>> {
        Err(FieldSetFailure::new(
            FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape),
            value,
        ))
    }
}

impl<Owner: 'static, Value: 'static> TypedFieldAdapter<Owner, Value> {
    /// Creates an adapter from typed shared and mutable field accessors.
    pub const fn new(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: None,
            active_variant_metadata: None,
        }
    }

    /// Creates an enum field adapter from a safe source variant selector.
    pub const fn for_variant(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
        active_variant: for<'a> fn(&'a Owner) -> VariantIdentity,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: Some(active_variant),
            active_variant_metadata: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn with_variant(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
        active_variant: for<'a> fn(&'a Owner) -> VariantIdentity,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: Some(active_variant),
            active_variant_metadata: None,
        }
    }
}

impl<Owner: 'static, Value: 'static> TypedFieldAdapter<Owner, Value> {
    fn active_variant(&self, owner: &Owner) -> Option<VariantIdentity> {
        self.active_variant
            .map(|active| active(owner))
            .or_else(|| self.active_variant_metadata.map(|active| active(owner).identity()))
    }
}

#[allow(private_bounds)]
impl<Owner, Value, M> DynamicFieldAdapter<M> for TypedFieldAdapter<Owner, Value>
where
    Owner: Reflect + ValueForRef<M> + ValueForMut<M> + 'static,
    Value: Reflect + ValueForRef<M> + ValueForMut<M> + ValueFor<M> + TypedDynamicRef<M> + 'static,
    M: ValueMode,
{
    fn get<'a>(
        &self,
        target: DynamicRef<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError> {
        let owner = target
            .downcast::<Owner>()
            .map_err(|failure| FieldAccessError::TargetTypeMismatch(failure.into_parts().0))?;
        if let Some(expected) = expected_variant {
            let Some(actual) = self.active_variant(owner) else {
                return Err(FieldAccessError::Unavailable(
                    FieldAccessUnavailableReason::AdapterUnavailable,
                ));
            };
            if actual != expected {
                return Err(FieldAccessError::InactiveVariant { expected, actual });
            }
        }
        Ok(<Value as TypedDynamicRef<M>>::new_any((self.get_ref)(owner)))
    }

    fn get_mut<'a>(
        &self,
        target: DynamicMut<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError> {
        let owner = target
            .downcast::<Owner>()
            .map_err(|failure| FieldAccessError::TargetTypeMismatch(failure.into_parts().0))?;
        if let Some(expected) = expected_variant {
            let Some(actual) = self.active_variant(owner) else {
                return Err(FieldAccessError::Unavailable(
                    FieldAccessUnavailableReason::AdapterUnavailable,
                ));
            };
            if actual != expected {
                return Err(FieldAccessError::InactiveVariant { expected, actual });
            }
        }
        Ok(DynamicMut::new((self.get_mut)(owner)))
    }

    fn set<'a>(
        &self,
        target: DynamicMut<'a, M>,
        value: DynamicOwned<M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>> {
        let owner = match target.downcast::<Owner>() {
            Ok(owner) => owner,
            Err(failure) => {
                let (mismatch, _) = failure.into_parts();
                return Err(FieldSetFailure::new(FieldSetError::TargetTypeMismatch(mismatch), value));
            }
        };
        if let Some(expected) = expected_variant {
            let Some(actual) = self.active_variant(owner) else {
                return Err(FieldSetFailure::new(
                    FieldSetError::Unavailable(FieldAccessUnavailableReason::AdapterUnavailable),
                    value,
                ));
            };
            if actual != expected {
                return Err(FieldSetFailure::new(
                    FieldSetError::InactiveVariant { expected, actual },
                    value,
                ));
            }
        }
        let replacement = match value.downcast::<Value>() {
            Ok(replacement) => replacement,
            Err(failure) => {
                let (mismatch, value) = failure.into_parts();
                return Err(FieldSetFailure::new(FieldSetError::ValueTypeMismatch(mismatch), value));
            }
        };
        let field = (self.get_mut)(owner);
        drop(std::mem::replace(field, replacement));
        Ok(())
    }
}

impl<Owner, Value> TypedFieldAdapter<Owner, Value>
where
    Owner: Reflect + ValueForRef<Local> + ValueForMut<Local> + 'static,
    Value: Reflect + ValueForRef<Local> + ValueForMut<Local> + ValueFor<Local> + 'static,
{
    pub(crate) const fn local_dynamic(&'static self) -> &'static dyn DynamicFieldAdapter<Local> {
        self
    }
}

impl<Owner, Value> TypedFieldAdapter<Owner, Value>
where
    Owner: Reflect + ValueForRef<ThreadSafe> + ValueForMut<ThreadSafe> + 'static,
    Value: Reflect + ValueForRef<ThreadSafe> + ValueForMut<ThreadSafe> + ValueFor<ThreadSafe> + Sync + 'static,
{
    pub(crate) const fn thread_safe_dynamic(&'static self) -> &'static dyn DynamicFieldAdapter<ThreadSafe> {
        self
    }
}

impl<Owner: 'static, Value: 'static> FieldAdapter for TypedFieldAdapter<Owner, Value> {
    fn owner(&self) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<Owner>(),
            rust_name: std::any::type_name::<Owner>(),
        }
    }

    fn value(&self) -> TypeIdentity {
        TypeIdentity {
            type_id: TypeId::of::<Value>(),
            rust_name: std::any::type_name::<Value>(),
        }
    }

    unsafe fn ref_ptr(&self, owner: *const ()) -> Option<*const ()> {
        // SAFETY: the descriptor checks the owner TypeId before this call.
        let owner = unsafe { &*owner.cast::<Owner>() };
        Some((self.get_ref)(owner) as *const Value as *const ())
    }

    unsafe fn mut_ptr(&self, owner: *mut ()) -> Option<*mut ()> {
        // SAFETY: the descriptor checks the owner TypeId and has exclusive access.
        let owner = unsafe { &mut *owner.cast::<Owner>() };
        Some((self.get_mut)(owner) as *mut Value as *mut ())
    }

    unsafe fn active_variant(&self, owner: *const ()) -> Option<VariantIdentity> {
        // SAFETY: required by this method's contract.
        let owner = unsafe { &*owner.cast::<Owner>() };
        self.active_variant(owner)
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::DynamicFieldAdapter;
    use super::DynamicOnlyFieldAdapter;
    use super::FieldAdapter;
    use super::TypedFieldAdapter;
    use crate::runtime::descriptor::Reflect;
    use crate::runtime::descriptor::field::FieldAccessUnavailableReason;
    use crate::runtime::descriptor::field::FieldMetadata;
    use crate::runtime::descriptor::field::FieldSetError;
    use crate::runtime::descriptor::variant::VariantIdentity;
    use crate::runtime::descriptor::variant::VariantMetadata;
    use crate::runtime::value::DynamicMut;
    use crate::runtime::value::DynamicOwned;
    use crate::runtime::value::DynamicRef;
    use crate::runtime::value::Local;

    fn identity_variant(_: &u32) -> VariantIdentity {
        VariantIdentity {
            owner: TypeId::of::<u32>(),
            index: 0,
            name: "Value",
        }
    }

    fn identity_ref(value: &u32) -> &u32 {
        value
    }

    fn identity_mut(value: &mut u32) -> &mut u32 {
        value
    }

    static DYNAMIC_ONLY: DynamicOnlyFieldAdapter<u32, u32> = DynamicOnlyFieldAdapter::new(identity_ref, identity_mut);

    #[test]
    fn dynamic_only_adapter_checks_variants_and_preserves_unsupported_set_value() {
        let expected = identity_variant(&0);
        assert_eq!(DYNAMIC_ONLY.check_variant(&7, None), Ok(()));
        assert_eq!(
            DYNAMIC_ONLY.check_variant(&7, Some(expected)),
            Err((expected, expected))
        );

        let selected = DynamicOnlyFieldAdapter {
            get_ref: identity_ref,
            get_mut: identity_mut,
            active_variant: Some(identity_variant),
            active_variant_metadata: None,
        };
        assert_eq!(selected.check_variant(&7, Some(expected)), Ok(()));
        let other = VariantIdentity {
            index: 1,
            name: "Other",
            ..expected
        };
        assert_eq!(selected.check_variant(&7, Some(other)), Err((other, expected)));

        let value = 11_u32;
        let result = DYNAMIC_ONLY.get(DynamicRef::<Local>::new(&value), None).unwrap();
        assert_eq!(*result.downcast_ref::<u32>().unwrap(), 11);

        let mut value = 12_u32;
        let result = DYNAMIC_ONLY
            .get_mut(DynamicMut::<Local>::new(&mut value), None)
            .unwrap();
        *result.downcast::<u32>().unwrap() = 13;
        assert_eq!(value, 13);

        let mut owner = 14_u32;
        let failure = DYNAMIC_ONLY
            .set(
                DynamicMut::<Local>::new(&mut owner),
                DynamicOwned::<Local>::new(15_u32),
                None,
            )
            .unwrap_err();
        let (error, replacement) = failure.into_parts();
        assert_eq!(
            error,
            FieldSetError::Unavailable(FieldAccessUnavailableReason::UnsupportedFieldShape)
        );
        assert_eq!(replacement.downcast::<u32>().unwrap(), 15);
        assert_eq!(owner, 14);
    }

    struct PointerOwner {
        value: u32,
    }

    impl Reflect for PointerOwner {}

    fn pointer_ref(owner: &PointerOwner) -> &u32 {
        &owner.value
    }

    fn pointer_mut(owner: &mut PointerOwner) -> &mut u32 {
        &mut owner.value
    }

    static POINTER_ADAPTER: TypedFieldAdapter<PointerOwner, u32> = TypedFieldAdapter::new(pointer_ref, pointer_mut);

    static POINTER_VARIANT_ADAPTER: crate::runtime::descriptor::variant::TypedVariantAdapter<PointerOwner> =
        crate::runtime::descriptor::variant::TypedVariantAdapter::new(|_| true);
    static NO_FIELDS: [FieldMetadata; 0] = [];
    static POINTER_VARIANT: VariantMetadata = VariantMetadata::new(
        0,
        "Active",
        crate::runtime::descriptor::variant::VariantShape::Unit,
        crate::runtime::descriptor::field::Visibility::Public,
        crate::runtime::descriptor::variant::VariantDiscriminant::new(
            crate::runtime::descriptor::variant::DiscriminantOrigin::Implicit,
            None,
        ),
        &NO_FIELDS,
        &POINTER_VARIANT_ADAPTER,
    );

    fn pointer_variant_metadata(_: &PointerOwner) -> &'static VariantMetadata {
        &POINTER_VARIANT
    }

    fn pointer_variant_identity(_: &PointerOwner) -> VariantIdentity {
        POINTER_VARIANT.identity()
    }

    #[test]
    fn typed_adapter_reports_identities_and_returns_field_pointers() {
        let owner_identity = POINTER_ADAPTER.owner();
        assert_eq!(owner_identity.type_id, TypeId::of::<PointerOwner>());
        assert_eq!(POINTER_ADAPTER.value().type_id, TypeId::of::<u32>());

        let variant_adapter = TypedFieldAdapter::for_variant(pointer_ref, pointer_mut, pointer_variant_identity);

        let mut owner = PointerOwner { value: 17 };
        // SAFETY: both pointers refer to the exact owner type expected by this adapter.
        let shared = unsafe { POINTER_ADAPTER.ref_ptr((&owner as *const PointerOwner).cast()).unwrap() };
        // SAFETY: the mutable pointer is unique and refers to the exact owner type.
        let mutable = unsafe {
            POINTER_ADAPTER
                .mut_ptr((&mut owner as *mut PointerOwner).cast())
                .unwrap()
        };
        assert_eq!(unsafe { *shared.cast::<u32>() }, 17);
        unsafe { *mutable.cast::<u32>() = 18 };
        assert_eq!(owner.value, 18);
        assert_eq!(variant_adapter.active_variant(&owner), Some(POINTER_VARIANT.identity()));
        // SAFETY: the pointer still refers to the exact owner type.
        assert_eq!(
            unsafe { FieldAdapter::active_variant(&POINTER_ADAPTER, (&owner as *const PointerOwner).cast(),) },
            None
        );

        let expected = POINTER_VARIANT.identity();
        let metadata_adapter = TypedFieldAdapter {
            get_ref: pointer_ref,
            get_mut: pointer_mut,
            active_variant: None,
            active_variant_metadata: Some(pointer_variant_metadata),
        };
        assert_eq!(metadata_adapter.active_variant(&owner), Some(expected));
        // SAFETY: pointer refers to the exact owner type.
        assert_eq!(
            unsafe { FieldAdapter::active_variant(&metadata_adapter, (&owner as *const PointerOwner).cast(),) },
            Some(expected)
        );

        let dynamic_metadata_adapter = DynamicOnlyFieldAdapter {
            get_ref: pointer_ref,
            get_mut: pointer_mut,
            active_variant: None,
            active_variant_metadata: Some(pointer_variant_metadata),
        };
        assert_eq!(dynamic_metadata_adapter.check_variant(&owner, Some(expected)), Ok(()));
    }
}
