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
    /// Registrations accumulated for the context.
    registrations: Vec<Registration>,
    /// Validation issues collected while registrations are added.
    issues: Vec<ContextIssue>,
}

impl ReflectionContextBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Includes all statically linked registration fragments.
    ///
    /// Each selected fragment's registration factory runs while this method is
    /// called. Calling this method again, or using another builder, can run the
    /// same factory again. A panic from a factory propagates from this call;
    /// [`build`](Self::build) only reports registration and metadata errors.
    ///
    /// Factories should only produce registration data. They must not
    /// recursively query or initialize [`ReflectionContext::global`], which
    /// panics during same-thread global initialization. Do not build another
    /// linked context from a factory to inspect registrations: that can invoke
    /// the same factories again.
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

    /// Includes the concrete metadata and registrations generated for `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — the reflected type to register.
    pub fn include_type<T: Reflect + ?Sized>(mut self) -> Self {
        self.registrations.push(Registration::of::<T>());
        self
    }

    /// Includes linked fragments reachable from the reflected type `T`.
    ///
    /// Selected fragment factories run while this method is called, and can
    /// run again for another builder or repeated inclusion. Factory panics
    /// propagate from this call; [`build`](Self::build) reports registration
    /// and metadata errors, not caught factory panics. Factories should only
    /// produce registration data and must not recursively query or initialize
    /// [`ReflectionContext::global`], which panics during same-thread global
    /// initialization. Building a linked context from a factory to inspect
    /// registrations can run the same factories again.
    ///
    /// This also includes the library's linked global definitions and
    /// capabilities. Selection is not a security boundary and does not
    /// guarantee that every unrelated metadata item is excluded.
    ///
    /// # Type Parameters
    ///
    /// * `T` — the root reflected type whose linked fragments are selected.
    pub fn include_linked_for<T: Reflect + ?Sized>(self) -> Self {
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

        let (selected, dependency_edges) = select_fragment_slice(
            &fragments,
            root,
            RegistrationFragment::target_type_id,
            |fragment| fragment.dependencies(),
            RegistrationFragment::include_in_linked_context,
        );

        if !fragments.iter().enumerate().any(|(index, fragment)| {
            selected[index] && fragment.includes_type() && fragment.target_type_id() == Some(root)
        }) {
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

    /// Adds a registration or registration fragment to this builder.
    ///
    /// # Parameters
    ///
    /// * `registration` — the registration to add.
    pub fn register(mut self, registration: impl Into<Registration>) -> Self {
        let mut registration = registration.into();
        self.issues.extend(registration.take_issues());
        self.registrations.push(registration);
        self
    }

    /// Adds a typed method binding to this context's target method catalog.
    ///
    /// # Type Parameters
    ///
    /// * `T` — the binding's target type.
    ///
    /// # Parameters
    ///
    /// * `binding` — the method binding to add.
    pub fn bind<T: Reflect>(self, binding: MethodBinding<T>) -> Self {
        self.register(binding)
    }

    /// Builds a context, returning an error for invalid metadata or conflicts.
    pub fn build(self) -> Result<ReflectionContext, ContextError> {
        if let Some(error) = ContextError::from_issues(self.issues.clone().into_boxed_slice()) {
            return Err(error);
        }
        Ok(ReflectionContext::from_data(ContextData::build(
            self.registrations,
        )?))
    }

    /// Builds and validates the context data for internal callers.
    pub(crate) fn build_data(self) -> Result<ContextData, ContextError> {
        if let Some(error) = ContextError::from_issues(self.issues.into_boxed_slice()) {
            return Err(error);
        }
        ContextData::build(self.registrations)
    }
}

/// Selects the transitive closure from an ordered fragment slice.
///
/// This is shared with the synthetic slice benchmark so it measures the
/// production selection logic rather than a copied implementation.
pub(crate) fn select_fragment_slice<K, F>(
    fragments: &[F],
    root: K,
    target: impl Fn(&F) -> Option<K>,
    dependencies: impl for<'a> Fn(&'a F) -> &'a [K],
    is_global: impl Fn(&F) -> bool,
) -> (Vec<bool>, Vec<(K, K)>)
where
    K: Copy + Eq + std::hash::Hash,
{
    let mut included = HashSet::from([root]);
    let mut dependency_edges = Vec::new();
    let mut selected = vec![false; fragments.len()];
    for (index, fragment) in fragments.iter().enumerate() {
        selected[index] = is_global(fragment);
    }
    loop {
        let mut changed = false;
        for (index, fragment) in fragments.iter().enumerate() {
            let Some(target_id) = target(fragment) else {
                continue;
            };
            if selected[index] || !included.contains(&target_id) {
                continue;
            }
            selected[index] = true;
            changed = true;
            for &dependency in dependencies(fragment) {
                dependency_edges.push((target_id, dependency));
                changed |= included.insert(dependency);
            }
        }
        if !changed {
            break;
        }
    }
    (selected, dependency_edges)
}

/// Creates a diagnostic identity for a reflected type identifier.
///
/// # Parameters
///
/// * `type_id` — the type identity to describe.
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
    ///
    /// # Type Parameters
    ///
    /// * `T` — the reflected type to describe.
    pub fn type_of<T: Reflect + ?Sized>(&self) -> TypeDescriptor<'_> {
        TypeDescriptor::of_in::<T>(self)
    }
}

#[cfg(test)]
mod selection_tests {
    use super::select_fragment_slice;

    #[derive(Clone)]
    struct Fragment {
        target: Option<usize>,
        global: bool,
        dependencies: Vec<usize>,
    }

    fn select(fragments: &[Fragment], root: usize) -> (Vec<bool>, Vec<(usize, usize)>) {
        select_fragment_slice(
            fragments,
            root,
            |fragment| fragment.target,
            |fragment| fragment.dependencies.as_slice(),
            |fragment| fragment.global,
        )
    }

    #[test]
    fn selection_keeps_global_roots_cycles_shared_edges_and_input_order() {
        let fragments = [
            Fragment {
                target: Some(4),
                global: false,
                dependencies: vec![1, 2],
            },
            Fragment {
                target: Some(3),
                global: false,
                dependencies: vec![4],
            },
            Fragment {
                target: Some(2),
                global: false,
                dependencies: vec![3, 99],
            },
            Fragment {
                target: Some(1),
                global: false,
                dependencies: vec![3],
            },
            Fragment {
                target: Some(8),
                global: true,
                dependencies: vec![77],
            },
            Fragment {
                target: Some(6),
                global: false,
                dependencies: vec![],
            },
        ];
        let (selected, edges) = select(&fragments, 4);
        assert_eq!(selected, [true, true, true, true, true, false]);
        assert_eq!(edges, [(4, 1), (4, 2), (2, 3), (2, 99), (1, 3), (3, 4)]);
    }

    #[test]
    fn selection_does_not_expand_dependencies_of_preselected_global_fragments() {
        let fragments = [
            Fragment {
                target: Some(1),
                global: true,
                dependencies: vec![2],
            },
            Fragment {
                target: Some(2),
                global: false,
                dependencies: vec![],
            },
        ];
        let (selected, edges) = select(&fragments, 9);
        assert_eq!(selected, [true, false]);
        assert!(edges.is_empty());
    }
}
