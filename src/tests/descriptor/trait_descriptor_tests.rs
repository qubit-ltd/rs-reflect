// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Crate-unit coverage for trait descriptor navigation and cache-facing facts.

use std::any::TypeId;
use std::sync::LazyLock;

use crate::descriptor::AssociatedConstDescriptor;
use crate::descriptor::AssociatedTypeDescriptor;
use crate::descriptor::TraitCompleteness;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TraitId;
use crate::descriptor::TraitImplPayload;
use crate::expression::DiagnosticText;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::TypeExpression;
use crate::identity::Visibility;

struct UnitTraitMarker;
struct UnitChildMarker;
struct UnitAlternateMarker;

static EMPTY_GENERIC: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
    parameters: Box::new([]),
    predicates: Box::new([]),
    diagnostic: DiagnosticText::default(),
});

static UNIT_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new_with_visibility(
        TraitId::Reflected(TypeId::of::<UnitTraitMarker>()),
        "UnitTrait",
        "crate::UnitTrait",
        "unit_trait",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC,
        Visibility::Public,
    )
});

static CHILD_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<UnitChildMarker>()),
        "UnitChild",
        "crate::UnitChild",
        "unit_child",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC,
    )
});

static ALTERNATE_DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
    TraitDefinitionDescriptor::new(
        TraitId::Reflected(TypeId::of::<UnitAlternateMarker>()),
        "UnitAlternate",
        "crate::UnitTrait",
        "alternate_trait",
        TraitCompleteness::Complete,
        &EMPTY_GENERIC,
    )
});

/// Verifies declaration, application, supertrait, and associated-item views in
/// the crate-unit compilation context used by coverage instrumentation.
#[test]
fn test_trait_descriptor_views_expose_all_local_facts() {
    UNIT_DEFINITION.initialize_members(|_| {
        (
            Box::new([]),
            Box::new([AssociatedTypeDescriptor::new(
                0,
                "Item",
                "item",
                Box::new([]),
                Some(TypeExpression::Never),
            )]),
            Box::new([AssociatedConstDescriptor::new(
                0,
                "LIMIT",
                "limit",
                TypeExpression::Never,
                true,
            )]),
        )
    });
    assert_eq!(UNIT_DEFINITION.visibility(), &Visibility::Public);
    assert_eq!(UNIT_DEFINITION.rust_name(), "UnitTrait");
    assert_eq!(UNIT_DEFINITION.rust_path(), "crate::UnitTrait");
    assert_eq!(UNIT_DEFINITION.query_name(), "unit_trait");
    assert_eq!(UNIT_DEFINITION.completeness(), TraitCompleteness::Complete);
    assert!(UNIT_DEFINITION.is_compatible_with(&CHILD_DEFINITION));
    assert!(UNIT_DEFINITION.methods().is_empty());
    assert_eq!(UNIT_DEFINITION.associated_types().len(), 1);
    assert_eq!(UNIT_DEFINITION.associated_consts().len(), 1);

    let root = Box::leak(Box::new(
        TraitDescriptor::builder(&UNIT_DEFINITION)
            .associated_types(vec![UNIT_DEFINITION.associated_types()[0].clone()])
            .associated_consts(vec![UNIT_DEFINITION.associated_consts()[0].clone()])
            .build()
            .expect("unit trait facts must build"),
    ));
    let child = TraitDescriptor::builder(&CHILD_DEFINITION)
        .direct_supertraits([root as &'static TraitDescriptor])
        .build()
        .expect("acyclic child trait facts must build");

    assert_eq!(root.definition().rust_name(), "UnitTrait");
    assert_eq!(root.trait_id().definition(), UNIT_DEFINITION.trait_id());
    assert!(root.trait_id().arguments().is_empty());
    assert!(root.trait_id().associated_type_arguments().is_empty());
    assert!(root.arguments().is_empty());
    assert!(root.associated_type_arguments().is_empty());
    assert_eq!(root.rust_name(), "UnitTrait");
    assert_eq!(root.query_name(), "unit_trait");
    assert_eq!(root.completeness(), TraitCompleteness::Complete);
    assert_eq!(root.associated_types().len(), 1);
    assert_eq!(root.associated_consts().len(), 1);
    assert!(root.method("missing").is_none());
    assert_eq!(
        root.associated_type("item").map(AssociatedTypeDescriptor::index),
        Some(0)
    );
    assert_eq!(
        root.associated_const("limit").map(AssociatedConstDescriptor::index),
        Some(0),
    );
    let associated_type = root.associated_type("item").expect("associated type");
    assert_eq!(associated_type.rust_name(), "Item");
    assert_eq!(associated_type.query_name(), "item");
    assert!(associated_type.bounds().is_empty());
    assert!(associated_type.generic_definition().parameters.is_empty());
    assert_eq!(associated_type.default(), Some(&TypeExpression::Never));
    let associated_const = root.associated_const("limit").expect("associated const");
    assert_eq!(associated_const.rust_name(), "LIMIT");
    assert_eq!(associated_const.query_name(), "limit");
    assert_eq!(associated_const.declared_type(), &TypeExpression::Never);
    assert!(associated_const.has_default());
    assert!(root.same_application(root));

    let direct = child.direct_supertraits();
    assert_eq!(direct.len(), 1);
    assert!(!direct.is_empty());
    assert_eq!(
        direct.iter().next().map(|descriptor| descriptor.rust_name()),
        Some("UnitTrait")
    );
    assert_eq!(child.all_supertraits().len(), 1);
    assert_eq!(child.all_supertraits().iter().count(), 1);

    let alternate = Box::leak(Box::new(
        TraitDescriptor::builder(&ALTERNATE_DEFINITION)
            .build()
            .expect("alternate trait facts must build"),
    ));
    let sorted = TraitDescriptor::builder(&CHILD_DEFINITION)
        .direct_supertraits([root as &'static TraitDescriptor, alternate as &'static TraitDescriptor])
        .build()
        .expect("same-path supertraits must use query names as a stable tie-breaker");
    let sorted_names = sorted
        .all_supertraits()
        .iter()
        .map(TraitDescriptor::query_name)
        .collect::<Vec<_>>();
    assert_eq!(sorted_names, ["alternate_trait", "unit_trait"]);

    let payload = TraitImplPayload::new(&UNIT_DEFINITION, root);
    assert!(std::ptr::eq(payload.definition(), &*UNIT_DEFINITION));
    assert!(std::ptr::eq(payload.applied(), root));
    assert!(payload.default_method_adapters().is_empty());
    assert!(payload.default_method_unavailable_reasons().is_empty());
    assert!(payload.associated_type_resolvers().is_empty());
    assert!(payload.associated_const_readers().is_empty());
    assert!(format!("{root:?}").contains("TraitDescriptor"));
}
