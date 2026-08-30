// qubit-style: allow multiple-public-types
//! Typed invocation inputs that retain a borrowed receiver's pin guarantee.

use std::fmt;
use std::pin::Pin;

use crate::identity::MemberId;
use crate::invoke::ArgumentExpectation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationError;
use crate::invoke::InvocationErrorKind;
use crate::invoke::InvocationInputMode;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationOutput;

/// A generated entry point for a method whose receiver is `Pin<&T>`.
pub type PinnedRefAdapter<T, M> =
    for<'call> fn(
        PinnedRefInvocation<'call, T, M>,
    ) -> Result<InvocationOutput<'call, M>, PinnedRefInvocationFailure<'call, T, M>>;

/// A generated entry point for a method whose receiver is `Pin<&mut T>`.
pub type PinnedMutAdapter<T, M> =
    for<'call> fn(
        PinnedMutInvocation<'call, T, M>,
    ) -> Result<InvocationOutput<'call, M>, PinnedMutInvocationFailure<'call, T, M>>;

/// Invocation input for a `Pin<&T>` receiver.
///
/// The receiver is never erased, so this type can invoke methods on `!Unpin`
/// values without reconstructing a pin proof from an ordinary reference.
pub struct PinnedRefInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocation<'call, T, M> {
    /// Creates an invocation from a pinned shared receiver and ordered
    /// arguments.
    pub fn new<I>(receiver: Pin<&'call T>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self {
            receiver,
            arguments: arguments.into_iter().collect(),
        }
    }

    /// Returns the pinned shared receiver without weakening its pin guarantee.
    pub const fn receiver(&self) -> Pin<&'call T> {
        self.receiver
    }

    /// Returns arguments in declaration order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Validates argument count, passing modes, and exact erased types.
    ///
    /// On error no input is extracted; the returned failure retains both the
    /// original pin and all arguments for inspection or retry.
    pub fn validate(
        self,
        method_identity: &MemberId,
        arguments: &[ArgumentExpectation],
    ) -> Result<PinnedValidatedRefInvocation<'call, T, M>, PinnedRefInvocationFailure<'call, T, M>> {
        if let Err(kind) = validate_arguments(&self.arguments, arguments) {
            return Err(self.into_failure(method_identity, kind));
        }
        Ok(PinnedValidatedRefInvocation {
            receiver: self.receiver,
            arguments: self.arguments,
        })
    }

    /// Creates a failure retaining the original inputs.
    fn into_failure(
        self,
        method_identity: &MemberId,
        kind: InvocationErrorKind,
    ) -> PinnedRefInvocationFailure<'call, T, M> {
        PinnedRefInvocationFailure {
            error: InvocationError::new(method_identity.clone(), kind),
            recovery: PinnedRefInvocationRecovery {
                receiver: self.receiver,
                arguments: self.arguments,
            },
        }
    }
}

/// Validated input for a `Pin<&T>` receiver method.
pub struct PinnedValidatedRefInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedValidatedRefInvocation<'call, T, M> {
    /// Consumes validation state and returns the pin proof with its arguments.
    pub fn into_parts(self) -> (Pin<&'call T>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}

/// Complete input retained after a pinned shared invocation fails validation.
pub struct PinnedRefInvocationRecovery<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedRefInvocationRecovery<'call, T, M> {
    /// Returns the recovered pinned receiver.
    pub const fn receiver(&self) -> Pin<&'call T> {
        self.receiver
    }

    /// Returns recovered arguments in their original order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Reconstitutes the exact typed invocation for inspection or retry.
    pub fn into_invocation(self) -> PinnedRefInvocation<'call, T, M> {
        PinnedRefInvocation {
            receiver: self.receiver,
            arguments: self.arguments,
        }
    }
}

/// A pinned shared invocation validation error and its complete recovery input.
pub struct PinnedRefInvocationFailure<'call, T: ?Sized, M: InvocationMode> {
    /// Structured reason user code was not called.
    pub error: InvocationError,
    /// Original pinned receiver and arguments.
    pub recovery: PinnedRefInvocationRecovery<'call, T, M>,
}

impl<T: ?Sized, M: InvocationMode> fmt::Debug for PinnedRefInvocationFailure<'_, T, M> {
    /// Formats validation metadata without requiring erased arguments to
    /// implement `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedRefInvocationFailure")
            .field("error", &self.error)
            .finish()
    }
}

impl<T: ?Sized, M: InvocationMode> fmt::Display for PinnedRefInvocationFailure<'_, T, M> {
    /// Formats the underlying structured validation error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T: ?Sized, M: InvocationMode> std::error::Error for PinnedRefInvocationFailure<'_, T, M> {}

/// Invocation input for a `Pin<&mut T>` receiver.
pub struct PinnedMutInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call mut T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedMutInvocation<'call, T, M> {
    /// Creates an invocation from a pinned mutable receiver and ordered
    /// arguments.
    pub fn new<I>(receiver: Pin<&'call mut T>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self {
            receiver,
            arguments: arguments.into_iter().collect(),
        }
    }

    /// Validates arguments while preserving the pinned mutable receiver on
    /// error.
    pub fn validate(
        self,
        method_identity: &MemberId,
        arguments: &[ArgumentExpectation],
    ) -> Result<PinnedValidatedMutInvocation<'call, T, M>, PinnedMutInvocationFailure<'call, T, M>> {
        if let Err(kind) = validate_arguments(&self.arguments, arguments) {
            return Err(self.into_failure(method_identity, kind));
        }
        Ok(PinnedValidatedMutInvocation {
            receiver: self.receiver,
            arguments: self.arguments,
        })
    }

    /// Creates a failure retaining the original inputs.
    fn into_failure(
        self,
        method_identity: &MemberId,
        kind: InvocationErrorKind,
    ) -> PinnedMutInvocationFailure<'call, T, M> {
        PinnedMutInvocationFailure {
            error: InvocationError::new(method_identity.clone(), kind),
            recovery: PinnedMutInvocationRecovery {
                receiver: self.receiver,
                arguments: self.arguments,
            },
        }
    }
}

/// Validated input for a `Pin<&mut T>` receiver method.
pub struct PinnedValidatedMutInvocation<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call mut T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedValidatedMutInvocation<'call, T, M> {
    /// Consumes validation state and returns the pin proof with its arguments.
    pub fn into_parts(self) -> (Pin<&'call mut T>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}

/// Complete input retained after a pinned mutable invocation fails validation.
pub struct PinnedMutInvocationRecovery<'call, T: ?Sized, M: InvocationMode> {
    receiver: Pin<&'call mut T>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, T: ?Sized, M: InvocationMode> PinnedMutInvocationRecovery<'call, T, M> {
    /// Returns a reborrowed pinned mutable receiver.
    pub fn receiver(&mut self) -> Pin<&mut T> {
        self.receiver.as_mut()
    }

    /// Returns recovered arguments in their original order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Reconstitutes the exact typed invocation for inspection or retry.
    pub fn into_invocation(self) -> PinnedMutInvocation<'call, T, M> {
        PinnedMutInvocation {
            receiver: self.receiver,
            arguments: self.arguments,
        }
    }
}

/// A pinned mutable invocation validation error and its complete recovery
/// input.
pub struct PinnedMutInvocationFailure<'call, T: ?Sized, M: InvocationMode> {
    /// Structured reason user code was not called.
    pub error: InvocationError,
    /// Original pinned receiver and arguments.
    pub recovery: PinnedMutInvocationRecovery<'call, T, M>,
}

impl<T: ?Sized, M: InvocationMode> fmt::Debug for PinnedMutInvocationFailure<'_, T, M> {
    /// Formats validation metadata without requiring erased arguments to
    /// implement `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PinnedMutInvocationFailure")
            .field("error", &self.error)
            .finish()
    }
}

impl<T: ?Sized, M: InvocationMode> fmt::Display for PinnedMutInvocationFailure<'_, T, M> {
    /// Formats the underlying structured validation error.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(formatter)
    }
}

impl<T: ?Sized, M: InvocationMode> std::error::Error for PinnedMutInvocationFailure<'_, T, M> {}

/// Validates arguments without extracting any erased value.
fn validate_arguments<M: InvocationMode>(
    actual: &[InvocationArg<'_, M>],
    expected: &[ArgumentExpectation],
) -> Result<(), InvocationErrorKind> {
    if actual.len() != expected.len() {
        return Err(InvocationErrorKind::ArgumentCountMismatch {
            expected: expected.len(),
            actual: actual.len(),
        });
    }
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        if !mode_matches(expected.mode(), actual.mode()) {
            return Err(InvocationErrorKind::ArgumentModeMismatch {
                index,
                expected: expected.mode(),
                actual: actual.mode(),
            });
        }
        if expected.type_id() != actual.type_id() {
            return Err(InvocationErrorKind::ArgumentTypeMismatch {
                index,
                expected: expected.type_id(),
                actual: actual.type_id(),
                expected_name: expected.type_name(),
            });
        }
    }
    Ok(())
}

/// Returns whether an actual argument can safely satisfy an expected mode.
fn mode_matches(expected: InvocationInputMode, actual: InvocationInputMode) -> bool {
    expected == actual || (expected == InvocationInputMode::Ref && actual == InvocationInputMode::Mut)
}
