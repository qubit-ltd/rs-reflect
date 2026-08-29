//! Outputs returned by reflected method adapters.

use crate::invoke::{InvocationMode, ReflectedFuture};
use crate::value::{DynamicMut, DynamicOwned, DynamicRef};

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
