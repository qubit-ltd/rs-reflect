// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Mode-preserving reflected futures.

use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use crate::invoke::InvocationOutput;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;
use crate::value::Local;
use crate::value::Mode;
use crate::value::ThreadSafe;

mod sealed {
    /// Restricts invocation modes to the runtime's two dynamic value modes.
    pub trait Sealed {}
}

/// A dynamic-value mode with a matching erased future boundary.
///
/// [`Local`] accepts futures that are not `Send`; [`ThreadSafe`] retains a
/// `Send` trait-object boundary. The trait is sealed so downstream code cannot
/// substitute a weaker boundary for either mode.
pub trait InvocationMode: Mode + sealed::Sealed + Sized {
    /// Erased future storage preserving this mode's thread boundary.
    #[doc(hidden)]
    type FutureStorage<'call>: Future<Output = InvocationOutput<'call, Self>> + 'call
    where
        Self: 'call;

    /// Returns the exact type identity stored in an owned dynamic value.
    #[doc(hidden)]
    fn owned_type_id(value: &DynamicOwned<Self>) -> TypeId;

    /// Returns the exact type identity stored in a shared dynamic value.
    #[doc(hidden)]
    fn ref_type_id(value: &DynamicRef<'_, Self>) -> TypeId;

    /// Returns the exact type identity stored in a mutable dynamic value.
    #[doc(hidden)]
    fn mut_type_id(value: &DynamicMut<'_, Self>) -> TypeId;
}

impl sealed::Sealed for Local {}

impl InvocationMode for Local {
    type FutureStorage<'call> = Pin<Box<dyn Future<Output = InvocationOutput<'call, Self>> + 'call>>;

    /// Reads the `Any` identity from local owned storage.
    fn owned_type_id(value: &DynamicOwned<Self>) -> TypeId {
        value
            .as_any()
            .unwrap_or_else(|| unreachable!("owned values are Any-compatible"))
            .type_id()
    }

    /// Reads either the `Any` identity or the dedicated `str` identity.
    fn ref_type_id(value: &DynamicRef<'_, Self>) -> TypeId {
        value.as_any().map_or_else(TypeId::of::<str>, std::any::Any::type_id)
    }

    /// Reads either the `Any` identity or the dedicated `str` identity.
    fn mut_type_id(value: &DynamicMut<'_, Self>) -> TypeId {
        value.as_any().map_or_else(TypeId::of::<str>, std::any::Any::type_id)
    }
}

impl sealed::Sealed for ThreadSafe {}

impl InvocationMode for ThreadSafe {
    type FutureStorage<'call> = Pin<Box<dyn Future<Output = InvocationOutput<'call, Self>> + Send + 'call>>;

    /// Reads the `Any` identity from thread-safe owned storage.
    fn owned_type_id(value: &DynamicOwned<Self>) -> TypeId {
        value
            .as_any()
            .unwrap_or_else(|| unreachable!("owned values are Any-compatible"))
            .type_id()
    }

    /// Reads either the `Any` identity or the dedicated `str` identity.
    fn ref_type_id(value: &DynamicRef<'_, Self>) -> TypeId {
        value.as_any().map_or_else(TypeId::of::<str>, std::any::Any::type_id)
    }

    /// Reads either the `Any` identity or the dedicated `str` identity.
    fn mut_type_id(value: &DynamicMut<'_, Self>) -> TypeId {
        value.as_any().map_or_else(TypeId::of::<str>, std::any::Any::type_id)
    }
}

/// A boxed reflected future retaining its call lifetime and dynamic-value mode.
///
/// Constructing this wrapper does not poll the future or choose an executor.
/// Local futures may be non-`Send`; thread-safe futures must be `Send` when
/// constructed and remain `Send` after erasure.
///
/// ```compile_fail
/// use std::rc::Rc;
/// use qubit_reflect::invoke::{InvocationOutput, ReflectedFuture};
/// use qubit_reflect::value::ThreadSafe;
///
/// let local_state = Rc::new(1_u32);
/// let _ = ReflectedFuture::<ThreadSafe>::new(async move {
///     let _value = *local_state;
///     InvocationOutput::Unit
/// });
/// ```
pub struct ReflectedFuture<'call, M: InvocationMode + 'call> {
    storage: M::FutureStorage<'call>,
}

impl<'call> ReflectedFuture<'call, Local> {
    /// Boxes a local future without polling it.
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = InvocationOutput<'call, Local>> + 'call,
    {
        Self {
            storage: Box::pin(future),
        }
    }
}

impl<'call> ReflectedFuture<'call, ThreadSafe> {
    /// Boxes a `Send` future without polling it.
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = InvocationOutput<'call, ThreadSafe>> + Send + 'call,
    {
        Self {
            storage: Box::pin(future),
        }
    }
}

impl<'call> Future for ReflectedFuture<'call, Local> {
    type Output = InvocationOutput<'call, Local>;

    /// Delegates one poll to the erased local future.
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.storage.as_mut().poll(context)
    }
}

impl<'call> Future for ReflectedFuture<'call, ThreadSafe> {
    type Output = InvocationOutput<'call, ThreadSafe>;

    /// Delegates one poll to the erased thread-safe future.
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.storage.as_mut().poll(context)
    }
}
