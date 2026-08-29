use std::any::TypeId;
use std::sync::LazyLock;

use qubit_reflect::descriptor::{
    AssociatedConstBindingDescriptor, AssociatedConstDescriptor,
    AssociatedConstImplementationSource, AssociatedConstReader, AssociatedTypeBindingDescriptor,
    AssociatedTypeDescriptor, ImplDefinitionDescriptor, ImplDescriptor, ImplKind,
    InvocationAdapter, InvocationUnavailableReason, MethodDeclarationOwner, MethodDescriptor,
    MethodImplementationSource, MethodInstanceBuildError, MethodInstanceDescriptor, MethodLookup,
    MethodQualifier, MethodQualifiers, MethodVisibility, ParameterDescriptor, ParameterPassingMode,
    ParameterPatternDescriptor, ReceiverDescriptor, ReturnDescriptor, ReturnKind,
    TraitCompleteness, TraitDefinitionDescriptor, TraitDescriptor, TraitDescriptorBuildError,
    TraitId,
};
use qubit_reflect::descriptor::{PrimitiveKind, TypeDescriptor, TypeDescriptorResolver};
use qubit_reflect::expression::{
    ConcreteTypeExpression, DiagnosticText, GenericArgument, GenericDefinitionDescriptor,
    GenericParameterDescriptor, LifetimeExpression, PredicateDescriptor, TypeExpression,
};
use qubit_reflect::identity::{ExternalTraitId, FragmentIdentity, MemberId, Visibility};
use qubit_reflect::value::ReflectedOwned;

struct RootMarker;
struct MiddleMarker;
struct LeafMarker;
struct GenericMarker;
struct OtherMarker;

static EMPTY_GENERIC_DEFINITION: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor {
        parameters: Box::new([]),
        predicates: Box::new([]),
        diagnostic: DiagnosticText::default(),
    });

static GENERIC_TRAIT_DEFINITION: LazyLock<GenericDefinitionDescriptor> =
    LazyLock::new(|| GenericDefinitionDescriptor {
        parameters: Box::new([GenericParameterDescriptor::Type {
            name: "T".into(),
            bounds: Box::new([]),
            default: None,
            diagnostic: DiagnosticText::default(),
        }]),
        predicates: Box::new([]),
        diagnostic: DiagnosticText::default(),
    });

static ROOT_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<RootMarker>()),
        "Root",
        "fixture::Root",
        "root",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC_DEFINITION,
    )
});

static MIDDLE_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<MiddleMarker>()),
        "Middle",
        "fixture::Middle",
        "middle",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC_DEFINITION,
    )
});

static LEAF_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<LeafMarker>()),
        "Leaf",
        "fixture::Leaf",
        "leaf",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC_DEFINITION,
    )
});

static GENERIC_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<GenericMarker>()),
        "Generic",
        "fixture::Generic",
        "generic",
        TraitCompleteness::Complete,
        &GENERIC_TRAIT_DEFINITION,
    )
});

static ROOT_TRAIT: LazyLock<&'static TraitDescriptor> = LazyLock::new(|| {
    Box::leak(Box::new(
        TraitDescriptor::builder(&ROOT_DEFINITION)
            .build()
            .expect("root trait facts must be valid"),
    ))
});

static MIDDLE_TRAIT: LazyLock<&'static TraitDescriptor> = LazyLock::new(|| {
    Box::leak(Box::new(
        TraitDescriptor::builder(&MIDDLE_DEFINITION)
            .direct_supertraits([*ROOT_TRAIT])
            .build()
            .expect("middle trait facts must be valid"),
    ))
});

fn target_type() -> &'static TypeDescriptor {
    static TARGET: TypeDescriptor =
        qubit_reflect::__private::descriptor::primitive::<u32>("u32", PrimitiveKind::U32);
    &TARGET
}

fn concrete_u32_expression() -> TypeExpression {
    TypeExpression::Concrete(qubit_reflect::expression::ConcreteTypeExpression {
        path: vec!["u32".into()].into_boxed_slice(),
        arguments: Box::new([]),
        diagnostic: DiagnosticText::from("u32"),
    })
}

fn member_id(kind: &str, index: usize) -> MemberId {
    MemberId::new(
        "fixture::Leaf",
        kind,
        index,
        FragmentIdentity::new("fixture", "traits", 1, 1, kind, index as u64),
    )
}

fn invocation_adapter_token() {}

static INVOCATION_ADAPTER: InvocationAdapter = InvocationAdapter::new(invocation_adapter_token);

fn read_limit() -> ReflectedOwned {
    ReflectedOwned::new(32_u32)
}

static ASSOCIATED_CONST_READER: AssociatedConstReader = AssociatedConstReader::new(read_limit);

#[test]
fn test_trait_descriptor_navigation_preserves_direct_order_and_builds_sorted_transitive_closure() {
    let leaf = TraitDescriptor::builder(&LEAF_DEFINITION)
        .direct_supertraits([*ROOT_TRAIT, *MIDDLE_TRAIT])
        .build()
        .expect("acyclic trait graph must build");

    let direct_paths: Vec<_> = leaf
        .direct_supertraits()
        .iter()
        .map(|descriptor| descriptor.rust_path())
        .collect();
    assert_eq!(direct_paths, ["fixture::Root", "fixture::Middle"]);

    let all_paths: Vec<_> = leaf
        .all_supertraits()
        .iter()
        .map(|descriptor| descriptor.rust_path())
        .collect();
    assert_eq!(all_paths, ["fixture::Middle", "fixture::Root"]);
}

#[test]
fn test_trait_descriptor_builder_rejects_recursive_identity_and_unproven_external_facts() {
    let recursion = TraitDescriptor::builder(&ROOT_DEFINITION)
        .direct_supertraits([*ROOT_TRAIT])
        .build();
    assert!(matches!(
        recursion,
        Err(TraitDescriptorBuildError::RecursiveSupertrait { .. })
    ));

    let external_definition = TraitDefinitionDescriptor::new(
        TraitId::External(
            ExternalTraitId::new("fixture.external.display")
                .expect("fixture external trait ID must be valid"),
        ),
        "Display",
        "std::fmt::Display",
        "display",
        TraitCompleteness::ExternalIncomplete,
        &EMPTY_GENERIC_DEFINITION,
    );
    let unproven_item = AssociatedTypeDescriptor::new(0, "Output", "output", Box::new([]), None);
    let external = TraitDescriptor::builder(Box::leak(Box::new(external_definition)))
        .associated_types(vec![unproven_item])
        .build();
    assert!(matches!(
        external,
        Err(TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts)
    ));
}

#[test]
fn test_trait_descriptor_method_preserves_signature_visibility_and_generic_facts() {
    let parameter = ParameterDescriptor::new(
        0,
        Some("value"),
        ParameterPatternDescriptor::Identifier,
        ParameterPassingMode::Owned,
        concrete_u32_expression(),
        Some(target_type as TypeDescriptorResolver),
    );
    let method = MethodDescriptor::builder(
        member_id("method", 0),
        "apply",
        "run",
        MethodDeclarationOwner::Trait(&GENERIC_DEFINITION),
    )
    .visibility(MethodVisibility::Declared(Visibility::Crate))
    .receiver(Some(ReceiverDescriptor::Shared))
    .parameters(vec![parameter])
    .return_value(ReturnDescriptor::new(
        ReturnKind::Concrete,
        Some(concrete_u32_expression()),
        Some(target_type as TypeDescriptorResolver),
    ))
    .qualifiers(MethodQualifiers {
        is_async: true,
        is_unsafe: false,
        is_const: false,
        abi: None,
        is_variadic: false,
    })
    .generic_definition(&GENERIC_TRAIT_DEFINITION)
    .build();

    assert_eq!(method.rust_name(), "apply");
    assert_eq!(method.query_name(), "run");
    assert_eq!(
        method.visibility(),
        &MethodVisibility::Declared(Visibility::Crate)
    );
    assert_eq!(method.receiver(), Some(&ReceiverDescriptor::Shared));
    assert_eq!(
        method.parameter("value").map(ParameterDescriptor::index),
        Some(0)
    );
    assert_eq!(
        method
            .parameter_at(0)
            .and_then(ParameterDescriptor::concrete_type)
            .map(TypeDescriptor::type_id),
        Some(target_type().type_id())
    );
    assert_eq!(method.return_value().kind(), ReturnKind::Concrete);
    assert!(method.qualifiers().is_async);
    assert_eq!(method.generic_definition().parameters.len(), 1);
    assert_eq!(
        method
            .declaring_trait()
            .map(TraitDefinitionDescriptor::rust_path),
        Some("fixture::Generic")
    );
}

#[test]
fn test_trait_descriptor_rejects_non_concrete_or_incomplete_applications() {
    let missing = TraitDescriptor::builder(&GENERIC_DEFINITION).build();
    assert!(matches!(
        missing,
        Err(TraitDescriptorBuildError::GenericArgumentCount { .. })
    ));

    let symbolic = TraitDescriptor::builder(&GENERIC_DEFINITION)
        .arguments(vec![GenericArgument::Type(TypeExpression::Parameter(
            "T".into(),
        ))])
        .build();
    assert!(matches!(
        symbolic,
        Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index: 0 })
    ));

    let concrete_with_lifetime = TypeExpression::Concrete(ConcreteTypeExpression {
        path: vec!["Cow".into()].into_boxed_slice(),
        arguments: vec![GenericArgument::Lifetime(LifetimeExpression::Static)].into_boxed_slice(),
        diagnostic: DiagnosticText::from("Cow<'static, str>"),
    });
    assert!(
        TraitDescriptor::builder(&GENERIC_DEFINITION)
            .arguments(vec![GenericArgument::Type(concrete_with_lifetime)])
            .build()
            .is_ok()
    );
}

#[test]
fn test_trait_descriptor_applied_impl_preserves_items_sources_and_qualified_lookup() {
    let arguments = vec![GenericArgument::Type(concrete_u32_expression())];
    let associated_type = AssociatedTypeDescriptor::new(
        0,
        "Item",
        "item",
        Box::<[PredicateDescriptor]>::default(),
        None,
    );
    let associated_const =
        AssociatedConstDescriptor::new(0, "LIMIT", "limit", concrete_u32_expression(), true);
    let methods: &'static [MethodDescriptor] = Box::leak(
        vec![
            MethodDescriptor::builder(
                member_id("method", 1),
                "same",
                "same",
                MethodDeclarationOwner::Trait(&GENERIC_DEFINITION),
            )
            .visibility(MethodVisibility::InheritedFromTrait)
            .return_value(ReturnDescriptor::unit())
            .build(),
        ]
        .into_boxed_slice(),
    );
    let declaration = &methods[0];
    let applied = Box::leak(Box::new(
        TraitDescriptor::builder(&GENERIC_DEFINITION)
            .arguments(arguments.clone())
            .methods(methods)
            .associated_types(vec![associated_type])
            .associated_consts(vec![associated_const])
            .build()
            .expect("complete applied trait facts must build"),
    ));

    assert_eq!(applied.definition().rust_path(), "fixture::Generic");
    assert_eq!(applied.arguments(), arguments);
    assert_eq!(
        applied.method("same").map(MethodDescriptor::rust_name),
        Some("same")
    );
    assert_eq!(
        applied
            .associated_type("item")
            .map(AssociatedTypeDescriptor::index),
        Some(0)
    );
    assert_eq!(
        applied
            .associated_const("limit")
            .map(AssociatedConstDescriptor::index),
        Some(0)
    );
    let associated_type = &applied.associated_types()[0];
    let associated_const = &applied.associated_consts()[0];

    let generic_impl_definition = Box::leak(Box::new(
        ImplDefinitionDescriptor::new(
            FragmentIdentity::new("fixture", "traits", 20, 1, "impl", 20),
            TypeExpression::Parameter("T".into()),
            ImplKind::Trait,
            Some(&GENERIC_DEFINITION),
            &GENERIC_TRAIT_DEFINITION,
        )
        .expect("generic impl definition must be valid"),
    ));
    let impl_methods: &'static [MethodDescriptor] = Box::leak(
        vec![
            MethodDescriptor::builder(
                member_id("impl_method", 0),
                "same",
                "same",
                MethodDeclarationOwner::Impl(generic_impl_definition),
            )
            .visibility(MethodVisibility::Declared(Visibility::Private))
            .return_value(ReturnDescriptor::unit())
            .build(),
        ]
        .into_boxed_slice(),
    );
    let impl_method = &impl_methods[0];
    let overridden = MethodInstanceDescriptor::new(
        declaration,
        Some(impl_method),
        MethodImplementationSource::Overridden,
        Some(&INVOCATION_ADAPTER),
        Box::new([]),
    )
    .expect("an overridden method must name its concrete impl method");
    let generic_impl = Box::leak(Box::new(
        ImplDescriptor::builder(generic_impl_definition, target_type)
            .implemented_trait(applied)
            .methods(impl_methods)
            .method_instances(vec![overridden])
            .associated_types(vec![AssociatedTypeBindingDescriptor::new(
                associated_type,
                concrete_u32_expression(),
                Some(target_type),
            )])
            .associated_consts(vec![AssociatedConstBindingDescriptor::new(
                associated_const,
                AssociatedConstImplementationSource::Defaulted,
                Some(&ASSOCIATED_CONST_READER),
            )])
            .arguments(arguments.clone())
            .build()
            .expect("trait impl must name its implemented trait"),
    ));
    let nested_symbolic = TypeExpression::Concrete(ConcreteTypeExpression {
        path: vec!["Vec".into()].into_boxed_slice(),
        arguments: vec![GenericArgument::Type(TypeExpression::Parameter("T".into()))]
            .into_boxed_slice(),
        diagnostic: DiagnosticText::from("Vec<T>"),
    });
    assert!(
        ImplDescriptor::builder(generic_impl_definition, target_type)
            .implemented_trait(applied)
            .arguments(vec![GenericArgument::Type(nested_symbolic)])
            .build()
            .is_err()
    );

    let other_definition = Box::leak(Box::new(TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<OtherMarker>()),
        "Other",
        "fixture::Other",
        "other",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC_DEFINITION,
    )));
    let other_methods: &'static [MethodDescriptor] = Box::leak(
        vec![
            MethodDescriptor::builder(
                member_id("other_method", 0),
                "same",
                "same",
                MethodDeclarationOwner::Trait(other_definition),
            )
            .visibility(MethodVisibility::InheritedFromTrait)
            .return_value(ReturnDescriptor::unit())
            .build(),
        ]
        .into_boxed_slice(),
    );
    let other_trait = Box::leak(Box::new(
        TraitDescriptor::builder(other_definition)
            .methods(other_methods)
            .build()
            .expect("second complete trait must build"),
    ));
    let defaulted = MethodInstanceDescriptor::new(
        &other_methods[0],
        None,
        MethodImplementationSource::Defaulted,
        None,
        Box::new([InvocationUnavailableReason::DisabledByPolicy]),
    )
    .expect("a defaulted method uses its trait declaration");
    let other_impl = Box::leak(Box::new(
        ImplDescriptor::builder(
            Box::leak(Box::new(
                ImplDefinitionDescriptor::new(
                    FragmentIdentity::new("fixture", "traits", 30, 1, "impl", 30),
                    concrete_u32_expression(),
                    ImplKind::Trait,
                    Some(other_definition),
                    &EMPTY_GENERIC_DEFINITION,
                )
                .expect("other impl definition must be valid"),
            )),
            target_type,
        )
        .implemented_trait(other_trait)
        .method_instances(vec![defaulted])
        .build()
        .expect("second trait impl must build"),
    ));

    let impls = [
        generic_impl as &ImplDescriptor,
        other_impl as &ImplDescriptor,
    ];
    assert!(matches!(
        ImplDescriptor::lookup_method(&impls, MethodQualifier::Any, "same"),
        MethodLookup::Ambiguous
    ));
    let lookup = ImplDescriptor::lookup_method(&impls, MethodQualifier::Trait(applied), "same");
    let MethodLookup::Unique(instance) = lookup else {
        panic!("qualified lookup must select the generic trait method");
    };
    assert_eq!(
        instance.implementation_source(),
        MethodImplementationSource::Overridden
    );
    assert_eq!(
        instance
            .implementation_method()
            .map(MethodDescriptor::rust_name),
        Some("same")
    );
    assert_eq!(
        generic_impl.associated_types()[0]
            .concrete_type()
            .map(TypeDescriptor::type_id),
        Some(target_type().type_id())
    );
    assert_eq!(
        generic_impl.associated_consts()[0].implementation_source(),
        AssociatedConstImplementationSource::Defaulted
    );
    let value = generic_impl.associated_consts()[0]
        .read()
        .expect("the associated const reader must be available");
    let Ok(value) = value.downcast::<u32>() else {
        panic!("the reader must preserve the declared type");
    };
    assert_eq!(value, 32);
    assert_eq!(
        generic_impl
            .definition()
            .generic_definition()
            .parameters
            .len(),
        1
    );
    assert_eq!(generic_impl.arguments(), arguments);
    assert_eq!(
        instance
            .implementation_method()
            .and_then(MethodDescriptor::declaring_impl)
            .map(ImplDefinitionDescriptor::fragment_identity),
        Some(generic_impl_definition.fragment_identity())
    );
    let MethodLookup::Unique(defaulted) =
        ImplDescriptor::lookup_method(&impls, MethodQualifier::Trait(other_trait), "same")
    else {
        panic!("qualified lookup must select the second trait method");
    };
    assert_eq!(
        defaulted.implementation_source(),
        MethodImplementationSource::Defaulted
    );

    let contradictory = MethodInstanceDescriptor::new(
        declaration,
        None,
        MethodImplementationSource::Required,
        Some(&INVOCATION_ADAPTER),
        Box::new([]),
    );
    assert!(matches!(
        contradictory,
        Err(MethodInstanceBuildError::RequiredMethodHasAdapter)
    ));

    let inherent_definition = Box::leak(Box::new(
        ImplDefinitionDescriptor::new(
            FragmentIdentity::new("fixture", "traits", 40, 1, "impl", 40),
            concrete_u32_expression(),
            ImplKind::Inherent,
            None,
            &EMPTY_GENERIC_DEFINITION,
        )
        .expect("inherent impl definition must be valid"),
    ));
    let inherent_methods: &'static [MethodDescriptor] = Box::leak(
        vec![
            MethodDescriptor::builder(
                member_id("inherent_method", 0),
                "same",
                "same",
                MethodDeclarationOwner::Impl(inherent_definition),
            )
            .return_value(ReturnDescriptor::unit())
            .build(),
        ]
        .into_boxed_slice(),
    );
    let declared = MethodInstanceDescriptor::new(
        &inherent_methods[0],
        None,
        MethodImplementationSource::Declared,
        Some(&INVOCATION_ADAPTER),
        Box::new([]),
    )
    .expect("an inherent method is a declared concrete instance");
    let inherent_impl = ImplDescriptor::builder(inherent_definition, target_type)
        .methods(inherent_methods)
        .method_instances(vec![declared])
        .build()
        .expect("inherent concrete impl must build");
    let inherent_impls = [&inherent_impl];
    let MethodLookup::Unique(inherent) =
        ImplDescriptor::lookup_method(&inherent_impls, MethodQualifier::Inherent, "same")
    else {
        panic!("inherent lookup must find its declared method");
    };
    assert_eq!(
        inherent
            .effective_method()
            .declaring_impl()
            .map(ImplDefinitionDescriptor::fragment_identity),
        Some(inherent_definition.fragment_identity())
    );
}
