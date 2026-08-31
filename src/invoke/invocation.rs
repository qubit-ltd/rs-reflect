// qubit-style: allow public-type-layout
//! Invocation collection and all-input validation.

use std::fmt;

use crate::descriptor::ParameterDescriptor;
use crate::descriptor::ParameterPatternDescriptor;
use crate::identity::MemberId;
use crate::invoke::ArgumentExpectation;
use crate::invoke::InvocationArg;
use crate::invoke::InvocationBinding;
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

/// An explicit receiver and caller-ordered arguments sharing one call lifetime.
pub struct Invocation<'call, M: InvocationMode> {
    receiver: Option<InvocationReceiver<'call, M>>,
    arguments: Box<[InvocationArg<'call, M>]>,
    argument_names: Box<[Option<Box<str>>]>,
    binding_recovery: Option<Box<BindingRecovery>>,
}

/// Result of converting a validated dynamic receiver through one typed
/// explicit-receiver capability.
pub type ReceiverAdaptationResult<'call, R, M> =
    Result<(R, Box<[InvocationArg<'call, M>]>), InvocationFailure<'call, M>>;

/// Caller-order metadata retained while a generated adapter validates inputs.
struct BindingRecovery {
    caller_index_for_argument: Box<[usize]>,
    caller_names: Box<[Option<Box<str>>]>,
}

impl<'call, M: InvocationMode> Invocation<'call, M> {
    /// Creates an associated-function invocation without a receiver.
    pub fn associated<I>(arguments: I) -> Self
    where
        I: IntoIterator<Item = InvocationArg<'call, M>>,
    {
        Self::new(None, arguments)
    }

    /// Creates an associated-function invocation from caller-ordered bindings.
    ///
    /// Named and positional bindings may be interleaved. A positional binding
    /// selects the first declaration-order parameter not already occupied by
    /// an earlier binding, while a named binding selects one unique identifier
    /// parameter. A descriptor-aware entry point such as
    /// [`MethodInstanceDescriptor::invoke_local`](crate::descriptor::MethodInstanceDescriptor::invoke_local)
    /// validates and orders these bindings before its generated adapter runs.
    pub fn associated_bindings<I>(bindings: I) -> Self
    where
        I: IntoIterator<Item = InvocationBinding<'call, M>>,
    {
        Self::from_bindings(None, bindings)
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
        let arguments = arguments.into_iter().collect::<Box<[_]>>();
        let argument_names = std::iter::repeat_with(|| None).take(arguments.len()).collect();
        Self {
            receiver,
            arguments,
            argument_names,
            binding_recovery: None,
        }
    }

    /// Creates an invocation from an optional receiver and caller-ordered
    /// positional or named bindings.
    ///
    /// This constructor only collects inputs. Method-aware binding, mode
    /// checks, and exact type checks happen later without extracting values.
    pub fn from_bindings<I>(receiver: Option<InvocationReceiver<'call, M>>, bindings: I) -> Self
    where
        I: IntoIterator<Item = InvocationBinding<'call, M>>,
    {
        let (argument_names, arguments): (Vec<_>, Vec<_>) =
            bindings.into_iter().map(InvocationBinding::into_parts).unzip();
        Self {
            receiver,
            arguments: arguments.into_boxed_slice(),
            argument_names: argument_names.into_boxed_slice(),
            binding_recovery: None,
        }
    }

    /// Returns the supplied receiver, or `None` for an associated function.
    pub const fn receiver(&self) -> Option<&InvocationReceiver<'call, M>> {
        self.receiver.as_ref()
    }

    /// Returns arguments in their current order.
    ///
    /// Newly collected bindings remain in caller order. Descriptor-aware
    /// binding internally reorders them immediately before adapter validation.
    pub fn arguments(&self) -> &[InvocationArg<'call, M>] {
        &self.arguments
    }

    /// Returns the original name of one caller-ordered binding.
    ///
    /// `Some(name)` identifies a named binding. `None` identifies either a
    /// positional binding or an index outside the supplied input range.
    pub fn argument_name(&self, index: usize) -> Option<&str> {
        self.argument_names.get(index).and_then(|name| name.as_deref())
    }

    /// Resolves named and positional bindings against parameter descriptors.
    ///
    /// Named inputs select a unique [`ParameterPatternDescriptor::Identifier`]
    /// with the same name. Positional inputs may be interleaved and select the
    /// next unoccupied declaration-order parameter. All binding checks finish
    /// before arguments are reordered or any dynamic value is extracted. On
    /// failure, recovery retains the receiver and every binding's original
    /// name, mode, caller order, and value.
    pub(crate) fn bind_arguments(
        self,
        method_identity: &MemberId,
        parameters: &[ParameterDescriptor],
    ) -> Result<Self, InvocationFailure<'call, M>> {
        if self.argument_names.iter().all(Option::is_none) {
            return Ok(self);
        }

        let mut parameter_for_input = Vec::with_capacity(self.arguments.len());
        let mut occupied_by = vec![None; parameters.len()];
        for input_index in 0..self.argument_names.len() {
            let name = self.argument_names[input_index].clone();
            let parameter_index = match name {
                Some(name) => {
                    let matches = parameters
                        .iter()
                        .enumerate()
                        .filter(|(_, parameter)| parameter.name() == Some(name.as_ref()))
                        .collect::<Vec<_>>();
                    match matches.as_slice() {
                        [] => {
                            return Err(self.into_failure(
                                method_identity,
                                InvocationErrorKind::UnknownArgumentName { input_index, name },
                            ));
                        }
                        [(parameter_index, parameter)]
                            if matches!(parameter.pattern(), ParameterPatternDescriptor::Identifier) =>
                        {
                            *parameter_index
                        }
                        [(parameter_index, _)] => {
                            return Err(self.into_failure(
                                method_identity,
                                InvocationErrorKind::NamedArgumentUnavailable {
                                    input_index,
                                    parameter_index: *parameter_index,
                                    name,
                                },
                            ));
                        }
                        _ => {
                            return Err(self.into_failure(
                                method_identity,
                                InvocationErrorKind::AmbiguousArgumentName {
                                    input_index,
                                    name,
                                    parameter_indices: matches
                                        .into_iter()
                                        .map(|(parameter_index, _)| parameter_index)
                                        .collect(),
                                },
                            ));
                        }
                    }
                }
                None => match occupied_by.iter().position(Option::is_none) {
                    Some(parameter_index) => parameter_index,
                    None => {
                        return Err(self.into_failure(
                            method_identity,
                            InvocationErrorKind::PositionalArgumentOverflow { input_index },
                        ));
                    }
                },
            };
            if occupied_by[parameter_index].replace(input_index).is_some() {
                return Err(self.into_failure(
                    method_identity,
                    InvocationErrorKind::DuplicateArgumentBinding {
                        input_index,
                        parameter_index,
                    },
                ));
            }
            parameter_for_input.push(parameter_index);
        }

        if let Some(parameter_index) = occupied_by.iter().position(Option::is_none) {
            return Err(self.into_failure(
                method_identity,
                InvocationErrorKind::MissingArgumentBinding {
                    parameter_index,
                    name: parameters[parameter_index].name(),
                },
            ));
        }

        let Self {
            receiver,
            arguments,
            argument_names,
            binding_recovery: _,
        } = self;
        let mut caller_arguments = arguments.into_vec().into_iter().map(Some).collect::<Vec<_>>();
        let mut ordered_arguments = std::iter::repeat_with(|| None)
            .take(parameters.len())
            .collect::<Vec<_>>();
        for (input_index, parameter_index) in parameter_for_input.into_iter().enumerate() {
            ordered_arguments[parameter_index] = caller_arguments[input_index].take();
        }
        Ok(Self {
            receiver,
            arguments: ordered_arguments
                .into_iter()
                .map(|argument| argument.unwrap_or_else(|| unreachable!("every parameter is bound exactly once")))
                .collect(),
            argument_names: std::iter::repeat_with(|| None).take(parameters.len()).collect(),
            binding_recovery: Some(Box::new(BindingRecovery {
                caller_index_for_argument: occupied_by
                    .into_iter()
                    .map(|input_index| {
                        input_index.unwrap_or_else(|| unreachable!("every parameter is bound exactly once"))
                    })
                    .collect(),
                caller_names: argument_names,
            })),
        })
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
        if let Some((input_index, name)) = self.first_named_binding() {
            return Err(self.into_failure(
                method_identity,
                InvocationErrorKind::NamedBindingRequiresDescriptor { input_index, name },
            ));
        }
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
            argument_names: self.argument_names,
            binding_recovery: self.binding_recovery,
        })
    }

    /// Validates only positional arguments while retaining the receiver for a
    /// separately registered receiver-adapter capability.
    ///
    /// This preserves every input until the adapter has accepted the receiver,
    /// so a later failure can still return the original complete invocation.
    pub fn validate_arguments(
        self,
        method_identity: &MemberId,
        arguments: &[ArgumentExpectation],
    ) -> Result<ValidatedInvocation<'call, M>, InvocationFailure<'call, M>> {
        if let Some((input_index, name)) = self.first_named_binding() {
            return Err(self.into_failure(
                method_identity,
                InvocationErrorKind::NamedBindingRequiresDescriptor { input_index, name },
            ));
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
            argument_names: self.argument_names,
            binding_recovery: self.binding_recovery,
        })
    }

    /// Rejects an invocation before user code runs while retaining every
    /// untouched input for recovery.
    pub fn reject(self, method_identity: &MemberId, kind: InvocationErrorKind) -> InvocationFailure<'call, M> {
        self.into_failure(method_identity, kind)
    }

    /// Creates an invocation from parts previously returned by validation or
    /// recovery.
    pub(crate) fn from_parts(
        receiver: Option<InvocationReceiver<'call, M>>,
        arguments: Box<[InvocationArg<'call, M>]>,
        argument_names: Box<[Option<Box<str>>]>,
    ) -> Self {
        Self {
            receiver,
            arguments,
            argument_names,
            binding_recovery: None,
        }
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

    /// Returns the first still-unbound caller name for raw-adapter rejection.
    fn first_named_binding(&self) -> Option<(usize, Box<str>)> {
        self.argument_names
            .iter()
            .enumerate()
            .find_map(|(input_index, name)| name.clone().map(|name| (input_index, name)))
    }

    /// Pairs one validation error with every untouched input.
    fn into_failure(self, method_identity: &MemberId, kind: InvocationErrorKind) -> InvocationFailure<'call, M> {
        let Self {
            receiver,
            arguments,
            argument_names,
            binding_recovery,
        } = self;
        let (arguments, argument_names) = match binding_recovery {
            Some(binding_recovery) => {
                let mut caller_arguments = std::iter::repeat_with(|| None)
                    .take(arguments.len())
                    .collect::<Vec<_>>();
                for (argument, caller_index) in arguments
                    .into_vec()
                    .into_iter()
                    .zip(binding_recovery.caller_index_for_argument)
                {
                    caller_arguments[caller_index] = Some(argument);
                }
                (
                    caller_arguments
                        .into_iter()
                        .map(|argument| {
                            argument.unwrap_or_else(|| unreachable!("every caller argument is recovered exactly once"))
                        })
                        .collect(),
                    binding_recovery.caller_names,
                )
            }
            None => (arguments, argument_names),
        };
        InvocationFailure {
            error: InvocationError::new(method_identity.clone(), kind),
            recovery: crate::invoke::InvocationRecovery::new(receiver, arguments, argument_names),
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
    argument_names: Box<[Option<Box<str>>]>,
    binding_recovery: Option<Box<BindingRecovery>>,
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

    /// Applies an optional explicit-receiver capability without losing the
    /// invocation recovery retained during descriptor-aware argument binding.
    ///
    /// A missing capability or rejected receiver restores arguments to exact
    /// caller order, including every original named-binding label. Successful
    /// conversion returns declaration-ordered arguments for generated method
    /// extraction.
    pub fn adapt_receiver<R: 'static>(
        self,
        method_identity: &MemberId,
        adapter: Option<&crate::invoke::ReceiverAdapter<R, M>>,
    ) -> ReceiverAdaptationResult<'call, R, M> {
        let Self {
            receiver,
            arguments,
            argument_names,
            binding_recovery,
        } = self;
        let expected_name = std::any::type_name::<R>();
        let Some(receiver) = receiver else {
            return Err(Invocation {
                receiver: None,
                arguments,
                argument_names,
                binding_recovery,
            }
            .reject(
                method_identity,
                InvocationErrorKind::ReceiverAdapterRejected { expected_name },
            ));
        };
        let Some(adapter) = adapter else {
            return Err(Invocation {
                receiver: Some(receiver),
                arguments,
                argument_names,
                binding_recovery,
            }
            .reject(
                method_identity,
                InvocationErrorKind::ReceiverAdapterUnavailable { expected_name },
            ));
        };
        match adapter(receiver) {
            Ok(receiver) => Ok((receiver, arguments)),
            Err(receiver) => Err(Invocation {
                receiver: Some(receiver),
                arguments,
                argument_names,
                binding_recovery,
            }
            .reject(
                method_identity,
                InvocationErrorKind::ReceiverAdapterRejected { expected_name },
            )),
        }
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

#[cfg(test)]
mod tests {
    use crate::descriptor::ParameterDescriptor;
    use crate::descriptor::ParameterPassingMode;
    use crate::descriptor::ParameterPatternDescriptor;
    use crate::expression::TypeExpression;
    use crate::identity::FragmentIdentity;
    use crate::identity::MemberId;
    use crate::invoke::Invocation;
    use crate::invoke::InvocationArg;
    use crate::invoke::InvocationBinding;
    use crate::invoke::InvocationErrorKind;
    use crate::invoke::InvocationReceiver;
    use crate::value::DynamicOwned;
    use crate::value::Local;

    /// Creates a stable identity for private binding-contract tests.
    fn method_identity() -> MemberId {
        MemberId::new(
            "invoke::binding_tests",
            "method",
            0,
            FragmentIdentity::new("qubit-reflect", "invoke::binding_tests", 1, 1, "method", 1),
        )
    }

    /// Creates one owned parameter with test-selected name and pattern.
    fn parameter(index: usize, name: Option<&'static str>, pattern: ParameterPatternDescriptor) -> ParameterDescriptor {
        ParameterDescriptor::new(
            index,
            name,
            pattern,
            ParameterPassingMode::Owned,
            TypeExpression::Parameter("T".into()),
            None,
        )
    }

    /// Returns an explicit receiver unchanged to simulate a rejected adapter.
    fn reject_receiver<'call>(
        receiver: InvocationReceiver<'call, Local>,
    ) -> Result<String, InvocationReceiver<'call, Local>> {
        Err(receiver)
    }

    /// Creates and validates named bindings whose declaration order differs
    /// from their original caller order.
    fn validated_named_receiver_invocation() -> super::ValidatedInvocation<'static, Local> {
        let parameters = [
            parameter(0, Some("first"), ParameterPatternDescriptor::Identifier),
            parameter(1, Some("second"), ParameterPatternDescriptor::Identifier),
        ];
        Invocation::from_bindings(
            Some(InvocationReceiver::Owned(DynamicOwned::<Local>::new(7_u32))),
            [
                InvocationBinding::named("second", InvocationArg::Owned(DynamicOwned::<Local>::new(22_u16))),
                InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(11_u8))),
            ],
        )
        .bind_arguments(&method_identity(), &parameters)
        .expect("named bindings must resolve")
        .validate_arguments(
            &method_identity(),
            &[
                crate::invoke::ArgumentExpectation::owned::<u8>(),
                crate::invoke::ArgumentExpectation::owned::<u16>(),
            ],
        )
        .expect("declaration-ordered arguments must validate")
    }

    /// Verifies a missing explicit-receiver capability restores exact caller
    /// binding metadata after argument validation has succeeded.
    #[test]
    fn test_validated_invocation_recovers_named_bindings_when_receiver_adapter_is_unavailable() {
        let result = validated_named_receiver_invocation().adapt_receiver::<String>(&method_identity(), None);
        let Err(failure) = result else {
            panic!("a missing receiver adapter must fail before extraction")
        };

        assert!(matches!(
            failure.error.kind(),
            InvocationErrorKind::ReceiverAdapterUnavailable { expected_name }
                if expected_name == &std::any::type_name::<String>()
        ));
        assert_eq!(failure.recovery.argument_name(0), Some("second"));
        assert_eq!(failure.recovery.argument_name(1), None);
        let (_, arguments) = failure.recovery.into_parts();
        let mut arguments = arguments.into_vec().into_iter();
        let Some(InvocationArg::Owned(second)) = arguments.next() else {
            panic!("first caller binding must remain owned")
        };
        let Some(InvocationArg::Owned(first)) = arguments.next() else {
            panic!("second caller binding must remain owned")
        };
        let Ok(second) = DynamicOwned::<Local>::downcast::<u16>(second) else {
            panic!("named value must remain intact")
        };
        let Ok(first) = DynamicOwned::<Local>::downcast::<u8>(first) else {
            panic!("positional value must remain intact")
        };
        assert_eq!(second, 22);
        assert_eq!(first, 11);
    }

    /// Verifies an explicit-receiver adapter rejection restores exact caller
    /// binding metadata after argument validation has succeeded.
    #[test]
    fn test_validated_invocation_recovers_named_bindings_when_receiver_adapter_rejects() {
        let result = validated_named_receiver_invocation().adapt_receiver(
            &method_identity(),
            Some(&(reject_receiver as crate::invoke::ReceiverAdapter<String, Local>)),
        );
        let Err(failure) = result else {
            panic!("a rejected receiver must fail before user code runs")
        };

        assert!(matches!(
            failure.error.kind(),
            InvocationErrorKind::ReceiverAdapterRejected { expected_name }
                if expected_name == &std::any::type_name::<String>()
        ));
        assert_eq!(failure.recovery.argument_name(0), Some("second"));
        assert_eq!(failure.recovery.argument_name(1), None);
        let (_, arguments) = failure.recovery.into_parts();
        let mut arguments = arguments.into_vec().into_iter();
        let Some(InvocationArg::Owned(second)) = arguments.next() else {
            panic!("first caller binding must remain owned")
        };
        let Some(InvocationArg::Owned(first)) = arguments.next() else {
            panic!("second caller binding must remain owned")
        };
        let Ok(second) = DynamicOwned::<Local>::downcast::<u16>(second) else {
            panic!("named value must remain intact")
        };
        let Ok(first) = DynamicOwned::<Local>::downcast::<u8>(first) else {
            panic!("positional value must remain intact")
        };
        assert_eq!(second, 22);
        assert_eq!(first, 11);
    }

    /// Verifies duplicate descriptor names produce deterministic ambiguity.
    #[test]
    fn test_bind_arguments_rejects_ambiguous_identifier_name() {
        let parameters = [
            parameter(0, Some("duplicate"), ParameterPatternDescriptor::Identifier),
            parameter(1, Some("duplicate"), ParameterPatternDescriptor::Identifier),
        ];
        let invocation = Invocation::associated_bindings([
            InvocationBinding::named("duplicate", InvocationArg::Owned(DynamicOwned::<Local>::new(1_u8))),
            InvocationBinding::positional(InvocationArg::Owned(DynamicOwned::<Local>::new(2_u8))),
        ]);

        let result = invocation.bind_arguments(&method_identity(), &parameters);
        let Err(failure) = result else {
            panic!("duplicate parameter names must not bind arbitrarily")
        };

        assert!(matches!(
            failure.error.kind(),
            InvocationErrorKind::AmbiguousArgumentName {
                input_index: 0,
                name,
                parameter_indices,
            } if name.as_ref() == "duplicate" && parameter_indices.as_ref() == [0, 1]
        ));
        assert_eq!(failure.recovery.argument_name(0), Some("duplicate"));
        assert_eq!(failure.recovery.arguments().len(), 2);
    }

    /// Verifies malformed metadata cannot make a wildcard name-bindable.
    #[test]
    fn test_bind_arguments_rejects_name_on_non_identifier_pattern() {
        let parameters = [parameter(
            0,
            Some("not_identifier"),
            ParameterPatternDescriptor::Wildcard,
        )];
        let invocation = Invocation::associated_bindings([InvocationBinding::named(
            "not_identifier",
            InvocationArg::Owned(DynamicOwned::<Local>::new(3_u8)),
        )]);

        let result = invocation.bind_arguments(&method_identity(), &parameters);
        let Err(failure) = result else {
            panic!("a wildcard pattern must not become name-bindable")
        };

        assert!(matches!(
            failure.error.kind(),
            InvocationErrorKind::NamedArgumentUnavailable {
                input_index: 0,
                parameter_index: 0,
                name,
            } if name.as_ref() == "not_identifier"
        ));
        assert_eq!(failure.recovery.argument_name(0), Some("not_identifier"));
        assert_eq!(failure.recovery.arguments().len(), 1);
    }
}
