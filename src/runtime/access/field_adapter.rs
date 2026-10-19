use std::any::TypeId;

use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::field::{
    FieldAccessError, FieldAccessUnavailableReason, FieldMetadata, FieldSetError, FieldSetFailure,
};
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::{
    DynamicMut, DynamicOwned, DynamicRef, Local, ThreadSafe, ValueFor, ValueForMut, ValueForRef,
    ValueMode,
};

use crate::runtime::descriptor::variant::VariantIdentity;

/// Concrete typed callbacks retained by reflected field metadata.
pub(crate) trait FieldAdapter: Sync {
    fn owner(&self) -> TypeIdentity;
    fn value(&self) -> TypeIdentity;

    /// # Safety
    /// `owner` must point to the exact owner type returned by `owner()`.
    unsafe fn ref_ptr(&self, owner: *const ()) -> Option<*const ()>;

    /// # Safety
    /// `owner` must be uniquely borrowed and point to the exact owner type.
    unsafe fn mut_ptr(&self, owner: *mut ()) -> Option<*mut ()>;
}

pub(crate) trait DynamicFieldAdapter<M: ValueMode>: Sync {
    fn can_get(&self) -> bool {
        true
    }

    fn can_get_mut(&self) -> bool {
        true
    }

    fn can_set(&self) -> bool {
        true
    }

    fn get<'a>(
        &self,
        target: DynamicRef<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicRef<'a, M>, FieldAccessError>;

    fn get_mut<'a>(
        &self,
        target: DynamicMut<'a, M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<DynamicMut<'a, M>, FieldAccessError>;

    fn set<'a>(
        &self,
        target: DynamicMut<'a, M>,
        value: DynamicOwned<M>,
        expected_variant: Option<VariantIdentity>,
    ) -> Result<(), FieldSetFailure<DynamicOwned<M>>>;
}

pub(crate) trait DynamicFieldMode: ValueMode + Sized {
    fn get<'a>(
        metadata: &'static FieldMetadata,
        target: DynamicRef<'a, Self>,
    ) -> Result<DynamicRef<'a, Self>, FieldAccessError>;

    fn get_mut<'a>(
        metadata: &'static FieldMetadata,
        target: DynamicMut<'a, Self>,
    ) -> Result<DynamicMut<'a, Self>, FieldAccessError>;

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
                adapter.get(target, metadata.variant.map(|variant| variant.identity()))
            }

            fn get_mut<'a>(
                metadata: &'static FieldMetadata,
                target: DynamicMut<'a, Self>,
            ) -> Result<DynamicMut<'a, Self>, FieldAccessError> {
                let Some(adapter) = metadata.$member else {
                    return Err(FieldAccessError::Unavailable(metadata.dynamic_unavailable));
                };
                adapter.get_mut(target, metadata.variant.map(|variant| variant.identity()))
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
                    metadata.variant.map(|variant| variant.identity()),
                )
            }
        }
    };
}

field_mode!(Local, dynamic_local);
field_mode!(ThreadSafe, dynamic_thread_safe);

/// An adapter generated from field accessors for a concrete owner and value.
pub(crate) struct TypedFieldAdapter<Owner: 'static, Value: 'static> {
    get_ref: for<'a> fn(&'a Owner) -> &'a Value,
    get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    active_variant: Option<for<'a> fn(&'a Owner) -> VariantIdentity>,
}

/// A dynamic only adapter for an unsized owner or field value.
pub(crate) struct DynamicOnlyFieldAdapter<Owner: ?Sized + 'static, Value: ?Sized + 'static> {
    get_ref: for<'a> fn(&'a Owner) -> &'a Value,
    get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    active_variant: Option<for<'a> fn(&'a Owner) -> VariantIdentity>,
}

impl<Owner: ?Sized + 'static, Value: ?Sized + 'static> DynamicOnlyFieldAdapter<Owner, Value> {
    pub(crate) const fn new(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: None,
        }
    }

    pub(crate) const fn with_variant(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
        active_variant: for<'a> fn(&'a Owner) -> VariantIdentity,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: Some(active_variant),
        }
    }
}

impl<Owner: ?Sized + 'static, Value: ?Sized + 'static> FieldAdapter
    for DynamicOnlyFieldAdapter<Owner, Value>
{
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
        if let Some(expected) = expected_variant {
            let Some(active) = self.active_variant else {
                return Err(FieldAccessError::Unavailable(
                    FieldAccessUnavailableReason::AdapterUnavailable,
                ));
            };
            let actual = active(owner);
            if actual != expected {
                return Err(FieldAccessError::InactiveVariant { expected, actual });
            }
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
        if let Some(expected) = expected_variant {
            let Some(active) = self.active_variant else {
                return Err(FieldAccessError::Unavailable(
                    FieldAccessUnavailableReason::AdapterUnavailable,
                ));
            };
            let actual = active(owner);
            if actual != expected {
                return Err(FieldAccessError::InactiveVariant { expected, actual });
            }
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
    pub(crate) const fn new(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: None,
        }
    }

    pub(crate) const fn with_variant(
        get_ref: for<'a> fn(&'a Owner) -> &'a Value,
        get_mut: for<'a> fn(&'a mut Owner) -> &'a mut Value,
        active_variant: for<'a> fn(&'a Owner) -> VariantIdentity,
    ) -> Self {
        Self {
            get_ref,
            get_mut,
            active_variant: Some(active_variant),
        }
    }
}

impl<Owner, Value, M> DynamicFieldAdapter<M> for TypedFieldAdapter<Owner, Value>
where
    Owner: Reflect + ValueForRef<M> + ValueForMut<M> + 'static,
    Value: Reflect + ValueForRef<M> + ValueForMut<M> + ValueFor<M> + 'static,
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
            let Some(active) = self.active_variant else {
                return Err(FieldAccessError::Unavailable(
                    FieldAccessUnavailableReason::AdapterUnavailable,
                ));
            };
            let actual = active(owner);
            if actual != expected {
                return Err(FieldAccessError::InactiveVariant { expected, actual });
            }
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
        if let Some(expected) = expected_variant {
            let Some(active) = self.active_variant else {
                return Err(FieldAccessError::Unavailable(
                    FieldAccessUnavailableReason::AdapterUnavailable,
                ));
            };
            let actual = active(owner);
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
                return Err(FieldSetFailure::new(
                    FieldSetError::TargetTypeMismatch(mismatch),
                    value,
                ));
            }
        };
        if let Some(expected) = expected_variant {
            let Some(active) = self.active_variant else {
                return Err(FieldSetFailure::new(
                    FieldSetError::Unavailable(FieldAccessUnavailableReason::AdapterUnavailable),
                    value,
                ));
            };
            let actual = active(owner);
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
                return Err(FieldSetFailure::new(
                    FieldSetError::ValueTypeMismatch(mismatch),
                    value,
                ));
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
    Value: Reflect
        + ValueForRef<ThreadSafe>
        + ValueForMut<ThreadSafe>
        + ValueFor<ThreadSafe>
        + 'static,
{
    pub(crate) const fn thread_safe_dynamic(
        &'static self,
    ) -> &'static dyn DynamicFieldAdapter<ThreadSafe> {
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
}
