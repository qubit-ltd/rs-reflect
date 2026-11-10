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

    /// Selects the generated adapter for ordinary validated calls in this value
    /// mode. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>>
    where
        Self: ValueMode;
    /// Selects the adapter that catches panics raised by the generated method
    /// body. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_catching_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>>
    where
        Self: ValueMode;
    /// Selects the adapter that receives the receiver through its pinned
    /// representation. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_pinned_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn PinnedInvocationAdapter<Self>>
    where
        Self: ValueMode;
}

impl ModeAdapter for Local {
    /// Future type produced by this mode; thread-safe mode additionally
    /// requires `Send`.
    type Future<'a, O: 'a> = Pin<Box<dyn Future<Output = O> + 'a>>;

    /// Selects the generated adapter for ordinary validated calls in this value
    /// mode. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.local
    }
    /// Selects the adapter that catches panics raised by the generated method
    /// body. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_catching_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.catching_local
    }
    /// Selects the adapter that receives the receiver through its pinned
    /// representation. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_pinned_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn PinnedInvocationAdapter<Self>> {
        adapters.pinned_local
    }
}

impl ModeAdapter for ThreadSafe {
    /// Future type produced by this mode; thread-safe mode additionally
    /// requires `Send`.
    type Future<'a, O: 'a> = Pin<Box<dyn Future<Output = O> + Send + 'a>>;

    /// Selects the generated adapter for ordinary validated calls in this value
    /// mode. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.thread_safe
    }
    /// Selects the adapter that catches panics raised by the generated method
    /// body. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_catching_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn InvocationAdapter<Self>> {
        adapters.catching_thread_safe
    }
    /// Selects the adapter that receives the receiver through its pinned
    /// representation. # Parameters
    ///
    /// - `adapters`: Generated invocation adapters registered for the method.
    /// # Parameters
    ///
    /// - `adapters`: Generated adapter set from which this mode selects its
    ///   invocation entry point.
    fn select_pinned_adapter(adapters: &InvocationAdapters) -> Option<&'static dyn PinnedInvocationAdapter<Self>> {
        adapters.pinned_thread_safe
    }
}

/// A mode-specific adapter generated for one statically bound Rust method.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub trait InvocationAdapter<M: ValueMode>: Sync {
    /// Executes a previously validated invocation.
    ///
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>;
}

/// Function-pointer adapter for generated method bodies.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct FunctionInvocationAdapter<M: ValueMode> {
    /// Runs the generated method body after invocation inputs have passed
    /// preflight validation.
    invoke: for<'call> fn(ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    /// Carries the selected value mode in the type without storing a runtime
    /// value.
    _mode: std::marker::PhantomData<fn() -> M>,
}

impl<M: ValueMode> FunctionInvocationAdapter<M> {
    /// Binds a generated function to the invocation adapter contract.
    ///
    /// # Parameters
    ///
    /// - `invoke`: Generated function pointer that executes the already
    ///   validated call.
    pub const fn new(invoke: for<'call> fn(ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>) -> Self {
        Self {
            invoke,
            _mode: std::marker::PhantomData,
        }
    }
}

impl<M: ValueMode> InvocationAdapter<M> for FunctionInvocationAdapter<M> {
    /// Runs the generated method body after invocation inputs have passed
    /// preflight validation. # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// # Parameters
    ///
    /// - `invocation`: Validated receiver and arguments passed to the generated
    ///   adapter.
    fn invoke<'call>(&self, invocation: ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M> {
        (self.invoke)(invocation)
    }
}

/// Adapter for a method whose receiver is pinned.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub(crate) trait PinnedInvocationAdapter<M: ValueMode>: Sync {
    /// Returns the concrete receiver type used to guard typed pinned recovery.
    fn receiver_type_id(&self) -> TypeId;
    /// Returns whether the pinned receiver grants shared or mutable access.
    fn receiver_kind(&self) -> ReceiverKind;

    /// Runs the generated method body after invocation inputs have passed
    /// preflight validation. # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// # Parameters
    ///
    /// - `receiver`: Pinned receiver value checked against the adapter’s exact
    ///   receiver type.
    /// - `invocation`: Validated receiver and arguments passed to the generated
    ///   adapter.
    fn invoke<'call>(
        &self,
        receiver: PinnedReceiver<'call>,
        invocation: ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M>;
}

/// Function-pointer adapter for a receiver whose type and pin mode are fixed.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
pub struct PinnedFunctionInvocationAdapter<M: ValueMode, T: ?Sized + 'static> {
    /// Records whether the pinned receiver grants shared or mutable access.
    kind: ReceiverKind,
    /// Runs the generated method body after invocation inputs have passed
    /// preflight validation.
    invoke: for<'call> fn(PinnedReceiver<'call>, ValidatedInvocation<'call, M>) -> InvocationOutput<'call, M>,
    /// Carries the pinned receiver type in the adapter without storing a
    /// receiver.
    _receiver: std::marker::PhantomData<fn(&T)>,
    /// Carries the selected value mode in the type without storing a runtime
    /// value.
    _mode: std::marker::PhantomData<fn() -> M>,
}

impl<M: ValueMode, T: ?Sized + 'static> PinnedFunctionInvocationAdapter<M, T> {
    /// Creates a pinned shared-receiver adapter for the exact type `T`.
    ///
    /// # Parameters
    ///
    /// - `invoke`: Generated function pointer that executes the already
    ///   validated call.
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
    ///
    /// # Parameters
    ///
    /// - `invoke`: Generated function pointer that executes the already
    ///   validated call.
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
    /// Returns the concrete receiver type used to guard typed pinned recovery.
    fn receiver_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    /// Returns whether the pinned receiver grants shared or mutable access.
    fn receiver_kind(&self) -> ReceiverKind {
        self.kind
    }

    /// Runs the generated method body after invocation inputs have passed
    /// preflight validation. # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
    /// # Parameters
    ///
    /// - `receiver`: Pinned receiver value checked against the adapter’s exact
    ///   receiver type.
    /// - `invocation`: Validated receiver and arguments passed to the generated
    ///   adapter.
    fn invoke<'call>(
        &self,
        receiver: PinnedReceiver<'call>,
        invocation: ValidatedInvocation<'call, M>,
    ) -> InvocationOutput<'call, M> {
        (self.invoke)(receiver, invocation)
    }
}

/// Carries the erasedpinnedreceiver required by invocation dispatch.
trait ErasedPinnedReceiver {
    /// Returns the concrete type identity represented by this erased receiver
    /// holder.
    fn type_id(&self) -> TypeId;
    /// Exposes the stable holder address used for the checked concrete
    /// downcast.
    fn holder_ptr(&mut self) -> *mut ();
    /// Reports whether the erased receiver grants mutable access.
    fn is_mutable(&self) -> bool;
}

/// Retains the PinnedSharedHolder required to validate or recover this
/// invocation. # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
/// # Type Parameters
///
/// - `'call`: Lifetime of the invocation borrows retained by its receiver or
///   result.
/// - `T`: Concrete value type represented by the operation.
struct PinnedSharedHolder<'call, T: ?Sized + 'static> {
    /// Retains the typed pinned receiver until the generated adapter recovers
    /// it.
    receiver: Pin<&'call T>,
}

impl<T: ?Sized + 'static> ErasedPinnedReceiver for PinnedSharedHolder<'_, T> {
    /// Returns the concrete type identity represented by this erased receiver
    /// holder.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Exposes the stable holder address used for the checked concrete
    /// downcast.
    fn holder_ptr(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }
    /// Reports whether the erased receiver grants mutable access.
    fn is_mutable(&self) -> bool {
        false
    }
}

/// Retains the PinnedMutableHolder required to validate or recover this
/// invocation. # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
/// # Type Parameters
///
/// - `'call`: Lifetime of the invocation borrows retained by its receiver or
///   result.
/// - `T`: Concrete value type represented by the operation.
struct PinnedMutableHolder<'call, T: ?Sized + 'static> {
    /// Retains the typed pinned receiver until the generated adapter recovers
    /// it.
    receiver: Option<Pin<&'call mut T>>,
}

impl<T: ?Sized + 'static> ErasedPinnedReceiver for PinnedMutableHolder<'_, T> {
    /// Returns the concrete type identity represented by this erased receiver
    /// holder.
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Exposes the stable holder address used for the checked concrete
    /// downcast.
    fn holder_ptr(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }
    /// Reports whether the erased receiver grants mutable access.
    fn is_mutable(&self) -> bool {
        true
    }
}

/// A pinned receiver erased for a generated typed adapter. The holder retains
/// the typed Pin, including any DST metadata, until the adapter downcasts it.
pub struct PinnedReceiver<'call> {
    /// Stores the runtime representation retained by this public wrapper.
    inner: PinnedReceiverKind<'call>,
}

/// Carries the pinnedreceiverkind required by invocation dispatch.
enum PinnedReceiverKind<'call> {
    /// Carries the receiver through shared pinned access.
    Shared(
        /// Type-erased holder that retains the typed pinned borrow.
        Box<dyn ErasedPinnedReceiver + 'call>,
    ),
    /// Carries the receiver through exclusive pinned access.
    Mutable(
        /// Type-erased holder that retains the typed pinned mutable borrow.
        Box<dyn ErasedPinnedReceiver + 'call>,
    ),
}

impl<'call> PinnedReceiver<'call> {
    /// Returns the concrete type identity represented by this erased receiver
    /// holder.
    fn type_id(&self) -> TypeId {
        match &self.inner {
            PinnedReceiverKind::Shared(holder) | PinnedReceiverKind::Mutable(holder) => {
                ErasedPinnedReceiver::type_id(holder.as_ref())
            }
        }
    }
    /// Reports whether the erased receiver grants mutable access.
    fn is_mutable(&self) -> bool {
        matches!(self.inner, PinnedReceiverKind::Mutable(_))
    }

    /// Recovers the pinned shared receiver as `T` when its type matches.
    ///
    /// Returns `None` for a different receiver type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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

    /// Recovers the pinned mutable receiver as `T` when its type matches.
    ///
    /// Returns `None` for a different receiver type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
                let holder = unsafe { &mut *(holder.holder_ptr() as *mut PinnedMutableHolder<'call, T>) };
                holder.receiver.take()
            }
            _ => None,
        }
    }
}

/// A shared or mutable receiver passed to an invocation.
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub enum InvocationReceiver<'call, M: ValueMode> {
    /// An owned receiver.
    Owned(
        /// Value transferred into the reflected method call.
        DynamicOwned<M>,
    ),
    /// A shared borrowed receiver.
    Ref(
        /// Shared borrow whose lifetime bounds the invocation.
        DynamicRef<'call, M>,
    ),
    /// An exclusively borrowed receiver.
    Mut(
        /// Mutable borrow retained for the duration of the invocation.
        DynamicMut<'call, M>,
    ),
}

impl<'call, M: ValueMode> InvocationReceiver<'call, M> {
    /// Records whether the pinned receiver grants shared or mutable access.
    fn kind(&self) -> DynamicValueKind {
        match self {
            Self::Owned(_) => DynamicValueKind::Owned,
            Self::Ref(_) => DynamicValueKind::Ref,
            Self::Mut(_) => DynamicValueKind::Mut,
        }
    }
    /// Returns the concrete type identity represented by this erased receiver
    /// holder.
    fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => value.type_id(),
            Self::Ref(value) => value.clone().type_id(),
            Self::Mut(value) => value.type_id(),
        }
    }

    /// Recovers an owned receiver of its validated concrete type.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M = Local`: Generic type or lifetime parameter carried by the descriptor.
pub(crate) type InvocationArgument<'call, M = Local> = InvocationValue<'call, M>;
/// A receiver supplied in owned, shared, or mutable form.
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M = Local`: Generic type or lifetime parameter carried by the descriptor.
pub(crate) type DynamicReceiver<'call, M = Local> = InvocationReceiver<'call, M>;

/// Retains the Binding required to validate or recover this invocation.
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
struct Binding<'call, M: ValueMode> {
    /// Stores the name used to resolve or execute this reflected invocation.
    name: Option<Box<str>>,
    /// Stores the value used to resolve or execute this reflected invocation.
    value: InvocationValue<'call, M>,
}

/// Caller-owned receiver and argument bindings for one invocation.
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct Invocation<'call, M: ValueMode = Local> {
    /// Retains the typed pinned receiver until the generated adapter recovers
    /// it.
    receiver: Option<InvocationReceiver<'call, M>>,
    /// Owned receiver and argument values supplied to the invocation.
    bindings: Vec<Binding<'call, M>>,
}

impl<'call, M: ValueMode> Invocation<'call, M> {
    /// Creates an invocation with no receiver or argument bindings.
    fn empty() -> Self {
        Self {
            receiver: None,
            bindings: Vec::new(),
        }
    }
    /// Adds an owned positional argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg<T: ValueFor<M>>(self, value: T) -> Self {
        self.arg_dynamic(DynamicOwned::new(value))
    }
    /// Adds a shared positional argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_ref<T: ValueForRef<M> + ?Sized>(self, value: &'call T) -> Self {
        self.arg_dynamic_ref(DynamicRef::new(value))
    }
    /// Adds a mutable positional argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_mut<T: ValueForMut<M> + ?Sized>(self, value: &'call mut T) -> Self {
        self.arg_dynamic_mut(DynamicMut::new(value))
    }
    /// Adds a named owned argument.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn named_arg<T: ValueFor<M>>(self, name: impl Into<Box<str>>, value: T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Owned(DynamicOwned::new(value)))
    }
    /// Adds a named shared argument.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn named_arg_ref<T: ValueForRef<M> + ?Sized>(self, name: impl Into<Box<str>>, value: &'call T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Ref(DynamicRef::new(value)))
    }
    /// Adds a named mutable argument.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn named_arg_mut<T: ValueForMut<M> + ?Sized>(self, name: impl Into<Box<str>>, value: &'call mut T) -> Self {
        self.named_arg_dynamic(name, InvocationValue::Mut(DynamicMut::new(value)))
    }
    /// Adds an already erased owned argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_dynamic(mut self, value: DynamicOwned<M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Owned(value),
        });
        self
    }
    /// Adds an already erased shared argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_dynamic_ref(mut self, value: DynamicRef<'call, M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Ref(value),
        });
        self
    }
    /// Adds an already erased mutable argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_dynamic_mut(mut self, value: DynamicMut<'call, M>) -> Self {
        self.bindings.push(Binding {
            name: None,
            value: InvocationValue::Mut(value),
        });
        self
    }
    /// Adds a named already erased argument in caller order.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
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
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field position to inspect.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based field position to inspect.
    /// - `value`: Owned value retained for validation and construction.
    pub(crate) fn replace_arg<T: ValueFor<M>>(mut self, index: usize, value: T) -> Result<Self, Self> {
        if index >= self.bindings.len() {
            return Err(self);
        }
        self.bindings[index].value = InvocationValue::Owned(DynamicOwned::new(value));
        Ok(self)
    }

    /// Checks receiver and argument shape and returns a typed invocation only
    /// on success. # Parameters
    ///
    /// - `method`: Method descriptor whose parameter and receiver metadata
    ///   govern this call.
    /// # Parameters
    ///
    /// - `method`: Method descriptor that defines receiver and argument
    ///   validation for this call.
    pub(crate) fn validate(
        self,
        method: MethodDescriptor<'_>,
    ) -> Result<ValidatedInvocation<'call, M>, InvocationFailure<'call, M>> {
        match validate_bindings(&self, method, None, None) {
            Ok(order) => Ok(ValidatedInvocation {
                invocation: self,
                order,
            }),
            Err(error) => Err(InvocationFailure::new(InvocationError::new(error), self)),
        }
    }

    /// Reorders named bindings into the method descriptor’s declared parameter
    /// order. # Parameters
    ///
    /// - `order`: Mapping from supplied bindings to the method’s declared
    ///   parameter order.
    /// # Parameters
    ///
    /// - `order`: Mapping that restores the method’s declared parameter order.
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
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
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn shared(receiver: Pin<&'call T>) -> Self {
        Self::Shared {
            receiver,
            arguments: Invocation::empty(),
        }
    }
    /// Starts an invocation with a pinned mutable receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn mutable(receiver: Pin<&'call mut T>) -> Self {
        Self::Mutable {
            receiver,
            arguments: Invocation::empty(),
        }
    }
    /// Adds an owned positional argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `U`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg<U: ValueFor<M>>(mut self, value: U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Owned(DynamicOwned::new(value)),
        });
        self
    }
    /// Adds a shared positional argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `U`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_ref<U: ValueForRef<M> + ?Sized>(mut self, value: &'call U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Ref(DynamicRef::new(value)),
        });
        self
    }
    /// Adds a mutable positional argument.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `U`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn arg_mut<U: ValueForMut<M> + ?Sized>(mut self, value: &'call mut U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: None,
            value: InvocationValue::Mut(DynamicMut::new(value)),
        });
        self
    }
    /// Adds a named owned argument.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `U`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn named_arg<U: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Owned(DynamicOwned::new(value)),
        });
        self
    }
    /// Adds a named shared argument.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `U`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn named_arg_ref<U: ValueForRef<M> + ?Sized>(mut self, name: impl Into<Box<str>>, value: &'call U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Ref(DynamicRef::new(value)),
        });
        self
    }
    /// Adds a named mutable argument.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `U`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `value`: Owned value retained for validation and construction.
    pub fn named_arg_mut<U: ValueForMut<M> + ?Sized>(mut self, name: impl Into<Box<str>>, value: &'call mut U) -> Self {
        self.arguments_mut().bindings.push(Binding {
            name: Some(name.into()),
            value: InvocationValue::Mut(DynamicMut::new(value)),
        });
        self
    }
    /// Returns the number of supplied argument bindings.
    pub(crate) fn argument_count(&self) -> usize {
        self.arguments().bindings.len()
    }
    /// Returns the concrete receiver type used to guard typed pinned recovery.
    pub(crate) fn receiver_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    /// Reports whether the invocation receiver carries exclusive access.
    pub(crate) fn receiver_is_mutable(&self) -> bool {
        matches!(self, Self::Mutable { .. })
    }
    /// Returns the argument bindings in their current supplied order.
    fn arguments(&self) -> &Invocation<'call, M> {
        match self {
            Self::Shared { arguments, .. } | Self::Mutable { arguments, .. } => arguments,
        }
    }
    /// Returns mutable access to the supplied bindings for validated
    /// reordering.
    fn arguments_mut(&mut self) -> &mut Invocation<'call, M> {
        match self {
            Self::Shared { arguments, .. } | Self::Mutable { arguments, .. } => arguments,
        }
    }
    /// Returns the pinned receiver kind and concrete type identity when
    /// present.
    pub(crate) fn pinned_kind_and_type(&self) -> (ReceiverKind, TypeId) {
        match self {
            Self::Shared { .. } => (ReceiverKind::PinnedShared, TypeId::of::<T>()),
            Self::Mutable { .. } => (ReceiverKind::PinnedMutable, TypeId::of::<T>()),
        }
    }
    /// Separates the error from the original invocation retained for recovery.
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
    /// Checks receiver and argument shape and returns a typed invocation only
    /// on success. # Parameters
    ///
    /// - `method`: Method descriptor whose parameter and receiver metadata
    ///   govern this call.
    /// # Parameters
    ///
    /// - `method`: Method descriptor that defines receiver and argument
    ///   validation for this call.
    pub(crate) fn validate(
        self,
        method: MethodDescriptor<'_>,
        parameters: Option<&'static [super::descriptor::method::ParameterMetadata]>,
    ) -> Result<ValidatedPinnedInvocation<'call, M, T>, PinnedInvocationFailure<'call, M, T>> {
        let (kind, type_id) = self.pinned_kind_and_type();
        let order = match validate_bindings(self.arguments(), method, Some((kind, type_id)), parameters) {
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
pub struct ValidatedPinnedInvocation<'call, M: ValueMode, T: ?Sized + 'static> {
    /// Retains the invocation required to validate or recover this invocation.
    invocation: PinnedInvocation<'call, M, T>,
    /// Parameter-order mapping produced by invocation preflight validation.
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
/// - `T`: Concrete Rust type whose metadata or value is used by the operation.
pub struct PinnedInvocationFailure<'call, M: ValueMode, T: ?Sized + 'static> {
    /// Returns the structured error that explains why invocation failed.
    error: InvocationError,
    /// Retains the invocation required to validate or recover this invocation.
    invocation: PinnedInvocation<'call, M, T>,
}

impl<'call, M: ValueMode, T: ?Sized + 'static> PinnedInvocationFailure<'call, M, T> {
    /// Creates a wrapper that retains the supplied runtime representation.
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
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
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_owned<T: ValueFor<Local>>(receiver: T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Owned(DynamicOwned::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a shared local receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_ref<T: ValueForRef<Local> + ?Sized>(receiver: &'call T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Ref(DynamicRef::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a mutable local receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_mut<T: ValueForMut<Local> + ?Sized>(receiver: &'call mut T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Mut(DynamicMut::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation from a pre-erased receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
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
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_owned<T: ValueFor<ThreadSafe>>(receiver: T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Owned(DynamicOwned::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a shared thread-safe receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_ref<T: ValueForRef<ThreadSafe> + ?Sized>(receiver: &'call T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Ref(DynamicRef::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation with a mutable thread-safe receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_mut<T: ValueForMut<ThreadSafe> + ?Sized>(receiver: &'call mut T) -> Self {
        Self {
            receiver: Some(InvocationReceiver::Mut(DynamicMut::new(receiver))),
            bindings: Vec::new(),
        }
    }
    /// Creates an invocation from a pre-erased receiver.
    ///
    /// # Parameters
    ///
    /// - `receiver`: Receiver value and access mode supplied by the caller.
    pub fn on_dynamic(receiver: InvocationReceiver<'call, ThreadSafe>) -> Self {
        Self {
            receiver: Some(receiver),
            bindings: Vec::new(),
        }
    }
}

/// An invocation after declaration-order binding and all dynamic checks passed.
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct ValidatedInvocation<'call, M: ValueMode> {
    /// Retains the invocation required to validate or recover this invocation.
    invocation: Invocation<'call, M>,
    /// Parameter-order mapping produced by invocation preflight validation.
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
    /// Returns the invocation after its arguments have been reordered for the
    /// method.
    pub(crate) fn into_invocation(self) -> Invocation<'call, M> {
        self.invocation
    }
}

/// Provides the validate bindings operation used by invocation validation or
/// recovery. # Parameters
///
/// - `invocation`: Validated call values passed to the generated method
///   adapter.
/// - `method`: Method descriptor whose parameter and receiver metadata govern
///   this call.
/// - `pinned_receiver`: Pinned receiver retained until its typed adapter has
///   finished.
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
///
/// # Parameters
///
/// - `invocation`: Validated call values passed to the generated method
///   adapter.
/// - `method`: Method descriptor whose parameter and receiver metadata govern
///   this call.
/// - `pinned_receiver`: Pinned receiver retained until its typed adapter has
///   finished.
fn validate_bindings<M: ValueMode>(
    invocation: &Invocation<'_, M>,
    method: MethodDescriptor<'_>,
    pinned_receiver: Option<(ReceiverKind, TypeId)>,
    concrete_parameters: Option<&'static [super::descriptor::method::ParameterMetadata]>,
) -> Result<Vec<(usize, usize)>, InvocationErrorKind> {
    let parameters = method.parameters().iter().collect::<Vec<_>>();
    if let Some(concrete) = concrete_parameters
        && (concrete.len() != parameters.len()
            || concrete.iter().zip(&parameters).any(|(binding, source)| {
                binding.index() != source.index()
                    || binding.passing() != source.passing()
                    || !matches!(
                        (binding.pattern(), source.pattern()),
                        (ParameterPattern::Identifier(_), ParameterPattern::Identifier(_))
                            | (ParameterPattern::Wildcard, ParameterPattern::Wildcard)
                            | (ParameterPattern::Destructure, ParameterPattern::Destructure)
                            | (
                                ParameterPattern::DestructurePattern(_),
                                ParameterPattern::DestructurePattern(_)
                            )
                    )
            }))
    {
        return Err(InvocationErrorKind::ArgumentTypeUnavailable { index: 0 });
    }
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
        let expected_type = concrete_parameters
            .and_then(|concrete| concrete[parameter_index].concrete_type_id())
            .or_else(|| parameter.find_concrete_type().map(|ty| ty.type_id()));
        let Some(expected_type) = expected_type else {
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

/// Checks whether a runtime value kind is compatible with a parameter passing
/// mode. # Parameters
///
/// - `parameter`: Declared parameter considered during argument matching.
/// - `value`: Owned value retained for validation and construction.
/// # Parameters
///
/// - `parameter`: Parameter metadata used when checking argument passing
///   compatibility.
/// - `value`: Owned value retained for validation and construction.
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
    /// Records whether the pinned receiver grants shared or mutable access.
    kind: InvocationErrorKind,
}

impl InvocationError {
    /// Creates a wrapper that retains the supplied runtime representation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Structured preflight or adapter error to expose.
    ///
    /// # Returns
    ///
    /// Returns an invocation error containing `kind`.
    pub(crate) const fn new(kind: InvocationErrorKind) -> Self {
        Self { kind }
    }
    /// Returns the structured reason preflight rejected the invocation.
    pub fn kind(&self) -> &InvocationErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for InvocationError {
    /// Formats the structured invocation failure for human-readable output.
    /// # Parameters
    ///
    /// - `formatter`: Formatter receiving the human-readable representation.
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct InvocationRecovery<'call, M: ValueMode>(
    /// Original receiver and argument values retained after preflight failure.
    Invocation<'call, M>,
);

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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct InvocationFailure<'call, M: ValueMode> {
    /// Returns the structured error that explains why invocation failed.
    error: InvocationError,
    /// Borrows the original invocation inputs retained after failure.
    recovery: InvocationRecovery<'call, M>,
}

impl<'call, M: ValueMode> InvocationFailure<'call, M> {
    /// Creates a wrapper that retains the supplied runtime representation.
    /// # Parameters
    ///
    /// - `invocation`: Validated call values passed to the generated method
    ///   adapter.
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
    /// Error type exposed by the recoverable failure contract.
    type Error = InvocationError;
    /// Owned input values preserved by the recoverable failure contract.
    type Recovery = InvocationRecovery<'call, M>;

    /// Returns the structured error that explains why invocation failed.
    fn error(&self) -> &Self::Error {
        &self.error
    }

    /// Borrows the original invocation inputs retained after failure.
    fn recovery(&self) -> &Self::Recovery {
        &self.recovery
    }

    /// Separates the error from the original invocation retained for recovery.
    fn into_parts(self) -> (Self::Error, Self::Recovery) {
        RecoverableFailureParts::new(self.error, self.recovery).into_parts()
    }
}

/// Result of an invocation attempt that may reject inputs or catch a panic.
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub(crate) enum InvocationAttempt<'call, M: ValueMode> {
    /// Carries the rejected required by invocation dispatch.
    Rejected(
        /// Preflight error and original inputs retained for recovery.
        InvocationFailure<'call, M>,
    ),
    /// Carries the panicked required by invocation dispatch.
    Panicked(
        /// Panic payload raised after the adapter started running.
        InvocationPanic,
    ),
}

/// A panic raised by a user adapter after invocation started.
#[derive(Debug)]
pub struct InvocationPanic {
    /// Returns the panic message when the payload can be converted to text.
    message: Option<Box<str>>,
    /// Returns a reference to the original panic payload for typed inspection.
    payload: Box<dyn std::any::Any + Send + 'static>,
}

impl InvocationPanic {
    /// Captures a panic payload and its displayable message when available.
    ///
    /// # Parameters
    ///
    /// - `payload`: Original panic value returned by the unwind mechanism.
    ///
    /// # Returns
    ///
    /// Returns the payload with its string message extracted when possible.
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub enum CaughtInvocationFailure<'call, M: ValueMode> {
    /// Preflight rejected the inputs before calling the method.
    Rejected(
        /// Error and original invocation values retained for recovery.
        InvocationFailure<'call, M>,
    ),
    /// The method adapter panicked after invocation began.
    Panicked(
        /// Captured panic payload and optional message.
        InvocationPanic,
    ),
}

/// The origin of a borrow returned from a reflected method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BorrowOrigin {
    /// The borrow comes from the method receiver.
    Receiver,
    /// The borrow comes from the zero-based parameter at this index.
    Parameter(
        /// Zero-based index into the method's declared parameter list.
        usize,
    ),
}

/// An asynchronous result whose executor is selected by the caller.
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
pub struct ReflectedFuture<'call, M: ValueMode> {
    /// Returns mutable access to the future retained by this invocation output.
    future: <M as ModeAdapter>::Future<'call, InvocationOutput<'call, M>>,
}

impl<'call, M: ValueMode> ReflectedFuture<'call, M> {
    /// Creates a wrapper that retains the supplied runtime representation.
    ///
    /// # Type Parameters
    ///
    /// - `F`: Concrete future produced by the generated method adapter.
    ///
    /// # Parameters
    ///
    /// - `future`: Future retained until the caller polls it to completion.
    ///
    /// # Returns
    ///
    /// Returns a future wrapper using the storage type selected by `M`.
    pub(crate) fn new<F>(future: F) -> Self
    where
        F: IntoModeFuture<M, InvocationOutput<'call, M>> + 'call,
        InvocationOutput<'call, M>: 'call,
    {
        Self { future: future.erase() }
    }
}
impl<'call, M: ValueMode> Future for ReflectedFuture<'call, M> {
    /// Result type yielded by the stored invocation future.
    type Output = InvocationOutput<'call, M>;
    /// Polls the stored future without changing its invocation output type.
    /// # Parameters
    ///
    /// - `context`: Reflection context used to resolve source expressions and
    ///   type metadata.
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
///
/// # Type Parameters
///
/// - `M`: Value mode that governs storage and thread-safety bounds.
/// - `O`: Concrete Rust type whose metadata or value is used by the operation.
#[allow(private_bounds)]
pub(crate) trait IntoModeFuture<M: ModeAdapter, O>: Future<Output = O> {
    /// Converts the concrete future into the future representation selected by
    /// the value mode.
    fn erase<'a>(self) -> M::Future<'a, O>
    where
        Self: 'a,
        O: 'a;
}

impl<F, O> IntoModeFuture<Local, O> for F
where
    F: Future<Output = O>,
{
    /// Converts the concrete future into the future representation selected by
    /// the value mode.
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
    /// Converts the concrete future into the future representation selected by
    /// the value mode.
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
///
/// # Type Parameters
///
/// - `'call`: Generic type or lifetime parameter carried by the descriptor.
/// - `M`: Value mode that governs storage and thread-safety bounds.
#[must_use]
pub enum InvocationOutput<'call, M: ValueMode = Local> {
    /// Unit return value.
    Unit,
    /// Owned return value.
    Owned(
        /// Dynamically typed owned result returned by the method.
        DynamicOwned<M>,
    ),
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
    Future(
        /// Future whose polling and `Send` requirements follow `M`.
        ReflectedFuture<'call, M>,
    ),
}

/// Keeps the output when a typed output extraction expects another kind.
///
/// # Type Parameters
///
/// - `O`: Concrete Rust type whose metadata or value is used by the operation.
pub struct OutputMismatch<O> {
    /// Output value together with any borrow origins retained from the call.
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
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    pub fn owned<T: ValueFor<M>>(value: T) -> Self {
        Self::Owned(DynamicOwned::new(value))
    }

    /// Erases a shared result and records which validated input it borrows.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origin`: Borrow origin associated with a returned reference.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origin`: Borrow origin associated with a returned reference.
    pub fn shared<T: ValueForRef<M> + ?Sized>(value: &'call T, origin: BorrowOrigin) -> Self {
        Self::shared_with_origins(value, &[origin])
    }

    /// Erases a shared result with all validated input origins it may borrow.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origins`: Allowed input origins for a returned reference.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origins`: Allowed input origins for a returned reference.
    pub fn shared_with_origins<T: ValueForRef<M> + ?Sized>(value: &'call T, origins: &[BorrowOrigin]) -> Self {
        Self::Ref {
            value: DynamicRef::new(value),
            origins: origins.into(),
        }
    }

    /// Erases an optional shared result and records all possible borrow
    /// origins.
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origins`: Allowed input origins for a returned reference.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origins`: Allowed input origins for a returned reference.
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
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origin`: Borrow origin associated with a returned reference.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `value`: Owned value retained for validation and construction.
    /// - `origin`: Borrow origin associated with a returned reference.
    pub fn mutable<T: ValueForMut<M> + ?Sized>(value: &'call mut T, origin: BorrowOrigin) -> Self {
        Self::Mut {
            value: DynamicMut::new(value),
            origin,
        }
    }

    /// Erases an asynchronous result for the invocation's selected value mode.
    ///
    /// # Parameters
    ///
    /// - `future`: Concrete future returned by the method adapter.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `F`: Generic type or lifetime parameter carried by the descriptor.
    ///
    ///
    /// # Parameters
    ///
    /// - `future`: Concrete future returned by the method adapter.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
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
    fn test_adapter_selection_uses_the_requested_value_mode_and_call_policy() {
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
    fn test_recoverable_failure_retains_all_original_invocation_bindings() {
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
    fn test_failed_argument_replacement_returns_the_unchanged_invocation() {
        let invocation = Invocation::<Local>::associated().arg(17_u32).named_arg("label", "kept");
        let invocation = match invocation.replace_arg(3, 99_u32) {
            Ok(_) => panic!("out-of-range replacement must preserve inputs"),
            Err(invocation) => invocation,
        };

        assert_eq!(invocation.argument_count(), 2);
    }

    #[test]
    fn test_owned_output_type_mismatch_can_be_recovered_and_retried() {
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
    fn test_shared_output_type_mismatch_preserves_its_borrow_origins() {
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
    fn test_optional_and_slice_output_mismatches_keep_the_original_values() {
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
    fn test_local_future_adapter_accepts_a_non_send_future() {
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
    fn test_pinned_receiver_downcasts_preserve_the_pin_and_reject_other_types() {
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
    fn test_invocation_receiver_type_mismatch_returns_the_original_receiver() {
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
    fn test_borrowed_and_named_arguments_keep_their_caller_bindings() {
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
    fn test_output_extraction_mismatch_retains_other_kinds_and_mutable_borrow_origin() {
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
    fn test_thread_safe_invocation_accepts_dynamic_and_associated_receivers() {
        let associated = Invocation::<ThreadSafe>::associated().arg(7_u32);
        assert_eq!(associated.argument_count(), 1);
        assert!(associated.receiver.is_none());

        let value = 8_u32;
        let dynamic = Invocation::<ThreadSafe>::on_dynamic(InvocationReceiver::Ref(DynamicRef::new(&value)));
        assert!(matches!(dynamic.receiver, Some(InvocationReceiver::Ref(_))));
    }

    #[test]
    fn test_validated_invocation_reorders_bindings_and_can_recover_original_order() {
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
    fn test_function_adapters_execute_validated_local_and_thread_safe_calls() {
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
    fn test_invocation_errors_and_panic_payloads_keep_their_display_and_message() {
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
