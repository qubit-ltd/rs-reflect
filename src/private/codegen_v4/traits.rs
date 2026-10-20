// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Typed trait declaration and application metadata for generated code.

use std::marker::PhantomData;

use crate::runtime::descriptor::generic::GenericArgumentMetadata;
use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadata;
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata;
pub use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata as AssociatedConst;
use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata;
pub use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata as AssociatedType;
use crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness;
pub use crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness as SupertraitStatus;
use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
pub use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata as TraitApplication;
use crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity;
pub use crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity as ArgumentIdentity;
use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
pub use crate::runtime::descriptor::trait_descriptor::TraitCompleteness as Completeness;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata;
pub use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata as TraitExpression;
use crate::runtime::descriptor::trait_descriptor::Visibility;
pub use crate::runtime::descriptor::trait_descriptor::Visibility as TraitVisibility;
use crate::runtime::expression::ExpressionData;
use crate::runtime::expression::PredicateData;
use crate::runtime::expression::TraitExpressionData;

/// Marker-bound static metadata for one trait declaration.
pub struct TraitDefinitionSpec<D: 'static> {
    metadata: TraitDefinitionMetadata,
    _marker: PhantomData<fn() -> D>,
}

impl<D: 'static> TraitDefinitionSpec<D> {
    /// Creates a trait definition without exposing or accepting its identity
    /// token.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: &'static str,
        path: &'static str,
        visibility: Visibility,
        completeness: TraitCompleteness,
        generics: &'static GenericDefinitionMetadata,
        supertraits: &'static [TraitExpressionMetadata],
        methods: &'static [MethodMetadata],
        associated_types: &'static [AssociatedTypeDefinitionMetadata],
        associated_consts: &'static [AssociatedConstDefinitionMetadata],
    ) -> Self {
        Self {
            metadata: TraitDefinitionMetadata::new::<D>(
                name,
                path,
                visibility,
                completeness,
                generics,
                supertraits,
                methods,
                associated_types,
                associated_consts,
            ),
            _marker: PhantomData,
        }
    }

    /// Borrows this static declaration for another typed metadata node.
    pub const fn metadata(&'static self) -> &'static TraitDefinitionMetadata {
        &self.metadata
    }

    /// Returns the fully qualified source path of this trait definition.
    pub const fn path(&self) -> &'static str {
        self.metadata.path
    }

    /// Returns a method declaration by its source-order index.
    pub const fn method_at(&'static self, index: usize) -> &'static MethodMetadata {
        if index >= self.metadata.methods.len() {
            panic!("method index is out of range");
        }
        &self.metadata.methods[index]
    }

    /// Returns the number of method declarations in this trait.
    pub const fn method_count(&self) -> usize {
        self.metadata.methods.len()
    }

    /// Returns an associated type declaration by its source-order index.
    pub const fn associated_type_at(&'static self, index: usize) -> &'static AssociatedTypeDefinitionMetadata {
        if index >= self.metadata.associated_types.len() {
            panic!("associated type index is out of range");
        }
        &self.metadata.associated_types[index]
    }

    /// Returns the number of associated type declarations.
    pub const fn associated_type_count(&self) -> usize {
        self.metadata.associated_types.len()
    }

    /// Finds an associated type declaration by its Rust item name.
    pub const fn associated_type_named(&'static self, name: &'static str) -> &'static AssociatedTypeDefinitionMetadata {
        let mut index = 0;
        while index < self.metadata.associated_types.len() {
            let item = &self.metadata.associated_types[index];
            if const_str_eq(item.name, name) {
                return item;
            }
            index += 1;
        }
        panic!("associated type name is not declared by this trait");
    }

    /// Returns a trait generic parameter by its source-order index.
    pub const fn generic_parameter_at(&'static self, index: usize) -> &'static GenericParameterMetadata {
        if index >= self.metadata.generics.parameters.len() {
            panic!("trait generic parameter index is out of range");
        }
        &self.metadata.generics.parameters[index]
    }

    /// Returns an associated constant declaration by its source-order index.
    pub const fn associated_const_at(&'static self, index: usize) -> &'static AssociatedConstDefinitionMetadata {
        if index >= self.metadata.associated_consts.len() {
            panic!("associated constant index is out of range");
        }
        &self.metadata.associated_consts[index]
    }

    /// Returns the number of associated constant declarations.
    pub const fn associated_const_count(&self) -> usize {
        self.metadata.associated_consts.len()
    }

    /// Finds an associated constant declaration by its Rust item name.
    pub const fn associated_const_named(
        &'static self,
        name: &'static str,
    ) -> &'static AssociatedConstDefinitionMetadata {
        let mut index = 0;
        while index < self.metadata.associated_consts.len() {
            let item = &self.metadata.associated_consts[index];
            if const_str_eq(item.name, name) {
                return item;
            }
            index += 1;
        }
        panic!("associated constant name is not declared by this trait");
    }
}

const fn const_str_eq(left: &str, right: &str) -> bool {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    if left_bytes.len() != right_bytes.len() {
        return false;
    }
    let mut index = 0;
    while index < left_bytes.len() {
        if left_bytes[index] != right_bytes[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Marker-bound static metadata for one compiled trait application.
pub struct TraitApplicationSpec<T: 'static> {
    metadata: TraitApplicationMetadata,
    _target: PhantomData<fn() -> T>,
}

impl<T: crate::runtime::descriptor::Reflect + 'static> TraitApplicationSpec<T> {
    /// Creates an application whose declaration and target are both type-bound.
    pub const fn for_target<D: 'static>(
        definition: &'static TraitDefinitionSpec<D>,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        direct_supertraits: &'static [&'static TraitApplicationMetadata],
        all_supertraits: &'static [&'static TraitApplicationMetadata],
        supertrait_completeness: SupertraitCompleteness,
    ) -> Self {
        Self {
            metadata: TraitApplicationMetadata::new(
                definition.metadata(),
                arguments,
                identity,
                direct_supertraits,
                all_supertraits,
                supertrait_completeness,
            ),
            _target: PhantomData,
        }
    }

    /// Creates a linked application and retains the exact path used by its
    /// impl.
    #[allow(clippy::too_many_arguments)]
    pub const fn for_target_with_source<D: 'static>(
        definition: &'static TraitDefinitionSpec<D>,
        expression: &'static TraitExpressionData,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        direct_supertraits: &'static [&'static TraitApplicationMetadata],
        all_supertraits: &'static [&'static TraitApplicationMetadata],
        supertrait_completeness: SupertraitCompleteness,
    ) -> Self {
        Self {
            metadata: TraitApplicationMetadata::linked(
                definition.metadata(),
                expression,
                arguments,
                identity,
                direct_supertraits,
                all_supertraits,
                supertrait_completeness,
            ),
            _target: PhantomData,
        }
    }

    /// Creates a typed application whose definition is resolved from its exact
    /// source path.
    pub const fn unresolved_for_target(
        expression: &'static TraitExpressionData,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
    ) -> Self {
        Self {
            metadata: TraitApplicationMetadata::unresolved(expression, arguments, identity),
            _target: PhantomData,
        }
    }

    /// Borrows this static application metadata for an implementation binding.
    pub const fn metadata(&'static self) -> &'static TraitApplicationMetadata {
        &self.metadata
    }
}

/// Creates an unresolved direct supertrait application while retaining the
/// concrete arguments supplied by generated code.
pub const fn unresolved_application(
    expression: &'static TraitExpressionData,
    arguments: &'static [GenericArgumentMetadata],
    identity: &'static [TraitArgumentIdentity],
) -> TraitApplication {
    TraitApplication::unresolved(expression, arguments, identity)
}

/// Builds the de-duplicated transitive closure for typed direct applications.
#[doc(hidden)]
pub fn all_supertrait_applications(
    direct: &'static [&'static TraitApplication],
) -> &'static [&'static TraitApplication] {
    let mut values = Vec::new();
    for application in direct {
        push_unique_application(&mut values, application);
        for inherited in application.all_supertraits {
            push_unique_application(&mut values, inherited);
        }
    }
    Box::leak(values.into_boxed_slice())
}

/// Returns whether every direct application and its retained closure is typed.
#[doc(hidden)]
pub fn supertrait_applications_complete(direct: &[&'static TraitApplication]) -> SupertraitCompleteness {
    if direct.iter().all(|application| {
        application
            .definition
            .is_some_and(|definition| definition.completeness == TraitCompleteness::Complete)
            && application.supertrait_completeness == SupertraitCompleteness::Complete
    }) {
        SupertraitCompleteness::Complete
    } else {
        SupertraitCompleteness::Incomplete
    }
}

fn push_unique_application(values: &mut Vec<&'static TraitApplication>, candidate: &'static TraitApplication) {
    let duplicate = values
        .iter()
        .any(|existing| match (existing.definition, candidate.definition) {
            (Some(left), Some(right)) => left.id == right.id && existing.identity == candidate.identity,
            _ => std::ptr::eq(*existing, candidate),
        });
    if !duplicate {
        values.push(candidate);
    }
}

/// Creates an unresolved source trait expression.
pub const fn unresolved_trait_expression(expression: &'static TraitExpressionData) -> TraitExpressionMetadata {
    TraitExpressionMetadata::unresolved(expression)
}

/// Associates a source trait expression with a typed local definition.
pub const fn trait_expression_for<D: 'static>(
    expression: &'static TraitExpressionData,
    definition: &'static TraitDefinitionSpec<D>,
) -> TraitExpressionMetadata {
    TraitExpressionMetadata::for_definition(expression, definition.metadata())
}

/// Creates metadata for a trait associated type.
pub const fn associated_type(
    name: &'static str,
    visibility: Visibility,
    bounds: &'static [PredicateData],
    default: Option<&'static ExpressionData>,
) -> AssociatedTypeDefinitionMetadata {
    AssociatedTypeDefinitionMetadata::new(name, visibility, bounds, default)
}

/// Creates metadata for a trait associated constant.
pub const fn associated_const(
    name: &'static str,
    visibility: Visibility,
    ty: &'static ExpressionData,
    default: Option<crate::runtime::expression::ConstExpression>,
) -> AssociatedConstDefinitionMetadata {
    AssociatedConstDefinitionMetadata::new(name, visibility, ty, default)
}

const fn no_dependencies() -> &'static [std::any::TypeId] {
    &[]
}

/// Builds a global trait definition registration from marker-bound metadata.
#[doc(hidden)]
pub fn register_trait_definition<D: 'static>(
    definition: &'static TraitDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::metadata_only()
        .with_trait_definition(definition.metadata())
}

/// Builds a trait definition registration from a cached static metadata
/// provider.
#[doc(hidden)]
pub fn register_trait_definition_from<D: 'static>(
    definition: fn() -> &'static TraitDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    register_trait_definition(definition())
}

/// Creates metadata for an external trait whose declaration is unavailable.
#[doc(hidden)]
pub fn external_trait_definition<D: 'static>(name: &'static str, path: &'static str) -> TraitDefinitionSpec<D> {
    static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    TraitDefinitionSpec::new(
        name,
        path,
        Visibility::Public,
        TraitCompleteness::ExternalIncomplete,
        &EMPTY_GENERICS,
        &[],
        &[],
        &[],
        &[],
    )
}

/// Builds a concrete trait application registration for target `T`.
#[doc(hidden)]
pub fn register_trait_application<T: crate::runtime::descriptor::Reflect + 'static>(
    application: &'static TraitApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::metadata_only()
        .with_trait_application(application.metadata())
}

/// Builds an application registration from a cached static metadata provider.
#[doc(hidden)]
pub fn register_trait_application_from<T: crate::runtime::descriptor::Reflect + 'static>(
    application: fn() -> &'static TraitApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    register_trait_application(application())
}

/// Creates a global metadata fragment without exposing raw inventory payloads.
#[doc(hidden)]
pub const fn trait_definition_fragment(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_global_registration(label, registration)
}

/// Creates an application fragment bound to target `T`.
#[doc(hidden)]
pub const fn trait_application_fragment<T: crate::runtime::descriptor::Reflect + 'static>(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_target::<T>(
        ("generated", None, "generated", Some(label), None, None, label),
        no_dependencies,
        registration,
    )
}

/// Submits a marker-bound trait definition without exposing inventory
/// internals.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_trait_definition {
    ($marker:ty, $definition:path) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::traits::register_trait_definition_from::<$marker>($definition)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::traits::trait_definition_fragment(
                    stringify!($marker),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_trait_definition as submit_trait_definition;

/// Submits a typed trait application for a concrete reflected target.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_trait_application {
    ($target:ty, $application:path) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::traits::register_trait_application_from::<$target>($application)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::traits::trait_application_fragment::<$target>(
                    concat!(stringify!($target), "::trait-application"),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_trait_application as submit_trait_application;

#[cfg(test)]
mod tests {
    use super::AssociatedConstDefinitionMetadata;
    use super::AssociatedTypeDefinitionMetadata;
    use super::GenericDefinitionMetadata;
    use super::GenericParameterMetadata;
    use super::MethodMetadata;
    use super::SupertraitCompleteness;
    use super::TraitApplicationSpec;
    use super::TraitArgumentIdentity;
    use super::TraitCompleteness;
    use super::TraitDefinitionSpec;
    use super::TraitExpressionData;
    use super::Visibility;
    use super::all_supertrait_applications;
    use super::associated_const;
    use super::associated_type;
    use super::external_trait_definition;
    use super::register_trait_application;
    use super::register_trait_application_from;
    use super::register_trait_definition;
    use super::register_trait_definition_from;
    use super::supertrait_applications_complete;
    use super::trait_application_fragment;
    use super::trait_definition_fragment;
    use super::unresolved_application;
    use crate::runtime::descriptor::Reflect;
    use crate::runtime::descriptor::generic::GenericParameterMetadataKind;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::expression::ConstExpression;
    use crate::runtime::expression::ExpressionData;

    struct TraitMarker;
    struct ParentMarker;
    struct OtherMarker;
    struct ExternalMarker;
    struct Target;

    impl Reflect for Target {}

    static GENERIC_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::type_parameter("T", 0, &[], None);
    static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[GENERIC_PARAMETER], &[]);
    static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static METHODS: [MethodMetadata; 1] = [MethodMetadata::inherent("run", "Trait::run")];
    static ASSOCIATED_TYPE: AssociatedTypeDefinitionMetadata = associated_type("Item", Visibility::Public, &[], None);
    static SELF_TYPE: ExpressionData = ExpressionData::SelfType;
    static ASSOCIATED_CONST: AssociatedConstDefinitionMetadata = associated_const(
        "COUNT",
        Visibility::Private,
        &SELF_TYPE,
        Some(ConstExpression::UnsignedInteger(3)),
    );
    static DEFINITION: TraitDefinitionSpec<TraitMarker> = TraitDefinitionSpec::new(
        "Trait",
        "crate::Trait",
        Visibility::Restricted("crate"),
        TraitCompleteness::Complete,
        &GENERICS,
        &[],
        &METHODS,
        &[ASSOCIATED_TYPE],
        &[ASSOCIATED_CONST],
    );
    static PARENT_DEFINITION: TraitDefinitionSpec<ParentMarker> = TraitDefinitionSpec::new(
        "Parent",
        "crate::Parent",
        Visibility::Public,
        TraitCompleteness::Complete,
        &EMPTY_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static OTHER_DEFINITION: TraitDefinitionSpec<OtherMarker> = TraitDefinitionSpec::new(
        "Other",
        "crate::Other",
        Visibility::Public,
        TraitCompleteness::Complete,
        &EMPTY_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static EXTERNAL_DEFINITION: TraitDefinitionSpec<ExternalMarker> = TraitDefinitionSpec::new(
        "External",
        "external_crate::External",
        Visibility::Public,
        TraitCompleteness::ExternalIncomplete,
        &EMPTY_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static SOURCE_EXPRESSION: TraitExpressionData = TraitExpressionData::new("alias::Trait", &[], &[]);
    static PARENT_APP: TraitApplicationMetadata = TraitApplicationMetadata::new(
        PARENT_DEFINITION.metadata(),
        &[],
        &[],
        &[],
        &[],
        SupertraitCompleteness::Complete,
    );
    static PARENT_APP_INCOMPLETE: TraitApplicationMetadata = TraitApplicationMetadata::new(
        PARENT_DEFINITION.metadata(),
        &[],
        &[],
        &[],
        &[],
        SupertraitCompleteness::Incomplete,
    );
    static PARENT_APP_WITH_CLOSURE: TraitApplicationMetadata = TraitApplicationMetadata::new(
        PARENT_DEFINITION.metadata(),
        &[],
        &[],
        &[],
        &[&PARENT_APP],
        SupertraitCompleteness::Complete,
    );
    static PARENT_APP_DIFFERENT_ARGUMENT: TraitApplicationMetadata = TraitApplicationMetadata::new(
        PARENT_DEFINITION.metadata(),
        &[],
        &[TraitArgumentIdentity::type_argument::<u8>()],
        &[],
        &[],
        SupertraitCompleteness::Complete,
    );
    static OTHER_APP: TraitApplicationMetadata = TraitApplicationMetadata::new(
        OTHER_DEFINITION.metadata(),
        &[],
        &[],
        &[],
        &[],
        SupertraitCompleteness::Complete,
    );
    static EXTERNAL_APP: TraitApplicationMetadata = TraitApplicationMetadata::new(
        EXTERNAL_DEFINITION.metadata(),
        &[],
        &[],
        &[],
        &[],
        SupertraitCompleteness::Complete,
    );
    static UNRESOLVED_APP: TraitApplicationMetadata =
        TraitApplicationMetadata::unresolved(&SOURCE_EXPRESSION, &[], &[]);
    static COMPLETE_DIRECT: [&TraitApplicationMetadata; 1] = [&PARENT_APP];
    static INCOMPLETE_DIRECT: [&TraitApplicationMetadata; 1] = [&UNRESOLVED_APP];
    static RESOLVED_INCOMPLETE_DIRECT: [&TraitApplicationMetadata; 1] = [&PARENT_APP_INCOMPLETE];
    static EXTERNAL_DIRECT: [&TraitApplicationMetadata; 1] = [&EXTERNAL_APP];

    #[test]
    fn trait_definition_spec_exposes_items_by_index_and_name() {
        assert_eq!(DEFINITION.path(), "crate::Trait");
        assert_eq!(DEFINITION.metadata().name, "Trait");
        assert_eq!(DEFINITION.method_count(), 1);
        assert!(std::ptr::eq(DEFINITION.method_at(0), &METHODS[0]));
        assert_eq!(DEFINITION.associated_type_count(), 1);
        assert_eq!(DEFINITION.associated_type_at(0).name, "Item");
        assert_eq!(DEFINITION.associated_type_named("Item").name, "Item");
        assert_eq!(DEFINITION.generic_parameter_at(0).name, "T");
        assert!(matches!(
            DEFINITION.generic_parameter_at(0).kind,
            GenericParameterMetadataKind::Type { .. }
        ));
        assert_eq!(DEFINITION.associated_const_count(), 1);
        assert_eq!(DEFINITION.associated_const_at(0).name, "COUNT");
        assert_eq!(DEFINITION.associated_const_named("COUNT").name, "COUNT");
    }

    #[test]
    fn trait_definition_spec_reports_invalid_item_lookups() {
        assert!(std::panic::catch_unwind(|| DEFINITION.method_at(1)).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.associated_type_at(1)).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.associated_type_named("Missing")).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.associated_type_named("IteM")).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.generic_parameter_at(1)).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.associated_const_at(1)).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.associated_const_named("MISSING")).is_err());
        assert!(std::panic::catch_unwind(|| DEFINITION.associated_const_named("COUnt")).is_err());
    }

    #[test]
    fn application_specs_preserve_linked_unresolved_and_source_metadata() {
        static LINKED: TraitApplicationSpec<Target> =
            TraitApplicationSpec::for_target(&DEFINITION, &[], &[], &[], &[], SupertraitCompleteness::Complete);
        static LINKED_WITH_SOURCE: TraitApplicationSpec<Target> = TraitApplicationSpec::for_target_with_source(
            &DEFINITION,
            &SOURCE_EXPRESSION,
            &[],
            &[],
            &[],
            &[],
            SupertraitCompleteness::Complete,
        );
        static UNRESOLVED: TraitApplicationSpec<Target> =
            TraitApplicationSpec::unresolved_for_target(&SOURCE_EXPRESSION, &[], &[]);

        assert!(LINKED.metadata().definition.is_some());
        assert!(LINKED.metadata().expression.is_none());
        assert!(LINKED_WITH_SOURCE.metadata().definition.is_some());
        assert_eq!(LINKED_WITH_SOURCE.metadata().expression.unwrap().path, "alias::Trait");
        assert!(UNRESOLVED.metadata().definition.is_none());
        assert_eq!(UNRESOLVED.metadata().expression.unwrap().path, "alias::Trait");
        assert_eq!(
            UNRESOLVED.metadata().supertrait_completeness,
            SupertraitCompleteness::Incomplete
        );

        let unresolved = unresolved_application(&SOURCE_EXPRESSION, &[], &[]);
        assert!(unresolved.definition.is_none());
        assert_eq!(unresolved.expression.unwrap().path, "alias::Trait");
    }

    #[test]
    fn supertrait_closure_deduplicates_by_definition_and_argument_identity() {
        static DIRECT: [&TraitApplicationMetadata; 4] = [
            &PARENT_APP_WITH_CLOSURE,
            &PARENT_APP,
            &PARENT_APP_DIFFERENT_ARGUMENT,
            &OTHER_APP,
        ];
        let closure = all_supertrait_applications(&DIRECT);
        assert_eq!(closure.len(), 3);
        assert!(std::ptr::eq(closure[0], &PARENT_APP_WITH_CLOSURE));
        assert!(std::ptr::eq(closure[1], &PARENT_APP_DIFFERENT_ARGUMENT));
        assert!(std::ptr::eq(closure[2], &OTHER_APP));

        static UNRESOLVED_DUPLICATE: [&TraitApplicationMetadata; 2] = [&UNRESOLVED_APP, &UNRESOLVED_APP];
        let unresolved_closure = all_supertrait_applications(&UNRESOLVED_DUPLICATE);
        assert_eq!(unresolved_closure.len(), 1);
        assert!(std::ptr::eq(unresolved_closure[0], &UNRESOLVED_APP));
    }

    #[test]
    fn supertrait_completeness_tracks_direct_application_metadata() {
        assert_eq!(supertrait_applications_complete(&[]), SupertraitCompleteness::Complete);
        assert_eq!(
            supertrait_applications_complete(&COMPLETE_DIRECT),
            SupertraitCompleteness::Complete
        );
        assert_eq!(
            supertrait_applications_complete(&INCOMPLETE_DIRECT),
            SupertraitCompleteness::Incomplete
        );
        assert_eq!(
            supertrait_applications_complete(&RESOLVED_INCOMPLETE_DIRECT),
            SupertraitCompleteness::Incomplete
        );
        assert_eq!(
            supertrait_applications_complete(&EXTERNAL_DIRECT),
            SupertraitCompleteness::Incomplete
        );
    }

    #[test]
    fn trait_registration_helpers_and_fragments_bind_the_expected_metadata() {
        fn definition_provider() -> &'static TraitDefinitionSpec<TraitMarker> {
            &DEFINITION
        }
        static APPLICATION: TraitApplicationSpec<Target> =
            TraitApplicationSpec::for_target(&DEFINITION, &[], &[], &[], &[], SupertraitCompleteness::Complete);
        fn application_provider() -> &'static TraitApplicationSpec<Target> {
            &APPLICATION
        }
        fn definition_registration() -> crate::__private::codegen_v4::registration::Registration {
            register_trait_definition_from(definition_provider)
        }
        fn application_registration() -> crate::__private::codegen_v4::registration::Registration {
            register_trait_application_from(application_provider)
        }

        let definition_registration_value = register_trait_definition(&DEFINITION);
        assert!(std::ptr::eq(
            definition_registration_value.trait_definitions()[0],
            DEFINITION.metadata()
        ));
        let provider_registration = register_trait_definition_from(definition_provider);
        assert!(std::ptr::eq(
            provider_registration.trait_definitions()[0],
            DEFINITION.metadata()
        ));
        let application_registration_value = register_trait_application(&APPLICATION);
        assert!(std::ptr::eq(
            application_registration_value.trait_applications()[0],
            APPLICATION.metadata()
        ));
        let provider_registration = register_trait_application_from(application_provider);
        assert!(std::ptr::eq(
            provider_registration.trait_applications()[0],
            APPLICATION.metadata()
        ));

        let definition_fragment = trait_definition_fragment("trait-definition", definition_registration);
        assert!(!definition_fragment.includes_type());
        assert!(definition_fragment.target_type_id().is_none());
        assert!(definition_fragment.dependencies().is_empty());
        assert!(std::ptr::eq(
            definition_fragment
                .materialize()
                .expect("global trait definition fragment materializes")
                .trait_definitions()[0],
            DEFINITION.metadata()
        ));

        let application_fragment = trait_application_fragment::<Target>("trait-application", application_registration);
        assert!(!application_fragment.includes_type());
        assert_eq!(
            application_fragment.target_type_id(),
            Some(std::any::TypeId::of::<Target>())
        );
        assert!(application_fragment.dependencies().is_empty());
        assert!(std::ptr::eq(
            application_fragment
                .materialize()
                .expect("target trait application fragment materializes")
                .trait_applications()[0],
            APPLICATION.metadata()
        ));
    }

    #[test]
    fn external_trait_definition_is_conservatively_incomplete() {
        let definition = Box::leak(Box::new(external_trait_definition::<OtherMarker>(
            "Debug",
            "std::fmt::Debug",
        )));
        let metadata = definition.metadata();
        assert_eq!(metadata.name, "Debug");
        assert_eq!(metadata.path, "std::fmt::Debug");
        assert_eq!(metadata.completeness, TraitCompleteness::ExternalIncomplete);
        assert_eq!(metadata.visibility, Visibility::Public);
        assert_eq!(metadata.generics.parameters().len(), 0);
        assert!(metadata.supertraits.is_empty());
        assert!(metadata.methods.is_empty());
        assert!(metadata.associated_types.is_empty());
        assert!(metadata.associated_consts.is_empty());
    }
}
