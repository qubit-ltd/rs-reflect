// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Outputs returned by reflected method adapters.

use crate::invoke::InvocationMode;
use crate::invoke::ReflectedFuture;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;

/// The invocation input from which a returned borrow may originate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BorrowOrigin {
    /// The output may borrow from the method receiver.
    Receiver,
    /// The output may borrow from the positional parameter at this index.
    Parameter(usize),
}

/// The result produced after a reflected method begins execution.
pub enum InvocationOutput<'call, M: InvocationMode + 'call> {
    /// The method returned `()`.
    Unit,
    /// The method returned an owned value.
    Owned(DynamicOwned<M>),
    /// The method returned a shared borrow with all conservative origins.
    Ref {
        /// Borrowed dynamic value tied to the common call lifetime.
        value: DynamicRef<'call, M>,
        /// Receiver or parameter origins that may supply this borrow.
        origins: Box<[BorrowOrigin]>,
    },
    /// The method returned a mutable borrow with its unique origin.
    Mut {
        /// Mutably borrowed dynamic value tied to the common call lifetime.
        value: DynamicMut<'call, M>,
        /// The receiver or parameter that supplies the exclusive borrow.
        origin: BorrowOrigin,
    },
    /// The async method returned a lazy, mode-preserving future.
    Future(ReflectedFuture<'call, M>),
}
