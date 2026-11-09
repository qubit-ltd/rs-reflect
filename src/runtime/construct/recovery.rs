// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Recoverable input handling for failed construction and update operations.

mod construction_failure;
mod construction_recovery;
mod recovered_construction_input;
mod update_failure;
mod update_recovery;

pub(crate) use construction_failure::ConstructionFailure;
pub(crate) use construction_recovery::ConstructionRecovery;
pub(crate) use recovered_construction_input::RecoveredConstructionInput;
pub(crate) use update_failure::UpdateFailure;
pub(crate) use update_recovery::UpdateRecovery;

#[cfg(test)]
mod tests {
    use super::ConstructionFailure;
    use super::RecoveredConstructionInput;
    use super::UpdateFailure;
    use crate::runtime::construct::input::ConstructionInputValue;
    use crate::runtime::construct::input::NamedFields;
    use crate::runtime::construct::input::UpdateInput;
    use crate::runtime::construct::schema::ConstructionError;
    use crate::runtime::construct::schema::UpdateError;
    use crate::runtime::error::RecoverableFailure;
    use crate::runtime::value::DynamicOwned;
    use crate::runtime::value::ThreadSafe;

    #[test]
    fn test_thread_safe_construction_failure_retains_each_input_shape() {
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
    fn test_thread_safe_update_failure_preserves_base_and_replacements() {
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
