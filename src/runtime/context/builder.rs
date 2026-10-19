use crate::runtime::descriptor::{Reflect, TypeDescriptor};
use crate::runtime::error::ContextError;

use super::ReflectionContext;
use super::data::ContextData;
use super::registration::{Registration, RegistrationFragment};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Default)]
pub(crate) struct ReflectionContextBuilder {
    registrations: Vec<Registration>,
}

impl ReflectionContextBuilder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn include_linked(mut self) -> Self {
        let mut fragments: Vec<_> = inventory::iter::<RegistrationFragment>
            .into_iter()
            .copied()
            .collect();
        fragments.sort_by_key(|fragment| fragment.sort_key());
        self.registrations
            .extend(fragments.into_iter().map(RegistrationFragment::materialize));
        self
    }

    pub(crate) fn include_type<T: Reflect>(mut self) -> Self {
        self.registrations.push(Registration::of::<T>());
        self
    }

    pub(crate) fn include_linked_for<T: Reflect>(self) -> Self {
        // This runtime slice only knows type registrations. Later registration
        // kinds can extend the target closure without changing builder callers.
        self.include_type::<T>()
    }

    pub(crate) fn register(mut self, registration: impl Into<Registration>) -> Self {
        self.registrations.push(registration.into());
        self
    }

    pub(crate) fn build(self) -> Result<ReflectionContext, ContextError> {
        Ok(ReflectionContext::from_data(ContextData::build(
            self.registrations,
        )?))
    }

    pub(crate) fn build_data(self) -> Result<ContextData, ContextError> {
        ContextData::build(self.registrations)
    }
}

impl ReflectionContext {
    pub(crate) fn builder() -> ReflectionContextBuilder {
        ReflectionContextBuilder::new()
    }

    pub(crate) fn type_of<T: Reflect>(&self) -> TypeDescriptor<'_> {
        TypeDescriptor::new(T::__reflect_registration().metadata(), self)
    }
}
