use std::any::TypeId;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::{
    GenericArgumentMetadata, GenericArgumentMetadataKind, GenericDefinitionMetadata,
    GenericParameterMetadata, GenericParameterMetadataKind,
};
use crate::runtime::descriptor::impl_descriptor::{
    AssociatedConstBindingMetadata, AssociatedConstValueAdapter, AssociatedItemSource,
    AssociatedTypeBindingMetadata, ImplApplicationMetadata, ImplDefinitionId,
    ImplDefinitionMetadata, ImplKind, ImplQualifiers, Impls,
};
use crate::runtime::descriptor::metadata::TypeMetadata;
use crate::runtime::descriptor::method::{
    EffectiveMethod, FunctionAbi, MethodAvailability, MethodImplementationSource, MethodMetadata,
    MethodNamespaceMetadata, MethodQualifiers, MethodVisibility, ReturnKind, ReturnMetadata,
};
use crate::runtime::descriptor::trait_descriptor::{
    AssociatedConstDefinitionMetadata, AssociatedTypeDefinitionMetadata, TraitApplicationMetadata,
    TraitArgumentIdentity, TraitCompleteness, TraitDefinitionDescriptor, TraitDefinitionId,
    TraitDefinitionMetadata, TraitExpressionMetadata, Visibility,
};
use crate::runtime::expression::{
    ConstExpression, ExpressionData, PredicateData, TraitExpressionData,
};
use crate::runtime::lookup::Lookup;
use crate::runtime::value::{DynamicOwned, Local, ThreadSafe};

struct CustomerId;
struct ErrorValue;
struct Answer(u32);
struct RepositoryMarker;
struct StringImplMarker;
struct U64ImplMarker;

impl crate::runtime::descriptor::Reflect for CustomerId {
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<CustomerId> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}
impl crate::runtime::descriptor::Reflect for Answer {
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Answer> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}

static EMPTY_PREDICATES: &[PredicateData] = &[];
static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: EMPTY_PREDICATES,
};
static TRAIT_PARAMETER: GenericParameterMetadata = GenericParameterMetadata {
    name: "T",
    index: 0,
    kind: GenericParameterMetadataKind::Type {
        bounds: EMPTY_PREDICATES,
        default: None,
    },
};
static TRAIT_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[TRAIT_PARAMETER],
    predicates: EMPTY_PREDICATES,
};
static STRING_METADATA: TypeMetadata = TypeMetadata::new::<String>();
static U64_METADATA: TypeMetadata = TypeMetadata::new::<u64>();
static TARGET_METADATA: TypeMetadata = TypeMetadata::new::<CustomerId>();
fn string_type_metadata() -> &'static TypeMetadata {
    &STRING_METADATA
}
fn u64_type_metadata() -> &'static TypeMetadata {
    &U64_METADATA
}
fn error_type_metadata() -> &'static TypeMetadata {
    &ERROR_METADATA
}
fn target_type_metadata() -> &'static TypeMetadata {
    &TARGET_METADATA
}
static ERROR_METADATA: TypeMetadata = TypeMetadata::new::<ErrorValue>();
static STRING_EXPR: ExpressionData = ExpressionData::Named {
    path: "alloc::string::String",
    definition: None,
    arguments: &[],
};
static U64_EXPR: ExpressionData = ExpressionData::Named {
    path: "u64",
    definition: None,
    arguments: &[],
};
static ERROR_EXPR: ExpressionData = ExpressionData::Named {
    path: "fixture::ErrorValue",
    definition: None,
    arguments: &[],
};
static TARGET_EXPR: ExpressionData = ExpressionData::Named {
    path: "fixture::CustomerId",
    definition: None,
    arguments: &[],
};
static STRING_ARGUMENT: GenericArgumentMetadata = GenericArgumentMetadata {
    parameter: &TRAIT_PARAMETER,
    kind: GenericArgumentMetadataKind::Type {
        expression: &STRING_EXPR,
        concrete: Some(string_type_metadata),
    },
};
static U64_ARGUMENT: GenericArgumentMetadata = GenericArgumentMetadata {
    parameter: &TRAIT_PARAMETER,
    kind: GenericArgumentMetadataKind::Type {
        expression: &U64_EXPR,
        concrete: Some(u64_type_metadata),
    },
};
static STRING_ARGUMENTS: &[GenericArgumentMetadata] = &[STRING_ARGUMENT];
static U64_ARGUMENTS: &[GenericArgumentMetadata] = &[U64_ARGUMENT];
fn string_type_id() -> TypeId {
    TypeId::of::<String>()
}
fn u64_type_id() -> TypeId {
    TypeId::of::<u64>()
}
static STRING_IDENTITY: &[TraitArgumentIdentity] = &[TraitArgumentIdentity::Type(string_type_id)];
static U64_IDENTITY: &[TraitArgumentIdentity] = &[TraitArgumentIdentity::Type(u64_type_id)];

static ASSOCIATED_TYPE_ERROR: AssociatedTypeDefinitionMetadata = AssociatedTypeDefinitionMetadata {
    name: "Error",
    visibility: Visibility::Public,
    bounds: EMPTY_PREDICATES,
    default: Some(&ERROR_EXPR),
};
static ASSOCIATED_CONST_COUNT: AssociatedConstDefinitionMetadata =
    AssociatedConstDefinitionMetadata {
        name: "COUNT",
        visibility: Visibility::Public,
        ty: &U64_EXPR,
        default: Some(ConstExpression::UnsignedInteger(1)),
    };
static ASSOCIATED_TYPES: &[AssociatedTypeDefinitionMetadata] = &[ASSOCIATED_TYPE_ERROR];
static ASSOCIATED_CONSTS: &[AssociatedConstDefinitionMetadata] = &[ASSOCIATED_CONST_COUNT];
static NO_ASSOCIATED_TYPES: &[AssociatedTypeDefinitionMetadata] = &[];
static NO_ASSOCIATED_CONSTS: &[AssociatedConstDefinitionMetadata] = &[];
static NO_SUPERTRAITS: &[TraitExpressionMetadata] = &[];
static STRING_TRAIT_EXPRESSION_DATA: TraitExpressionData = TraitExpressionData {
    path: "fixture::Convert",
    arguments: &[crate::runtime::expression::GenericExpressionArgument::Type(
        &STRING_EXPR,
    )],
    lifetimes: &[],
};
static U64_TRAIT_EXPRESSION_DATA: TraitExpressionData = TraitExpressionData {
    path: "fixture::Convert",
    arguments: &[crate::runtime::expression::GenericExpressionArgument::Type(
        &U64_EXPR,
    )],
    lifetimes: &[],
};
static STRING_TRAIT_EXPRESSION: TraitExpressionMetadata = TraitExpressionMetadata {
    expression: &STRING_TRAIT_EXPRESSION_DATA,
    definition: Some(&TRAIT_DEFINITION),
};
static U64_TRAIT_EXPRESSION: TraitExpressionMetadata = TraitExpressionMetadata {
    expression: &U64_TRAIT_EXPRESSION_DATA,
    definition: Some(&TRAIT_DEFINITION),
};

static DEFAULT_OUTPUT: ReturnMetadata = ReturnMetadata {
    kind: ReturnKind::Unit,
    expression: None,
    concrete_type: None,
};
const fn make_method(default_body: bool) -> MethodMetadata {
    MethodMetadata {
        name: "normalize",
        namespace: MethodNamespaceMetadata::Trait {
            trait_: "fixture::Convert",
            implementation: "fixture::CustomerId",
        },
        visibility: MethodVisibility::Public,
        receiver: None,
        parameters: &[],
        output: &DEFAULT_OUTPUT,
        qualifiers: MethodQualifiers {
            is_const: false,
            is_async: false,
            is_unsafe: false,
            abi: None::<FunctionAbi>,
            is_variadic: false,
        },
        generics: &EMPTY_GENERICS,
        is_generic: false,
        default_body,
    }
}
static DEFAULT_METHODS: [MethodMetadata; 1] = [make_method(true)];
static OVERRIDE_METHODS: [MethodMetadata; 1] = [make_method(false)];
static TRAIT_METHODS: &[MethodMetadata] = &DEFAULT_METHODS;
static EMPTY_METHODS: &[MethodMetadata] = &[];
static DEFAULT_EFFECTIVE_METHODS: [EffectiveMethod; 1] = [EffectiveMethod {
    target: target_type_metadata,
    metadata: &DEFAULT_METHODS[0],
    source: MethodImplementationSource::Defaulted,
    availability: MethodAvailability::Callable,
    specializations: &[],
    specialization_specs: &[],
    adapters: None,
}];
static OVERRIDE_EFFECTIVE_METHODS: [EffectiveMethod; 1] = [EffectiveMethod {
    target: target_type_metadata,
    metadata: &OVERRIDE_METHODS[0],
    source: MethodImplementationSource::Overridden,
    availability: MethodAvailability::Callable,
    specializations: &[],
    specialization_specs: &[],
    adapters: None,
}];

static TRAIT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata {
    id: TraitDefinitionId::of::<RepositoryMarker>(),
    name: "Convert",
    path: "fixture::Convert",
    visibility: Visibility::Public,
    completeness: TraitCompleteness::Complete,
    generics: &TRAIT_GENERICS,
    supertraits: NO_SUPERTRAITS,
    methods: TRAIT_METHODS,
    associated_types: ASSOCIATED_TYPES,
    associated_consts: ASSOCIATED_CONSTS,
};
static STRING_TRAIT: TraitApplicationMetadata = TraitApplicationMetadata {
    definition: &TRAIT_DEFINITION,
    arguments: STRING_ARGUMENTS,
    identity: STRING_IDENTITY,
    direct_supertraits: &[],
    all_supertraits: &[],
    supertrait_completeness:
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
};
static U64_TRAIT: TraitApplicationMetadata = TraitApplicationMetadata {
    definition: &TRAIT_DEFINITION,
    arguments: U64_ARGUMENTS,
    identity: U64_IDENTITY,
    direct_supertraits: &[],
    all_supertraits: &[],
    supertrait_completeness:
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
};
fn dynamic_answer_local() -> DynamicOwned<Local> {
    DynamicOwned::new(Answer(42))
}
fn dynamic_answer_thread_safe() -> DynamicOwned<ThreadSafe> {
    DynamicOwned::new(Answer(42))
}
static ANSWER_ADAPTER: AssociatedConstValueAdapter = AssociatedConstValueAdapter {
    local: Some(dynamic_answer_local),
    thread_safe: Some(dynamic_answer_thread_safe),
};
static ERROR_BINDING: AssociatedTypeBindingMetadata = AssociatedTypeBindingMetadata {
    definition: &ASSOCIATED_TYPE_ERROR,
    expression: &ERROR_EXPR,
    concrete: Some(error_type_metadata),
    source: AssociatedItemSource::Declared,
};
static DEFAULT_ERROR_BINDING: AssociatedTypeBindingMetadata = AssociatedTypeBindingMetadata {
    source: AssociatedItemSource::Defaulted,
    ..ERROR_BINDING
};
static COUNT_BINDING: AssociatedConstBindingMetadata = AssociatedConstBindingMetadata {
    definition: &ASSOCIATED_CONST_COUNT,
    expression: ConstExpression::UnsignedInteger(3),
    source: AssociatedItemSource::Overridden,
    value: Some(&ANSWER_ADAPTER),
};
static DEFAULT_COUNT_BINDING: AssociatedConstBindingMetadata = AssociatedConstBindingMetadata {
    expression: ConstExpression::UnsignedInteger(1),
    source: AssociatedItemSource::Defaulted,
    value: None,
    ..COUNT_BINDING
};
static STRING_TYPES: &[AssociatedTypeBindingMetadata] = &[ERROR_BINDING];
static U64_TYPES: &[AssociatedTypeBindingMetadata] = &[DEFAULT_ERROR_BINDING];
static STRING_CONSTS: &[AssociatedConstBindingMetadata] = &[COUNT_BINDING];
static U64_CONSTS: &[AssociatedConstBindingMetadata] = &[DEFAULT_COUNT_BINDING];

static STRING_IMPL_DEF: ImplDefinitionMetadata = ImplDefinitionMetadata {
    id: ImplDefinitionId::of::<StringImplMarker>(),
    kind: ImplKind::Trait,
    target: &TARGET_EXPR,
    implemented_trait: Some(&STRING_TRAIT_EXPRESSION),
    generics: &EMPTY_GENERICS,
    qualifiers: ImplQualifiers { is_unsafe: false },
    methods: EMPTY_METHODS,
    associated_types: ASSOCIATED_TYPES,
    associated_consts: ASSOCIATED_CONSTS,
};
static U64_IMPL_DEF: ImplDefinitionMetadata = ImplDefinitionMetadata {
    id: ImplDefinitionId::of::<U64ImplMarker>(),
    kind: ImplKind::Trait,
    target: &TARGET_EXPR,
    implemented_trait: Some(&U64_TRAIT_EXPRESSION),
    generics: &EMPTY_GENERICS,
    qualifiers: ImplQualifiers { is_unsafe: false },
    methods: EMPTY_METHODS,
    associated_types: NO_ASSOCIATED_TYPES,
    associated_consts: NO_ASSOCIATED_CONSTS,
};
static STRING_IMPL: ImplApplicationMetadata = ImplApplicationMetadata {
    definition: &STRING_IMPL_DEF,
    target: target_type_metadata,
    implemented_trait: Some(&STRING_TRAIT),
    arguments: STRING_ARGUMENTS,
    identity: STRING_IDENTITY,
    methods: &OVERRIDE_EFFECTIVE_METHODS,
    associated_types: STRING_TYPES,
    associated_consts: STRING_CONSTS,
};
static U64_IMPL: ImplApplicationMetadata = ImplApplicationMetadata {
    definition: &U64_IMPL_DEF,
    target: target_type_metadata,
    implemented_trait: Some(&U64_TRAIT),
    arguments: U64_ARGUMENTS,
    identity: U64_IDENTITY,
    methods: &DEFAULT_EFFECTIVE_METHODS,
    associated_types: U64_TYPES,
    associated_consts: U64_CONSTS,
};
fn make_context() -> ReflectionContext {
    crate::runtime::context::handle::builder::ReflectionContextBuilder::new()
        .register(
            crate::runtime::context::handle::registration::Registration::of::<CustomerId>()
                .with_trait_definition(&TRAIT_DEFINITION)
                .with_trait_application(&STRING_TRAIT)
                .with_trait_application(&U64_TRAIT)
                .with_impl_definition(&STRING_IMPL_DEF)
                .with_impl_definition(&U64_IMPL_DEF)
                .with_impl_application(&STRING_IMPL)
                .with_impl_application(&U64_IMPL),
        )
        .build()
        .expect("fixture context builds")
}

#[test]
fn trait_applications_keep_distinct_generic_arguments() {
    let context = make_context();
    let definition = TraitDefinitionDescriptor::new(&TRAIT_DEFINITION, &context);
    let applications = definition.applications().expect("trait applications");
    assert_eq!(applications.len(), 2);
    let first = applications.get(0).expect("first application");
    let second = applications.get(1).expect("second application");
    assert_ne!(
        first
            .arguments()
            .argument("T")
            .expect("T")
            .type_argument()
            .expect("type")
            .concrete_type()
            .expect("String")
            .type_id(),
        second
            .arguments()
            .argument("T")
            .expect("T")
            .type_argument()
            .expect("type")
            .concrete_type()
            .expect("u64")
            .type_id()
    );
    assert!(!first.same_application(second));
    assert!(first.same_application(first));
    let other_context = make_context();
    assert!(!first.same_application(
        crate::runtime::descriptor::trait_descriptor::TraitDescriptor::new(
            &STRING_TRAIT,
            &other_context,
        ),
    ));
}

#[test]
fn impl_lookup_is_exact_by_application_and_ambiguous_by_definition() {
    let context = make_context();
    let applications = Impls::new(
        vec![
            crate::runtime::descriptor::impl_descriptor::ImplDescriptor::new(
                &STRING_IMPL,
                &context,
            ),
            crate::runtime::descriptor::impl_descriptor::ImplDescriptor::new(&U64_IMPL, &context),
        ]
        .into_boxed_slice(),
        &context,
    );
    let string_trait =
        crate::runtime::descriptor::trait_descriptor::TraitDescriptor::new(&STRING_TRAIT, &context);
    let u64_trait =
        crate::runtime::descriptor::trait_descriptor::TraitDescriptor::new(&U64_TRAIT, &context);
    assert!(
        applications
            .find_trait_application(string_trait)
            .expect("valid context")
            .expect("String impl")
            .same_application(
                crate::runtime::descriptor::impl_descriptor::ImplDescriptor::new(
                    &STRING_IMPL,
                    &context
                )
            )
    );
    assert!(
        applications
            .find_trait_application(u64_trait)
            .expect("valid context")
            .is_some()
    );
    assert_eq!(
        applications
            .get(0)
            .expect("impl")
            .implemented_trait()
            .expect("trait application")
            .context()
            .id(),
        context.id()
    );
    assert_eq!(
        applications.get(0).expect("impl").definition().kind(),
        ImplKind::Trait
    );
    assert_eq!(
        crate::runtime::descriptor::impl_descriptor::ImplDefinitionDescriptor::new(
            &STRING_IMPL_DEF,
            &context
        )
        .instances()
        .expect("definition instances")
        .len(),
        1
    );
    assert_eq!(
        crate::runtime::descriptor::impl_descriptor::ImplDefinitionDescriptor::new(
            &U64_IMPL_DEF,
            &context
        )
        .implemented_trait()
        .expect("source trait expression")
        .path(),
        "fixture::Convert"
    );
    assert!(matches!(
        applications
            .lookup_trait_definition(TraitDefinitionDescriptor::new(
                &TRAIT_DEFINITION,
                &context
            ))
            .expect("same context"),
        Lookup::Ambiguous(values) if values.len() == 2
    ));
    let other_context = make_context();
    assert!(
        applications
            .lookup_trait_definition(TraitDefinitionDescriptor::new(
                &TRAIT_DEFINITION,
                &other_context
            ))
            .is_err()
    );
    assert!(
        applications
            .find_trait_application(
                crate::runtime::descriptor::trait_descriptor::TraitDescriptor::new(
                    &STRING_TRAIT,
                    &other_context
                )
            )
            .is_err()
    );
    assert!(matches!(
        applications.get(0).expect("impl").method("missing"),
        Err(crate::runtime::error::LookupError::Missing {
            subject: crate::runtime::error::LookupSubject::Method,
            ..
        })
    ));
}

#[test]
fn effective_methods_and_associated_item_sources_preserve_overrides() {
    let context = make_context();
    let overridden =
        crate::runtime::descriptor::impl_descriptor::ImplDescriptor::new(&STRING_IMPL, &context);
    let defaulted =
        crate::runtime::descriptor::impl_descriptor::ImplDescriptor::new(&U64_IMPL, &context);
    assert_eq!(
        overridden
            .method("normalize")
            .expect("override")
            .implementation_source(),
        MethodImplementationSource::Overridden
    );
    assert_eq!(
        defaulted
            .method("normalize")
            .expect("default")
            .implementation_source(),
        MethodImplementationSource::Defaulted
    );
    assert_eq!(
        overridden
            .associated_type("Error")
            .expect("associated type")
            .source(),
        AssociatedItemSource::Declared
    );
    assert_eq!(
        defaulted
            .associated_type("Error")
            .expect("associated type default")
            .source(),
        AssociatedItemSource::Defaulted
    );
    assert_eq!(
        overridden
            .associated_const("COUNT")
            .expect("associated const")
            .source(),
        AssociatedItemSource::Overridden
    );
    assert_eq!(
        defaulted
            .associated_const("COUNT")
            .expect("associated const default")
            .source(),
        AssociatedItemSource::Defaulted
    );
    assert!(defaulted.definition().associated_type("Error").is_err());
    assert!(defaulted.definition().associated_const("COUNT").is_err());
    assert_eq!(
        overridden
            .associated_type("Error")
            .expect("concrete type")
            .find_concrete_type()
            .expect("type descriptor")
            .type_id(),
        TypeId::of::<ErrorValue>()
    );
    let value = overridden
        .associated_const("COUNT")
        .expect("value binding")
        .value::<Local>()
        .expect("local value")
        .expect("adapter value");
    assert_eq!(
        value.downcast_ref::<Answer>().expect("dynamic answer").0,
        42
    );
    let thread_safe = overridden
        .associated_const("COUNT")
        .expect("value binding")
        .value::<ThreadSafe>()
        .expect("thread-safe value")
        .expect("adapter value");
    assert_eq!(
        thread_safe
            .downcast_ref::<Answer>()
            .expect("dynamic answer")
            .0,
        42
    );
}

#[test]
fn definition_descriptors_retain_visibility_and_missing_queries() {
    let context = make_context();
    let definition = TraitDefinitionDescriptor::new(&TRAIT_DEFINITION, &context);
    assert_eq!(definition.visibility(), Visibility::Public);
    assert_eq!(
        definition.method("normalize").expect("method").name(),
        "normalize"
    );
    assert!(
        definition
            .method("normalize")
            .expect("method")
            .has_default_body()
    );
    assert!(definition.method("missing").is_err());
    assert!(definition.associated_type("missing").is_err());
    assert!(definition.associated_const("missing").is_err());
    assert_eq!(
        definition
            .associated_type("Error")
            .expect("type definition")
            .name(),
        "Error"
    );
    assert_eq!(
        definition
            .associated_const("COUNT")
            .expect("const definition")
            .default(),
        Some(ConstExpression::UnsignedInteger(1))
    );
    assert!(definition.supertraits().iter().next().is_none());
    assert_eq!(definition.completeness(), TraitCompleteness::Complete);
    assert_eq!(
        definition
            .applications()
            .expect("applications")
            .context()
            .id(),
        context.id()
    );
    assert_eq!(
        TypeDescriptor::new(&TARGET_METADATA, &context).type_id(),
        TypeId::of::<CustomerId>()
    );
}
