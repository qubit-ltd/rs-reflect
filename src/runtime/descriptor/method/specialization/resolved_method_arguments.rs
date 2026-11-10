// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Resolved generic arguments retained for invocation and display.
#[derive(Clone)]
pub(crate) enum ResolvedMethodArguments {
    #[cfg(test)]
    Static(
        /// Static identities used by hand-written method catalog fixtures.
        &'static [GenericArgumentIdentity],
    ),
    /// Owns resolved argument identities and their matching selectors.
    Owned {
        /// Provides the display operation used to resolve or dispatch reflected
        /// methods.
        display: Box<[GenericArgumentIdentity]>,
        /// Returns the selector list used to compare generic arguments.
        selectors: Box<[GenericArgumentSelector]>,
    },
}

impl ResolvedMethodArguments {
    /// Borrows the resolved generic argument identities as a slice.
    pub(crate) fn as_slice(&self) -> &[GenericArgumentIdentity] {
        match self {
            #[cfg(test)]
            Self::Static(arguments) => arguments,
            Self::Owned { display, .. } => display,
        }
    }

    /// Compares resolved selectors to determine whether two registrations
    /// select the same specialization. # Parameters
    ///
    /// - `other`: ABI name or metadata value that is not one of the standard
    ///   forms.
    /// # Parameters
    ///
    /// - `other`: Second metadata value used in the equivalence comparison.
    pub(crate) fn same_selection(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Owned { selectors: left, .. }, Self::Owned { selectors: right, .. }) => {
                selector_slices_match(left, right)
            }
            #[cfg(test)]
            (Self::Static(left), Self::Static(right)) => left.is_empty() && right.is_empty(),
            #[cfg(test)]
            _ => false,
        }
    }

    /// Checks whether the supplied generic arguments satisfy this
    /// specialization. # Parameters
    ///
    /// - `selectors`: Resolved generic argument selectors used to compare
    ///   specializations.
    /// # Parameters
    ///
    /// - `selectors`: Resolved generic argument selectors compared during
    ///   specialization lookup.
    pub(super) fn matches(&self, selectors: &[GenericArgumentSelector]) -> bool {
        match self {
            Self::Owned {
                selectors: own_selectors,
                ..
            } => selector_slices_match(own_selectors, selectors),
            #[cfg(test)]
            Self::Static(arguments) => arguments.is_empty() && selectors.is_empty(),
        }
    }
}

/// Compares two selector lists element by element.
/// # Parameters
///
/// - `left`: First metadata value in the equality comparison.
/// - `right`: Second metadata value in the equality comparison.
fn selector_slices_match(left: &[GenericArgumentSelector], right: &[GenericArgumentSelector]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(left, right)| left.matches(right))
}
