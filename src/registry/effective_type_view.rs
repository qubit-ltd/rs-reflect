//! Deterministic effective-method views over one registered reflected root.

use crate::descriptor::ImplDescriptor;
use crate::descriptor::ImplKind;
use crate::descriptor::MethodImplementationSource;
use crate::descriptor::MethodInstanceDescriptor;
use crate::descriptor::TraitId;

/// A frozen effective view of the implementations and methods targeting one
/// concrete reflected type.
#[derive(Debug)]
pub struct EffectiveTypeView {
    implementations: Box<[&'static ImplDescriptor]>,
    methods: Box<[&'static MethodInstanceDescriptor]>,
}

impl EffectiveTypeView {
    /// Merges already deterministically ordered implementation fragments.
    pub(crate) fn new(implementations: &[&'static ImplDescriptor]) -> Self {
        let mut methods: Vec<&'static MethodInstanceDescriptor> = Vec::new();
        for implementation in implementations {
            for candidate in implementation.method_instances() {
                let duplicate = methods
                    .iter()
                    .position(|current| same_method_namespace(current, candidate, implementation));
                match duplicate {
                    Some(index) if candidate.implementation_source() == MethodImplementationSource::Overridden => {
                        methods[index] = candidate;
                    }
                    Some(_) => {}
                    None => methods.push(candidate),
                }
            }
        }
        Self {
            implementations: implementations.to_vec().into_boxed_slice(),
            methods: methods.into_boxed_slice(),
        }
    }

    /// Returns all contributing impl fragments in registry-stable order.
    pub const fn implementations(&self) -> &[&'static ImplDescriptor] {
        &self.implementations
    }

    /// Returns effective methods in deterministic implementation and source
    /// declaration order. An explicit trait implementation replaces its
    /// corresponding defaulted method instance.
    pub const fn methods(&self) -> &[&'static MethodInstanceDescriptor] {
        &self.methods
    }

    /// Looks up a method using the same qualified lookup contract as the raw
    /// implementation slice.
    pub fn lookup_method<'a>(
        &'a self,
        qualifier: crate::descriptor::MethodQualifier<'_>,
        name: &str,
    ) -> crate::descriptor::MethodLookup<'a> {
        ImplDescriptor::lookup_method(self.implementations(), qualifier, name)
    }
}

/// Returns whether two instances occupy the same effective method slot.
fn same_method_namespace(
    current: &MethodInstanceDescriptor,
    candidate: &MethodInstanceDescriptor,
    candidate_implementation: &ImplDescriptor,
) -> bool {
    let same_declaration = current.declaration().identity() == candidate.declaration().identity();
    if !same_declaration {
        return false;
    }
    let current_kind = current
        .declaration()
        .declaring_impl()
        .map_or(ImplKind::Trait, |_| ImplKind::Inherent);
    match current_kind {
        ImplKind::Inherent => candidate_implementation.kind() == ImplKind::Inherent,
        ImplKind::Trait => candidate_implementation
            .implemented_trait()
            .is_some_and(|trait_descriptor| trait_namespace_matches(current, trait_descriptor.definition().trait_id())),
    }
}

/// Checks whether a trait method instance belongs to one applied trait ID.
fn trait_namespace_matches(instance: &MethodInstanceDescriptor, trait_id: &TraitId) -> bool {
    instance
        .declaration()
        .declaring_trait()
        .is_some_and(|definition| definition.trait_id() == trait_id)
}
