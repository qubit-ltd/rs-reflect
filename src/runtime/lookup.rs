use super::error::{LookupError, LookupQuery, LookupSubject};
use super::identity::{IntoLookupCandidate, LookupCandidate};

#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Lookup<T> {
    Missing,
    Unique(T),
    Ambiguous(Box<[T]>),
}

impl<T> Lookup<T> {
    pub fn is_missing(&self) -> bool { matches!(self, Self::Missing) }
    pub fn is_unique(&self) -> bool { matches!(self, Self::Unique(_)) }
    pub fn is_ambiguous(&self) -> bool { matches!(self, Self::Ambiguous(_)) }

    pub fn len(&self) -> usize {
        match self {
            Self::Missing => 0,
            Self::Unique(_) => 1,
            Self::Ambiguous(candidates) => candidates.len(),
        }
    }

    pub fn is_empty(&self) -> bool { self.is_missing() }

    pub fn candidates(&self) -> &[T] {
        match self {
            Self::Missing => &[],
            Self::Unique(candidate) => std::slice::from_ref(candidate),
            Self::Ambiguous(candidates) => candidates,
        }
    }

    pub fn only(self) -> Option<T> {
        match self {
            Self::Unique(candidate) => Some(candidate),
            Self::Missing | Self::Ambiguous(_) => None,
        }
    }

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
