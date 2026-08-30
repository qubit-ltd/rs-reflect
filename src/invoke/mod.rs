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

pub use adapter::CatchingInvocationAdapter;
pub use adapter::CatchingInvocationResult;
pub use adapter::InvocationAdapter;
pub use argument::ArgumentExpectation;
pub use argument::InvocationArg;
pub use argument::InvocationInputMode;
pub use error::InvocationError;
pub use error::InvocationErrorKind;
pub use error::InvocationPanic;
pub use future::InvocationMode;
pub use future::ReflectedFuture;
pub use invocation::Invocation;
pub use invocation::ValidatedInvocation;
pub use output::BorrowOrigin;
pub use output::InvocationOutput;
pub use receiver::InvocationReceiver;
pub use receiver::ReceiverExpectation;
pub use recovery::InvocationFailure;
pub use recovery::InvocationRecovery;
