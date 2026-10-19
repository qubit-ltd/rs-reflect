use std::fmt;

use crate::runtime::value::{DynamicOwned, ValueMode};

use super::input::{ConstructionInputValue, UpdateInput};
use super::schema::{ConstructionError, UpdateError};

#[must_use]
pub(crate) struct ConstructionRecovery<M: ValueMode> {
    input: ConstructionInputValue<M>,
}

impl<M: ValueMode> ConstructionRecovery<M> {
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
            ConstructionInputValue::Unit => {
                RecoveredConstructionInput::Unit(super::input::UnitFields)
            }
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

    pub(crate) fn recovery(&self) -> &ConstructionRecovery<M> {
        &self.recovery
    }

    pub(crate) fn into_parts(self) -> (ConstructionError, ConstructionRecovery<M>) {
        (self.error, self.recovery)
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

#[must_use]
pub(crate) struct UpdateRecovery<M: ValueMode> {
    input: UpdateInput<M>,
}

impl<M: ValueMode> UpdateRecovery<M> {
    pub(crate) fn base(&self) -> &DynamicOwned<M> {
        &self.input.base
    }

    pub(crate) fn replacements(&self) -> &super::input::NamedFields<M> {
        &self.input.replacements
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

    pub(crate) fn recovery(&self) -> &UpdateRecovery<M> {
        &self.recovery
    }

    pub(crate) fn into_parts(self) -> (UpdateError, UpdateRecovery<M>) {
        (self.error, self.recovery)
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
