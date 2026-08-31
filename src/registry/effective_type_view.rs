//! Deterministic effective-method views over one registered reflected root.

use crate::descriptor::ImplDescriptor;
use crate::descriptor::ImplKind;
use crate::descriptor::MethodImplementationSource;
use crate::descriptor::MethodInstanceDescriptor;
use crate::descriptor::MethodLookup;
use crate::descriptor::MethodQualifier;

/// A frozen effective view of the implementations and methods targeting one
/// concrete reflected type.
#[derive(Debug)]
pub struct EffectiveTypeView {
    implementations: Box<[&'static ImplDescriptor]>,
    method_implementations: Box<[&'static ImplDescriptor]>,
    methods: Box<[&'static MethodInstanceDescriptor]>,
}

impl EffectiveTypeView {
    /// Creates the stable empty view owned by a frozen registry.
    pub(crate) fn empty() -> Self {
        Self {
            implementations: Box::new([]),
            method_implementations: Box::new([]),
            methods: Box::new([]),
        }
    }

    /// Merges already deterministically ordered implementation fragments.
    pub(crate) fn new(implementations: &[&'static ImplDescriptor]) -> Self {
        let mut method_implementations = Vec::new();
        let mut methods: Vec<&'static MethodInstanceDescriptor> = Vec::new();
        for implementation in implementations {
            for candidate in implementation.method_instances() {
                let duplicate = methods.iter().enumerate().position(|(index, current)| {
                    same_method_application(method_implementations[index], current, implementation, candidate)
                });
                match duplicate {
                    Some(index) if candidate.implementation_source() == MethodImplementationSource::Overridden => {
                        method_implementations[index] = *implementation;
                        methods[index] = candidate;
                    }
                    Some(_) => {}
                    None => {
                        method_implementations.push(*implementation);
                        methods.push(candidate);
                    }
                }
            }
        }
        Self {
            implementations: implementations.to_vec().into_boxed_slice(),
            method_implementations: method_implementations.into_boxed_slice(),
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

    /// Looks up one of the same frozen effective entries returned by
    /// [`Self::methods`].
    pub fn lookup_method<'a>(&'a self, qualifier: MethodQualifier<'_>, name: &str) -> MethodLookup<'a> {
        let mut found = None;
        for (implementation, instance) in self.method_implementations.iter().zip(&self.methods) {
            if !matches_qualifier(implementation, qualifier) || instance.declaration().query_name() != name {
                continue;
            }
            if found.is_some() {
                return MethodLookup::Ambiguous;
            }
            found = Some(*instance);
        }
        found.map_or(MethodLookup::Missing, MethodLookup::Unique)
    }
}

/// Returns whether two instances occupy the same complete effective
/// application slot.
fn same_method_application(
    current_implementation: &ImplDescriptor,
    current: &MethodInstanceDescriptor,
    candidate_implementation: &ImplDescriptor,
    candidate: &MethodInstanceDescriptor,
) -> bool {
    if current.declaration().identity() != candidate.declaration().identity()
        || current.arguments() != candidate.arguments()
        || !same_impl_application(current_implementation, candidate_implementation)
    {
        return false;
    }
    match (
        current_implementation.implemented_trait(),
        candidate_implementation.implemented_trait(),
    ) {
        (None, None) => true,
        (Some(current_trait), Some(candidate_trait)) => current_trait.same_application(candidate_trait),
        _ => false,
    }
}

/// Returns whether two descriptors represent the same concrete impl
/// application before method specialization is considered.
fn same_impl_application(current: &ImplDescriptor, candidate: &ImplDescriptor) -> bool {
    current.kind() == candidate.kind()
        && current.definition().fragment_identity() == candidate.definition().fragment_identity()
        && current.arguments() == candidate.arguments()
        && current.target_type().type_id() == candidate.target_type().type_id()
}

/// Returns whether an effective method's concrete implementation belongs to
/// the requested namespace.
fn matches_qualifier(implementation: &ImplDescriptor, qualifier: MethodQualifier<'_>) -> bool {
    match qualifier {
        MethodQualifier::Any => true,
        MethodQualifier::Inherent => implementation.kind() == ImplKind::Inherent,
        MethodQualifier::Trait(expected) => implementation
            .implemented_trait()
            .is_some_and(|actual| actual.same_application(expected)),
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use super::EffectiveTypeView;
    use crate::descriptor::ImplDefinitionDescriptor;
    use crate::descriptor::ImplDescriptor;
    use crate::descriptor::ImplKind;
    use crate::descriptor::InvocationUnavailableReason;
    use crate::descriptor::MethodDeclarationOwner;
    use crate::descriptor::MethodDescriptor;
    use crate::descriptor::MethodImplementationSource;
    use crate::descriptor::MethodInstanceDescriptor;
    use crate::descriptor::MethodLookup;
    use crate::descriptor::MethodQualifier;
    use crate::descriptor::PrimitiveKind;
    use crate::descriptor::TraitCompleteness;
    use crate::descriptor::TraitDefinitionDescriptor;
    use crate::descriptor::TraitDescriptor;
    use crate::descriptor::TraitId;
    use crate::descriptor::TypeDescriptor;
    use crate::expression::ConcreteTypeExpression;
    use crate::expression::DiagnosticText;
    use crate::expression::GenericArgument;
    use crate::expression::GenericDefinitionDescriptor;
    use crate::expression::GenericParameterDescriptor;
    use crate::expression::TypeExpression;
    use crate::identity::FragmentIdentity;
    use crate::identity::MemberId;

    struct Target;
    struct GenericTraitMarker;

    /// Returns the exact reflected target shared by view fixtures.
    fn target_type() -> &'static TypeDescriptor {
        static TARGET: TypeDescriptor = crate::__private::descriptor::primitive::<Target>("Target", PrimitiveKind::U8);
        &TARGET
    }

    /// Leaks an empty generic declaration for descriptor fixtures.
    fn empty_generics() -> &'static GenericDefinitionDescriptor {
        Box::leak(Box::new(GenericDefinitionDescriptor {
            parameters: Box::new([]),
            predicates: Box::new([]),
            diagnostic: DiagnosticText::default(),
        }))
    }

    /// Leaks one type-parameter generic declaration for descriptor fixtures.
    fn one_type_parameter() -> &'static GenericDefinitionDescriptor {
        Box::leak(Box::new(GenericDefinitionDescriptor {
            parameters: Box::new([GenericParameterDescriptor::Type {
                name: "T".into(),
                bounds: Box::new([]),
                default: None,
                diagnostic: DiagnosticText::default(),
            }]),
            predicates: Box::new([]),
            diagnostic: DiagnosticText::default(),
        }))
    }

    /// Builds one concrete type argument with stable structural identity.
    fn type_argument(name: &str) -> GenericArgument {
        GenericArgument::Type(TypeExpression::Concrete(ConcreteTypeExpression {
            path: Box::new([name.into()]),
            arguments: Box::new([]),
            diagnostic: DiagnosticText::from(name),
        }))
    }

    /// Builds one stable fixture member identity.
    fn method_id(declaring_identity: &str) -> MemberId {
        MemberId::new(
            declaring_identity,
            "method",
            0,
            FragmentIdentity::new("fixture", declaring_identity, 1, 1, "method", 1),
        )
    }

    /// Builds one unavailable instance without introducing an adapter.
    fn instance(
        declaration: &'static MethodDescriptor,
        arguments: Box<[GenericArgument]>,
        source: MethodImplementationSource,
    ) -> MethodInstanceDescriptor {
        MethodInstanceDescriptor::with_arguments(
            declaration,
            None,
            source,
            None,
            arguments,
            Box::new([InvocationUnavailableReason::DisabledByPolicy]),
        )
        .expect("the fixture method instance must be internally consistent")
    }

    /// Verifies method specializations sharing a declaration remain distinct
    /// effective entries and therefore require a qualified specialization API.
    #[test]
    fn test_effective_view_preserves_method_specialization_arguments() {
        let definition = Box::leak(Box::new(
            ImplDefinitionDescriptor::new(
                FragmentIdentity::new("fixture", "method_specialization", 1, 1, "impl", 1),
                TypeExpression::Parameter("Self".into()),
                ImplKind::Inherent,
                None,
                empty_generics(),
            )
            .expect("the inherent impl definition must build"),
        ));
        let declaration = Box::leak(Box::new(
            MethodDescriptor::builder(
                method_id("method_specialization"),
                "specialized",
                "specialized",
                MethodDeclarationOwner::Impl(definition),
            )
            .build(),
        ));
        let implementation = Box::leak(Box::new(
            ImplDescriptor::builder(definition, target_type)
                .methods(std::slice::from_ref(declaration))
                .method_instances(vec![
                    instance(
                        declaration,
                        Box::new([type_argument("u8")]),
                        MethodImplementationSource::Declared,
                    ),
                    instance(
                        declaration,
                        Box::new([type_argument("u16")]),
                        MethodImplementationSource::Declared,
                    ),
                ])
                .build()
                .expect("the specialized inherent impl must build"),
        ));

        let view = EffectiveTypeView::new(&[implementation]);

        assert_eq!(view.methods().len(), 2);
        assert!(matches!(
            view.lookup_method(MethodQualifier::Inherent, "specialized"),
            MethodLookup::Ambiguous
        ));
    }

    /// Verifies distinct applications of one generic impl retain separate
    /// effective method slots even when they target the same root.
    #[test]
    fn test_effective_view_preserves_impl_application_arguments() {
        let definition = Box::leak(Box::new(
            ImplDefinitionDescriptor::new(
                FragmentIdentity::new("fixture", "impl_application", 1, 1, "impl", 2),
                TypeExpression::Parameter("Self".into()),
                ImplKind::Inherent,
                None,
                one_type_parameter(),
            )
            .expect("the generic inherent impl definition must build"),
        ));
        let declaration = Box::leak(Box::new(
            MethodDescriptor::builder(
                method_id("impl_application"),
                "applied",
                "applied",
                MethodDeclarationOwner::Impl(definition),
            )
            .build(),
        ));
        let build_application = |argument| {
            Box::leak(Box::new(
                ImplDescriptor::builder(definition, target_type)
                    .methods(std::slice::from_ref(declaration))
                    .method_instances(vec![instance(
                        declaration,
                        Box::new([]),
                        MethodImplementationSource::Declared,
                    )])
                    .arguments(vec![argument])
                    .build()
                    .expect("the generic impl application must build"),
            )) as &'static ImplDescriptor
        };
        let first = build_application(type_argument("u8"));
        let second = build_application(type_argument("u16"));

        let view = EffectiveTypeView::new(&[first, second]);

        assert_eq!(view.methods().len(), 2);
        assert!(matches!(
            view.lookup_method(MethodQualifier::Inherent, "applied"),
            MethodLookup::Ambiguous
        ));
    }

    /// Verifies two applications of one generic trait remain distinct and
    /// qualified lookup uses the complete applied trait identity.
    #[test]
    fn test_effective_view_preserves_applied_trait_arguments() {
        let trait_definition = Box::leak(Box::new(TraitDefinitionDescriptor::new(
            TraitId::Reflected(TypeId::of::<GenericTraitMarker>()),
            "GenericTrait",
            "fixture::GenericTrait",
            "generic_trait",
            TraitCompleteness::Complete,
            one_type_parameter(),
        )));
        let declaration = Box::leak(Box::new(
            MethodDescriptor::builder(
                method_id("generic_trait"),
                "same",
                "same",
                MethodDeclarationOwner::Trait(trait_definition),
            )
            .build(),
        ));
        let build_trait = |argument| {
            Box::leak(Box::new(
                TraitDescriptor::builder(trait_definition)
                    .arguments(vec![argument])
                    .methods(std::slice::from_ref(declaration))
                    .build()
                    .expect("the generic trait application must build"),
            )) as &'static TraitDescriptor
        };
        let first_trait = build_trait(type_argument("u8"));
        let second_trait = build_trait(type_argument("u16"));
        let impl_definition = Box::leak(Box::new(
            ImplDefinitionDescriptor::new(
                FragmentIdentity::new("fixture", "generic_trait_impl", 3, 1, "impl", 3),
                TypeExpression::Parameter("Self".into()),
                ImplKind::Trait,
                Some(trait_definition),
                empty_generics(),
            )
            .expect("the trait impl definition must build"),
        ));
        let build_impl = |implemented_trait: &'static TraitDescriptor| {
            Box::leak(Box::new(
                ImplDescriptor::builder(impl_definition, target_type)
                    .implemented_trait(implemented_trait)
                    .method_instances(vec![instance(
                        declaration,
                        Box::new([]),
                        MethodImplementationSource::Defaulted,
                    )])
                    .build()
                    .expect("the applied trait impl must build"),
            )) as &'static ImplDescriptor
        };
        let first_impl = build_impl(first_trait);
        let second_impl = build_impl(second_trait);

        let view = EffectiveTypeView::new(&[first_impl, second_impl]);

        assert_eq!(view.methods().len(), 2);
        assert!(matches!(
            view.lookup_method(MethodQualifier::Any, "same"),
            MethodLookup::Ambiguous
        ));
        assert!(matches!(
            view.lookup_method(MethodQualifier::Trait(first_trait), "same"),
            MethodLookup::Unique(_)
        ));
        assert!(matches!(
            view.lookup_method(MethodQualifier::Trait(second_trait), "same"),
            MethodLookup::Unique(_)
        ));
    }
}
