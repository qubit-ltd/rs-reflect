// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Safe invocation inputs, preflight validation, outputs, and recovery.

// Invocation builders and adapters are referenced by generated downstream
// code; this crate's local dead-code analysis cannot observe those expansions.
#![allow(dead_code)]

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;

use crate::runtime::descriptor::method::InvocationAdapters;
use crate::runtime::descriptor::method::MethodDescriptor;
use crate::runtime::descriptor::method::ParameterDescriptor;
use crate::runtime::descriptor::method::ParameterPattern;
use crate::runtime::descriptor::method::PassingMode;
use crate::runtime::descriptor::method::ReceiverKind;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::error::RecoverableFailureParts;
use crate::runtime::value::DynamicMut;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::DynamicRef;
pub use crate::runtime::value::DynamicValue as InvocationValue;
use crate::runtime::value::DynamicValueKind;
pub(crate) use crate::runtime::value::Local;
pub(crate) use crate::runtime::value::ThreadSafe;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueForMut;
use crate::runtime::value::ValueForRef;
pub(crate) use crate::runtime::value::ValueMode;

/// Selects the statically generated adapter for one sealed value mode.
pub(crate) trait ModeAdapter: Sized {
    /// Future object retained by this invocation mode.
    type Future<'a, O: 'a>: Future<Output = O> + Unpin + 'a;

    fn select_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>>
    where
        Self: ValueMode;
    fn select_catching_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>>
    where
        Self: ValueMode;
    fn select_pinned_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn PinnedInvocationAdapter<Self>>
    where
        Self: ValueMode;
}

impl ModeAdapter for Local {
    type Future<'a, O: 'a> = Pin<Box<dyn Future<Output = O> + 'a>>;

    fn select_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.local
    }
    fn select_catching_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.catching_local
    }
    fn select_pinned_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn PinnedInvocationAdapter<Self>> {
        adapters.pinned_local
    }
}

impl ModeAdapter for ThreadSafe {
    type Future<'a, O: 'a> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

    fn select_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.thread_safe
    }
    fn select_catching_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.catching_thread_safe
    }
    fn select_pinned_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn PinnedInvocationAdapter<Self>> {
        adapters.pinned_thread_safe
    }
}

/// A mode-specific adapter generated for one statically bound Rust method.
pub trait InvocationAdapter<M: ValueMode>: Sync {
    /// Executes a previously validated invocation.
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>;
}

/// Function-pointer adapter for generated method bodies.
pub struct FunctionInvocationAdapter<M: ValueMode> {
    invoke: for<'call> fn(ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    _mode: std::marker::PhantomData<fn() -> M>,
}

impl<M: ValueMode> FunctionInvocationAdapter<M> {
    /// Binds a generated function to the invocation adapter contract.
    pub const fn new(invoke: for<'call> fn(ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>) -> Self {
        Self {
            invoke,
            _mode: std::marker::PhantomData,
        }
    }
}

impl<M: ValueMode> InvocationAdapter<M> for FunctionInvocationAdapter<M> {
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M> {
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
    invoke: for<'call> fn(PinnedReceiver<'call>, ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    _receiver: std::marker::PhantomData<fn(&T)>,
    _mode: std::marker::PhantomData<fn() -> M>,
}

impl<M: ValueMode, T: ?Sized + 'static> PinnedFunctionInvocationAdapter<M, T> {
    /// Creates a pinned shared-receiver adapter for the exact type `T`.
    pub const fn shared(
        invoke: for<'call> fn(PinnedReceiver<'call>, ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
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
        invoke: for<'call> fn(PinnedReceiver<'call>, ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    ) -> Self {
        Self {
            kind: ReceiverKind::PinnedMutable,
            invoke,
            _receiver: std::marker::PhantomData,
            _mode: std::marker::PhantomData,
        }
    }
}

impl<M: ValueMode, T: ?Sized + 'static> PinnedInvocationAdapter<M> for PinnedFunctionInvocationAdapter<M, T> {
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
    /// Recovers the pinned shared receiver as `T` when its type matches.
    ///
    /// Returns `None` for a different receiver type.
    pub fn downcast_ref<T: ?Sized + 'static>(self) -> Option<Pin<&'call T>> {
        match self.inner {
            PinnedReceiverKind::Shared(mut holder)
                if ErasedPinnedReceiver::type_id(holder.as_ref()) == TypeId::of::<T>() =>
            {
                // SAFETY: `ErasedPinnedReceiver` is private and only the two holder
                // structs in this module implement it. This receiver's exact TypeId
                // matches T, so the holder pointer has `PinnedSharedHolder<T>` layout;
                // its typed Pin field retains any DST metadata.
                let holder = unsafe { &*(holder.holder_ptr() as *const PinnedSharedHolder<'call, T>) };
                Some(holder.receiver)
            }
            PinnedReceiverKind::Mutable(mut holder)
                if ErasedPinnedReceiver::type_id(holder.as_ref()) == TypeId::of::<T>() =>
            {
                // SAFETY: `ErasedPinnedReceiver` is private and only the two holder
                // structs in this module implement it. This receiver's exact TypeId
                // matches T, so the holder pointer has `PinnedMutableHolder<T>` layout;
                // its typed Pin field retains any DST metadata.
                let holder = unsafe { &mut *(holder.holder_ptr() as *mut PinnedMutableHolder<'call, T>) };
                holder.receiver.take().map(Pin::into_ref)
            }
            _ => None,
        }
    }

    #[allow(unsafe_code)]
    /// Recovers the pinned mutable receiver as `T` when its type matches.
    ///
    /// Returns `None` for a different receiver type.
    pub fn downcast_mut<T: ?Sized + 'static>(self) -> Option<Pin<&'call mut T>> {
        match self.inner {
            PinnedReceiverKind::Mutable(mut holder)
                if ErasedPinnedReceiver::type_id(holder.as_ref()) == TypeId::of::<T>() =>
            {
                // SAFETY: `ErasedPinnedReceiver` is private and only the two holder
                // structs in this module implement it. This receiver's exact TypeId
                // matches T, so the holder pointer has `PinnedMutableHolder<T>` layout;
                // its typed Pin field retains any DST metadata.
                let holder = unsafe { &mut *(holder.holder_ptr() as *mut PinnedMutableHolder<'call, T>) };
                holder.receiver.take()
            }
            _ => None,
        }
    }
}

/// A shared or mutable receiver passed to an invocation.
pub enum InvocationReceiver<'call, M: ValueMode> {
    /// An owned receiver.
    Owned(DynamicOwned<M>),
    /// A shared borrowed receiver.
    Ref(DynamicRef<'call, M>),
    /// An exclusively borrowed receiver.
    Mut(DynamicMut<'call, M>),
}

impl<'call, M: ValueMode> InvocationReceiver<'call, M> {
    fn kind(&self) -> DynamicValueKind {
        match self {
            Self::Owned(_) => DynamicValueKind::Owned,
            Self::Ref(_) => DynamicValueKind::Ref,
            Self::Mut(_) => DynamicValueKind::Mut,
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

/// A receiver or argument supplied to a dynamic invocation builder.
pub(crate) type InvocationArgument<'call, M = Local> = InvocationValue<'call, M>;
/// A receiver supplied in owned, shared, or mutable form.
pub(crate) type DynamicReceiver<'call, M = Local> = InvocationReceiver<'call, M>;

struct Binding<'call, M: ValueMode> {
    name: Option<Box<str>>,
    value: InvocationValue<'call, M>,
}

/// Caller-owned receiver and argument bindings for one invocation.
pub struct Invocation<'call, M: ValueMode = Local> {
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
    pub fn arg<T: ValueFor<M>>(self, value: T) -> Self {
        self.arg_dynamic(DynamicOwned::new(value))
    }
    /// Adds a shared positional argument.
    pub fn arg_ref<T: ValueForRef<M> + ?Sized>(self, value: &'call T) -> Self {
        self.arg_dynamic_ref(DynamicRef::new(value))
    }
    /// Adds a mutable positional argument.
    pub fn arg_mut<T: ValueForMut<M> + ?Sized>(self, value: &'call mut T) -> Self {
        self.arg_dynamic_mut(DynamicMut::new(value))
    }
    /// Adds a named owned argument.
    pub fn named_arg<T: ValueFor<M>>(self, name: impl Into<Box<str>>, value: T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Owned(DynamicOwned::new(value)))
    }
    /// Adds a named shared argument.
    pub fn named_arg_ref<T: ValueForRef<M> + ?Sized>(self, name: impl Into<Box<str>>, value: &'call T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Ref(DynamicRef::new(value)))
    }
    /// Adds a named mutable argument.
    pub fn named_arg_mut<T: ValueForMut<M> + ?Sized>(self, name: impl Into<Box<str>>, value: &'call mut T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Mut(DynamicMut::new(value)))
    }
    /// Adds an already erased owned argument.
    pub fn arg_dynamic(mut self, value: DynamicOwned<M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Owned(value),
        });
        self
    }
    /// Adds an already erased shared argument.
    pub fn arg_dynamic_ref(mut self, value: DynamicRef<'call, M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Ref(value),
        });
        self
    }
    /// Adds an already erased mutable argument.
    pub fn arg_dynamic_mut(mut self, value: DynamicMut<'call, M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Mut(value),
        });
        self
    }
    /// Adds a named already erased argument in caller order.
    pub fn named_arg_dynamic(mut self, name: impl Into<Box<str>>, value: InvocationValue<'call, M>) -> Self {
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
    pub(crate) fn replace_arg<T: ValueFor<M>>(mut self, index: usize, value: T) -> Result<Self, Self> {
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
pub enum PinnedInvocation<'call, M: ValueMode, T: ?Sized + 'static> {
    /// Invocation on a pinned shared receiver.
    Shared {
        /// The receiver whose address must remain stable during the call.
        receiver: Pin<&'call T>,
        /// Arguments in the order supplied by the caller.
        arguments: Invocation<'call, M>,
    },
    /// Invocation on a pinned mutable receiver.
    Mutable {
        /// The receiver whose address must remain stable during the call.
        receiver: Pin<&'call mut T>,
        /// Arguments in the order supplied by the caller.
        arguments: Invocation<'call, M>,
    },
}

impl<'call, M: ValueMode, T: ?Sized + 'static> PinnedInvocation<'call, M, T> {
    /// Starts an invocation with a pinned shared receiver.
    pub fn shared(receiver: Pin<&'call T>) -> Self {
        Self::Shared {
            receiver,
            arguments: Invocation::empty(),
        }
    }
    /// Starts an invocation with a pinned mutable receiver.
    pub fn mutable(receiver: Pin<&'call mut T>) -> Self {
        Self::Mutable {
            receiver,
            arguments: Invocation::empty(),
        }
    }
    /// Adds an owned positional argument.
    pub fn arg<U: ValueFor<M>>(mut self, value: U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Owned(DynamicOwned::new(value)),
        });
        self
    }
    /// Adds a shared positional argument.
    pub fn arg_ref<U: ValueForRef<M> + ?Sized>(mut self, value: &'call U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Ref(DynamicRef::new(value)),
        });
        self
    }
    /// Adds a mutable positional argument.
    pub fn arg_mut<U: ValueForMut<M> + ?Sized>(mut self, value: &'call mut U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Mut(DynamicMut::new(value)),
        });
        self
    }
    /// Adds a named owned argument.
    pub fn named_arg<U: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Owned(DynamicOwned::new(value)),
        });
        self
    }
    /// Adds a named shared argument.
    pub fn named_arg_ref<U: ValueForRef<M> + ?Sized>(mut self, name: impl Into<Box<str>>, value: &'call U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Ref(DynamicRef::new(value)),
        });
        self
    }
    /// Adds a named mutable argument.
    pub fn named_arg_mut<U: ValueForMut<M> + ?Sized>(mut self, name: impl Into<Box<str>>, value: &'call mut U) -> Self {
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
            Self::Shared { receiver, arguments } => (
                PinnedReceiver {
                    inner: PinnedReceiverKind::Shared(Box::new(PinnedSharedHolder { receiver })),
                },
                arguments,
            ),
            Self::Mutable { receiver, arguments } => (
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

/// A pinned invocation whose receiver and argument bindings passed preflight.
pub struct ValidatedPinnedInvocation<'call, M: ValueMode, T: ?Sized + 'static> {
    invocation: PinnedInvocation<'call, M, T>,
    order: Vec<(usize, usize)>,
}

impl<'call, M: ValueMode, T: ?Sized + 'static> ValidatedPinnedInvocation<'call, M, T> {
    /// Separates the erased pinned receiver from declaration-ordered arguments.
    pub fn into_parts(self) -> (PinnedReceiver<'call>, ValidatedInvocation<'call, M>) {
        let (receiver, invocation) = self.invocation.into_parts();
        (
            receiver,
            ValidatedInvocation {
                invocation,
                order: self.order,
            },
        )
    }
    /// Recovers the caller-ordered invocation while retaining its pinned
    /// receiver.
    pub fn into_invocation(self) -> PinnedInvocation<'call, M, T> {
        self.invocation
    }
}

/// A failed pinned invocation with the original Pin and all arguments retained.
pub struct PinnedInvocationFailure<'call, M: ValueMode, T: ?Sized + 'static> {
    error: InvocationError,
    invocation: PinnedInvocation<'call, M, T>,
}

impl<'call, M: ValueMode, T: ?Sized + 'static> PinnedInvocationFailure<'call, M, T> {
    pub(crate) fn new(error: InvocationError, invocation: PinnedInvocation<'call, M, T>) -> Self {
        Self { error, invocation }
    }
    /// Returns the preflight error while retaining all original inputs.
    pub fn error(&self) -> &InvocationError {
        &self.error
    }
    /// Recovers the original pinned receiver and arguments.
    pub fn into_invocation(self) -> PinnedInvocation<'call, M, T> {
        self.invocation
    }
}

impl<'call> Invocation<'call, Local> {
    /// Creates an associated-function invocation in the default local mode.
    pub fn associated() -> Self {
        Self {
            receiver: None,
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with an owned local receiver.
    pub fn on_owned<T: ValueFor<Local>>(receiver: T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Owned(DynamicOwned::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a shared local receiver.
    pub fn on_ref<T: ValueForRef<Local> + ?Sized>(receiver: &'call T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Ref(DynamicRef::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a mutable local receiver.
    pub fn on_mut<T: ValueForMut<Local> + ?Sized>(receiver: &'call mut T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Mut(DynamicMut::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation from a pre-erased receiver.
    pub fn on_dynamic(receiver: InvocationReceiver<'call, Local>) -> Self {
        Self {
            receiver: Some(receiver),
            bindings: Vec::new(),
        }
    }
}

impl<'call> Invocation<'call, ThreadSafe> {
    /// Creates an associated-function invocation in thread-safe mode.
    pub fn associated() -> Self {
        Self {
            receiver: None,
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with an owned thread-safe receiver.
    pub fn on_owned<T: ValueFor<ThreadSafe>>(receiver: T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Owned(DynamicOwned::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a shared thread-safe receiver.
    pub fn on_ref<T: ValueForRef<ThreadSafe> + ?Sized>(receiver: &'call T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Ref(DynamicRef::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a mutable thread-safe receiver.
    pub fn on_mut<T: ValueForMut<ThreadSafe> + ?Sized>(receiver: &'call mut T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Mut(DynamicMut::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation from a pre-erased receiver.
    pub fn on_dynamic(receiver: InvocationReceiver<'call, ThreadSafe>) -> Self {
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
    pub fn into_parts(self) -> (Option<InvocationReceiver<'call, M>>, Box<[InvocationValue<'call, M>]>) {
        let invocation = self.invocation.reorder(self.order);
        (
            invocation.receiver,
            invocation.bindings.into_iter().map(|binding| binding.value).collect(),
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
    let parameters = method.parameters().iter().collect::<Vec<_>>();
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
                    if parameters
                        .iter()
                        .any(|parameter| !matches!(parameter.pattern(), ParameterPattern::Identifier(_))) =>
                {
                    return Err(InvocationErrorKind::NamedArgumentUnavailable { name: name.clone() });
                }
                (None, _) => {
                    return Err(InvocationErrorKind::UnknownArgumentName { name: name.clone() });
                }
                (Some((index, _)), None) => index,
                _ => {
                    return Err(InvocationErrorKind::DuplicateArgumentBinding { name: name.clone() });
                }
            }
        } else {
            occupied
                .iter()
                .position(|is_occupied| !is_occupied)
                .ok_or(InvocationErrorKind::TooManyArguments {
                    expected: parameters.len(),
                    actual: invocation.bindings.len(),
                })?
        };
        if occupied[parameter_index] {
            return Err(InvocationErrorKind::DuplicateArgumentBinding {
                name: binding
                    .name
                    .clone()
                    .unwrap_or_else(|| parameters[parameter_index].name().unwrap_or("<pattern>").into()),
            });
        }
        occupied[parameter_index] = true;
        order.push(parameter_index);
    }

    if let Some((actual_kind, actual_type)) = pinned_receiver {
        match method.receiver() {
            None => return Err(InvocationErrorKind::UnexpectedReceiver),
            Some(expected) => {
                let mode_matches = matches!(
                    (expected.kind, actual_kind),
                    (ReceiverKind::PinnedShared, ReceiverKind::PinnedShared)
                        | (ReceiverKind::PinnedShared, ReceiverKind::PinnedMutable)
                        | (ReceiverKind::PinnedMutable, ReceiverKind::PinnedMutable)
                );
                if !mode_matches {
                    return Err(InvocationErrorKind::ReceiverModeMismatch);
                }
                let expected_type = (expected
                    .type_metadata
                    .expect("effective method receiver is bound to its target"))()
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
                if matches!(expected.kind, ReceiverKind::PinnedShared | ReceiverKind::PinnedMutable) {
                    return Err(InvocationErrorKind::ReceiverAdapterUnavailable);
                }
                let mode_matches = match expected.kind {
                    ReceiverKind::Owned => actual.kind() == DynamicValueKind::Owned,
                    ReceiverKind::Shared => {
                        matches!(actual.kind(), DynamicValueKind::Ref | DynamicValueKind::Mut)
                    }
                    ReceiverKind::Mutable => actual.kind() == DynamicValueKind::Mut,
                    ReceiverKind::PinnedShared | ReceiverKind::PinnedMutable => false,
                };
                if !mode_matches {
                    return Err(InvocationErrorKind::ReceiverModeMismatch);
                }
                let expected_type = (expected
                    .type_metadata
                    .expect("effective method receiver is bound to its target"))()
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
            return Err(InvocationErrorKind::ArgumentModeMismatch { index: parameter_index });
        }
        let Some(expected_type) = parameter.find_concrete_type().map(|ty| ty.type_id()) else {
            return Err(InvocationErrorKind::ArgumentTypeUnavailable { index: parameter_index });
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
    let mut indexed = order.into_iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(_, parameter_index)| *parameter_index);
    Ok(indexed)
}

fn passing_matches(parameter: ParameterDescriptor<'_>, value: DynamicValueKind) -> bool {
    matches!(
        (parameter.passing(), value),
        (PassingMode::Owned, DynamicValueKind::Owned)
            | (PassingMode::Shared, DynamicValueKind::Ref)
            | (PassingMode::Shared, DynamicValueKind::Mut)
            | (PassingMode::Mutable, DynamicValueKind::Mut)
    )
}

/// A mode-specific availability result for a specialization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationAvailability {
    /// The specialization has an adapter for the selected value mode.
    Callable,
    /// The specialization cannot be called for the listed reasons.
    Unavailable {
        /// Conditions that prevent constructing a callable adapter.
        reasons: &'static [InvocationUnavailableReason],
    },
}

/// Reasons a method cannot cross the dynamic invocation boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvocationUnavailableReason {
    /// The method's receiver form is not supported by the adapter.
    UnsupportedReceiver,
    /// No generated adapter exists for this value mode.
    AdapterUnavailable,
    /// A parameter pattern cannot be bound dynamically.
    UnsupportedParameterPattern,
    /// The method has generic parameters without a selected specialization.
    UnspecializedGeneric,
    /// The method is unsafe and cannot be invoked through the safe API.
    UnsafeMethod,
    /// The method uses an unsupported ABI.
    UnsupportedAbi,
    /// The method is variadic.
    Variadic,
    /// The return borrow cannot be tied to supported input origins.
    UnsupportedBorrowedReturn,
    /// The return type is opaque to the generated adapter.
    OpaqueReturn,
    /// The method requires an unsized value form not supported by the adapter.
    UnsupportedUnsizedValue,
    /// A trait constraint needed by the adapter could not be proven.
    UnprovenTraitConstraint,
    /// Invocation policy explicitly disabled this method.
    DisabledByPolicy,
}

/// Invocation preflight and adapter errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvocationError {
    kind: InvocationErrorKind,
}

impl InvocationError {
    pub(crate) const fn new(kind: InvocationErrorKind) -> Self {
        Self { kind }
    }
    /// Returns the structured reason preflight rejected the invocation.
    pub fn kind(&self) -> &InvocationErrorKind {
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
pub enum InvocationErrorKind {
    /// A generic method needs a concrete specialization before invocation.
    SpecializationRequired,
    /// Multiple specializations match; the caller must select one.
    SpecializationSelectionRequired {
        /// Number of matching specializations.
        count: usize,
    },
    /// The selected method is unavailable in the current mode or policy.
    MethodUnavailable {
        /// Reasons the method cannot be invoked.
        reasons: &'static [InvocationUnavailableReason],
    },
    /// The current visibility policy denies access to the method.
    VisibilityDenied,
    /// An unsafe method cannot be called through the safe invocation API.
    UnsafeMethod,
    /// The method requires a receiver but none was provided.
    ReceiverMissing,
    /// An associated method was given a receiver.
    UnexpectedReceiver,
    /// The provided receiver ownership/borrowing form does not match the
    /// method.
    ReceiverModeMismatch,
    /// The receiver's concrete type differs from the method's target type.
    ReceiverTypeMismatch {
        /// Concrete type required by the method.
        expected: TypeId,
        /// Concrete type supplied by the caller.
        actual: TypeId,
    },
    /// No typed adapter accepts this receiver type and pin mode.
    ReceiverAdapterUnavailable,
    /// A named binding does not identify a method parameter.
    UnknownArgumentName {
        /// Name that was not found.
        name: Box<str>,
    },
    /// Parameter names are unavailable for this method declaration.
    NamedArgumentUnavailable {
        /// Name supplied by the caller.
        name: Box<str>,
    },
    /// The same parameter was bound more than once.
    DuplicateArgumentBinding {
        /// Name of the duplicated parameter.
        name: Box<str>,
    },
    /// No caller binding was supplied for the parameter.
    MissingArgument {
        /// Zero-based parameter index.
        index: usize,
    },
    /// More bindings were supplied than the method declares.
    TooManyArguments {
        /// Number of declared parameters.
        expected: usize,
        /// Number of supplied bindings.
        actual: usize,
    },
    /// An argument's ownership or borrowing form does not match its parameter.
    ArgumentModeMismatch {
        /// Zero-based parameter index.
        index: usize,
    },
    /// The parameter's concrete type is not available for validation.
    ArgumentTypeUnavailable {
        /// Zero-based parameter index.
        index: usize,
    },
    /// An argument's concrete type differs from the parameter type.
    ArgumentTypeMismatch {
        /// Zero-based parameter index.
        index: usize,
        /// Concrete type required by the parameter.
        expected: TypeId,
        /// Concrete type supplied by the caller.
        actual: TypeId,
    },
    /// No adapter is available for the selected value mode.
    ValueModeUnavailable,
}

/// Inputs retained after an invocation preflight failure.
pub struct InvocationRecovery<'call, M: ValueMode>(Invocation<'call, M>);

impl<'call, M: ValueMode> InvocationRecovery<'call, M> {
    /// Borrows the original invocation retained after preflight validation
    /// failed.
    pub fn as_invocation(&self) -> &Invocation<'call, M> {
        &self.0
    }

    /// Recovers the original invocation retained after preflight failed.
    pub fn into_invocation(self) -> Invocation<'call, M> {
        self.0
    }
}

/// An invocation error paired with all original, unconsumed inputs.
pub struct InvocationFailure<'call, M: ValueMode> {
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
    /// Returns the preflight error while preserving the original inputs.
    pub fn error(&self) -> &InvocationError {
        &self.error
    }
    /// Borrows the original invocation retained after preflight failed.
    pub fn recovery(&self) -> &InvocationRecovery<'call, M> {
        &self.recovery
    }
    /// Separates the error from the retained recovery inputs.
    pub fn into_parts(self) -> (InvocationError, InvocationRecovery<'call, M>) {
        (self.error, self.recovery)
    }
    /// Recovers the original invocation and discards the error.
    pub fn into_invocation(self) -> Invocation<'call, M> {
        self.recovery.into_invocation()
    }
}

impl<'call, M: ValueMode> RecoverableFailure for InvocationFailure<'call, M> {
    type Error = InvocationError;
    type Recovery = InvocationRecovery<'call, M>;

    fn error(&self) -> &Self::Error {
        &self.error
    }

    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        RecoverableFailureParts::new(self.error, self.recovery).into_parts()
    }
}

/// Result of an invocation attempt that may reject inputs or catch a panic.
pub(crate) enum InvocationAttempt<'call, M: ValueMode> {
    Rejected(InvocationFailure<'call, M>),
    Panicked(InvocationPanic),
}

/// A panic raised by a user adapter after invocation started.
#[derive(Debug)]
pub struct InvocationPanic {
    message: Option<Box<str>>,
    payload: Box<dyn std::any::Any + Send + 'static>,
}

impl InvocationPanic {
    pub(crate) fn from_payload(payload: Box<dyn std::any::Any + Send>) -> Self {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&'static str>().copied())
            .map(Into::into);
        Self { message, payload }
    }
    /// Returns the original panic payload without consuming this panic value.
    ///
    /// The payload retains Rust's panic payload bounds: it is `Any + Send +
    /// 'static`, and is not required to be `Sync`.
    pub fn payload(&self) -> &(dyn std::any::Any + Send + 'static) {
        &*self.payload
    }
    /// Returns a string panic payload when one was provided.
    ///
    /// Returns `None` for non-string payloads; the original payload remains
    /// available through [`Self::payload`].
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

/// Failure from explicit panic-catching invocation.
pub enum CaughtInvocationFailure<'call, M: ValueMode> {
    /// Preflight rejected the inputs before calling the method.
    Rejected(InvocationFailure<'call, M>),
    /// The method adapter panicked after invocation began.
    Panicked(InvocationPanic),
}

/// The origin of a borrow returned from a reflected method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BorrowOrigin {
    /// The borrow comes from the method receiver.
    Receiver,
    /// The borrow comes from the zero-based parameter at this index.
    Parameter(usize),
}

/// An asynchronous result whose executor is selected by the caller.
pub struct ReflectedFuture<'call, M: ValueMode> {
    future: <M as ModeAdapter>::Future<'call, InvocationOutput<'call, M>>,
}

impl<'call, M: ValueMode> ReflectedFuture<'call, M> {
    pub(crate) fn new<F>(future: F) -> Self
    where
        F: IntoModeFuture<M, InvocationOutput<'call, M>> + 'call,
        InvocationOutput<'call, M>: 'call,
    {
        Self { future: future.erase() }
    }
}
impl<'call, M: ValueMode> Future for ReflectedFuture<'call, M> {
    type Output = InvocationOutput<'call, M>;
    fn poll(self: Pin<&mut Self>, context: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        Pin::new(&mut this.future).poll(context)
    }
}

/// Converts a concrete invocation future to the storage type selected by its
/// value mode.
///
/// `Local` accepts futures that are not `Send`; the `ThreadSafe` implementation
/// exists only for futures that satisfy `Send`.
#[allow(private_bounds)]
pub(crate) trait IntoModeFuture<M: ModeAdapter, O>: Future<Output = O> {
    fn erase<'a>(self) -> M::Future<'a, O>
    where
        Self: 'a,
        O: 'a;
}

impl<F, O> IntoModeFuture<Local, O> for F
where
    F: Future<Output = O>,
{
    fn erase<'a>(self) -> <Local as ModeAdapter>::Future<'a, O>
    where
        Self: 'a,
        O: 'a,
    {
        Box::pin(self)
    }
}

impl<F, O> IntoModeFuture<ThreadSafe, O> for F
where
    F: Future<Output = O> + Send,
{
    fn erase<'a>(self) -> <ThreadSafe as ModeAdapter>::Future<'a, O>
    where
        Self: 'a,
        O: 'a,
    {
        Box::pin(self)
    }
}

/// A reflected method's result, retaining the borrow origins of reference
/// outputs.
#[must_use]
pub enum InvocationOutput<'call, M: ValueMode = Local> {
    /// Unit return value.
    Unit,
    /// Owned return value.
    Owned(DynamicOwned<M>),
    /// Shared return borrow and the inputs from which it may originate.
    Ref {
        /// The returned shared value.
        value: DynamicRef<'call, M>,
        /// Receiver or parameter origins permitted by the method signature.
        origins: Box<[BorrowOrigin]>,
    },
    /// Optional shared return borrow and its possible input origins.
    OptionalRef {
        /// `None` when the method returned no value.
        value: Option<DynamicRef<'call, M>>,
        /// Receiver or parameter origins permitted by the method signature.
        origins: Box<[BorrowOrigin]>,
    },
    /// A collection of shared return borrows and their possible input origins.
    RefSlice {
        /// Returned shared values.
        values: Box<[DynamicRef<'call, M>]>,
        /// Receiver or parameter origins permitted by the method signature.
        origins: Box<[BorrowOrigin]>,
    },
    /// Mutable return borrow and its input origin.
    Mut {
        /// The returned mutable value.
        value: DynamicMut<'call, M>,
        /// Receiver or parameter from which the value is borrowed.
        origin: BorrowOrigin,
    },
    /// Asynchronous return value.
    Future(ReflectedFuture<'call, M>),
}

/// Keeps the output when a typed output extraction expects another kind.
pub struct OutputMismatch<O> {
    output: O,
}
impl<O> OutputMismatch<O> {
    /// Recovers the output that did not match the requested extraction type.
    pub fn into_output(self) -> O {
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
    pub fn shared_with_origins<T: ValueForRef<M> + ?Sized>(value: &'call T, origins: &[BorrowOrigin]) -> Self {
        Self::Ref {
            value: DynamicRef::new(value),
            origins: origins.into(),
        }
    }

    /// Erases an optional shared result and records all possible borrow
    /// origins.
    pub fn optional_shared_with_origins<T: ValueForRef<M> + ?Sized>(
        value: Option<&'call T>,
        origins: &[BorrowOrigin],
    ) -> Self {
        Self::OptionalRef {
            value: value.map(DynamicRef::new),
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
    #[allow(private_bounds)]
    pub fn future<F>(future: F) -> Self
    where
        F: IntoModeFuture<M, Self> + 'call,
    {
        Self::Future(ReflectedFuture::new(future))
    }

    /// Extracts unit, returning the unchanged output on a different result
    /// kind.
    pub fn into_unit(self) -> Result<(), OutputMismatch<Self>> {
        match self {
            Self::Unit => Ok(()),
            output => Err(OutputMismatch { output }),
        }
    }
    /// Extracts an owned value of type `T`.
    ///
    /// Returns the original output if the result is not owned or has another
    /// concrete type.
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
    /// Extracts a shared value of type `T`.
    ///
    /// Returns the original output if its kind or concrete type differs.
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
    /// Extracts a mutable value of type `T`.
    ///
    /// Returns the original output if its kind or concrete type differs.
    pub fn into_mut<T: ValueForMut<M> + ?Sized>(self) -> Result<&'call mut T, OutputMismatch<Self>> {
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
    /// Extracts an optional shared value of type `T`.
    ///
    /// Returns `Ok(None)` when the method returned no value and retains the
    /// output in `Err` if its kind or concrete type differs.
    pub fn into_optional_ref<T: ValueForRef<M> + ?Sized>(self) -> Result<Option<&'call T>, OutputMismatch<Self>> {
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
    /// Extracts a boxed slice of shared values of type `T`.
    ///
    /// Returns the original output if its kind or any element's concrete type
    /// differs.
    pub fn into_ref_slice<T: ValueForRef<M> + ?Sized>(self) -> Result<Box<[&'call T]>, OutputMismatch<Self>> {
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
    /// Extracts the asynchronous result, retaining other output kinds on error.
    pub fn into_future(self) -> Result<ReflectedFuture<'call, M>, OutputMismatch<Self>> {
        match self {
            Self::Future(future) => Ok(future),
            output => Err(OutputMismatch { output }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::Context;
    use std::task::Poll;
    use std::task::Waker;

    use super::BorrowOrigin;
    use super::DynamicRef;
    use super::FunctionInvocationAdapter;
    use super::Invocation;
    use super::InvocationAdapter;
    use super::InvocationAdapters;
    use super::InvocationError;
    use super::InvocationErrorKind;
    use super::InvocationFailure;
    use super::InvocationOutput;
    use super::InvocationReceiver;
    use super::Local;
    use super::ModeAdapter;
    use super::PinnedFunctionInvocationAdapter;
    use super::PinnedInvocation;
    use super::PinnedInvocationAdapter;
    use super::PinnedReceiver;
    use super::ThreadSafe;
    use super::ValidatedInvocation;
    use crate::runtime::error::RecoverableFailure;

    fn unit_local<'call>(_: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
        InvocationOutput::unit()
    }

    fn unit_thread_safe<'call>(_: ValidatedInvocation<'call, ThreadSafe>) -> InvocationOutput<'call, ThreadSafe> {
        InvocationOutput::unit()
    }

    fn pinned_unit_local<'call>(
        _: PinnedReceiver<'call>,
        _: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
        InvocationOutput::unit()
    }

    fn pinned_unit_thread_safe<'call>(
        _: PinnedReceiver<'call>,
        _: ValidatedInvocation<'call, ThreadSafe>,
    ) -> InvocationOutput<'call, ThreadSafe> {
        InvocationOutput::unit()
    }

    static LOCAL_ADAPTER: FunctionInvocationAdapter<Local> = FunctionInvocationAdapter::new(unit_local);
    static THREAD_SAFE_ADAPTER: FunctionInvocationAdapter<ThreadSafe> =
        FunctionInvocationAdapter::new(unit_thread_safe);
    static CATCHING_LOCAL_ADAPTER: FunctionInvocationAdapter<Local> = FunctionInvocationAdapter::new(unit_local);
    static CATCHING_THREAD_SAFE_ADAPTER: FunctionInvocationAdapter<ThreadSafe> =
        FunctionInvocationAdapter::new(unit_thread_safe);
    static PINNED_LOCAL_ADAPTER: PinnedFunctionInvocationAdapter<Local, u32> =
        PinnedFunctionInvocationAdapter::shared(pinned_unit_local);
    static PINNED_THREAD_SAFE_ADAPTER: PinnedFunctionInvocationAdapter<ThreadSafe, u32> =
        PinnedFunctionInvocationAdapter::shared(pinned_unit_thread_safe);

    #[test]
    fn adapter_selection_uses_the_requested_value_mode_and_call_policy() {
        let adapters = InvocationAdapters {
            local: Some(&LOCAL_ADAPTER),
            thread_safe: Some(&THREAD_SAFE_ADAPTER),
            catching_local: Some(&CATCHING_LOCAL_ADAPTER),
            catching_thread_safe: Some(&CATCHING_THREAD_SAFE_ADAPTER),
            pinned_local: Some(&PINNED_LOCAL_ADAPTER),
            pinned_thread_safe: Some(&PINNED_THREAD_SAFE_ADAPTER),
        };

        assert!(<Local as ModeAdapter>::select_adapter(&adapters).is_some());
        assert!(<ThreadSafe as ModeAdapter>::select_adapter(&adapters).is_some());
        assert!(<Local as ModeAdapter>::select_catching_adapter(&adapters).is_some());
        assert!(<ThreadSafe as ModeAdapter>::select_catching_adapter(&adapters).is_some());
        assert!(<Local as ModeAdapter>::select_pinned_adapter(&adapters).is_some());
        assert!(<ThreadSafe as ModeAdapter>::select_pinned_adapter(&adapters).is_some());

        let empty = InvocationAdapters::builder().build();
        assert!(<Local as ModeAdapter>::select_adapter(&empty).is_none());
        assert!(<ThreadSafe as ModeAdapter>::select_catching_adapter(&empty).is_none());
        assert!(<Local as ModeAdapter>::select_pinned_adapter(&empty).is_none());
    }

    #[test]
    fn recoverable_failure_retains_all_original_invocation_bindings() {
        let invocation = Invocation::<Local>::associated().arg(17_u32).named_arg("label", "kept");
        let failure = InvocationFailure::new(
            InvocationError::new(InvocationErrorKind::ValueModeUnavailable),
            invocation,
        );

        assert_eq!(failure.error().kind(), &InvocationErrorKind::ValueModeUnavailable);
        assert_eq!(failure.recovery().as_invocation().argument_count(), 2);

        let (error, recovery) = RecoverableFailure::into_parts(failure);
        assert_eq!(error.kind(), &InvocationErrorKind::ValueModeUnavailable);
        let recovered = recovery.into_invocation();
        assert_eq!(recovered.argument_count(), 2);
    }

    #[test]
    fn failed_argument_replacement_returns_the_unchanged_invocation() {
        let invocation = Invocation::<Local>::associated().arg(17_u32).named_arg("label", "kept");
        let invocation = match invocation.replace_arg(3, 99_u32) {
            Ok(_) => panic!("out-of-range replacement must preserve inputs"),
            Err(invocation) => invocation,
        };

        assert_eq!(invocation.argument_count(), 2);
    }

    #[test]
    fn owned_output_type_mismatch_can_be_recovered_and_retried() {
        let mismatch = InvocationOutput::<Local>::owned(42_u32)
            .into_owned::<String>()
            .expect_err("the requested type differs from the output type");
        let recovered = mismatch
            .into_output()
            .into_owned::<u32>()
            .unwrap_or_else(|_| panic!("the original owned value remains available"));

        assert_eq!(recovered, 42);
    }

    #[test]
    fn shared_output_type_mismatch_preserves_its_borrow_origins() {
        let value = 12_u32;
        let mismatch = InvocationOutput::<Local>::shared_with_origins(
            &value,
            &[BorrowOrigin::Receiver, BorrowOrigin::Parameter(2)],
        )
        .into_ref::<String>()
        .expect_err("the requested reference type differs from the result");

        let output = mismatch.into_output();
        let InvocationOutput::Ref {
            value: reference,
            origins,
        } = output
        else {
            panic!("the mismatched shared output must remain shared");
        };
        assert_eq!(origins.as_ref(), &[BorrowOrigin::Receiver, BorrowOrigin::Parameter(2)]);
        assert_eq!(*reference.downcast::<u32>().expect("original reference"), 12);
    }

    #[test]
    fn optional_and_slice_output_mismatches_keep_the_original_values() {
        let value = 23_u32;
        assert!(
            InvocationOutput::<Local>::optional_shared_with_origins(None::<&u32>, &[])
                .into_optional_ref::<u32>()
                .unwrap_or_else(|_| panic!("an empty optional result is a successful extraction"))
                .is_none()
        );

        let mismatch =
            InvocationOutput::<Local>::optional_shared_with_origins(Some(&value), &[BorrowOrigin::Parameter(0)])
                .into_optional_ref::<String>()
                .expect_err("the optional result has another concrete type");
        let recovered = mismatch
            .into_output()
            .into_optional_ref::<u32>()
            .unwrap_or_else(|_| panic!("the optional value remains available"))
            .expect("the original optional result is populated");
        assert_eq!(*recovered, 23);

        let output: InvocationOutput<'_, Local> = InvocationOutput::RefSlice {
            values: vec![DynamicRef::<Local>::new(&value), DynamicRef::<Local>::new(&value)].into_boxed_slice(),
            origins: vec![BorrowOrigin::Parameter(1)].into_boxed_slice(),
        };
        let mismatch = output
            .into_ref_slice::<String>()
            .expect_err("slice elements have another concrete type");
        let recovered = mismatch
            .into_output()
            .into_ref_slice::<u32>()
            .unwrap_or_else(|_| panic!("all original slice elements remain available"));
        assert_eq!(recovered.iter().map(|value| **value).collect::<Vec<_>>(), [23, 23]);
    }

    #[test]
    fn local_future_adapter_accepts_a_non_send_future() {
        let local_state = std::rc::Rc::new(31_u32);
        let output = InvocationOutput::<Local>::future(async move { InvocationOutput::owned(*local_state) });
        let mut future = output.into_future().unwrap_or_else(|_| panic!("future output"));
        let mut context = Context::from_waker(Waker::noop());

        let Poll::Ready(output) = Future::poll(Pin::new(&mut future), &mut context) else {
            panic!("ready async block should complete on its first poll");
        };
        assert_eq!(
            output.into_owned::<u32>().unwrap_or_else(|_| panic!("future result")),
            31
        );
    }

    #[test]
    fn pinned_receiver_downcasts_preserve_the_pin_and_reject_other_types() {
        let shared = Box::pin(41_u32);
        let (receiver, _) = PinnedInvocation::<Local, u32>::shared(shared.as_ref()).into_parts();
        assert_eq!(receiver.type_id(), std::any::TypeId::of::<u32>());
        assert!(!receiver.is_mutable());
        assert_eq!(*receiver.downcast_ref::<u32>().expect("matching shared receiver"), 41);

        let shared = Box::pin(42_u32);
        let (receiver, _) = PinnedInvocation::<Local, u32>::shared(shared.as_ref()).into_parts();
        assert!(receiver.downcast_ref::<String>().is_none());

        let mut mutable = Box::pin(43_u32);
        let (receiver, _) = PinnedInvocation::<Local, u32>::mutable(mutable.as_mut()).into_parts();
        assert!(receiver.is_mutable());
        *receiver
            .downcast_mut::<u32>()
            .expect("matching mutable receiver")
            .get_mut() += 1;
        assert_eq!(*mutable, 44);

        let mut mutable = Box::pin(45_u32);
        let (receiver, _) = PinnedInvocation::<Local, u32>::mutable(mutable.as_mut()).into_parts();
        assert!(receiver.downcast_mut::<String>().is_none());
        assert_eq!(*mutable, 45);

        let mutable_adapter = PinnedFunctionInvocationAdapter::<Local, u32>::mutable(pinned_unit_local);
        assert_eq!(
            mutable_adapter.receiver_kind(),
            crate::runtime::descriptor::method::ReceiverKind::PinnedMutable
        );
        assert_eq!(mutable_adapter.receiver_type_id(), std::any::TypeId::of::<u32>());
    }

    #[test]
    fn invocation_receiver_type_mismatch_returns_the_original_receiver() {
        let owned = InvocationReceiver::<Local>::Owned(crate::runtime::value::DynamicOwned::new(17_u32));
        let owned = owned
            .into_owned::<String>()
            .expect_err("a different concrete type must retain the owned receiver");
        assert_eq!(
            owned
                .into_owned::<u32>()
                .unwrap_or_else(|_| panic!("recover owned receiver")),
            17
        );

        let value = 18_u32;
        let shared = InvocationReceiver::<Local>::Ref(DynamicRef::new(&value));
        let shared = shared
            .into_ref::<String>()
            .expect_err("a different concrete type must retain the shared receiver");
        assert_eq!(
            *shared
                .into_ref::<u32>()
                .unwrap_or_else(|_| panic!("recover shared receiver")),
            18
        );

        let mut value = 19_u32;
        let mutable = InvocationReceiver::<Local>::Mut(crate::runtime::value::DynamicMut::new(&mut value));
        let mutable = mutable
            .into_mut::<String>()
            .expect_err("a different concrete type must retain the mutable receiver");
        *mutable
            .into_mut::<u32>()
            .unwrap_or_else(|_| panic!("recover mutable receiver")) += 1;
        assert_eq!(value, 20);
    }

    #[test]
    fn borrowed_and_named_arguments_keep_their_caller_bindings() {
        let shared = 30_u32;
        let mut mutable = 31_u32;
        let mut named_mutable = 32_u32;
        let invocation = Invocation::<Local>::associated()
            .arg_ref(&shared)
            .arg_mut(&mut mutable)
            .named_arg_ref("shared", &shared)
            .named_arg_mut("mutable", &mut named_mutable);
        assert_eq!(invocation.argument_count(), 4);

        let pinned_receiver = Box::pin(33_u32);
        let mut pinned_mutable = 34_u32;
        let mut pinned_named_mutable = 35_u32;
        let pinned = PinnedInvocation::<Local, u32>::shared(pinned_receiver.as_ref())
            .arg_ref(&shared)
            .arg_mut(&mut pinned_mutable)
            .named_arg("owned", 36_u32)
            .named_arg_ref("shared", &shared)
            .named_arg_mut("mutable", &mut pinned_named_mutable);
        assert_eq!(pinned.argument_count(), 5);
    }

    #[test]
    fn output_extraction_mismatch_retains_other_kinds_and_mutable_borrow_origin() {
        let unit = InvocationOutput::<Local>::unit()
            .into_owned::<u32>()
            .expect_err("unit is not an owned value")
            .into_output();
        assert!(unit.into_unit().is_ok());

        let value = 27_u32;
        let ref_slice = InvocationOutput::RefSlice {
            values: vec![DynamicRef::<Local>::new(&value)].into_boxed_slice(),
            origins: vec![BorrowOrigin::Parameter(3)].into_boxed_slice(),
        };
        let recovered = ref_slice
            .into_ref::<u32>()
            .expect_err("a slice result is not a single reference")
            .into_output()
            .into_ref_slice::<u32>()
            .unwrap_or_else(|_| panic!("the original slice result remains available"));
        assert_eq!(recovered.iter().map(|item| **item).collect::<Vec<_>>(), [27]);

        let mut value = 28_u32;
        let mismatch = InvocationOutput::<Local>::mutable(&mut value, BorrowOrigin::Receiver)
            .into_mut::<String>()
            .expect_err("the mutable result has another concrete type");
        let output = mismatch.into_output();
        let InvocationOutput::Mut { origin, .. } = &output else {
            panic!("the original mutable output must be retained");
        };
        assert_eq!(*origin, BorrowOrigin::Receiver);
        *output
            .into_mut::<u32>()
            .unwrap_or_else(|_| panic!("recover original mutable result")) += 1;
        assert_eq!(value, 29);
    }

    #[test]
    fn thread_safe_invocation_accepts_dynamic_and_associated_receivers() {
        let associated = Invocation::<ThreadSafe>::associated().arg(7_u32);
        assert_eq!(associated.argument_count(), 1);
        assert!(associated.receiver.is_none());

        let value = 8_u32;
        let dynamic = Invocation::<ThreadSafe>::on_dynamic(InvocationReceiver::Ref(DynamicRef::new(&value)));
        assert!(matches!(dynamic.receiver, Some(InvocationReceiver::Ref(_))));
    }

    #[test]
    fn validated_invocation_reorders_bindings_and_can_recover_original_order() {
        let invocation = Invocation::<Local>::associated().arg(10_u32).arg(20_u32);
        let validated = ValidatedInvocation {
            invocation,
            order: vec![(0, 1), (1, 0)],
        };

        let (receiver, arguments) = validated.into_parts();
        assert!(receiver.is_none());
        let mut arguments = arguments.into_vec().into_iter();
        let super::InvocationValue::Owned(first) = arguments.next().expect("first reordered argument") else {
            panic!("reordered argument stays owned");
        };
        assert_eq!(first.downcast::<u32>().expect("argument type"), 20);
        let super::InvocationValue::Owned(second) = arguments.next().expect("second reordered argument") else {
            panic!("reordered argument stays owned");
        };
        assert_eq!(second.downcast::<u32>().expect("argument type"), 10);

        let validated = ValidatedInvocation {
            invocation: Invocation::<Local>::associated().arg(30_u32),
            order: vec![(0, 0)],
        };
        assert_eq!(validated.into_invocation().argument_count(), 1);
    }

    #[test]
    fn function_adapters_execute_validated_local_and_thread_safe_calls() {
        let local = FunctionInvocationAdapter::<Local>::new(unit_local);
        let output = local.invoke(ValidatedInvocation {
            invocation: Invocation::<Local>::associated(),
            order: Vec::new(),
        });
        assert!(output.into_unit().is_ok());

        let thread_safe = FunctionInvocationAdapter::<ThreadSafe>::new(unit_thread_safe);
        let output = thread_safe.invoke(ValidatedInvocation {
            invocation: Invocation::<ThreadSafe>::associated(),
            order: Vec::new(),
        });
        assert!(output.into_unit().is_ok());
    }

    #[test]
    fn invocation_errors_and_panic_payloads_keep_their_display_and_message() {
        let error = InvocationError::new(InvocationErrorKind::ReceiverMissing);
        assert_eq!(error.to_string(), "ReceiverMissing");

        let string = super::InvocationPanic::from_payload(Box::new(String::from("owned panic")));
        assert_eq!(string.message(), Some("owned panic"));
        assert_eq!(
            string.payload().downcast_ref::<String>().map(String::as_str),
            Some("owned panic")
        );

        let literal = super::InvocationPanic::from_payload(Box::new("literal panic"));
        assert_eq!(literal.message(), Some("literal panic"));

        let other = super::InvocationPanic::from_payload(Box::new(42_u32));
        assert_eq!(other.message(), None);
        assert_eq!(other.payload().downcast_ref::<u32>(), Some(&42));
    }
}
