// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::error::LookupError;
use super::error::LookupQuery;
use super::error::LookupSubject;
use super::identity::IntoLookupCandidate;
use super::identity::LookupCandidate;

#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
/// Result of looking up an item before requiring a unique match.
pub enum Lookup<T> {
    /// No item matched the lookup criteria.
    Missing,
    /// Exactly one item matched.
    Unique(T),
    /// Multiple items matched; candidates retain their lookup order.
    Ambiguous(Box<[T]>),
}

impl<T> Lookup<T> {
    /// Returns whether no candidate matched.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    /// Returns whether exactly one candidate matched.
    pub fn is_unique(&self) -> bool {
        matches!(self, Self::Unique(_))
    }
    /// Returns whether multiple candidates matched.
    pub fn is_ambiguous(&self) -> bool {
        matches!(self, Self::Ambiguous(_))
    }

    /// Returns the number of candidates: zero, one, or the ambiguous count.
    pub fn len(&self) -> usize {
        match self {
            Self::Missing => 0,
            Self::Unique(_) => 1,
            Self::Ambiguous(candidates) => candidates.len(),
        }
    }

    /// Returns whether the lookup found no candidates.
    pub fn is_empty(&self) -> bool {
        self.is_missing()
    }

    /// Returns all candidates as a slice, including the unique candidate.
    pub fn candidates(&self) -> &[T] {
        match self {
            Self::Missing => &[],
            Self::Unique(candidate) => std::slice::from_ref(candidate),
            Self::Ambiguous(candidates) => candidates,
        }
    }

    /// Returns the candidate only when the lookup is unique.
    pub fn only(self) -> Option<T> {
        match self {
            Self::Unique(candidate) => Some(candidate),
            Self::Missing | Self::Ambiguous(_) => None,
        }
    }

    /// Requires a unique match, preserving query details in lookup errors.
    ///
    /// Returns [`LookupError::Missing`](super::error::LookupError::Missing) if
    /// no candidate matched, or
    /// [`LookupError::Ambiguous`](super::error::LookupError::Ambiguous)
    /// with candidate identities and origins if multiple items matched.
    pub fn into_unique(self, subject: LookupSubject, query: LookupQuery) -> Result<T, LookupError>
    where
        T: IntoLookupCandidate,
    {
        match self {
            Self::Missing => Err(LookupError::Missing { subject, query }),
            Self::Unique(candidate) => Ok(candidate),
            Self::Ambiguous(candidates) => Err(LookupError::Ambiguous {
                subject,
                query,
                candidates: candidates
                    .into_vec()
                    .into_iter()
                    .map(IntoLookupCandidate::into_lookup_candidate)
                    .collect::<Vec<LookupCandidate>>()
                    .into_boxed_slice(),
            }),
        }
    }
}
