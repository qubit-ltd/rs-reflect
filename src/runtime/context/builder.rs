use std::any::TypeId;
use std::collections::HashSet;

use crate::runtime::descriptor::{Reflect, TypeDescriptor};
use crate::runtime::error::{ContextError, ContextIssue};
use crate::runtime::identity::{ItemIdentity, ItemKind, StableItemId};

use super::ReflectionContext;
use super::data::ContextData;
use super::registration::{Registration, RegistrationFragment};

#[derive(Default)]
pub struct ReflectionContextBuilder {
    registrations: Vec<Registration>,
    issues: Vec<ContextIssue>,
}

impl ReflectionContextBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include_linked(mut self) -> Self {
        let mut fragments: Vec<_> = inventory::iter::<RegistrationFragment>
            .into_iter()
            .copied()
            .collect();
        fragments.sort_by_key(|fragment| fragment.sort_key());
        for fragment in fragments {
            match fragment.materialize() {
                Ok(registration) => self.registrations.push(registration),
                Err(error) => self.issues.extend(error.issues().iter().cloned()),
            }
        }
        self
    }

    pub fn include_type<T: Reflect>(mut self) -> Self {
        self.registrations.push(Registration::of::<T>());
        self
    }

    pub fn include_linked_for<T: Reflect>(self) -> Self {
        let ReflectionContextBuilder {
            mut registrations,
            mut issues,
        } = self;
        let root = TypeId::of::<T>();
        let mut fragments: Vec<_> = inventory::iter::<RegistrationFragment>
            .into_iter()
            .copied()
            .collect();
        fragments.sort_by_key(|fragment| fragment.sort_key());

        let mut included = HashSet::from([root]);
        let mut dependency_edges = Vec::new();
        let mut selected = vec![false; fragments.len()];
        loop {
            let mut changed = false;
            for (index, fragment) in fragments.iter().enumerate() {
                if selected[index]
                    || fragment
                        .target_type_id()
                        .is_none_or(|target| !included.contains(&target))
                {
                    continue;
                }
                selected[index] = true;
                changed = true;
                for dependency in fragment.dependencies() {
                    dependency_edges.push((fragment.target_type_id().unwrap(), *dependency));
                    changed |= included.insert(*dependency);
                }
            }
            if !changed {
                break;
            }
        }

        if !fragments
            .iter()
            .enumerate()
            .any(|(index, fragment)| selected[index] && fragment.includes_type())
        {
            registrations.push(Registration::of::<T>());
        }
        let mut registered_types: HashSet<_> = registrations
            .iter()
            .filter_map(|registration| registration.type_record().map(|record| record.type_id()))
            .collect();
        for (fragment, include) in fragments.iter().zip(&selected) {
            if *include && fragment.includes_type() {
                if let Some(target) = fragment.target_type_id() {
                    registered_types.insert(target);
                }
            }
        }
        for (source, dependency) in dependency_edges {
            if !registered_types.contains(&dependency) {
                let issue = ContextIssue::MissingDependency {
                    item: type_item_identity(source),
                    dependency: type_item_identity(dependency),
                };
                issues.push(issue);
            }
        }
        if !issues.is_empty() {
            return Self {
                registrations,
                issues,
            };
        }
        for (fragment, include) in fragments.into_iter().zip(selected) {
            if include {
                match fragment.materialize() {
                    Ok(registration) => registrations.push(registration),
                    Err(error) => issues.extend(error.issues().iter().cloned()),
                }
            }
        }
        Self {
            registrations,
            issues,
        }
    }

    pub fn register(mut self, registration: impl Into<Registration>) -> Self {
        let mut registration = registration.into();
        self.issues.extend(registration.take_issues());
        self.registrations.push(registration);
        self
    }

    pub fn build(self) -> Result<ReflectionContext, ContextError> {
        if let Some(error) = ContextError::from_issues(self.issues.clone().into_boxed_slice()) {
            return Err(error);
        }
        Ok(ReflectionContext::from_data(ContextData::build(
            self.registrations,
        )?))
    }

    pub(crate) fn build_data(self) -> Result<ContextData, ContextError> {
        if let Some(error) = ContextError::from_issues(self.issues.into_boxed_slice()) {
            return Err(error);
        }
        ContextData::build(self.registrations)
    }
}

fn type_item_identity(type_id: TypeId) -> ItemIdentity {
    let name = format!("{type_id:?}");
    ItemIdentity {
        kind: ItemKind::Type,
        stable_id: StableItemId(name.clone().into()),
        display_name: name.into(),
    }
}

impl ReflectionContext {
    pub fn builder() -> ReflectionContextBuilder {
        ReflectionContextBuilder::new()
    }

    pub fn type_of<T: Reflect + ?Sized>(&self) -> TypeDescriptor<'_> {
        TypeDescriptor::of_in::<T>(self)
    }
}
