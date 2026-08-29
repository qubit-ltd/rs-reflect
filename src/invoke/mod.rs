//! Safe runtime contracts for invoking reflected methods.
//!
//! Invocation is split into validation and execution. Validation checks every
//! receiver and argument without extracting owned values. A validation error
//! therefore returns the complete input through
//! [`InvocationRecovery`](crate::invoke::InvocationRecovery).

mod adapter;
mod argument;
mod error;
mod future;
mod invocation;
mod output;
mod receiver;
mod recovery;

pub use adapter::InvocationAdapter;
pub use argument::{ArgumentExpectation, InvocationArg, InvocationInputMode};
pub use error::{InvocationError, InvocationErrorKind, InvocationPanic};
pub use future::{InvocationMode, ReflectedFuture};
pub use invocation::{Invocation, ValidatedInvocation};
pub use output::{BorrowOrigin, InvocationOutput};
pub use receiver::{InvocationReceiver, ReceiverExpectation};
pub use recovery::{InvocationFailure, InvocationRecovery};
