//! Safe invocation inputs, preflight validation, outputs, and recovery.

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;

use crate::runtime::descriptor::method::{
    InvocationAdapters, MethodDescriptor, ParameterDescriptor, ParameterPattern, PassingMode,
    ReceiverKind,
};
use crate::runtime::value::{
    DynamicMut, DynamicOwned, DynamicRef, ValueFor, ValueForMut, ValueForRef,
};

pub(crate) use crate::runtime::value::{Local, ThreadSafe, ValueMode};

/// Selects the statically generated adapter for one sealed value mode.
pub(crate) trait ModeAdapter: ValueMode {
    fn select_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn InvocationAdapter<Self>>;
    fn select_catching_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn InvocationAdapter<Self>>;
    fn select_pinned_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn PinnedInvocationAdapter<Self>>;
}

impl ModeAdapter for Local {
    fn select_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.local
    }
    fn select_catching_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.catching_local
    }
    fn select_pinned_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn PinnedInvocationAdapter<Self>> {
        adapters.pinned_local
    }
}

impl ModeAdapter for ThreadSafe {
    fn select_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.thread_safe
    }
    fn select_catching_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.catching_thread_safe
    }
    fn select_pinned_adapter(
        adapters: &InvocationAdapters,
    ) -> Option<&'static dyn PinnedInvocationAdapter<Self>> {
        adapters.pinned_thread_safe
    }
}

/// A mode-specific adapter generated for one statically bound Rust method.
pub trait InvocationAdapter<M: ValueMode>: Sync {
    /// Executes a previously validated invocation.
    fn invoke<'call>(
        &self,
        invocation: ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M>;
}

/// Function-pointer adapter for generated method bodies.
pub struct FunctionInvocationAdapter<M: ValueMode> {
    invoke: for<'call> fn(ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    _mode: std::marker::PhantomData<fn() -> M>,
}

impl<M: ValueMode> FunctionInvocationAdapter<M> {
    /// Binds a generated function to the invocation adapter contract.
    pub const fn new(
        invoke: for<'call> fn(ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    ) -> Self {
        Self {
            invoke,
            _mode: std::marker::PhantomData,
        }
    }
}

impl<M: ValueMode> InvocationAdapter<M> for FunctionInvocationAdapter<M> {
    fn invoke<'call>(
        &self,
        invocation: ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M> {
        (self.invoke)(invocation)
    }
}

/// Adapter for a method whose receiver is pinned.
pub(crate) trait PinnedInvocationAdapter<M: ValueMode>: Sync {
    fn receiver_type_id(&self) -> TypeId;
    fn receiver_kind(&self) -> ReceiverKind;

    fn invoke<'call>(
        &self,
        receiver: PinnedReceiver<'call>,
        invocation: ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M>;
}

/// Function-pointer adapter for a receiver whose type and pin mode are fixed.
pub struct PinnedFunctionInvocationAdapter<M: ValueMode, T: ?Sized + 'static> {
    kind: ReceiverKind,
    invoke: for<'call> fn(
        PinnedReceiver<'call>,
        ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M>,
    _receiver: std::marker::PhantomData<fn(&T)>,
    _mode: std::marker::PhantomData<fn() -> M>,
}

impl<M: ValueMode, T: ?Sized + 'static> PinnedFunctionInvocationAdapter<M, T> {
    /// Creates a pinned shared-receiver adapter for the exact type `T`.
    pub const fn shared(
        invoke: for<'call> fn(
            PinnedReceiver<'call>,
            ValidatedInvocation<'call, M>,
        ) -> InvocationOutput<'call, M>,
    ) -> Self {
        Self {
            kind: ReceiverKind::PinnedShared,
            invoke,
            _receiver: std::marker::PhantomData,
            _mode: std::marker::PhantomData,
        }
    }

    /// Creates a pinned mutable-receiver adapter for the exact type `T`.
    pub const fn mutable(
        invoke: for<'call> fn(
            PinnedReceiver<'call>,
            ValidatedInvocation<'call, M>,
        ) -> InvocationOutput<'call, M>,
    ) -> Self {
        Self {
            kind: ReceiverKind::PinnedMutable,
            invoke,
            _receiver: std::marker::PhantomData,
            _mode: std::marker::PhantomData,
        }
    }
}

impl<M: ValueMode, T: ?Sized + 'static> PinnedInvocationAdapter<M>
    for PinnedFunctionInvocationAdapter<M, T>
{
    fn receiver_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn receiver_kind(&self) -> ReceiverKind {
        self.kind
    }

    fn invoke<'call>(
        &self,
        receiver: PinnedReceiver<'call>,
        invocation: ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M> {
        (self.invoke)(receiver, invocation)
    }
}

trait ErasedPinnedReceiver {
    fn type_id(&self) -> TypeId;
    fn holder_ptr(&mut self) -> *mut ();
    fn is_mutable(&self) -> bool;
}

struct PinnedSharedHolder<'call, T: ?Sized + 'static> {
    receiver: Pin<&'call T>,
}

impl<T: ?Sized + 'static> ErasedPinnedReceiver for PinnedSharedHolder<'_, T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn holder_ptr(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }
    fn is_mutable(&self) -> bool {
        false
    }
}

struct PinnedMutableHolder<'call, T: ?Sized + 'static> {
    receiver: Option<Pin<&'call mut T>>,
}

impl<T: ?Sized + 'static> ErasedPinnedReceiver for PinnedMutableHolder<'_, T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn holder_ptr(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }
    fn is_mutable(&self) -> bool {
        true
    }
}

/// A pinned receiver erased for a generated typed adapter. The holder retains
/// the typed Pin, including any DST metadata, until the adapter downcasts it.
pub struct PinnedReceiver<'call> {
    inner: PinnedReceiverKind<'call>,
}

enum PinnedReceiverKind<'call> {
    Shared(Box<dyn ErasedPinnedReceiver + 'call>),
    Mutable(Box<dyn ErasedPinnedReceiver + 'call>),
}

impl<'call> PinnedReceiver<'call> {
    fn type_id(&self) -> TypeId {
        match &self.inner {
            PinnedReceiverKind::Shared(holder) | PinnedReceiverKind::Mutable(holder) => {
                ErasedPinnedReceiver::type_id(holder.as_ref())
            }
        }
    }
    fn is_mutable(&self) -> bool {
        matches!(self.inner, PinnedReceiverKind::Mutable(_))
    }

    #[allow(unsafe_code)]
    pub fn downcast_ref<T: ?Sized + 'static>(self) -> Option<Pin<&'call T>> {
        match self.inner {
            PinnedReceiverKind::Shared(mut holder)
                if ErasedPinnedReceiver::type_id(holder.as_ref()) == TypeId::of::<T>() =>
            {
                // SAFETY: `ErasedPinnedReceiver` is private and only the two holder
                // structs in this module implement it. This receiver's exact TypeId
                // matches T, so the holder pointer has `PinnedSharedHolder<T>` layout;
                // its typed Pin field retains any DST metadata.
                let holder =
                    unsafe { &*(holder.holder_ptr() as *const PinnedSharedHolder<'call, T>) };
                Some(holder.receiver)
            }
            PinnedReceiverKind::Mutable(mut holder)
                if ErasedPinnedReceiver::type_id(holder.as_ref()) == TypeId::of::<T>() =>
            {
                // SAFETY: `ErasedPinnedReceiver` is private and only the two holder
                // structs in this module implement it. This receiver's exact TypeId
                // matches T, so the holder pointer has `PinnedMutableHolder<T>` layout;
                // its typed Pin field retains any DST metadata.
                let holder =
                    unsafe { &mut *(holder.holder_ptr() as *mut PinnedMutableHolder<'call, T>) };
                holder.receiver.take().map(Pin::into_ref)
            }
            _ => None,
        }
    }

    #[allow(unsafe_code)]
    pub fn downcast_mut<T: ?Sized + 'static>(self) -> Option<Pin<&'call mut T>> {
        match self.inner {
            PinnedReceiverKind::Mutable(mut holder)
                if ErasedPinnedReceiver::type_id(holder.as_ref()) == TypeId::of::<T>() =>
            {
                // SAFETY: `ErasedPinnedReceiver` is private and only the two holder
                // structs in this module implement it. This receiver's exact TypeId
                // matches T, so the holder pointer has `PinnedMutableHolder<T>` layout;
                // its typed Pin field retains any DST metadata.
                let holder =
                    unsafe { &mut *(holder.holder_ptr() as *mut PinnedMutableHolder<'call, T>) };
                holder.receiver.take()
            }
            _ => None,
        }
    }
}

/// A shared or mutable receiver passed to an invocation.
pub enum InvocationReceiver<'call, M: ValueMode> {
    Owned(DynamicOwned<M>),
    Ref(DynamicRef<'call, M>),
    Mut(DynamicMut<'call, M>),
}

impl<'call, M: ValueMode> InvocationReceiver<'call, M> {
    fn kind(&self) -> InvocationValueKind {
        match self {
            Self::Owned(_) => InvocationValueKind::Owned,
            Self::Ref(_) => InvocationValueKind::Ref,
            Self::Mut(_) => InvocationValueKind::Mut,
        }
    }
    fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => value.type_id(),
            Self::Ref(value) => value.clone().type_id(),
            Self::Mut(value) => value.type_id(),
        }
    }

    /// Recovers an owned receiver of its validated concrete type.
    pub fn into_owned<T: ValueFor<M>>(self) -> Result<T, Self> {
        match self {
            Self::Owned(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Owned(failure.into_value())),
            },
            receiver => Err(receiver),
        }
    }

    /// Recovers a shared receiver of its validated concrete type.
    pub fn into_ref<T: ValueForRef<M> + ?Sized>(self) -> Result<&'call T, Self> {
        match self {
            Self::Ref(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Ref(failure.into_value())),
            },
            receiver => Err(receiver),
        }
    }

    /// Recovers a mutable receiver of its validated concrete type.
    pub fn into_mut<T: ValueForMut<M> + ?Sized>(self) -> Result<&'call mut T, Self> {
        match self {
            Self::Mut(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Mut(failure.into_value())),
            },
            receiver => Err(receiver),
        }
    }
}

/// A bound input value, preserving whether it is owned or borrowed.
pub enum InvocationValue<'call, M: ValueMode> {
    Owned(DynamicOwned<M>),
    Ref(DynamicRef<'call, M>),
    Mut(DynamicMut<'call, M>),
}

/// A receiver or argument supplied to a dynamic invocation builder.
pub(crate) type InvocationArgument<'call, M = Local> = InvocationValue<'call, M>;
/// A receiver supplied in owned, shared, or mutable form.
pub(crate) type DynamicReceiver<'call, M = Local> = InvocationReceiver<'call, M>;

impl<'call, M: ValueMode> InvocationValue<'call, M> {
    fn kind(&self) -> InvocationValueKind {
        match self {
            Self::Owned(_) => InvocationValueKind::Owned,
            Self::Ref(_) => InvocationValueKind::Ref,
            Self::Mut(_) => InvocationValueKind::Mut,
        }
    }
    fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => value.type_id(),
            Self::Ref(value) => value.clone().type_id(),
            Self::Mut(value) => value.type_id(),
        }
    }

    /// Recovers an owned argument of its validated concrete type.
    pub fn into_owned<T: ValueFor<M>>(self) -> Result<T, Self> {
        match self {
            Self::Owned(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Owned(failure.into_value())),
            },
            value => Err(value),
        }
    }

    /// Recovers a shared argument of its validated concrete type.
    pub fn into_ref<T: ValueForRef<M> + ?Sized>(self) -> Result<&'call T, Self> {
        match self {
            Self::Ref(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Ref(failure.into_value())),
            },
            value => Err(value),
        }
    }

    /// Recovers a mutable argument of its validated concrete type.
    pub fn into_mut<T: ValueForMut<M> + ?Sized>(self) -> Result<&'call mut T, Self> {
        match self {
            Self::Mut(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(Self::Mut(failure.into_value())),
            },
            value => Err(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InvocationValueKind {
    Owned,
    Ref,
    Mut,
}

struct Binding<'call, M: ValueMode> {
    name: Option<Box<str>>,
    value: InvocationValue<'call, M>,
}

/// Caller-owned receiver and argument bindings for one invocation.
pub(crate) struct Invocation<'call, M: ValueMode = Local> {
    receiver: Option<InvocationReceiver<'call, M>>,
    bindings: Vec<Binding<'call, M>>,
}

impl<'call, M: ValueMode> Invocation<'call, M> {
    fn empty() -> Self {
        Self {
            receiver: None,
            bindings: Vec::new(),
        }
    }
    /// Adds an owned positional argument.
    pub(crate) fn arg<T: ValueFor<M>>(self, value: T) -> Self {
        self.arg_dynamic(DynamicOwned::new(value))
    }
    /// Adds a shared positional argument.
    pub(crate) fn arg_ref<T: ValueForRef<M> + ?Sized>(self, value: &'call T) -> Self {
        self.arg_dynamic_ref(DynamicRef::new(value))
    }
    /// Adds a mutable positional argument.
    pub(crate) fn arg_mut<T: ValueForMut<M> + ?Sized>(self, value: &'call mut T) -> Self {
        self.arg_dynamic_mut(DynamicMut::new(value))
    }
    /// Adds a named owned argument.
    pub(crate) fn named_arg<T: ValueFor<M>>(self, name: impl Into<Box<str>>, value: T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Owned(DynamicOwned::new(value)))
    }
    /// Adds a named shared argument.
    pub(crate) fn named_arg_ref<T: ValueForRef<M> + ?Sized>(
        self,
        name: impl Into<Box<str>>,
        value: &'call T,
    ) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Ref(DynamicRef::new(value)))
    }
    /// Adds a named mutable argument.
    pub(crate) fn named_arg_mut<T: ValueForMut<M> + ?Sized>(
        self,
        name: impl Into<Box<str>>,
        value: &'call mut T,
    ) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Mut(DynamicMut::new(value)))
    }
    /// Adds an already erased owned argument.
    pub(crate) fn arg_dynamic(mut self, value: DynamicOwned<M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Owned(value),
        });
        self
    }
    /// Adds an already erased shared argument.
    pub(crate) fn arg_dynamic_ref(mut self, value: DynamicRef<'call, M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Ref(value),
        });
        self
    }
    /// Adds an already erased mutable argument.
    pub(crate) fn arg_dynamic_mut(mut self, value: DynamicMut<'call, M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Mut(value),
        });
        self
    }
    /// Adds a named already erased argument in caller order.
    pub(crate) fn named_arg_dynamic(
        mut self,
        name: impl Into<Box<str>>,
        value: InvocationValue<'call, M>,
    ) -> Self {
        self.bindings.push(Binding {
            name: Some(name.into()),
            value,
        });
        self
    }
    /// Returns the number of caller-supplied argument bindings.
    pub(crate) fn argument_count(&self) -> usize {
        self.bindings.len()
    }
    /// Replaces one recovered caller binding while retaining the other inputs.
    pub(crate) fn replace_arg<T: ValueFor<M>>(
        mut self,
        index: usize,
        value: T,
    ) -> Result<Self, Self> {
        if index >= self.bindings.len() {
            return Err(self);
        }
        self.bindings[index].value = InvocationValue::Owned(DynamicOwned::new(value));
        Ok(self)
    }

    pub(crate) fn validate(
        self,
        method: MethodDescriptor<'_>,
    ) -> Result<ValidatedInvocation<'call, M>, InvocationFailure<'call, M>> {
        match validate_bindings(&self, method, None) {
            Ok(order) => Ok(ValidatedInvocation {
                invocation: self,
                order,
            }),
            Err(error) => Err(InvocationFailure::new(InvocationError::new(error), self)),
        }
    }

    fn reorder(self, order: Vec<(usize, usize)>) -> Self {
        let mut inputs = self.bindings.into_iter().map(Some).collect::<Vec<_>>();
        let mut slots = (0..inputs.len()).map(|_| None).collect::<Vec<_>>();
        for (binding_index, parameter_index) in order {
            slots[parameter_index] = inputs[binding_index].take();
        }
        Self {
            receiver: self.receiver,
            bindings: slots
                .into_iter()
                .map(|binding| binding.expect("validated invocation has every parameter bound"))
                .collect(),
        }
    }
}

/// A typed pinned receiver and its caller-ordered arguments.
pub(crate) enum PinnedInvocation<'call, M: ValueMode, T: ?Sized + 'static> {
    Shared {
        receiver: Pin<&'call T>,
        arguments: Invocation<'call, M>,
    },
    Mutable {
        receiver: Pin<&'call mut T>,
        arguments: Invocation<'call, M>,
    },
}

impl<'call, M: ValueMode, T: ?Sized + 'static> PinnedInvocation<'call, M, T> {
    pub(crate) fn shared(receiver: Pin<&'call T>) -> Self {
        Self::Shared {
            receiver,
            arguments: Invocation::empty(),
        }
    }
    pub(crate) fn mutable(receiver: Pin<&'call mut T>) -> Self {
        Self::Mutable {
            receiver,
            arguments: Invocation::empty(),
        }
    }
    pub(crate) fn arg<U: ValueFor<M>>(mut self, value: U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Owned(DynamicOwned::new(value)),
        });
        self
    }
    pub(crate) fn arg_ref<U: ValueForRef<M> + ?Sized>(mut self, value: &'call U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Ref(DynamicRef::new(value)),
        });
        self
    }
    pub(crate) fn arg_mut<U: ValueForMut<M> + ?Sized>(mut self, value: &'call mut U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Mut(DynamicMut::new(value)),
        });
        self
    }
    pub(crate) fn named_arg<U: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Owned(DynamicOwned::new(value)),
        });
        self
    }
    pub(crate) fn named_arg_ref<U: ValueForRef<M> + ?Sized>(
        mut self,
        name: impl Into<Box<str>>,
        value: &'call U,
    ) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Ref(DynamicRef::new(value)),
        });
        self
    }
    pub(crate) fn named_arg_mut<U: ValueForMut<M> + ?Sized>(
        mut self,
        name: impl Into<Box<str>>,
        value: &'call mut U,
    ) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Mut(DynamicMut::new(value)),
        });
        self
    }
    pub(crate) fn argument_count(&self) -> usize {
        self.arguments().bindings.len()
    }
    pub(crate) fn receiver_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    pub(crate) fn receiver_is_mutable(&self) -> bool {
        matches!(self, Self::Mutable { .. })
    }
    fn arguments(&self) -> &Invocation<'call, M> {
        match self {
            Self::Shared { arguments, .. } | Self::Mutable { arguments, .. } => arguments,
        }
    }
    fn arguments_mut(&mut self) -> &mut Invocation<'call, M> {
        match self {
            Self::Shared { arguments, .. } | Self::Mutable { arguments, .. } => arguments,
        }
    }
    pub(crate) fn pinned_kind_and_type(&self) -> (ReceiverKind, TypeId) {
        match self {
            Self::Shared { .. } => (ReceiverKind::PinnedShared, TypeId::of::<T>()),
            Self::Mutable { .. } => (ReceiverKind::PinnedMutable, TypeId::of::<T>()),
        }
    }
    pub(crate) fn into_parts(self) -> (PinnedReceiver<'call>, Invocation<'call, M>) {
        match self {
            Self::Shared {
                receiver,
                arguments,
            } => (
                PinnedReceiver {
                    inner: PinnedReceiverKind::Shared(Box::new(PinnedSharedHolder { receiver })),
                },
                arguments,
            ),
            Self::Mutable {
                receiver,
                arguments,
            } => (
                PinnedReceiver {
                    inner: PinnedReceiverKind::Mutable(Box::new(PinnedMutableHolder {
                        receiver: Some(receiver),
                    })),
                },
                arguments,
            ),
        }
    }
    pub(crate) fn validate(
        self,
        method: MethodDescriptor<'_>,
    ) -> Result<ValidatedPinnedInvocation<'call, M, T>, PinnedInvocationFailure<'call, M, T>> {
        let (kind, type_id) = self.pinned_kind_and_type();
        let order = match validate_bindings(self.arguments(), method, Some((kind, type_id))) {
            Ok(order) => order,
            Err(error) => {
                return Err(PinnedInvocationFailure {
                    error: InvocationError::new(error),
                    invocation: self,
                });
            }
        };
        Ok(ValidatedPinnedInvocation {
            invocation: self,
            order,
        })
    }
}

pub(crate) struct ValidatedPinnedInvocation<'call, M: ValueMode, T: ?Sized + 'static> {
    invocation: PinnedInvocation<'call, M, T>,
    order: Vec<(usize, usize)>,
}

impl<'call, M: ValueMode, T: ?Sized + 'static> ValidatedPinnedInvocation<'call, M, T> {
    pub(crate) fn into_parts(self) -> (PinnedReceiver<'call>, ValidatedInvocation<'call, M>) {
        let (receiver, invocation) = self.invocation.into_parts();
        (
            receiver,
            ValidatedInvocation {
                invocation,
                order: self.order,
            },
        )
    }
    pub(crate) fn into_invocation(self) -> PinnedInvocation<'call, M, T> {
        self.invocation
    }
}

/// A failed pinned invocation with the original Pin and all arguments retained.
pub(crate) struct PinnedInvocationFailure<'call, M: ValueMode, T: ?Sized + 'static> {
    error: InvocationError,
    invocation: PinnedInvocation<'call, M, T>,
}

impl<'call, M: ValueMode, T: ?Sized + 'static> PinnedInvocationFailure<'call, M, T> {
    pub(crate) fn new(error: InvocationError, invocation: PinnedInvocation<'call, M, T>) -> Self {
        Self { error, invocation }
    }
    pub(crate) const fn error(&self) -> &InvocationError {
        &self.error
    }
    pub(crate) fn into_invocation(self) -> PinnedInvocation<'call, M, T> {
        self.invocation
    }
}

impl<'call> Invocation<'call, Local> {
    /// Creates an associated-function invocation in the default local mode.
    pub(crate) fn associated() -> Self {
        Self {
            receiver: None,
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with an owned local receiver.
    pub(crate) fn on_owned<T: ValueFor<Local>>(receiver: T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Owned(DynamicOwned::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a shared local receiver.
    pub(crate) fn on_ref<T: ValueForRef<Local> + ?Sized>(receiver: &'call T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Ref(DynamicRef::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a mutable local receiver.
    pub(crate) fn on_mut<T: ValueForMut<Local> + ?Sized>(receiver: &'call mut T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Mut(DynamicMut::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation from a pre-erased receiver.
    pub(crate) fn on_dynamic(receiver: DynamicReceiver<'call, Local>) -> Self {
        Self {
            receiver: Some(receiver),
            bindings: Vec::new(),
        }
    }
}

impl<'call> Invocation<'call, ThreadSafe> {
    /// Creates an associated-function invocation in thread-safe mode.
    pub(crate) fn associated() -> Self {
        Self {
            receiver: None,
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with an owned thread-safe receiver.
    pub(crate) fn on_owned<T: ValueFor<ThreadSafe>>(receiver: T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Owned(DynamicOwned::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a shared thread-safe receiver.
    pub(crate) fn on_ref<T: ValueForRef<ThreadSafe> + ?Sized>(receiver: &'call T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Ref(DynamicRef::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a mutable thread-safe receiver.
    pub(crate) fn on_mut<T: ValueForMut<ThreadSafe> + ?Sized>(receiver: &'call mut T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Mut(DynamicMut::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation from a pre-erased receiver.
    pub(crate) fn on_dynamic(receiver: DynamicReceiver<'call, ThreadSafe>) -> Self {
        Self {
            receiver: Some(receiver),
            bindings: Vec::new(),
        }
    }
}

/// An invocation after declaration-order binding and all dynamic checks passed.
pub struct ValidatedInvocation<'call, M: ValueMode> {
    invocation: Invocation<'call, M>,
    order: Vec<(usize, usize)>,
}

impl<'call, M: ValueMode> ValidatedInvocation<'call, M> {
    /// Returns the receiver for a generated adapter.
    pub fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.invocation.receiver.as_ref()
    }
    /// Returns arguments in declaration order for a generated adapter.
    pub fn arguments(&self) -> impl ExactSizeIterator<Item = &InvocationValue<'call, M>> {
        self.order
            .iter()
            .map(|(binding_index, _)| &self.invocation.bindings[*binding_index].value)
    }
    /// Consumes the validated inputs for adapter execution.
    pub fn into_parts(
        self,
    ) -> (
        Option<InvocationReceiver<'call, M>>,
        Box<[InvocationValue<'call, M>]>,
    ) {
        let invocation = self.invocation.reorder(self.order);
        (
            invocation.receiver,
            invocation
                .bindings
                .into_iter()
                .map(|binding| binding.value)
                .collect(),
        )
    }
    pub(crate) fn into_invocation(self) -> Invocation<'call, M> {
        self.invocation
    }
}

fn validate_bindings<M: ValueMode>(
    invocation: &Invocation<'_, M>,
    method: MethodDescriptor<'_>,
    pinned_receiver: Option<(ReceiverKind, TypeId)>,
) -> Result<Vec<(usize, usize)>, InvocationErrorKind> {
    let parameters = method.parameters().collect::<Vec<_>>();
    let mut order = Vec::with_capacity(invocation.bindings.len());
    let mut occupied = vec![false; parameters.len()];
    for binding in &invocation.bindings {
        let parameter_index = if let Some(name) = &binding.name {
            let mut matches = parameters
                .iter()
                .enumerate()
                .filter(|(_, parameter)| parameter.name() == Some(name));
            match (matches.next(), matches.next()) {
                (None, _)
                    if parameters.iter().any(|parameter| {
                        !matches!(parameter.pattern(), ParameterPattern::Identifier(_))
                    }) =>
                {
                    return Err(InvocationErrorKind::NamedArgumentUnavailable {
                        name: name.clone(),
                    });
                }
                (None, _) => {
                    return Err(InvocationErrorKind::UnknownArgumentName { name: name.clone() });
                }
                (Some((index, _)), None) => index,
                _ => {
                    return Err(InvocationErrorKind::DuplicateArgumentBinding {
                        name: name.clone(),
                    });
                }
            }
        } else {
            occupied.iter().position(|is_occupied| !is_occupied).ok_or(
                InvocationErrorKind::TooManyArguments {
                    expected: parameters.len(),
                    actual: invocation.bindings.len(),
                },
            )?
        };
        if occupied[parameter_index] {
            return Err(InvocationErrorKind::DuplicateArgumentBinding {
                name: binding.name.clone().unwrap_or_else(|| {
                    parameters[parameter_index]
                        .name()
                        .unwrap_or("<pattern>")
                        .into()
                }),
            });
        }
        occupied[parameter_index] = true;
        order.push(parameter_index);
    }

    if let Some((actual_kind, actual_type)) = pinned_receiver {
        match method.receiver() {
            None => return Err(InvocationErrorKind::UnexpectedReceiver),
            Some(expected) => {
                let mode_matches = match (expected.kind, actual_kind) {
                    (ReceiverKind::PinnedShared, ReceiverKind::PinnedShared)
                    | (ReceiverKind::PinnedShared, ReceiverKind::PinnedMutable)
                    | (ReceiverKind::PinnedMutable, ReceiverKind::PinnedMutable) => true,
                    _ => false,
                };
                if !mode_matches {
                    return Err(InvocationErrorKind::ReceiverModeMismatch);
                }
                let expected_type = (expected
                    .type_metadata
                    .expect("effective method receiver is bound to its target"))(
                )
                .type_id();
                if actual_type != expected_type {
                    return Err(InvocationErrorKind::ReceiverTypeMismatch {
                        expected: expected_type,
                        actual: actual_type,
                    });
                }
            }
        }
    } else {
        match (method.receiver(), invocation.receiver.as_ref()) {
            (None, Some(_)) => return Err(InvocationErrorKind::UnexpectedReceiver),
            (Some(_), None) => return Err(InvocationErrorKind::ReceiverMissing),
            (Some(expected), Some(actual)) => {
                if matches!(
                    expected.kind,
                    ReceiverKind::PinnedShared | ReceiverKind::PinnedMutable
                ) {
                    return Err(InvocationErrorKind::ReceiverAdapterUnavailable);
                }
                let mode_matches = match expected.kind {
                    ReceiverKind::Owned => actual.kind() == InvocationValueKind::Owned,
                    ReceiverKind::Shared => matches!(
                        actual.kind(),
                        InvocationValueKind::Ref | InvocationValueKind::Mut
                    ),
                    ReceiverKind::Mutable => actual.kind() == InvocationValueKind::Mut,
                    ReceiverKind::PinnedShared | ReceiverKind::PinnedMutable => false,
                };
                if !mode_matches {
                    return Err(InvocationErrorKind::ReceiverModeMismatch);
                }
                let expected_type = (expected
                    .type_metadata
                    .expect("effective method receiver is bound to its target"))(
                )
                .type_id();
                if actual.type_id() != expected_type {
                    return Err(InvocationErrorKind::ReceiverTypeMismatch {
                        expected: expected_type,
                        actual: actual.type_id(),
                    });
                }
            }
            (None, None) => {}
        }
    }

    if invocation.bindings.len() > parameters.len() {
        return Err(InvocationErrorKind::TooManyArguments {
            expected: parameters.len(),
            actual: invocation.bindings.len(),
        });
    }
    if let Some(index) = occupied.iter().position(|occupied| !occupied) {
        return Err(InvocationErrorKind::MissingArgument { index });
    }
    for (binding_index, parameter_index) in order.iter().copied().enumerate() {
        let parameter = parameters[parameter_index];
        let binding = &invocation.bindings[binding_index];
        if !passing_matches(parameter, binding.value.kind()) {
            return Err(InvocationErrorKind::ArgumentModeMismatch {
                index: parameter_index,
            });
        }
        let Some(expected_type) = parameter.find_concrete_type().map(|ty| ty.type_id()) else {
            return Err(InvocationErrorKind::ArgumentTypeUnavailable {
                index: parameter_index,
            });
        };
        let actual_type = binding.value.type_id();
        if expected_type != actual_type {
            return Err(InvocationErrorKind::ArgumentTypeMismatch {
                index: parameter_index,
                expected: expected_type,
                actual: actual_type,
            });
        }
    }
    let mut indexed = order
        .into_iter()
        .enumerate()
        .map(|(binding_index, parameter_index)| (binding_index, parameter_index))
        .collect::<Vec<_>>();
    indexed.sort_by_key(|(_, parameter_index)| *parameter_index);
    Ok(indexed)
}

fn passing_matches(parameter: ParameterDescriptor<'_>, value: InvocationValueKind) -> bool {
    matches!(
        (parameter.passing(), value),
        (PassingMode::Owned, InvocationValueKind::Owned)
            | (PassingMode::Shared, InvocationValueKind::Ref)
            | (PassingMode::Shared, InvocationValueKind::Mut)
            | (PassingMode::Mutable, InvocationValueKind::Mut)
    )
}

/// A mode-specific availability result for a specialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InvocationAvailability {
    Callable,
    Unavailable {
        reasons: &'static [InvocationUnavailableReason],
    },
}

/// Reasons a method cannot cross the dynamic invocation boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationUnavailableReason {
    UnsupportedReceiver,
    AdapterUnavailable,
    UnsupportedParameterPattern,
    UnspecializedGeneric,
    UnsafeMethod,
    UnsupportedAbi,
    Variadic,
    UnsupportedBorrowedReturn,
    OpaqueReturn,
    UnsupportedUnsizedValue,
    UnprovenTraitConstraint,
    DisabledByPolicy,
}

/// Invocation preflight and adapter errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InvocationError {
    kind: InvocationErrorKind,
}

impl InvocationError {
    pub(crate) const fn new(kind: InvocationErrorKind) -> Self {
        Self { kind }
    }
    pub(crate) const fn kind(&self) -> &InvocationErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for InvocationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self.kind)
    }
}

impl std::error::Error for InvocationError {}

/// A precise reason that invocation could not begin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InvocationErrorKind {
    SpecializationRequired,
    SpecializationSelectionRequired {
        count: usize,
    },
    MethodUnavailable {
        reasons: &'static [InvocationUnavailableReason],
    },
    VisibilityDenied,
    UnsafeMethod,
    ReceiverMissing,
    UnexpectedReceiver,
    ReceiverModeMismatch,
    ReceiverTypeMismatch {
        expected: TypeId,
        actual: TypeId,
    },
    ReceiverAdapterUnavailable,
    UnknownArgumentName {
        name: Box<str>,
    },
    NamedArgumentUnavailable {
        name: Box<str>,
    },
    DuplicateArgumentBinding {
        name: Box<str>,
    },
    MissingArgument {
        index: usize,
    },
    TooManyArguments {
        expected: usize,
        actual: usize,
    },
    ArgumentModeMismatch {
        index: usize,
    },
    ArgumentTypeUnavailable {
        index: usize,
    },
    ArgumentTypeMismatch {
        index: usize,
        expected: TypeId,
        actual: TypeId,
    },
    ValueModeUnavailable,
}

/// Inputs retained after an invocation preflight failure.
pub(crate) struct InvocationRecovery<'call, M: ValueMode>(Invocation<'call, M>);

impl<'call, M: ValueMode> InvocationRecovery<'call, M> {
    pub(crate) fn into_invocation(self) -> Invocation<'call, M> {
        self.0
    }
}

/// An invocation error paired with all original, unconsumed inputs.
pub(crate) struct InvocationFailure<'call, M: ValueMode> {
    error: InvocationError,
    recovery: InvocationRecovery<'call, M>,
}

impl<'call, M: ValueMode> InvocationFailure<'call, M> {
    pub(crate) fn new(error: InvocationError, invocation: Invocation<'call, M>) -> Self {
        Self {
            error,
            recovery: InvocationRecovery(invocation),
        }
    }
    pub(crate) const fn error(&self) -> &InvocationError {
        &self.error
    }
    pub(crate) const fn recovery(&self) -> &InvocationRecovery<'call, M> {
        &self.recovery
    }
    pub(crate) fn into_parts(self) -> (InvocationError, InvocationRecovery<'call, M>) {
        (self.error, self.recovery)
    }
    pub(crate) fn into_invocation(self) -> Invocation<'call, M> {
        self.recovery.into_invocation()
    }
}

/// Result of an invocation attempt that may reject inputs or catch a panic.
pub(crate) enum InvocationAttempt<'call, M: ValueMode> {
    Rejected(InvocationFailure<'call, M>),
    Panicked(InvocationPanic),
}

/// A panic raised by a user adapter after invocation started.
#[derive(Debug)]
pub(crate) struct InvocationPanic {
    message: Option<Box<str>>,
}

impl InvocationPanic {
    pub(crate) fn from_payload(payload: Box<dyn std::any::Any + Send>) -> Self {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&'static str>().copied())
            .map(Into::into);
        Self { message }
    }
    pub(crate) fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// Failure from explicit panic-catching invocation.
pub(crate) enum CaughtInvocationFailure<'call, M: ValueMode> {
    Rejected(InvocationFailure<'call, M>),
    Panicked(InvocationPanic),
}

/// The origin of a borrow returned from a reflected method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BorrowOrigin {
    Receiver,
    Parameter(usize),
}

/// An asynchronous result whose executor is selected by the caller.
pub(crate) struct ReflectedFuture<'call, M: ValueMode> {
    future: Pin<Box<dyn Future<Output = InvocationOutput<'call, M>> + 'call>>,
}

impl<'call, M: ValueMode> ReflectedFuture<'call, M> {
    pub(crate) fn new(future: impl Future<Output = InvocationOutput<'call, M>> + 'call) -> Self {
        Self {
            future: Box::pin(future),
        }
    }
}
impl<'call, M: ValueMode> Future for ReflectedFuture<'call, M> {
    type Output = InvocationOutput<'call, M>;
    fn poll(
        self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.get_mut().future.as_mut().poll(context)
    }
}

/// A reflected method's result, retaining the borrow origins of reference outputs.
#[must_use]
pub enum InvocationOutput<'call, M: ValueMode = Local> {
    Unit,
    Owned(DynamicOwned<M>),
    Ref {
        value: DynamicRef<'call, M>,
        origins: Box<[BorrowOrigin]>,
    },
    OptionalRef {
        value: Option<DynamicRef<'call, M>>,
        origins: Box<[BorrowOrigin]>,
    },
    RefSlice {
        values: Box<[DynamicRef<'call, M>]>,
        origins: Box<[BorrowOrigin]>,
    },
    Mut {
        value: DynamicMut<'call, M>,
        origin: BorrowOrigin,
    },
    Future(ReflectedFuture<'call, M>),
}

/// Keeps the output when a typed output extraction expects another kind.
pub(crate) struct OutputMismatch<O> {
    output: O,
}
impl<O> OutputMismatch<O> {
    pub(crate) fn into_output(self) -> O {
        self.output
    }
}

impl<'call, M: ValueMode> InvocationOutput<'call, M> {
    /// Creates a unit result.
    pub fn unit() -> Self {
        Self::Unit
    }

    /// Erases an owned result while retaining its value mode.
    pub fn owned<T: ValueFor<M>>(value: T) -> Self {
        Self::Owned(DynamicOwned::new(value))
    }

    /// Erases a shared result and records which validated input it borrows.
    pub fn shared<T: ValueForRef<M> + ?Sized>(value: &'call T, origin: BorrowOrigin) -> Self {
        Self::shared_with_origins(value, &[origin])
    }

    /// Erases a shared result with all validated input origins it may borrow.
    pub fn shared_with_origins<T: ValueForRef<M> + ?Sized>(
        value: &'call T,
        origins: &[BorrowOrigin],
    ) -> Self {
        Self::Ref {
            value: DynamicRef::new(value),
            origins: origins.into(),
        }
    }

    /// Erases a mutable result and records which validated input it borrows.
    pub fn mutable<T: ValueForMut<M> + ?Sized>(value: &'call mut T, origin: BorrowOrigin) -> Self {
        Self::Mut {
            value: DynamicMut::new(value),
            origin,
        }
    }

    /// Erases an asynchronous result for the invocation's selected value mode.
    pub fn future(future: impl Future<Output = Self> + 'call) -> Self {
        Self::Future(ReflectedFuture::new(future))
    }

    pub fn into_unit(self) -> Result<(), OutputMismatch<Self>> {
        match self {
            Self::Unit => Ok(()),
            output => Err(OutputMismatch { output }),
        }
    }
    pub fn into_owned<T: ValueFor<M>>(self) -> Result<T, OutputMismatch<Self>> {
        match self {
            Self::Owned(value) => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(OutputMismatch {
                    output: Self::Owned(failure.into_value()),
                }),
            },
            output => Err(OutputMismatch { output }),
        }
    }
    pub fn into_ref<T: ValueForRef<M> + ?Sized>(self) -> Result<&'call T, OutputMismatch<Self>> {
        match self {
            Self::Ref { value, origins } => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(OutputMismatch {
                    output: Self::Ref {
                        value: failure.into_value(),
                        origins,
                    },
                }),
            },
            output => Err(OutputMismatch { output }),
        }
    }
    pub fn into_mut<T: ValueForMut<M> + ?Sized>(
        self,
    ) -> Result<&'call mut T, OutputMismatch<Self>> {
        match self {
            Self::Mut { value, origin } => match value.downcast::<T>() {
                Ok(value) => Ok(value),
                Err(failure) => Err(OutputMismatch {
                    output: Self::Mut {
                        value: failure.into_value(),
                        origin,
                    },
                }),
            },
            output => Err(OutputMismatch { output }),
        }
    }
    pub fn into_optional_ref<T: ValueForRef<M> + ?Sized>(
        self,
    ) -> Result<Option<&'call T>, OutputMismatch<Self>> {
        match self {
            Self::OptionalRef { value: None, .. } => Ok(None),
            Self::OptionalRef {
                value: Some(value),
                origins,
            } => match value.downcast::<T>() {
                Ok(value) => Ok(Some(value)),
                Err(failure) => Err(OutputMismatch {
                    output: Self::OptionalRef {
                        value: Some(failure.into_value()),
                        origins,
                    },
                }),
            },
            output => Err(OutputMismatch { output }),
        }
    }
    pub fn into_ref_slice<T: ValueForRef<M> + ?Sized>(
        self,
    ) -> Result<Box<[&'call T]>, OutputMismatch<Self>> {
        match self {
            Self::RefSlice { values, origins } => {
                let mut references = Vec::with_capacity(values.len());
                for value in &values {
                    match value.clone().downcast::<T>() {
                        Ok(reference) => references.push(reference),
                        Err(_) => {
                            return Err(OutputMismatch {
                                output: Self::RefSlice { values, origins },
                            });
                        }
                    }
                }
                Ok(references.into_boxed_slice())
            }
            output => Err(OutputMismatch { output }),
        }
    }
    pub fn into_future(self) -> Result<ReflectedFuture<'call, M>, OutputMismatch<Self>> {
        match self {
            Self::Future(future) => Ok(future),
            output => Err(OutputMismatch { output }),
        }
    }
}
