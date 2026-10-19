use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) mod sealed {
    pub trait Sealed {}
}

pub(crate) trait ErasedRef {
    fn type_id(&self) -> TypeId;
    fn holder_ptr(&self) -> *const ();
    fn is_mutable(&self) -> bool;
}

pub(crate) trait ErasedMut {
    fn type_id(&self) -> TypeId;
    fn holder_ptr(&self) -> *const ();
    fn holder_ptr_mut(&mut self) -> *mut ();
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_>;
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b>;
}

pub(crate) trait ErasedMutThreadSafe: ErasedMut + Send + Sync {
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_>;
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b>;
}

pub(crate) trait ErasedOwned {
    fn type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn shared_local(&self) -> Rc<dyn ErasedRef + '_>;
    fn mutable_local(&mut self) -> Box<dyn ErasedMut + '_>;
}

pub(crate) trait ErasedOwnedThreadSafe: ErasedOwned + Send + Sync {
    fn shared_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_>;
    fn mutable_thread_safe(&mut self) -> Box<dyn ErasedMutThreadSafe + '_>;
}

pub(crate) trait ValueMode: sealed::Sealed + 'static {
    type Owned: 'static;
    type Marker;
    type Shared<'a>: Clone;
    type Mutable<'a>;

    fn owned_type_id(value: &Self::Owned) -> TypeId;
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId;
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId;
    fn owned_into_any(value: Self::Owned) -> Box<dyn Any>;
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a>;
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b;
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a>;
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b>;
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b>;
}

pub(crate) enum Local {}
pub(crate) enum ThreadSafe {}

pub(crate) struct LocalOwned(pub(super) Box<dyn ErasedOwned>);
pub(crate) struct ThreadSafeOwned(pub(super) Box<dyn ErasedOwnedThreadSafe>);

impl sealed::Sealed for Local {}
impl sealed::Sealed for ThreadSafe {}

impl ValueMode for Local {
    type Owned = LocalOwned;
    type Marker = Rc<()>;
    type Shared<'a> = Rc<dyn ErasedRef + 'a>;
    type Mutable<'a> = Box<dyn ErasedMut + 'a>;

    fn owned_type_id(value: &Self::Owned) -> TypeId {
        ErasedOwned::type_id(value.0.as_ref())
    }
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId {
        ErasedRef::type_id(value.as_ref())
    }
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId {
        ErasedMut::type_id(value.as_ref())
    }
    fn owned_into_any(value: Self::Owned) -> Box<dyn Any> {
        value.0.into_any()
    }
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a> {
        value.0.shared_local()
    }
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b,
    {
        let view: Rc<dyn ErasedRef + 'b> = value.clone();
        view
    }
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a> {
        value.0.mutable_local()
    }
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b> {
        value.shared_view_local()
    }
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b> {
        value.reborrow()
    }
}

impl ValueMode for ThreadSafe {
    type Owned = ThreadSafeOwned;
    type Marker = ();
    type Shared<'a> = Arc<dyn ErasedRef + Send + Sync + 'a>;
    type Mutable<'a> = Box<dyn ErasedMutThreadSafe + 'a>;

    fn owned_type_id(value: &Self::Owned) -> TypeId {
        ErasedOwned::type_id(value.0.as_ref())
    }
    fn shared_type_id(value: &Self::Shared<'_>) -> TypeId {
        ErasedRef::type_id(value.as_ref())
    }
    fn mutable_type_id(value: &Self::Mutable<'_>) -> TypeId {
        ErasedMut::type_id(value.as_ref())
    }
    fn owned_into_any(value: Self::Owned) -> Box<dyn Any> {
        let ThreadSafeOwned(value) = value;
        let value: Box<dyn ErasedOwned> = value;
        value.into_any()
    }
    fn shared_from_owned<'a>(value: &'a Self::Owned) -> Self::Shared<'a> {
        value.0.shared_thread_safe()
    }
    fn shared_reborrow<'a, 'b>(value: &'b Self::Shared<'a>) -> Self::Shared<'b>
    where
        'a: 'b,
    {
        let view: Arc<dyn ErasedRef + Send + Sync + 'b> = value.clone();
        view
    }
    fn mutable_from_owned<'a>(value: &'a mut Self::Owned) -> Self::Mutable<'a> {
        value.0.mutable_thread_safe()
    }
    fn shared_from_mut<'a, 'b>(value: &'b Self::Mutable<'a>) -> Self::Shared<'b> {
        value.shared_view_thread_safe()
    }
    fn reborrow_mut<'a, 'b>(value: &'b mut Self::Mutable<'a>) -> Self::Mutable<'b> {
        value.reborrow_thread_safe()
    }
}

struct MutRefView<'a>(&'a (dyn ErasedMut + Send + Sync));
impl ErasedRef for MutRefView<'_> {
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    fn is_mutable(&self) -> bool {
        true
    }
}

pub(super) fn thread_safe_ref_into_local(
    value: Arc<dyn ErasedRef + Send + Sync + '_>,
) -> Rc<dyn ErasedRef + '_> {
    Rc::new(ArcRefView(value))
}

struct ArcRefView<'a>(Arc<dyn ErasedRef + Send + Sync + 'a>);
impl ErasedRef for ArcRefView<'_> {
    fn type_id(&self) -> TypeId {
        ErasedRef::type_id(self.0.as_ref())
    }
    fn holder_ptr(&self) -> *const () {
        self.0.holder_ptr()
    }
    fn is_mutable(&self) -> bool {
        self.0.is_mutable()
    }
}
