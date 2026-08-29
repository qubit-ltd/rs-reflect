// qubit-style: allow explicit-imports
//! Integration tests for the reflected construction runtime.
mod construction_runtime {
    use std::any::TypeId;
    use std::cell::Cell;
    use std::rc::Rc;

    use qubit_reflect as reflect;
    use reflect::__private::descriptor;
    use reflect::access::VariantActiveAdapter;
    use reflect::construct::ConstructionError;
    use reflect::construct::ConstructionField;
    use reflect::construct::ConstructionRecovery;
    use reflect::construct::ConstructionShape;
    use reflect::construct::ConstructionUnavailableReason;
    use reflect::construct::NamedConstructionInput;
    use reflect::construct::RecoveredConstructionValue;
    use reflect::construct::StructConstructor;
    use reflect::construct::StructUpdateInput;
    use reflect::construct::StructUpdater;
    use reflect::construct::TupleConstructionInput;
    use reflect::construct::UpdateField;
    use reflect::construct::ValidatedConstructionInput;
    use reflect::construct::ValidatedUpdateInput;
    use reflect::construct::VariantConstructor;
    use reflect::descriptor::FieldDescriptor;
    use reflect::descriptor::OpaqueTypeDescriptor;
    use reflect::descriptor::StructKind;
    use reflect::descriptor::TypeDescriptor;
    use reflect::descriptor::TypeRef;
    use reflect::descriptor::VariantDescriptor;
    use reflect::descriptor::VariantKind;
    use reflect::identity::Visibility;
    use reflect::value::DynamicOwned;
    use reflect::value::Local;
    use reflect::value::Mode;
    use reflect::value::ReflectedOwned;
    use reflect::value::ReflectedRef;
    use reflect::value::ThreadSafe;

    #[derive(Debug, Eq, PartialEq)]
    struct Profile {
        id: u32,
        label: String,
        optional_note: Option<String>,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Defaulted {
        id: u32,
        label: String,
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Pair(u32, String);

    #[derive(Debug, Eq, PartialEq)]
    struct Triple(u32, String, Option<String>);

    #[derive(Debug, Eq, PartialEq)]
    struct Marker;

    #[derive(Debug, Eq, PartialEq)]
    enum Event {
        Started,
        Failed { code: u32, message: String },
    }

    /// Returns the root descriptor for the named profile fixture.
    fn profile_descriptor() -> &'static TypeDescriptor {
        &PROFILE_DESCRIPTOR
    }

    /// Returns the root descriptor for the explicit-default fixture.
    fn defaulted_descriptor() -> &'static TypeDescriptor {
        &DEFAULTED_DESCRIPTOR
    }

    /// Returns the root descriptor for the tuple-struct fixture.
    fn pair_descriptor() -> &'static TypeDescriptor {
        &PAIR_DESCRIPTOR
    }

    /// Returns the root descriptor for the three-field tuple fixture.
    fn triple_descriptor() -> &'static TypeDescriptor {
        &TRIPLE_DESCRIPTOR
    }

    /// Returns the root descriptor for the enum fixture.
    fn event_descriptor() -> &'static TypeDescriptor {
        &EVENT_DESCRIPTOR
    }

    /// Tests whether the unit variant is active after descriptor validation.
    fn started_is_active(value: ReflectedRef<'_>) -> Result<bool, reflect::error::TypeMismatch> {
        let event = value
            .downcast_ref::<Event>()
            .unwrap_or_else(|| unreachable!("the descriptor validates the enum root type"));
        Ok(matches!(event, Event::Started))
    }

    /// Tests whether the named variant is active after descriptor validation.
    fn failed_is_active(value: ReflectedRef<'_>) -> Result<bool, reflect::error::TypeMismatch> {
        let event = value
            .downcast_ref::<Event>()
            .unwrap_or_else(|| unreachable!("the descriptor validates the enum root type"));
        Ok(matches!(event, Event::Failed { .. }))
    }

    static U32_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<u32>();
    static STRING_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<String>();
    static OPTION_STRING_TYPE: OpaqueTypeDescriptor = descriptor::opaque_member::<Option<String>>();
    static U32_REF: TypeRef = TypeRef::Opaque(&U32_TYPE);
    static STRING_REF: TypeRef = TypeRef::Opaque(&STRING_TYPE);
    static OPTION_STRING_REF: TypeRef = TypeRef::Opaque(&OPTION_STRING_TYPE);

    static PROFILE_FIELDS: [FieldDescriptor; 3] = [
        descriptor::field(
            profile_descriptor,
            0,
            Some("id"),
            Some("id"),
            &U32_REF,
            Visibility::Private,
        ),
        descriptor::field(
            profile_descriptor,
            1,
            Some("label"),
            Some("label"),
            &STRING_REF,
            Visibility::Private,
        ),
        descriptor::field(
            profile_descriptor,
            2,
            Some("optional_note"),
            Some("optional_note"),
            &OPTION_STRING_REF,
            Visibility::Private,
        ),
    ];
    static PROFILE_DESCRIPTOR: TypeDescriptor =
        descriptor::struct_type::<Profile>("construct::Profile", StructKind::Named, &PROFILE_FIELDS);
    static PROFILE_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 3] = [
        ConstructionField::required(&PROFILE_FIELDS[0]),
        ConstructionField::required(&PROFILE_FIELDS[1]),
        ConstructionField::required(&PROFILE_FIELDS[2]),
    ];
    static PROFILE_UPDATE_FIELDS: [UpdateField; 3] = [
        UpdateField::allowed(&PROFILE_FIELDS[0]),
        UpdateField::allowed(&PROFILE_FIELDS[1]),
        UpdateField::allowed(&PROFILE_FIELDS[2]),
    ];

    static DEFAULTED_FIELDS: [FieldDescriptor; 2] = [
        descriptor::field(
            defaulted_descriptor,
            0,
            Some("id"),
            Some("id"),
            &U32_REF,
            Visibility::Private,
        ),
        descriptor::field(
            defaulted_descriptor,
            1,
            Some("label"),
            Some("label"),
            &STRING_REF,
            Visibility::Private,
        ),
    ];
    static DEFAULTED_DESCRIPTOR: TypeDescriptor =
        descriptor::struct_type::<Defaulted>("construct::Defaulted", StructKind::Named, &DEFAULTED_FIELDS);

    /// Supplies the explicit per-field default used by reflected construction.
    fn default_label() -> DynamicOwned<Local> {
        DynamicOwned::<Local>::new(String::from("explicit default"))
    }

    static DEFAULTED_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 2] = [
        ConstructionField::required(&DEFAULTED_FIELDS[0]),
        ConstructionField::defaulted(&DEFAULTED_FIELDS[1], default_label),
    ];
    static PROVIDER_ONLY_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 2] = [
        ConstructionField::required(&DEFAULTED_FIELDS[0]),
        ConstructionField::provider_only(&DEFAULTED_FIELDS[1], default_label),
    ];
    static UNAVAILABLE_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 2] = [
        ConstructionField::required(&DEFAULTED_FIELDS[0]),
        ConstructionField::unavailable(
            &DEFAULTED_FIELDS[1],
            ConstructionUnavailableReason::MissingDefaultProvider,
        ),
    ];
    static DEFAULTED_UPDATE_FIELDS: [UpdateField; 2] = [
        UpdateField::allowed(&DEFAULTED_FIELDS[0]),
        UpdateField::allowed(&DEFAULTED_FIELDS[1]),
    ];
    static RESTRICTED_DEFAULTED_UPDATE_FIELDS: [UpdateField; 2] = [
        UpdateField::allowed(&DEFAULTED_FIELDS[0]),
        UpdateField::unavailable(&DEFAULTED_FIELDS[1], ConstructionUnavailableReason::UpdateForbidden),
    ];

    /// Panics if an invalid field set evaluates a provider before structural
    /// validation.
    fn provider_must_not_run() -> DynamicOwned<Local> {
        panic!("providers must not run before every missing field is validated")
    }

    static DEFAULT_BEFORE_REQUIRED_NAMED: [ConstructionField<Local>; 2] = [
        ConstructionField::defaulted(&DEFAULTED_FIELDS[0], provider_must_not_run),
        ConstructionField::required(&DEFAULTED_FIELDS[1]),
    ];

    static PAIR_FIELDS: [FieldDescriptor; 2] = [
        descriptor::field(pair_descriptor, 0, None, None, &U32_REF, Visibility::Private),
        descriptor::field(pair_descriptor, 1, None, None, &STRING_REF, Visibility::Private),
    ];
    static PAIR_DESCRIPTOR: TypeDescriptor =
        descriptor::struct_type::<Pair>("construct::Pair", StructKind::Tuple, &PAIR_FIELDS);
    static PAIR_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 2] = [
        ConstructionField::required(&PAIR_FIELDS[0]),
        ConstructionField::required(&PAIR_FIELDS[1]),
    ];
    static DEFAULT_BEFORE_REQUIRED_TUPLE: [ConstructionField<Local>; 2] = [
        ConstructionField::defaulted(&PAIR_FIELDS[0], provider_must_not_run),
        ConstructionField::required(&PAIR_FIELDS[1]),
    ];

    /// Supplies a provider-only leading tuple field.
    fn default_first_pair_value() -> DynamicOwned<Local> {
        DynamicOwned::<Local>::new(99_u32)
    }

    static LEADING_PROVIDER_ONLY_PAIR_FIELDS: [ConstructionField<Local>; 2] = [
        ConstructionField::provider_only(&PAIR_FIELDS[0], default_first_pair_value),
        ConstructionField::required(&PAIR_FIELDS[1]),
    ];

    static TRIPLE_FIELDS: [FieldDescriptor; 3] = [
        descriptor::field(triple_descriptor, 0, None, None, &U32_REF, Visibility::Private),
        descriptor::field(triple_descriptor, 1, None, None, &STRING_REF, Visibility::Private),
        descriptor::field(
            triple_descriptor,
            2,
            None,
            None,
            &OPTION_STRING_REF,
            Visibility::Private,
        ),
    ];
    static TRIPLE_DESCRIPTOR: TypeDescriptor =
        descriptor::struct_type::<Triple>("construct::Triple", StructKind::Tuple, &TRIPLE_FIELDS);
    static MIDDLE_PROVIDER_ONLY_TRIPLE_FIELDS: [ConstructionField<Local>; 3] = [
        ConstructionField::required(&TRIPLE_FIELDS[0]),
        ConstructionField::provider_only(&TRIPLE_FIELDS[1], default_label),
        ConstructionField::required(&TRIPLE_FIELDS[2]),
    ];

    static MARKER_DESCRIPTOR: TypeDescriptor =
        descriptor::struct_type::<Marker>("construct::Marker", StructKind::Unit, &[]);
    static MARKER_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 0] = [];

    static FAILED_FIELDS: [FieldDescriptor; 2] = [
        descriptor::field(
            event_descriptor,
            0,
            Some("code"),
            Some("code"),
            &U32_REF,
            Visibility::Private,
        )
        .with_variant(1, "Failed"),
        descriptor::field(
            event_descriptor,
            1,
            Some("message"),
            Some("message"),
            &STRING_REF,
            Visibility::Private,
        )
        .with_variant(1, "Failed"),
    ];
    static EVENT_VARIANTS: [VariantDescriptor; 2] = [
        descriptor::variant(
            event_descriptor,
            0,
            "Started",
            "Started",
            VariantKind::Unit,
            &[],
            started_is_active as VariantActiveAdapter,
        ),
        descriptor::variant(
            event_descriptor,
            1,
            "Failed",
            "Failed",
            VariantKind::Struct,
            &FAILED_FIELDS,
            failed_is_active as VariantActiveAdapter,
        ),
    ];
    static EVENT_DESCRIPTOR: TypeDescriptor = descriptor::enum_type::<Event>("construct::Event", &EVENT_VARIANTS);
    static FAILED_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 2] = [
        ConstructionField::required(&FAILED_FIELDS[0]),
        ConstructionField::required(&FAILED_FIELDS[1]),
    ];
    static STARTED_CONSTRUCTION_FIELDS: [ConstructionField<Local>; 0] = [];

    /// Builds a profile from descriptor-ordered, already validated values.
    fn construct_profile(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        let [id, label, optional_note] = input
            .into_values()
            .into_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("validation guarantees three fields"));
        DynamicOwned::<Local>::new(Profile {
            id: id
                .downcast::<u32>()
                .unwrap_or_else(|_| unreachable!("validation guarantees u32")),
            label: label
                .downcast::<String>()
                .unwrap_or_else(|_| unreachable!("validation guarantees String")),
            optional_note: optional_note
                .downcast::<Option<String>>()
                .unwrap_or_else(|_| unreachable!("validation guarantees Option<String>")),
        })
    }

    /// Panics if a failed profile validation incorrectly crosses the adapter
    /// boundary.
    fn unreachable_profile_adapter(_: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        panic!("a validation failure must not execute the construction adapter")
    }

    /// Builds the explicit-default fixture from validated values.
    fn construct_defaulted(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        let [id, label] = input
            .into_values()
            .into_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("validation guarantees two fields"));
        DynamicOwned::<Local>::new(Defaulted {
            id: id
                .downcast::<u32>()
                .unwrap_or_else(|_| unreachable!("validation guarantees u32")),
            label: label
                .downcast::<String>()
                .unwrap_or_else(|_| unreachable!("validation guarantees String")),
        })
    }

    /// Builds a tuple struct from validated positional values.
    fn construct_pair(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        let [first, second] = input
            .into_values()
            .into_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("validation guarantees two fields"));
        DynamicOwned::<Local>::new(Pair(
            first
                .downcast::<u32>()
                .unwrap_or_else(|_| unreachable!("validation guarantees u32")),
            second
                .downcast::<String>()
                .unwrap_or_else(|_| unreachable!("validation guarantees String")),
        ))
    }

    /// Builds a three-field tuple from descriptor-ordered validated values.
    fn construct_triple(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        let [first, second, third] = input
            .into_values()
            .into_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("validation guarantees three fields"));
        DynamicOwned::<Local>::new(Triple(
            first
                .downcast::<u32>()
                .unwrap_or_else(|_| unreachable!("validation guarantees u32")),
            second
                .downcast::<String>()
                .unwrap_or_else(|_| unreachable!("validation guarantees String")),
            third
                .downcast::<Option<String>>()
                .unwrap_or_else(|_| unreachable!("validation guarantees Option<String>")),
        ))
    }

    /// Builds a fieldless struct after unit-shape validation.
    fn construct_marker(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        assert!(input.values().is_empty());
        DynamicOwned::<Local>::new(Marker)
    }

    /// Builds the unit enum variant after unit-shape validation.
    fn construct_started(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        assert!(input.values().is_empty());
        DynamicOwned::<Local>::new(Event::Started)
    }

    /// Builds the named enum variant from descriptor-ordered values.
    fn construct_failed(input: ValidatedConstructionInput<Local>) -> DynamicOwned<Local> {
        let [code, message] = input
            .into_values()
            .into_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("validation guarantees two fields"));
        DynamicOwned::<Local>::new(Event::Failed {
            code: code
                .downcast::<u32>()
                .unwrap_or_else(|_| unreachable!("validation guarantees u32")),
            message: message
                .downcast::<String>()
                .unwrap_or_else(|_| unreachable!("validation guarantees String")),
        })
    }

    /// Applies validated named overrides to an owned profile base value.
    fn update_profile(input: ValidatedUpdateInput<Local>) -> DynamicOwned<Local> {
        let (base, overrides) = input.into_parts();
        let mut profile = base
            .downcast::<Profile>()
            .unwrap_or_else(|_| unreachable!("validation guarantees the base type"));
        for value in overrides.into_vec() {
            let (index, value) = value.into_parts();
            match index {
                0 => {
                    profile.id = value
                        .downcast::<u32>()
                        .unwrap_or_else(|_| unreachable!("validation guarantees u32"));
                }
                1 => {
                    profile.label = value
                        .downcast::<String>()
                        .unwrap_or_else(|_| unreachable!("validation guarantees String"));
                }
                2 => {
                    profile.optional_note = value
                        .downcast::<Option<String>>()
                        .unwrap_or_else(|_| unreachable!("validation guarantees Option<String>"));
                }
                _ => unreachable!("validation only accepts declared fields"),
            }
        }
        DynamicOwned::<Local>::new(profile)
    }

    /// Applies validated named overrides to the explicit-default fixture.
    fn update_defaulted(input: ValidatedUpdateInput<Local>) -> DynamicOwned<Local> {
        let (base, overrides) = input.into_parts();
        let mut value = base
            .downcast::<Defaulted>()
            .unwrap_or_else(|_| unreachable!("validation guarantees the base type"));
        for field in overrides.into_vec() {
            let (index, replacement) = field.into_parts();
            match index {
                0 => {
                    value.id = replacement
                        .downcast::<u32>()
                        .unwrap_or_else(|_| unreachable!("validation guarantees u32"));
                }
                1 => {
                    value.label = replacement
                        .downcast::<String>()
                        .unwrap_or_else(|_| unreachable!("validation guarantees String"));
                }
                _ => unreachable!("validation only accepts declared fields"),
            }
        }
        DynamicOwned::<Local>::new(value)
    }

    /// Panics if a failed update validation incorrectly crosses the adapter
    /// boundary.
    fn unreachable_update_adapter(_: ValidatedUpdateInput<Local>) -> DynamicOwned<Local> {
        panic!("a validation failure must not execute the update adapter")
    }

    /// Downcasts a successful local result and returns the concrete value.
    fn downcast_result<T: 'static>(value: DynamicOwned<Local>) -> T {
        value
            .downcast::<T>()
            .unwrap_or_else(|_| panic!("construction should return the declared root type"))
    }

    /// Extracts a required construction validation failure without requiring a
    /// dynamic successful value to implement `Debug`.
    fn expect_construction_failure<M: Mode>(
        result: Result<DynamicOwned<M>, ConstructionRecovery<M>>,
        message: &str,
    ) -> ConstructionRecovery<M> {
        match result {
            Ok(_) => panic!("{message}"),
            Err(failure) => failure,
        }
    }

    #[test]
    fn test_named_construction_validates_then_orders_values_by_descriptor_index() {
        let constructor = StructConstructor::new(&PROFILE_DESCRIPTOR, &PROFILE_CONSTRUCTION_FIELDS, construct_profile);
        let input = NamedConstructionInput::new([
            ("optional_note", ReflectedOwned::new(Some(String::from("note")))),
            ("label", ReflectedOwned::new(String::from("alpha"))),
            ("id", ReflectedOwned::new(7_u32)),
        ]);

        let profile = downcast_result::<Profile>(
            constructor
                .construct_named(input)
                .expect("complete exact input should construct"),
        );

        assert_eq!(
            profile,
            Profile {
                id: 7,
                label: String::from("alpha"),
                optional_note: Some(String::from("note")),
            }
        );
    }

    #[test]
    fn test_option_field_is_not_automatically_defaulted() {
        let constructor = StructConstructor::new(
            &PROFILE_DESCRIPTOR,
            &PROFILE_CONSTRUCTION_FIELDS,
            unreachable_profile_adapter,
        );
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([
                ("id", ReflectedOwned::new(7_u32)),
                ("label", ReflectedOwned::new(String::from("alpha"))),
            ])),
            "an Option field is required without an explicit provider",
        );

        let ConstructionError::MissingField { field } = failure.error() else {
            panic!("the failure should identify the missing Option field")
        };
        assert_eq!(field.query_name(), Some("optional_note"));
        assert_eq!(failure.values().len(), 2);
    }

    #[test]
    fn test_explicit_field_provider_supplies_a_missing_value() {
        let constructor = StructConstructor::new(
            &DEFAULTED_DESCRIPTOR,
            &DEFAULTED_CONSTRUCTION_FIELDS,
            construct_defaulted,
        );

        let value = constructor
            .construct_named(NamedConstructionInput::new([("id", ReflectedOwned::new(9_u32))]))
            .expect("an explicit provider may fill its field");

        assert_eq!(
            downcast_result::<Defaulted>(value),
            Defaulted {
                id: 9,
                label: String::from("explicit default"),
            }
        );

        let supplied = constructor
            .construct_named(NamedConstructionInput::new([
                ("id", ReflectedOwned::new(10_u32)),
                ("label", ReflectedOwned::new(String::from("caller value"))),
            ]))
            .expect("an ordinary default field may still be supplied by the caller");
        assert_eq!(downcast_result::<Defaulted>(supplied).label, "caller value");
    }

    #[test]
    fn test_provider_only_field_uses_provider_and_rejects_caller_binding() {
        let constructor = StructConstructor::new(
            &DEFAULTED_DESCRIPTOR,
            &PROVIDER_ONLY_CONSTRUCTION_FIELDS,
            construct_defaulted,
        );
        let provided = constructor
            .construct_named(NamedConstructionInput::new([("id", ReflectedOwned::new(12_u32))]))
            .expect("a provider-only field should be filled by its provider");
        assert_eq!(
            downcast_result::<Defaulted>(provided),
            Defaulted {
                id: 12,
                label: String::from("explicit default"),
            }
        );

        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([
                ("id", ReflectedOwned::new(13_u32)),
                ("label", ReflectedOwned::new(String::from("forbidden"))),
            ])),
            "provider-only fields must reject direct caller values",
        );
        assert!(matches!(
            failure.error(),
            ConstructionError::Unavailable {
                field,
                reason: ConstructionUnavailableReason::CallerValueForbidden,
            } if field.query_name() == Some("label")
        ));
        assert_eq!(failure.values().len(), 2);
    }

    #[test]
    fn test_update_policy_is_independent_from_from_zero_construction_policy() {
        let constructor = StructConstructor::new(
            &DEFAULTED_DESCRIPTOR,
            &UNAVAILABLE_CONSTRUCTION_FIELDS,
            construct_defaulted,
        );
        let construction_failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([("id", ReflectedOwned::new(1_u32))])),
            "from-zero construction should remain unavailable",
        );
        assert!(matches!(
            construction_failure.error(),
            ConstructionError::Unavailable { .. }
        ));

        let updater = StructUpdater::new(&DEFAULTED_DESCRIPTOR, &DEFAULTED_UPDATE_FIELDS, update_defaulted);
        let updated = updater
            .update(StructUpdateInput::new(
                ReflectedOwned::new(Defaulted {
                    id: 1,
                    label: String::from("before"),
                }),
                NamedConstructionInput::new([("label", ReflectedOwned::new(String::from("after")))]),
            ))
            .expect("independent update policy may allow an override");
        assert_eq!(downcast_result::<Defaulted>(updated).label, "after");
    }

    #[test]
    fn test_unavailable_update_field_recovers_base_and_override() {
        let updater = StructUpdater::new(
            &DEFAULTED_DESCRIPTOR,
            &RESTRICTED_DEFAULTED_UPDATE_FIELDS,
            unreachable_update_adapter,
        );
        let failure = expect_construction_failure(
            updater.update(StructUpdateInput::new(
                ReflectedOwned::new(Defaulted {
                    id: 1,
                    label: String::from("before"),
                }),
                NamedConstructionInput::new([("label", ReflectedOwned::new(String::from("forbidden")))]),
            )),
            "an unavailable update field must fail before its adapter",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::Unavailable {
                field,
                reason: ConstructionUnavailableReason::UpdateForbidden,
            } if field.query_name() == Some("label")
        ));
        assert_eq!(failure.values().len(), 2);
    }

    #[test]
    fn test_named_input_validates_every_missing_field_before_running_a_provider() {
        let constructor = StructConstructor::new(
            &DEFAULTED_DESCRIPTOR,
            &DEFAULT_BEFORE_REQUIRED_NAMED,
            construct_defaulted,
        );

        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new(std::iter::empty::<(&str, ReflectedOwned)>())),
            "the later required field must fail before the earlier provider runs",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::MissingField { field }
                if field.query_name() == Some("label")
        ));
        assert!(failure.values().is_empty());
    }

    #[test]
    fn test_tuple_input_validates_every_missing_field_before_running_a_provider() {
        let constructor = StructConstructor::new(&PAIR_DESCRIPTOR, &DEFAULT_BEFORE_REQUIRED_TUPLE, construct_pair);

        let failure = expect_construction_failure(
            constructor.construct_tuple(TupleConstructionInput::new(std::iter::empty::<ReflectedOwned>())),
            "the later required position must fail before the earlier provider runs",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::MissingField { field } if field.index() == 1
        ));
        assert!(failure.values().is_empty());
    }

    #[test]
    fn test_missing_provider_marks_whole_constructor_unavailable() {
        let constructor = StructConstructor::new(
            &DEFAULTED_DESCRIPTOR,
            &UNAVAILABLE_CONSTRUCTION_FIELDS,
            construct_defaulted,
        );
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([("id", ReflectedOwned::new(9_u32))])),
            "a skipped or no-construct field needs an explicit provider",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::Unavailable {
                reason: ConstructionUnavailableReason::MissingDefaultProvider,
                ..
            }
        ));
        assert_eq!(failure.values().len(), 1);
    }

    #[test]
    fn test_duplicate_named_field_returns_every_owned_input() {
        let constructor = StructConstructor::new(
            &PROFILE_DESCRIPTOR,
            &PROFILE_CONSTRUCTION_FIELDS,
            unreachable_profile_adapter,
        );
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([
                ("id", ReflectedOwned::new(7_u32)),
                ("id", ReflectedOwned::new(8_u32)),
                ("label", ReflectedOwned::new(String::from("alpha"))),
                ("optional_note", ReflectedOwned::new(None::<String>)),
            ])),
            "a field may only appear once",
        );

        let (error, recovered) = failure.into_parts();
        assert!(matches!(
            error,
            ConstructionError::DuplicateField { name } if name.as_ref() == "id"
        ));
        let recovered = recovered.into_vec();
        assert_eq!(recovered.len(), 4);
        let RecoveredConstructionValue::Named { name, value } = &recovered[1] else {
            panic!("named recovery should preserve the original binding")
        };
        assert_eq!(name.as_ref(), "id");
        assert_eq!(value.downcast_ref::<u32>(), Some(&8));
    }

    #[test]
    fn test_unknown_named_field_is_rejected_before_adapter_execution() {
        let constructor = StructConstructor::new(
            &PROFILE_DESCRIPTOR,
            &PROFILE_CONSTRUCTION_FIELDS,
            unreachable_profile_adapter,
        );
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([("unknown", ReflectedOwned::new(7_u32))])),
            "unknown query names must be rejected",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::UnknownField { name } if name.as_ref() == "unknown"
        ));
        assert_eq!(failure.values().len(), 1);
    }

    #[test]
    fn test_wrong_field_type_uses_exact_type_id_and_recovers_all_values() {
        let constructor = StructConstructor::new(
            &PROFILE_DESCRIPTOR,
            &PROFILE_CONSTRUCTION_FIELDS,
            unreachable_profile_adapter,
        );
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([
                ("id", ReflectedOwned::new(7_i32)),
                ("label", ReflectedOwned::new(String::from("alpha"))),
                ("optional_note", ReflectedOwned::new(None::<String>)),
            ])),
            "numeric values are not coerced across exact Rust types",
        );

        let ConstructionError::ValueTypeMismatch { field, mismatch } = failure.error() else {
            panic!("the failure should identify the exact field type mismatch")
        };
        assert_eq!(field.query_name(), Some("id"));
        assert_eq!(mismatch.expected(), TypeId::of::<u32>());
        assert_eq!(mismatch.actual(), TypeId::of::<i32>());
        assert_eq!(failure.values().len(), 3);
    }

    #[test]
    fn test_wrong_shape_returns_named_values_untouched() {
        let constructor = StructConstructor::new(&PAIR_DESCRIPTOR, &PAIR_CONSTRUCTION_FIELDS, construct_pair);
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([("first", ReflectedOwned::new(1_u32))])),
            "a tuple constructor must reject named input",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::WrongShape {
                expected: ConstructionShape::Tuple,
                actual: ConstructionShape::Named,
            }
        ));
        assert_eq!(failure.values().len(), 1);
    }

    #[test]
    fn test_tuple_and_unit_struct_construction_use_matching_shapes() {
        let pair_constructor = StructConstructor::new(&PAIR_DESCRIPTOR, &PAIR_CONSTRUCTION_FIELDS, construct_pair);
        let marker_constructor =
            StructConstructor::new(&MARKER_DESCRIPTOR, &MARKER_CONSTRUCTION_FIELDS, construct_marker);

        let pair = pair_constructor
            .construct_tuple(TupleConstructionInput::new([
                ReflectedOwned::new(3_u32),
                ReflectedOwned::new(String::from("three")),
            ]))
            .expect("ordered exact tuple input should construct");
        let marker = marker_constructor
            .construct_unit()
            .expect("a unit constructor takes no values");

        assert_eq!(downcast_result::<Pair>(pair), Pair(3, String::from("three")));
        assert_eq!(downcast_result::<Marker>(marker), Marker);
    }

    #[test]
    fn test_tuple_input_skips_leading_provider_only_field() {
        let constructor = StructConstructor::new(&PAIR_DESCRIPTOR, &LEADING_PROVIDER_ONLY_PAIR_FIELDS, construct_pair);

        let pair = constructor
            .construct_tuple(TupleConstructionInput::new([ReflectedOwned::new(String::from(
                "caller field",
            ))]))
            .expect("caller positions should skip a leading provider-only field");

        assert_eq!(downcast_result::<Pair>(pair), Pair(99, String::from("caller field")));
    }

    #[test]
    fn test_tuple_input_skips_middle_provider_only_field_and_recovers_caller_order() {
        let constructor = StructConstructor::new(
            &TRIPLE_DESCRIPTOR,
            &MIDDLE_PROVIDER_ONLY_TRIPLE_FIELDS,
            construct_triple,
        );
        let triple = constructor
            .construct_tuple(TupleConstructionInput::new([
                ReflectedOwned::new(7_u32),
                ReflectedOwned::new(Some(String::from("tail"))),
            ]))
            .expect("caller positions should skip a middle provider-only field");
        assert_eq!(
            downcast_result::<Triple>(triple),
            Triple(7, String::from("explicit default"), Some(String::from("tail")),)
        );

        let failure = expect_construction_failure(
            constructor.construct_tuple(TupleConstructionInput::new([
                ReflectedOwned::new(8_u32),
                ReflectedOwned::new(9_i32),
            ])),
            "the second caller value should validate against the third descriptor field",
        );
        let ConstructionError::ValueTypeMismatch { field, mismatch } = failure.error() else {
            panic!("the failure should retain the mapped descriptor field")
        };
        assert_eq!(field.index(), 2);
        assert_eq!(mismatch.actual(), TypeId::of::<i32>());
        let recovered = failure.into_values().into_vec();
        assert_eq!(recovered.len(), 2);
        let RecoveredConstructionValue::Positional { index, value } = &recovered[1] else {
            panic!("tuple recovery must preserve the caller's positional order")
        };
        assert_eq!(*index, 1);
        assert_eq!(value.downcast_ref::<i32>(), Some(&9));
    }

    #[test]
    fn test_unit_and_named_variant_construction_return_enum_root_values() {
        let started_constructor =
            VariantConstructor::new(&EVENT_VARIANTS[0], &STARTED_CONSTRUCTION_FIELDS, construct_started);
        let failed_constructor =
            VariantConstructor::new(&EVENT_VARIANTS[1], &FAILED_CONSTRUCTION_FIELDS, construct_failed);

        let started = started_constructor
            .construct_unit()
            .expect("the unit variant should construct");
        let failed = failed_constructor
            .construct_named(NamedConstructionInput::new([
                ("message", ReflectedOwned::new(String::from("timeout"))),
                ("code", ReflectedOwned::new(500_u32)),
            ]))
            .expect("the named variant should construct");

        assert_eq!(downcast_result::<Event>(started), Event::Started);
        assert_eq!(
            downcast_result::<Event>(failed),
            Event::Failed {
                code: 500,
                message: String::from("timeout"),
            }
        );
    }

    #[test]
    fn test_update_validates_all_overrides_before_mutating_owned_base() {
        let updater = StructUpdater::new(&PROFILE_DESCRIPTOR, &PROFILE_UPDATE_FIELDS, update_profile);
        let base = Profile {
            id: 1,
            label: String::from("before"),
            optional_note: Some(String::from("kept")),
        };
        let input = StructUpdateInput::new(
            ReflectedOwned::new(base),
            NamedConstructionInput::new([
                ("label", ReflectedOwned::new(String::from("after"))),
                ("id", ReflectedOwned::new(2_u32)),
            ]),
        );

        let updated = updater.update(input).expect("valid overrides should update");

        assert_eq!(
            downcast_result::<Profile>(updated),
            Profile {
                id: 2,
                label: String::from("after"),
                optional_note: Some(String::from("kept")),
            }
        );
    }

    #[test]
    fn test_update_failure_recovers_base_and_every_override_in_original_order() {
        let updater = StructUpdater::new(&PROFILE_DESCRIPTOR, &PROFILE_UPDATE_FIELDS, unreachable_update_adapter);
        let input = StructUpdateInput::new(
            ReflectedOwned::new(Profile {
                id: 1,
                label: String::from("before"),
                optional_note: None,
            }),
            NamedConstructionInput::new([
                ("label", ReflectedOwned::new(String::from("after"))),
                ("id", ReflectedOwned::new(2_i32)),
            ]),
        );
        let failure = expect_construction_failure(
            updater.update(input),
            "all override types must validate before mutation",
        );

        assert!(matches!(
            failure.error(),
            ConstructionError::ValueTypeMismatch { field, mismatch }
                if field.query_name() == Some("id")
                    && mismatch.actual() == TypeId::of::<i32>()
        ));
        let recovered = failure.into_values().into_vec();
        assert_eq!(recovered.len(), 3);
        let RecoveredConstructionValue::Base(base) = &recovered[0] else {
            panic!("update recovery must return the base first")
        };
        assert_eq!(
            base.downcast_ref::<Profile>().map(|profile| profile.label.as_str()),
            Some("before")
        );
        let RecoveredConstructionValue::Named { name, .. } = &recovered[1] else {
            panic!("the first override should remain named")
        };
        assert_eq!(name.as_ref(), "label");
    }

    #[test]
    fn test_update_rejects_wrong_base_type_without_losing_overrides() {
        let updater = StructUpdater::new(&PROFILE_DESCRIPTOR, &PROFILE_UPDATE_FIELDS, unreachable_update_adapter);
        let failure = expect_construction_failure(
            updater.update(StructUpdateInput::new(
                ReflectedOwned::new(Defaulted {
                    id: 1,
                    label: String::from("wrong root"),
                }),
                NamedConstructionInput::new([("id", ReflectedOwned::new(2_u32))]),
            )),
            "update requires the exact reflected root type",
        );

        let ConstructionError::BaseTypeMismatch { mismatch } = failure.error() else {
            panic!("the failure should classify the base mismatch")
        };
        assert_eq!(mismatch.expected(), TypeId::of::<Profile>());
        assert_eq!(mismatch.actual(), TypeId::of::<Defaulted>());
        assert_eq!(failure.values().len(), 2);
    }

    #[derive(Debug)]
    struct DropProbe(Rc<Cell<usize>>);

    impl Drop for DropProbe {
        /// Records exactly one destructor call for recovery ownership tests.
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn test_validation_failure_neither_drops_nor_duplicates_owned_inputs() {
        let drops = Rc::new(Cell::new(0));
        let constructor = StructConstructor::new(&PROFILE_DESCRIPTOR, &PROFILE_CONSTRUCTION_FIELDS, construct_profile);
        let failure = expect_construction_failure(
            constructor.construct_named(NamedConstructionInput::new([
                ("unknown", ReflectedOwned::new(DropProbe(Rc::clone(&drops)))),
                ("also_unknown", ReflectedOwned::new(DropProbe(Rc::clone(&drops)))),
            ])),
            "unknown fields must fail before consuming values",
        );

        assert_eq!(drops.get(), 0);
        assert_eq!(failure.values().len(), 2);
        drop(failure);
        assert_eq!(drops.get(), 2);
    }

    /// Constructs a thread-safe scalar root from one validated positional
    /// value.
    fn construct_thread_scalar(input: ValidatedConstructionInput<ThreadSafe>) -> DynamicOwned<ThreadSafe> {
        let [value] = input
            .into_values()
            .into_vec()
            .try_into()
            .unwrap_or_else(|_| unreachable!("validation guarantees one value"));
        let value = value
            .downcast::<u32>()
            .unwrap_or_else(|_| unreachable!("validation guarantees u32"));
        DynamicOwned::<ThreadSafe>::new(ThreadScalar(value))
    }

    #[derive(Debug, Eq, PartialEq)]
    struct ThreadScalar(u32);

    /// Returns the descriptor for the thread-safe tuple-struct fixture.
    fn thread_scalar_descriptor() -> &'static TypeDescriptor {
        &THREAD_SCALAR_DESCRIPTOR
    }

    static THREAD_SCALAR_FIELDS: [FieldDescriptor; 1] = [descriptor::field(
        thread_scalar_descriptor,
        0,
        None,
        None,
        &U32_REF,
        Visibility::Private,
    )];
    static THREAD_SCALAR_DESCRIPTOR: TypeDescriptor =
        descriptor::struct_type::<ThreadScalar>("construct::ThreadScalar", StructKind::Newtype, &THREAD_SCALAR_FIELDS);
    static THREAD_SCALAR_CONSTRUCTION_FIELDS: [ConstructionField<ThreadSafe>; 1] =
        [ConstructionField::required(&THREAD_SCALAR_FIELDS[0])];

    #[test]
    fn test_thread_safe_mode_uses_the_same_exact_validation_contract() {
        let constructor = StructConstructor::new(
            &THREAD_SCALAR_DESCRIPTOR,
            &THREAD_SCALAR_CONSTRUCTION_FIELDS,
            construct_thread_scalar,
        );
        let value = constructor
            .construct_tuple(TupleConstructionInput::new([DynamicOwned::<ThreadSafe>::new(11_u32)]))
            .expect("thread-safe input should validate without mode conversion");

        let scalar = value
            .downcast::<ThreadScalar>()
            .unwrap_or_else(|_| panic!("the adapter result should retain thread-safe storage"));
        assert_eq!(scalar, ThreadScalar(11));
    }
}
