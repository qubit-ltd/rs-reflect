// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::collections::HashSet;

use super::ReflectionContext;
use super::data::ContextData;
use super::registration::Registration;
use super::registration::RegistrationFragment;
use crate::__private::codegen_v4::registration::MethodBinding;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::error::ContextError;
use crate::runtime::error::ContextIssue;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::StableItemId;

/// Accumulates registrations and options used to construct a reflection
/// context.
#[derive(Default)]
pub struct ReflectionContextBuilder {
    registrations: Vec<Registration>,
    issues: Vec<ContextIssue>,
}

impl ReflectionContextBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Includes all statically linked registration fragments.
    pub fn include_linked(mut self) -> Self {
        let mut fragments: Vec<_> = inventory::iter::<RegistrationFragment>.into_iter().copied().collect();
        fragments.sort_by_key(|fragment| fragment.sort_key());
        for fragment in fragments {
            match fragment.materialize() {
                Ok(registration) => self.registrations.push(registration),
                Err(error) => self.issues.extend(error.issues().iter().cloned()),
            }
        }
        self
    }

    /// Includes the concrete metadata and registrations generated for `T`.
    pub fn include_type<T: Reflect + ?Sized>(mut self) -> Self {
        self.registrations.push(Registration::of::<T>());
        self
    }

    /// Includes linked fragments reachable from the reflected type `T`.
    pub fn include_linked_for<T: Reflect + ?Sized>(self) -> Self {
        let ReflectionContextBuilder {
            mut registrations,
            mut issues,
        } = self;
        let root = TypeId::of::<T>();
        let mut fragments: Vec<_> = inventory::iter::<RegistrationFragment>.into_iter().copied().collect();
        fragments.sort_by_key(|fragment| fragment.sort_key());

        let mut included = HashSet::from([root]);
        let mut dependency_edges = Vec::new();
        let mut selected = vec![false; fragments.len()];
        for (index, fragment) in fragments.iter().enumerate() {
            if fragment.include_in_linked_context() {
                selected[index] = true;
            }
        }
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
            if *include
                && fragment.includes_type()
                && let Some(target) = fragment.target_type_id()
            {
                registered_types.insert(target);
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
            return Self { registrations, issues };
        }
        for (fragment, include) in fragments.into_iter().zip(selected) {
            if include {
                match fragment.materialize() {
                    Ok(registration) => registrations.push(registration),
                    Err(error) => issues.extend(error.issues().iter().cloned()),
                }
            }
        }
        Self { registrations, issues }
    }

    /// Adds a registration or registration fragment to this builder.
    pub fn register(mut self, registration: impl Into<Registration>) -> Self {
        let mut registration = registration.into();
        self.issues.extend(registration.take_issues());
        self.registrations.push(registration);
        self
    }

    /// Adds a typed method binding to this context's target method catalog.
    pub fn bind<T: Reflect>(self, binding: MethodBinding<T>) -> Self {
        self.register(binding)
    }

    /// Builds a context, returning an error for invalid metadata or conflicts.
    pub fn build(self) -> Result<ReflectionContext, ContextError> {
        if let Some(error) = ContextError::from_issues(self.issues.clone().into_boxed_slice()) {
            return Err(error);
        }
        Ok(ReflectionContext::from_data(ContextData::build(self.registrations)?))
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
    /// Creates a builder for a new explicit reflection context.
    pub fn builder() -> ReflectionContextBuilder {
        ReflectionContextBuilder::new()
    }

    /// Returns a context-bound descriptor for the concrete Rust type `T`.
    pub fn type_of<T: Reflect + ?Sized>(&self) -> TypeDescriptor<'_> {
        TypeDescriptor::of_in::<T>(self)
    }
}
