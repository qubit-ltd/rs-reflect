use qubit_reflect::__private::codegen_v4::{access, construct, expression};

use access::{
    FieldAccess, FieldMetadata, TypeKindMetadata, TypedFieldAdapter, TypedVariantAdapter,
    VariantDiscriminant, VariantMetadata, VariantShape, Visibility,
};
use construct::{
    ConstructField, ConstructProvider, ConstructionAvailability, ConstructionFieldPolicy,
    ConstructionPlan, ConstructionProviders, ConstructionShape, UpdateFieldPolicy,
};
use expression::{
    ConstExpression, ExpressionData, GenericArgumentMetadata, GenericDefinitionMetadata,
    GenericParameterMetadata, LifetimeExpression, PredicateData, StructKind, TypeDefinitionKind,
    TypeDefinitionMetadata,
};
use qubit_reflect::__private::codegen_v4::descriptor::{Reflect, TypeRegistration};
use qubit_reflect::__private::codegen_v4::value::{DynamicOwned, Local, ThreadSafe};

struct Record {
    value: u8,
}

enum Choice {
    Number(u8),
    Empty,
}

impl Reflect for Record {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        static REGISTRATION: TypeRegistration<Record> = TypeRegistration::new();
        &REGISTRATION
    }
}

impl Reflect for Choice {
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::<Choice>::get_or_init_with(|| {
            TypeRegistration::<Choice>::with_shape(&CHOICE_SHAPE, None, &CHOICE_PLANS)
        })
    }
}

static CHOICE_VALUE_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "u8",
    definition: None,
    arguments: &[],
};

fn number_ref(choice: &Choice) -> &u8 {
    match choice {
        Choice::Number(value) => value,
        Choice::Empty => panic!("Number field is inactive"),
    }
}

fn number_mut(choice: &mut Choice) -> &mut u8 {
    match choice {
        Choice::Number(value) => value,
        Choice::Empty => panic!("Number field is inactive"),
    }
}

fn active_choice_variant(choice: &Choice) -> &'static VariantMetadata {
    let index = match choice {
        Choice::Number(_) => 0,
        Choice::Empty => 1,
    };
    <Choice as Reflect>::__reflect_registration()
        .variant(index)
        .expect("each Choice variant has static metadata")
}

fn number_variant() -> &'static VariantMetadata {
    <Choice as Reflect>::__reflect_registration()
        .variant(0)
        .expect("Number variant metadata is registered")
}

static CHOICE_FIELD_ADAPTER: TypedFieldAdapter<Choice, u8> =
    TypedFieldAdapter::for_variant(number_ref, number_mut, active_choice_variant);
static CHOICE_FIELDS: [FieldMetadata; 1] = [FieldMetadata::typed_variant::<Choice, u8>(
    0,
    "0",
    Visibility::Public,
    &CHOICE_VALUE_EXPRESSION,
    &CHOICE_FIELD_ADAPTER,
    number_variant,
    FieldAccess::new(true, true, true, true),
)];
static CHOICE_VARIANTS: [VariantMetadata; 2] = [
    VariantMetadata::new(
        0,
        "Number",
        VariantShape::Tuple,
        Visibility::Public,
        VariantDiscriminant::new(access::DiscriminantOrigin::Implicit, None),
        &CHOICE_FIELDS,
        &CHOICE_NUMBER_VARIANT_ADAPTER,
    ),
    VariantMetadata::new(
        1,
        "Empty",
        VariantShape::Unit,
        Visibility::Public,
        VariantDiscriminant::new(access::DiscriminantOrigin::Implicit, None),
        &[],
        &CHOICE_EMPTY_VARIANT_ADAPTER,
    ),
];
static CHOICE_NUMBER_VARIANT_ADAPTER: TypedVariantAdapter<Choice> =
    TypedVariantAdapter::new(|choice| matches!(choice, Choice::Number(_)));
static CHOICE_EMPTY_VARIANT_ADAPTER: TypedVariantAdapter<Choice> =
    TypedVariantAdapter::new(|choice| matches!(choice, Choice::Empty));
static CHOICE_SHAPE: TypeKindMetadata = TypeKindMetadata::Enum(&CHOICE_VARIANTS);
static CHOICE_CONSTRUCT_FIELDS: [ConstructField; 1] = [ConstructField::typed::<u8>(
    "0",
    Visibility::Public,
    ConstructionFieldPolicy::Required,
    UpdateFieldPolicy::Replaceable,
)];

fn construct_choice_number(fields: Vec<Option<DynamicOwned<Local>>>) -> Choice {
    let [Some(value)] = fields.as_slice() else {
        panic!("the validated construction input has one field");
    };
    Choice::Number(
        *value
            .downcast_ref::<u8>()
            .expect("field type was validated"),
    )
}

fn update_choice_number(_base: Choice, replacements: Vec<Option<DynamicOwned<Local>>>) -> Choice {
    construct_choice_number(replacements)
}

static CHOICE_CONSTRUCT_PROVIDER: ConstructProvider<Choice, Local> =
    ConstructProvider::new(construct_choice_number, update_choice_number);
static CHOICE_CONSTRUCTION_PROVIDERS: ConstructionProviders =
    ConstructionProviders::new::<Choice>(Some(&CHOICE_CONSTRUCT_PROVIDER));
static CHOICE_PLANS: [ConstructionPlan; 1] = [ConstructionPlan::for_variant::<Choice>(
    0,
    ConstructionShape::Positional,
    &CHOICE_CONSTRUCT_FIELDS,
    CHOICE_CONSTRUCTION_PROVIDERS,
    ConstructionAvailability::Available,
)];

fn value_ref(record: &Record) -> &u8 {
    &record.value
}

fn value_mut(record: &mut Record) -> &mut u8 {
    &mut record.value
}

fn construct_record(fields: Vec<Option<DynamicOwned<Local>>>) -> Record {
    let [Some(value)] = fields.as_slice() else {
        panic!("the validated construction input has one field");
    };
    Record {
        value: *value
            .downcast_ref::<u8>()
            .expect("the runtime validates construction field types"),
    }
}

fn update_record(mut record: Record, replacements: Vec<Option<DynamicOwned<Local>>>) -> Record {
    let [Some(value)] = replacements.as_slice() else {
        return record;
    };
    record.value = *value
        .downcast_ref::<u8>()
        .expect("the runtime validates replacement field types");
    record
}

fn construct_record_thread_safe(fields: Vec<Option<DynamicOwned<ThreadSafe>>>) -> Record {
    let [Some(value)] = fields.as_slice() else {
        panic!("the validated construction input has one field");
    };
    Record {
        value: *value
            .downcast_ref::<u8>()
            .expect("the runtime validates construction field types"),
    }
}

fn update_record_thread_safe(
    mut record: Record,
    replacements: Vec<Option<DynamicOwned<ThreadSafe>>>,
) -> Record {
    let [Some(value)] = replacements.as_slice() else {
        return record;
    };
    record.value = *value
        .downcast_ref::<u8>()
        .expect("the runtime validates replacement field types");
    record
}

#[test]
fn test_codegen_v4_exposes_typed_structural_metadata_constructors() {
    static VALUE_EXPRESSION: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    static FIELD_ADAPTER: TypedFieldAdapter<Record, u8> =
        TypedFieldAdapter::new(value_ref, value_mut);
    static FIELDS: [FieldMetadata; 1] = [FieldMetadata::typed::<Record, u8>(
        0,
        "value",
        Visibility::Public,
        &VALUE_EXPRESSION,
        &FIELD_ADAPTER,
        FieldAccess::new(true, true, true, true),
    )];
    static SHAPE: TypeKindMetadata = TypeKindMetadata::Struct(&FIELDS);

    let first = TypeRegistration::<Record>::get_or_init_with(|| {
        TypeRegistration::<Record>::with_shape(&SHAPE, None, &[])
    });
    let second = TypeRegistration::<Record>::get_or_init_with(|| {
        panic!("a concrete type registration is initialized once")
    });
    assert!(std::ptr::eq(first, second));
}

#[test]
fn test_codegen_v4_exposes_typed_generic_definition_metadata() {
    struct DefinitionMarker;
    static PARAMETER: GenericParameterMetadata =
        GenericParameterMetadata::lifetime("'a", 0, &[LifetimeExpression::Static]);
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(
        &[PARAMETER],
        &[PredicateData::ConstEvaluatable(ConstExpression::Boolean(
            true,
        ))],
    );
    static TYPE_PARAMETER: GenericParameterMetadata =
        GenericParameterMetadata::type_parameter("T", 0, &[], None);
    static TYPE_ARGUMENT: GenericArgumentMetadata = GenericArgumentMetadata::type_expression(
        &TYPE_PARAMETER,
        &ExpressionData::ParameterName("T"),
    );
    static CONST_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::const_parameter(
        "N",
        1,
        &ExpressionData::Named {
            path: "usize",
            definition: None,
            arguments: &[],
        },
        None,
    );
    static CONST_ARGUMENT: GenericArgumentMetadata = GenericArgumentMetadata::const_argument(
        &CONST_PARAMETER,
        ConstExpression::UnsignedInteger(4),
    );
    static DEFINITION: TypeDefinitionMetadata = TypeDefinitionMetadata::new::<DefinitionMarker>(
        "Record",
        "example::Record",
        TypeDefinitionKind::Struct(StructKind::Named),
        &GENERICS,
    );

    let _ = (DEFINITION, TYPE_ARGUMENT, CONST_ARGUMENT);
}

#[test]
fn test_codegen_v4_exposes_typed_construction_provider_and_plan() {
    static PROVIDER: ConstructProvider<Record, Local> =
        ConstructProvider::new(construct_record, update_record);
    static PROVIDERS: ConstructionProviders = ConstructionProviders::new::<Record>(Some(&PROVIDER));
    static FIELDS: [ConstructField; 1] = [ConstructField::typed::<u8>(
        "value",
        Visibility::Public,
        ConstructionFieldPolicy::Required,
        UpdateFieldPolicy::Replaceable,
    )];
    static PLAN: ConstructionPlan = ConstructionPlan::for_type::<Record>(
        ConstructionShape::Named,
        &FIELDS,
        PROVIDERS,
        ConstructionAvailability::Available,
    );

    let _ = PLAN;
}

#[test]
fn test_codegen_v4_exposes_thread_safe_construction_provider() {
    static PROVIDER: ConstructProvider<Record, ThreadSafe> =
        ConstructProvider::new(construct_record_thread_safe, update_record_thread_safe);
    static PROVIDERS: ConstructionProviders =
        ConstructionProviders::new::<Record>(None).with_thread_safe::<Record>(&PROVIDER);
    static FIELDS: [ConstructField; 1] = [ConstructField::typed::<u8>(
        "value",
        Visibility::Public,
        ConstructionFieldPolicy::Required,
        UpdateFieldPolicy::Replaceable,
    )];
    static PLAN: ConstructionPlan = ConstructionPlan::for_type::<Record>(
        ConstructionShape::Named,
        &FIELDS,
        PROVIDERS,
        ConstructionAvailability::Available,
    );

    let _ = PLAN;
}

#[test]
fn test_codegen_v4_exposes_typed_variant_activity_adapter() {
    fn number_active(choice: &Choice) -> bool {
        matches!(choice, Choice::Number(_))
    }

    static ADAPTER: TypedVariantAdapter<Choice> = TypedVariantAdapter::new(number_active);
    static VARIANT: VariantMetadata = VariantMetadata::new(
        0,
        "Number",
        VariantShape::Tuple,
        Visibility::Public,
        VariantDiscriminant::new(access::DiscriminantOrigin::Implicit, None),
        &[],
        &ADAPTER,
    );
    let _ = VARIANT;
}
