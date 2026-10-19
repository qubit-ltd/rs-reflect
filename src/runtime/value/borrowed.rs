use std::any::TypeId;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::{Reflect, TypeDescriptor};
use crate::runtime::identity::TypeIdentity;

use super::failure::DowncastFailure;
use super::mode::{
    ErasedMut, ErasedMutThreadSafe, ErasedRef, Local, ThreadSafe, ValueMode,
    thread_safe_ref_into_local,
};

type DescriptorFactory = for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>;

fn descriptor_for<'ctx, T: Reflect + ?Sized>(
    context: &'ctx ReflectionContext,
) -> TypeDescriptor<'ctx> {
    TypeDescriptor::of_in::<T>(context)
}

struct RefHolder<'a, T: Reflect + ?Sized>(&'a T);
impl<T: Reflect + ?Sized> ErasedRef for RefHolder<'_, T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn holder_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    fn is_mutable(&self) -> bool {
        false
    }
}

pub(super) fn erase_ref_local<'a, T: Reflect + ?Sized>(
    value: &'a T,
) -> std::rc::Rc<dyn ErasedRef + 'a> {
    std::rc::Rc::new(RefHolder(value))
}

pub(super) fn erase_ref_thread_safe<'a, T: Reflect + Sync + ?Sized>(
    value: &'a T,
) -> std::sync::Arc<dyn ErasedRef + Send + Sync + 'a> {
    std::sync::Arc::new(RefHolder(value))
}

struct MutHolder<'a, T: Reflect + ?Sized>(Option<&'a mut T>);

impl<'a, T: Reflect + ?Sized> ErasedMut for MutHolder<'a, T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn holder_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    fn holder_ptr_mut(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }
    fn shared_view_local(&self) -> std::rc::Rc<dyn ErasedRef + '_> {
        std::rc::Rc::new(MutRefViewLocal(self))
    }
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b> {
        Box::new(BorrowedMutViewLocal(self))
    }
}

impl<T: Reflect + Send + Sync + ?Sized> ErasedMutThreadSafe for MutHolder<'_, T> {
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_> {
        Arc::new(MutRefViewThreadSafe(self))
    }
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b> {
        Box::new(BorrowedMutViewThreadSafe(self))
    }
}

pub(super) fn erase_mut_local<'a, T: Reflect + ?Sized>(
    value: &'a mut T,
) -> Box<dyn ErasedMut + 'a> {
    Box::new(MutHolder(Some(value)))
}

pub(super) fn erase_mut_thread_safe<'a, T: Reflect + Send + Sync + ?Sized>(
    value: &'a mut T,
) -> Box<dyn ErasedMutThreadSafe + 'a> {
    Box::new(MutHolder(Some(value)))
}

impl<T: Reflect + ?Sized> ErasedRef for MutHolder<'_, T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn holder_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    fn is_mutable(&self) -> bool {
        true
    }
}

struct MutRefViewLocal<'a>(&'a dyn ErasedMut);
impl ErasedRef for MutRefViewLocal<'_> {
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

struct MutRefViewThreadSafe<'a>(&'a (dyn ErasedMut + Send + Sync));
impl ErasedRef for MutRefViewThreadSafe<'_> {
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

struct BorrowedMutViewLocal<'a>(&'a mut dyn ErasedMut);
impl ErasedMut for BorrowedMutViewLocal<'_> {
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    fn holder_ptr_mut(&mut self) -> *mut () {
        ErasedMut::holder_ptr_mut(self.0)
    }
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_> {
        self.0.shared_view_local()
    }
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b> {
        Box::new(BorrowedMutViewLocal(&mut *self.0))
    }
}

struct BorrowedMutViewThreadSafe<'a>(&'a mut (dyn ErasedMut + Send + Sync));
impl ErasedMut for BorrowedMutViewThreadSafe<'_> {
    fn type_id(&self) -> TypeId {
        ErasedMut::type_id(self.0)
    }
    fn holder_ptr(&self) -> *const () {
        ErasedMut::holder_ptr(self.0)
    }
    fn holder_ptr_mut(&mut self) -> *mut () {
        ErasedMut::holder_ptr_mut(self.0)
    }
    fn shared_view_local(&self) -> Rc<dyn ErasedRef + '_> {
        self.0.shared_view_local()
    }
    fn reborrow<'b>(&'b mut self) -> Box<dyn ErasedMut + 'b> {
        Box::new(BorrowedMutViewLocal(&mut *self.0))
    }
}

impl ErasedMutThreadSafe for BorrowedMutViewThreadSafe<'_> {
    fn shared_view_thread_safe(&self) -> Arc<dyn ErasedRef + Send + Sync + '_> {
        Arc::new(MutRefViewThreadSafe(&*self.0))
    }
    fn reborrow_thread_safe<'b>(&'b mut self) -> Box<dyn ErasedMutThreadSafe + 'b> {
        Box::new(BorrowedMutViewThreadSafe(&mut *self.0))
    }
}

pub(crate) trait ValueForRef<M: ValueMode>: Reflect {
    fn erase_ref<'a>(value: &'a Self) -> M::Shared<'a>;
    fn downcast_ref<'a, 'b>(value: &'b M::Shared<'a>) -> Option<&'b Self>;
    fn downcast_mut_ref<'a, 'b>(value: &'b M::Mutable<'a>) -> Option<&'b Self>;
    fn recover_ref<'a>(value: M::Shared<'a>) -> Result<&'a Self, M::Shared<'a>>
    where
        Self: 'a;
    fn identity() -> TypeIdentity;
}

impl<T: Reflect + ?Sized> ValueForRef<Local> for T {
    fn erase_ref<'a>(value: &'a Self) -> Rc<dyn ErasedRef + 'a> {
        Rc::new(RefHolder(value))
    }
    fn downcast_ref<'a, 'b>(value: &'b Rc<dyn ErasedRef + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: `type_id` is checked first. `holder_ptr` is produced by `RefHolder<T>`
        // or forwarded by a mutable view, and `is_mutable` selects the matching layout.
        unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                    .0
                    .as_deref()
            } else {
                Some(&(*(value.holder_ptr() as *const RefHolder<'a, Self>)).0)
            }
        }
    }
    fn downcast_mut_ref<'a, 'b>(value: &'b Box<dyn ErasedMut + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe {
            (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                .0
                .as_deref()
        }
    }
    fn recover_ref<'a>(value: Rc<dyn ErasedRef + 'a>) -> Result<&'a Self, Rc<dyn ErasedRef + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: as above, the type ID and mutable-kind discriminator establish the
        // concrete holder layout. The erased object lifetime is exactly `'a`.
        Ok(unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                    .0
                    .as_deref()
                    .expect("mutable holder always contains its original borrow")
            } else {
                &(*(value.holder_ptr() as *const RefHolder<'a, Self>)).0
            }
        })
    }
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

impl<T: Reflect + Sync + ?Sized> ValueForRef<ThreadSafe> for T {
    fn erase_ref<'a>(value: &'a Self) -> Arc<dyn ErasedRef + Send + Sync + 'a> {
        Arc::new(RefHolder(value))
    }
    fn downcast_ref<'a, 'b>(value: &'b Arc<dyn ErasedRef + Send + Sync + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: `type_id` is checked first. `holder_ptr` is produced by `RefHolder<T>`
        // or forwarded by a mutable view, and `is_mutable` selects the matching layout.
        unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                    .0
                    .as_deref()
            } else {
                Some(&(*(value.holder_ptr() as *const RefHolder<'a, Self>)).0)
            }
        }
    }
    fn downcast_mut_ref<'a, 'b>(value: &'b Box<dyn ErasedMutThreadSafe + 'a>) -> Option<&'b Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe {
            (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                .0
                .as_deref()
        }
    }
    fn recover_ref<'a>(
        value: Arc<dyn ErasedRef + Send + Sync + 'a>,
    ) -> Result<&'a Self, Arc<dyn ErasedRef + Send + Sync + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: as above, the type ID and mutable-kind discriminator establish the
        // concrete holder layout. The erased object lifetime is exactly `'a`.
        Ok(unsafe {
            if value.is_mutable() {
                (*(value.holder_ptr() as *const MutHolder<'a, Self>))
                    .0
                    .as_deref()
                    .expect("mutable holder always contains its original borrow")
            } else {
                &(*(value.holder_ptr() as *const RefHolder<'a, Self>)).0
            }
        })
    }
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

pub(crate) trait ValueForMut<M: ValueMode>: Reflect {
    fn erase_mut<'a>(value: &'a mut Self) -> M::Mutable<'a>;
    fn downcast_mut<'b, 'a>(value: &'b mut M::Mutable<'a>) -> Option<&'b mut Self>;
    fn recover_mut<'a>(value: M::Mutable<'a>) -> Result<&'a mut Self, M::Mutable<'a>>
    where
        Self: 'a;
    fn identity() -> TypeIdentity;
}

impl<T: Reflect + ?Sized> ValueForMut<Local> for T {
    fn erase_mut<'a>(value: &'a mut Self) -> Box<dyn ErasedMut + 'a> {
        Box::new(MutHolder(Some(value)))
    }
    fn downcast_mut<'b, 'a>(value: &'b mut Box<dyn ErasedMut + 'a>) -> Option<&'b mut Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe {
            (*(value.holder_ptr_mut() as *mut MutHolder<'a, Self>))
                .0
                .as_deref_mut()
        }
    }
    fn recover_mut<'a>(
        mut value: Box<dyn ErasedMut + 'a>,
    ) -> Result<&'a mut Self, Box<dyn ErasedMut + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: the type ID is checked and `holder_ptr_mut` forwards the concrete
        // `MutHolder<T>` address; exclusive access to `value` preserves aliasing rules.
        let holder = unsafe { &mut *(value.holder_ptr_mut() as *mut MutHolder<'a, Self>) };
        if holder.0.is_none() {
            return Err(value);
        }
        Ok(holder.0.take().expect("checked mutable holder contents"))
    }
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

impl<T: Reflect + Send + Sync + ?Sized> ValueForMut<ThreadSafe> for T {
    fn erase_mut<'a>(value: &'a mut Self) -> Box<dyn ErasedMutThreadSafe + 'a> {
        Box::new(MutHolder(Some(value)))
    }
    fn downcast_mut<'b, 'a>(
        value: &'b mut Box<dyn ErasedMutThreadSafe + 'a>,
    ) -> Option<&'b mut Self> {
        if value.type_id() != TypeId::of::<Self>() {
            return None;
        }
        // SAFETY: only `MutHolder<T>` and its forwarding views report this type ID;
        // the pointer is forwarded unchanged and therefore has the matching layout.
        unsafe {
            (*(value.holder_ptr_mut() as *mut MutHolder<'a, Self>))
                .0
                .as_deref_mut()
        }
    }
    fn recover_mut<'a>(
        mut value: Box<dyn ErasedMutThreadSafe + 'a>,
    ) -> Result<&'a mut Self, Box<dyn ErasedMutThreadSafe + 'a>>
    where
        Self: 'a,
    {
        if value.type_id() != TypeId::of::<Self>() {
            return Err(value);
        }
        // SAFETY: the type ID is checked and `holder_ptr_mut` forwards the concrete
        // `MutHolder<T>` address; exclusive access to `value` preserves aliasing rules.
        let holder = unsafe { &mut *(value.holder_ptr_mut() as *mut MutHolder<'a, Self>) };
        if holder.0.is_none() {
            return Err(value);
        }
        Ok(holder.0.take().expect("checked mutable holder contents"))
    }
    fn identity() -> TypeIdentity {
        type_identity::<Self>()
    }
}

fn type_identity<T: Reflect + ?Sized>() -> TypeIdentity {
    TypeIdentity {
        type_id: TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

pub(crate) struct DynamicRef<'a, M: ValueMode = Local> {
    value: M::Shared<'a>,
    identity: TypeIdentity,
    descriptor_factory: DescriptorFactory,
    _mode: PhantomData<M>,
}

impl<'a, M: ValueMode> DynamicRef<'a, M> {
    pub(crate) fn new<T: ValueForRef<M> + ?Sized>(value: &'a T) -> Self {
        Self {
            value: T::erase_ref(value),
            identity: T::identity(),
            descriptor_factory: descriptor_for::<T>,
            _mode: PhantomData,
        }
    }
    pub(crate) fn type_id(self) -> TypeId {
        self.identity.type_id
    }
    pub(crate) fn ty(self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }
    pub(crate) fn ty_in<'ctx>(self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        (self.descriptor_factory)(context)
    }
    pub(crate) fn is<T: ValueForRef<M> + ?Sized>(self) -> bool {
        self.type_id() == T::identity().type_id
    }
    pub(crate) fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        T::downcast_ref(&self.value)
    }
    pub(crate) fn reborrow<'b>(&'b self) -> DynamicRef<'b, M>
    where
        'a: 'b,
    {
        DynamicRef::from_erased(
            M::shared_reborrow(&self.value),
            self.identity,
            self.descriptor_factory,
        )
    }
    pub(crate) fn downcast<T: ValueForRef<M> + ?Sized>(
        self,
    ) -> Result<&'a T, DowncastFailure<Self>> {
        let expected = T::identity();
        if expected.type_id != self.identity.type_id {
            return Err(DowncastFailure::new(expected, self.identity, self));
        }
        match T::recover_ref(self.value) {
            Ok(value) => Ok(value),
            Err(value) => Err(DowncastFailure::new(
                expected,
                self.identity,
                Self {
                    value,
                    identity: self.identity,
                    descriptor_factory: self.descriptor_factory,
                    _mode: PhantomData,
                },
            )),
        }
    }
    pub(super) fn from_erased(
        value: M::Shared<'a>,
        identity: TypeIdentity,
        descriptor_factory: DescriptorFactory,
    ) -> Self {
        Self {
            value,
            identity,
            descriptor_factory,
            _mode: PhantomData,
        }
    }
}

impl<'a, M: ValueMode> Clone for DynamicRef<'a, M>
where
    M::Shared<'a>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}

impl<'a> DynamicRef<'a, ThreadSafe> {
    pub(crate) fn into_local(self) -> DynamicRef<'a, Local> {
        let value = thread_safe_ref_into_local(self.value);
        DynamicRef {
            value,
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}
impl<'a> DynamicRef<'a, Local> {
    pub(crate) fn into_local(self) -> Self {
        self
    }
}

pub(crate) struct DynamicMut<'a, M: ValueMode = Local> {
    value: M::Mutable<'a>,
    identity: TypeIdentity,
    descriptor_factory: DescriptorFactory,
    _mode: PhantomData<M::Marker>,
}

impl<'a, M: ValueMode> DynamicMut<'a, M> {
    pub(crate) fn new<T: ValueForMut<M> + ?Sized>(value: &'a mut T) -> Self {
        Self {
            value: T::erase_mut(value),
            identity: T::identity(),
            descriptor_factory: descriptor_for::<T>,
            _mode: PhantomData,
        }
    }
    pub(crate) fn type_id(&self) -> TypeId {
        self.identity.type_id
    }
    pub(crate) fn ty(&self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }
    pub(crate) fn ty_in<'ctx>(&self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        (self.descriptor_factory)(context)
    }
    pub(crate) fn is<T: ValueForMut<M> + ?Sized>(&self) -> bool {
        self.type_id() == T::identity().type_id
    }
    pub(crate) fn downcast_ref<T: ValueForRef<M> + ?Sized>(&self) -> Option<&T> {
        T::downcast_mut_ref(&self.value)
    }
    pub(crate) fn downcast_mut<T: ValueForMut<M> + ?Sized>(&mut self) -> Option<&mut T> {
        T::downcast_mut(&mut self.value)
    }
    pub(crate) fn downcast<T: ValueForMut<M> + ?Sized>(
        self,
    ) -> Result<&'a mut T, DowncastFailure<Self>> {
        let expected = T::identity();
        if expected.type_id != self.identity.type_id {
            return Err(DowncastFailure::new(expected, self.identity, self));
        }
        match T::recover_mut(self.value) {
            Ok(value) => Ok(value),
            Err(value) => Err(DowncastFailure::new(
                expected,
                self.identity,
                Self {
                    value,
                    identity: self.identity,
                    descriptor_factory: self.descriptor_factory,
                    _mode: PhantomData,
                },
            )),
        }
    }
    pub(crate) fn as_ref(&self) -> DynamicRef<'_, M> {
        DynamicRef::from_erased(
            M::shared_from_mut(&self.value),
            self.identity,
            self.descriptor_factory,
        )
    }
    pub(crate) fn reborrow(&mut self) -> DynamicMut<'_, M> {
        DynamicMut {
            value: M::reborrow_mut(&mut self.value),
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
    pub(super) fn from_erased(
        value: M::Mutable<'a>,
        identity: TypeIdentity,
        descriptor_factory: DescriptorFactory,
    ) -> Self {
        Self {
            value,
            identity,
            descriptor_factory,
            _mode: PhantomData,
        }
    }
}

impl<'a> DynamicMut<'a, ThreadSafe> {
    pub(crate) fn into_local(self) -> DynamicMut<'a, Local> {
        let value: Box<dyn ErasedMut + 'a> = self.value;
        DynamicMut {
            value,
            identity: self.identity,
            descriptor_factory: self.descriptor_factory,
            _mode: PhantomData,
        }
    }
}
impl<'a> DynamicMut<'a, Local> {
    pub(crate) fn into_local(self) -> Self {
        self
    }
}
