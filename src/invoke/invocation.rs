//! Invocation collection and all-input validation.

use std::fmt;

use crate::identity::MemberId;
use crate::invoke::ArgumentExpectation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationError;
use crate::invoke::InvocationErrorKind;
use crate::invoke::InvocationFailure;
use crate::invoke::InvocationInputMode;
use crate::invoke::InvocationMode;
use crate::invoke::InvocationReceiver;
use crate::invoke::ReceiverExpectation;
use crate::value::DynamicMut;
use crate::value::DynamicOwned;
use crate::value::DynamicRef;

/// An explicit receiver and ordered positional arguments sharing one call
/// lifetime.
pub struct Invocation<'call, M: InvocationMode> {
    receiver: Option<InvocationReceiver<'call, M>>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, M: InvocationMode> Invocation<'call, M> {
    /// Creates an associated-function invocation without a receiver.
    pub fn associated<I>(arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self::new(None, arguments)
    }

    /// Creates an invocation with an owned receiver.
    pub fn owned<I>(receiver: DynamicOwned<M>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self::new(Some(InvocationReceiver::Owned(receiver)), arguments)
    }

    /// Creates an invocation with a shared borrowed receiver.
    pub fn borrowed<I>(receiver: DynamicRef<'call, M>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self::new(Some(InvocationReceiver::Ref(receiver)), arguments)
    }

    /// Creates an invocation with a mutable borrowed receiver.
    pub fn borrowed_mut<I>(receiver: DynamicMut<'call, M>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self::new(Some(InvocationReceiver::Mut(receiver)), arguments)
    }

    /// Creates an invocation from an optional explicit receiver and arguments.
    pub fn new<I>(receiver: Option<InvocationReceiver<'call, M>>, arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self {
            receiver,
            arguments: arguments.into_iter().collect(),
        }
    }

    /// Returns the supplied receiver, or `None` for an associated function.
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns the positional arguments in declaration order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Validates receiver shape, count, input modes, and exact types.
    ///
    /// A mutable input satisfies a shared-borrow expectation through safe
    /// reborrowing. Every other mode must match exactly; in particular, owned
    /// values are never implicitly borrowed. Any error returns all original
    /// inputs without downcasting or extracting an owned value.
    pub fn validate(
        self,
        method_identity: &MemberId,
        receiver: ReceiverExpectation,
        arguments: &[ArgumentExpectation],
    ) -> Result<ValidatedInvocation<'call, M>, InvocationFailure<'call, M>> {
        if let Err(kind) = self.validate_receiver(receiver) {
            return Err(self.into_failure(method_identity, kind));
        }
        if self.arguments.len() != arguments.len() {
            let kind = InvocationErrorKind::ArgumentCountMismatch {
                expected: arguments.len(),
                actual: self.arguments.len(),
            };
            return Err(self.into_failure(method_identity, kind));
        }
        for (index, (actual, expected)) in self.arguments.iter().zip(arguments).enumerate() {
            if !mode_matches(expected.mode(), actual.mode()) {
                let kind = InvocationErrorKind::ArgumentModeMismatch {
                    index,
                    expected: expected.mode(),
                    actual: actual.mode(),
                };
                return Err(self.into_failure(method_identity, kind));
            }
            if expected.type_id() != actual.type_id() {
                let kind = InvocationErrorKind::ArgumentTypeMismatch {
                    index,
                    expected: expected.type_id(),
                    actual: actual.type_id(),
                    expected_name: expected.type_name(),
                };
                return Err(self.into_failure(method_identity, kind));
            }
        }
        Ok(ValidatedInvocation {
            receiver: self.receiver,
            arguments: self.arguments,
        })
    }

    /// Creates an invocation from parts previously returned by validation or
    /// recovery.
    pub(crate) fn from_parts(
        receiver: Option<InvocationReceiver<'call, M>>,
        arguments: Box<[InvocationArg<'call, M>]>,
    ) -> Self {
        Self { receiver, arguments }
    }

    /// Validates receiver presence, mode, and exact type without consuming it.
    fn validate_receiver(&self, expected: ReceiverExpectation) -> Result<(), InvocationErrorKind> {
        let actual_mode = self.receiver.as_ref().map(InvocationReceiver::mode);
        if !receiver_mode_matches(expected.mode(), actual_mode) {
            return Err(InvocationErrorKind::ReceiverModeMismatch {
                expected: expected.mode(),
                actual: actual_mode,
            });
        }
        if let (Some(expected_type), Some(expected_name), Some(actual)) =
            (expected.type_id(), expected.type_name(), self.receiver.as_ref())
            && expected_type != actual.type_id()
        {
            return Err(InvocationErrorKind::ReceiverTypeMismatch {
                expected: expected_type,
                actual: actual.type_id(),
                expected_name,
            });
        }
        Ok(())
    }

    /// Pairs one validation error with every untouched input.
    fn into_failure(self, method_identity: &MemberId, kind: InvocationErrorKind) -> InvocationFailure<'call, M> {
        InvocationFailure {
            error: InvocationError::new(method_identity.clone(), kind),
            recovery: crate::invoke::InvocationRecovery::new(self.receiver, self.arguments),
        }
    }
}

/// Invocation input proven to match one generated method adapter signature.
///
/// Owned values remain inside their dynamic wrappers until the adapter consumes
/// this state, so validating later inputs cannot lose earlier owned inputs.
pub struct ValidatedInvocation<'call, M: InvocationMode> {
    receiver: Option<InvocationReceiver<'call, M>>,
    arguments: Box<[InvocationArg<'call, M>]>,
}

impl<'call, M: InvocationMode> ValidatedInvocation<'call, M> {
    /// Returns the validated receiver, or `None` for an associated function.
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns the validated positional arguments in declaration order.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Consumes the validation state so an adapter may extract owned values.
    pub fn into_parts(self) -> (Option<InvocationReceiver<'call, M>>, Box<[InvocationArg<'call, M>]>) {
        (self.receiver, self.arguments)
    }
}

impl<M: InvocationMode> fmt::Debug for ValidatedInvocation<'_, M> {
    /// Formats modes and argument count without requiring erased values to be
    /// `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidatedInvocation")
            .field("receiver_mode", &self.receiver.as_ref().map(InvocationReceiver::mode))
            .field("argument_count", &self.arguments.len())
            .finish()
    }
}

/// Returns whether `actual` can safely satisfy `expected`.
fn mode_matches(expected: InvocationInputMode, actual: InvocationInputMode) -> bool {
    expected == actual || (expected == InvocationInputMode::Ref && actual == InvocationInputMode::Mut)
}

/// Applies argument mode compatibility to optional receiver modes.
fn receiver_mode_matches(expected: Option<InvocationInputMode>, actual: Option<InvocationInputMode>) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some(actual)) => mode_matches(expected, actual),
        (None, Some(_)) | (Some(_), None) => false,
    }
}
