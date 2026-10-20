// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

use super::input::ConstructionInputValue;
use super::input::UpdateInput;
use super::schema::ConstructionError;
use super::schema::UpdateError;
use crate::runtime::error::RecoverableFailure;
use crate::runtime::error::RecoverableFailureParts;
#[cfg(test)]
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::ValueMode;

#[must_use]
pub(crate) struct ConstructionRecovery<M: ValueMode> {
    input: ConstructionInputValue<M>,
}

impl<M: ValueMode> ConstructionRecovery<M> {
    #[cfg(test)]
    pub(crate) fn input(&self) -> &ConstructionInputValue<M> {
        &self.input
    }

    pub(crate) fn into_input(self) -> RecoveredConstructionInput<M> {
        match self.input {
            ConstructionInputValue::Named(entries) => {
                RecoveredConstructionInput::Named(super::input::NamedFields { entries })
            }
            ConstructionInputValue::Positional(values) => {
                RecoveredConstructionInput::Positional(super::input::PositionalFields { values })
            }
            ConstructionInputValue::Unit => RecoveredConstructionInput::Unit(super::input::UnitFields),
        }
    }
}

pub(crate) enum RecoveredConstructionInput<M: ValueMode> {
    Named(super::input::NamedFields<M>),
    Positional(super::input::PositionalFields<M>),
    Unit(super::input::UnitFields),
}

#[must_use]
pub(crate) struct ConstructionFailure<M: ValueMode> {
    error: ConstructionError,
    recovery: ConstructionRecovery<M>,
}

impl<M: ValueMode> ConstructionFailure<M> {
    pub(crate) fn new(error: ConstructionError, input: ConstructionInputValue<M>) -> Self {
        Self {
            error,
            recovery: ConstructionRecovery { input },
        }
    }

    pub(crate) fn error(&self) -> &ConstructionError {
        &self.error
    }

    #[cfg(test)]
    pub(crate) fn recovery(&self) -> &ConstructionRecovery<M> {
        &self.recovery
    }

    pub(crate) fn into_input(self) -> RecoveredConstructionInput<M> {
        self.recovery.into_input()
    }
}

impl<M: ValueMode> fmt::Debug for ConstructionFailure<M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructionFailure")
            .field("error", &self.error)
            .field("shape", &self.recovery.input.shape())
            .finish()
    }
}

impl<M: ValueMode> RecoverableFailure for ConstructionFailure<M> {
    type Error = ConstructionError;
    type Recovery = ConstructionRecovery<M>;

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

#[must_use]
pub(crate) struct UpdateRecovery<M: ValueMode> {
    input: UpdateInput<M>,
}

impl<M: ValueMode> UpdateRecovery<M> {
    #[cfg(test)]
    pub(crate) fn base(&self) -> &DynamicOwned<M> {
        &self.input.base
    }

    pub(crate) fn into_input(self) -> UpdateInput<M> {
        self.input
    }
}

#[must_use]
pub(crate) struct UpdateFailure<M: ValueMode> {
    error: UpdateError,
    recovery: UpdateRecovery<M>,
}

impl<M: ValueMode> UpdateFailure<M> {
    pub(crate) fn new(error: UpdateError, input: UpdateInput<M>) -> Self {
        Self {
            error,
            recovery: UpdateRecovery { input },
        }
    }

    pub(crate) fn error(&self) -> &UpdateError {
        &self.error
    }

    pub(crate) fn into_input(self) -> UpdateInput<M> {
        self.recovery.into_input()
    }
}

impl<M: ValueMode> fmt::Debug for UpdateFailure<M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateFailure")
            .field("error", &self.error)
            .field("base_type_id", &self.recovery.input.base.type_id())
            .finish()
    }
}

impl<M: ValueMode> RecoverableFailure for UpdateFailure<M> {
    type Error = UpdateError;
    type Recovery = UpdateRecovery<M>;

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

#[cfg(test)]
mod tests {
    use super::ConstructionFailure;
    use super::ConstructionInputValue;
    use super::RecoveredConstructionInput;
    use super::UpdateFailure;
    use crate::runtime::construct::input::NamedFields;
    use crate::runtime::construct::input::UpdateInput;
    use crate::runtime::construct::schema::ConstructionError;
    use crate::runtime::construct::schema::UpdateError;
    use crate::runtime::error::RecoverableFailure;
    use crate::runtime::value::DynamicOwned;
    use crate::runtime::value::ThreadSafe;

    #[test]
    fn thread_safe_construction_failure_retains_each_input_shape() {
        let named = ConstructionFailure::new(
            ConstructionError::MissingProvider,
            ConstructionInputValue::<ThreadSafe>::Named(Vec::new()),
        );
        assert_eq!(named.error(), &ConstructionError::MissingProvider);
        assert!(format!("{named:?}").contains("shape: Named"));
        assert!(matches!(named.recovery().input(), ConstructionInputValue::Named(_)));
        let (error, recovery) = RecoverableFailure::into_parts(named);
        assert_eq!(error, ConstructionError::MissingProvider);
        assert!(matches!(recovery.into_input(), RecoveredConstructionInput::Named(_)));

        let positional = ConstructionFailure::new(
            ConstructionError::MissingProvider,
            ConstructionInputValue::<ThreadSafe>::Positional(Vec::new()),
        );
        assert!(format!("{positional:?}").contains("shape: Positional"));
        assert!(matches!(
            positional.into_input(),
            RecoveredConstructionInput::Positional(_)
        ));

        let unit = ConstructionFailure::new(
            ConstructionError::MissingProvider,
            ConstructionInputValue::<ThreadSafe>::Unit,
        );
        assert!(format!("{unit:?}").contains("shape: Unit"));
        assert!(matches!(unit.into_input(), RecoveredConstructionInput::Unit(_)));
    }

    #[test]
    fn thread_safe_update_failure_preserves_base_and_replacements() {
        let update = UpdateInput {
            base: DynamicOwned::<ThreadSafe>::new(17_u32),
            replacements: NamedFields::new(),
        };
        let failure = UpdateFailure::new(UpdateError::MissingProvider, update);
        assert_eq!(failure.error(), &UpdateError::MissingProvider);
        assert!(format!("{failure:?}").contains("base_type_id"));
        assert_eq!(failure.recovery().base().downcast_ref::<u32>(), Some(&17));

        let (error, recovery) = RecoverableFailure::into_parts(failure);
        assert_eq!(error, UpdateError::MissingProvider);
        let recovered = recovery.into_input();
        assert_eq!(recovered.base.downcast_ref::<u32>(), Some(&17));
        assert!(recovered.replacements.entries.is_empty());

        let update = UpdateInput {
            base: DynamicOwned::<ThreadSafe>::new(23_u32),
            replacements: NamedFields::new(),
        };
        let failure = UpdateFailure::new(UpdateError::MissingProvider, update);
        let recovered = failure.into_input();
        assert_eq!(recovered.base.downcast_ref::<u32>(), Some(&23));
    }
}
