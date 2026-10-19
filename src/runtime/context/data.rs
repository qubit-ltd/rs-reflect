use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use crate::runtime::capability::capabilities::Capabilities;
use crate::runtime::capability::capability_error::CapabilityError;
use crate::runtime::capability::capability_origin::CapabilityOrigin;
use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::capability::capability_target::CapabilityTargetId;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::descriptor::collections::TypeShapeRegistration;
use crate::runtime::descriptor::generic::{TypeDefinitionId, TypeDefinitionMetadata};
use crate::runtime::descriptor::impl_descriptor::{
    ImplApplicationMetadata, ImplDefinitionId, ImplDefinitionMetadata,
};
use crate::runtime::descriptor::method::{EffectiveMethod, MethodCatalogRegistration};
use crate::runtime::descriptor::trait_descriptor::{
    TraitApplicationMetadata, TraitDefinitionId, TraitDefinitionMetadata,
};
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::error::{
    AdapterValidationError, ContextError, ContextIssue, MetadataValidationError,
};
use crate::runtime::identity::{ItemIdentity, ItemKind, RegistrationOrigin, StableItemId};

use super::indexes::ContextIndexes;
use super::registration::{Registration, TypeRecord};

pub(crate) struct ContextData {
    pub(crate) types: Box<[TypeRecord]>,
    pub(crate) indexes: ContextIndexes,
    shapes: HashMap<TypeId, &'static TypeKindMetadata>,
    methods: HashMap<TypeId, &'static [EffectiveMethod]>,
    type_definitions: HashMap<TypeDefinitionId, &'static TypeDefinitionMetadata>,
    trait_definitions: HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    impl_definitions: HashMap<ImplDefinitionId, &'static ImplDefinitionMetadata>,
    capabilities: HashMap<CapabilityTargetId, Arc<Capabilities>>,
    type_instances: HashMap<TypeDefinitionId, Box<[TypeId]>>,
    trait_applications: HashMap<TraitDefinitionId, Box<[&'static TraitApplicationMetadata]>>,
    impl_applications: HashMap<ImplDefinitionId, Box<[&'static ImplApplicationMetadata]>>,
    construction_plans: HashMap<TypeId, Box<[&'static ConstructionPlan]>>,
}

impl ContextData {
    pub(crate) fn empty() -> Self {
        Self {
            types: Box::new([]),
            indexes: ContextIndexes::default(),
            shapes: HashMap::new(),
            methods: HashMap::new(),
            type_definitions: HashMap::new(),
            trait_definitions: HashMap::new(),
            impl_definitions: HashMap::new(),
            capabilities: HashMap::new(),
            type_instances: HashMap::new(),
            trait_applications: HashMap::new(),
            impl_applications: HashMap::new(),
            construction_plans: HashMap::new(),
        }
    }

    pub(crate) fn build(mut registrations: Vec<Registration>) -> Result<Self, ContextError> {
        registrations.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));

        let mut registration_group_by_type = HashMap::with_capacity(registrations.len());
        let mut origins_by_type = Vec::<(TypeId, Vec<_>)>::with_capacity(registrations.len());
        let mut types = Vec::with_capacity(registrations.len());
        let mut type_definitions = HashMap::new();
        let mut trait_definitions = HashMap::new();
        let mut impl_definitions = HashMap::new();
        let mut capability_registrations = Vec::<CapabilityRegistration>::new();
        let mut type_instances = HashMap::<TypeDefinitionId, Vec<TypeId>>::new();
        let mut trait_applications =
            HashMap::<TraitDefinitionId, Vec<&'static TraitApplicationMetadata>>::new();
        let mut impl_applications =
            HashMap::<ImplDefinitionId, Vec<&'static ImplApplicationMetadata>>::new();
        let mut registered_shapes = HashMap::<TypeId, &'static TypeKindMetadata>::new();
        let mut construction_plans = HashMap::<TypeId, Vec<&'static ConstructionPlan>>::new();
        let mut type_definition_origins =
            HashMap::<TypeDefinitionId, Vec<RegistrationOrigin>>::new();
        let mut trait_definition_origins =
            HashMap::<TraitDefinitionId, Vec<RegistrationOrigin>>::new();
        let mut impl_definition_origins =
            HashMap::<ImplDefinitionId, Vec<RegistrationOrigin>>::new();
        for mut registration in registrations {
            for metadata in registration.type_definitions() {
                let origins = type_definition_origins.entry(metadata.id).or_default();
                origins.push(registration.origin().clone());
                if type_definitions.insert(metadata.id, *metadata).is_some() {
                    return Err(ContextError::from_issues(vec![
                        ContextIssue::ConflictingTypeDefinition {
                            identity: item_identity(
                                ItemKind::TypeDefinition,
                                metadata.path,
                                metadata.name,
                            ),
                            registrations: origins.clone().into_boxed_slice(),
                        },
                    ])
                    .expect("one context issue is nonempty"));
                }
            }
            for metadata in registration.trait_definitions() {
                let origins = trait_definition_origins.entry(metadata.id).or_default();
                origins.push(registration.origin().clone());
                if trait_definitions.insert(metadata.id, *metadata).is_some() {
                    return Err(ContextError::from_issues(vec![
                        ContextIssue::ConflictingTraitDefinition {
                            identity: item_identity(
                                ItemKind::TraitDefinition,
                                metadata.path,
                                metadata.name,
                            ),
                            registrations: origins.clone().into_boxed_slice(),
                        },
                    ])
                    .expect("one context issue is nonempty"));
                }
            }
            for metadata in registration.impl_definitions() {
                let origins = impl_definition_origins.entry(metadata.id).or_default();
                origins.push(registration.origin().clone());
                if impl_definitions.insert(metadata.id, *metadata).is_some() {
                    return Err(
                        ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                            item: item_identity(
                                ItemKind::Impl,
                                format!("{:?}", metadata.id).as_str(),
                                "impl definition",
                            ),
                            reason: MetadataValidationError::new(
                                "duplicate impl definition identity",
                            ),
                        }])
                        .expect("one context issue is nonempty"),
                    );
                }
            }
            capability_registrations.extend(registration.take_capabilities());
            let Some(type_record) = registration.type_record() else {
                continue;
            };
            let type_id = type_record.type_id();
            if let Some(shape) = registration.shape() {
                if registered_shapes.insert(type_id, shape).is_some() {
                    return Err(
                        ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                            item: type_identity(type_id),
                            reason: MetadataValidationError::new(
                                "multiple type shape registrations",
                            ),
                        }])
                        .expect("one context issue is nonempty"),
                    );
                }
            }
            if let Some(definition) = registration.type_instance_of() {
                type_instances.entry(definition).or_default().push(type_id);
            }
            for application in registration.trait_applications() {
                trait_applications
                    .entry(application.definition.id)
                    .or_default()
                    .push(*application);
            }
            for application in registration.impl_applications() {
                impl_applications
                    .entry(application.definition.id)
                    .or_default()
                    .push(*application);
            }
            for plan in registration.construction_plans() {
                if plan.target.identity().type_id != type_id {
                    let item = type_identity(type_id);
                    return Err(
                        ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                            item,
                            reason: MetadataValidationError::new(
                                "construction plan target does not match its registration",
                            ),
                        }])
                        .expect("one context issue is nonempty"),
                    );
                }
                construction_plans
                    .entry(plan.target.identity().type_id)
                    .or_default()
                    .push(*plan);
            }
            let index = *registration_group_by_type
                .entry(type_id)
                .or_insert_with(|| {
                    let index = origins_by_type.len();
                    origins_by_type.push((type_id, Vec::new()));
                    index
                });
            let origins = &mut origins_by_type[index].1;
            origins.push(registration.origin().clone());
            if origins.len() == 1 {
                types.push(type_record);
            }
        }

        let issues: Box<[_]> = origins_by_type
            .into_iter()
            .filter_map(|(type_id, origins)| {
                (origins.len() > 1).then(|| ContextIssue::DuplicateTypeIdentity {
                    type_id,
                    registrations: origins.into_boxed_slice(),
                })
            })
            .collect();
        if let Some(error) = ContextError::from_issues(issues) {
            return Err(error);
        }

        let types = types.into_boxed_slice();
        let indexes = ContextIndexes::build(&types);
        let included_types: std::collections::HashSet<_> =
            types.iter().map(|record| record.type_id()).collect();
        for (definition, instances) in &type_instances {
            if !type_definitions.contains_key(definition) {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Type,
                            format!("{:?}", definition).as_str(),
                            "generic application",
                        ),
                        dependency: item_identity(
                            ItemKind::TypeDefinition,
                            format!("{definition:?}").as_str(),
                            "generic definition",
                        ),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
            if let Some(missing) = instances
                .iter()
                .find(|instance| !included_types.contains(instance))
            {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::TypeDefinition,
                            "generic application",
                            "generic application",
                        ),
                        dependency: type_identity(*missing),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
        }
        for (definition, applications) in &trait_applications {
            if !trait_definitions.contains_key(definition) {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::TraitApplication,
                            format!("{definition:?}").as_str(),
                            "trait application",
                        ),
                        dependency: item_identity(
                            ItemKind::TraitDefinition,
                            format!("{definition:?}").as_str(),
                            "trait definition",
                        ),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
            for application in applications {
                for supertrait in application
                    .direct_supertraits
                    .iter()
                    .chain(application.all_supertraits.iter())
                {
                    if !trait_definitions.contains_key(&supertrait.definition.id) {
                        return Err(ContextError::from_issues(vec![
                            ContextIssue::MissingDependency {
                                item: item_identity(
                                    ItemKind::TraitApplication,
                                    format!("{definition:?}").as_str(),
                                    "trait application",
                                ),
                                dependency: item_identity(
                                    ItemKind::TraitDefinition,
                                    format!("{:?}", supertrait.definition.id).as_str(),
                                    supertrait.definition.name,
                                ),
                            },
                        ])
                        .expect("one context issue is nonempty"));
                    }
                }
            }
        }
        for (definition, applications) in &impl_applications {
            if !impl_definitions.contains_key(definition) {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Impl,
                            format!("{definition:?}").as_str(),
                            "impl application",
                        ),
                        dependency: item_identity(
                            ItemKind::Impl,
                            format!("{definition:?}").as_str(),
                            "impl definition",
                        ),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
            if let Some(application) = applications
                .iter()
                .find(|application| !included_types.contains(&application.target.type_id()))
            {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Impl,
                            format!("{:?}", application.definition.id).as_str(),
                            "impl application",
                        ),
                        dependency: type_identity(application.target.type_id()),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
            if let Some(application) = applications.iter().find(|application| {
                application
                    .implemented_trait
                    .is_some_and(|trait_application| {
                        !trait_definitions.contains_key(&trait_application.definition.id)
                    })
            }) {
                let trait_application = application.implemented_trait.expect("checked above");
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Impl,
                            format!("{:?}", application.definition.id).as_str(),
                            "impl application",
                        ),
                        dependency: item_identity(
                            ItemKind::TraitDefinition,
                            format!("{:?}", trait_application.definition.id).as_str(),
                            trait_application.definition.name,
                        ),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
        }
        for (target, _) in &construction_plans {
            if !included_types.contains(target) {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Impl,
                            "construction plan",
                            "construction plan",
                        ),
                        dependency: type_identity(*target),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
        }
        let mut shapes = registered_shapes;
        for registration in inventory::iter::<TypeShapeRegistration> {
            let type_id = registration.type_id();
            if !included_types.contains(&type_id) {
                continue;
            }
            if shapes.insert(type_id, registration.metadata()).is_some() {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                        item: type_identity(type_id),
                        reason: MetadataValidationError::new("multiple type shape registrations"),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
        }
        let mut methods = HashMap::new();
        for registration in inventory::iter::<MethodCatalogRegistration> {
            let type_id = registration.type_id();
            if !included_types.contains(&type_id) {
                continue;
            }
            if methods.insert(type_id, registration.methods()).is_some() {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                        item: type_identity(type_id),
                        reason: MetadataValidationError::new(
                            "multiple method catalog registrations",
                        ),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
        }
        let mut grouped_capabilities =
            HashMap::<CapabilityTargetId, Vec<CapabilityRegistration>>::new();
        for registration in capability_registrations {
            if let CapabilityTargetId::Type(identity) = &registration.target {
                if !included_types.contains(&identity.type_id) {
                    return Err(
                        ContextError::from_issues(vec![ContextIssue::MissingDependency {
                            item: item_identity(ItemKind::Capability, "capability", "capability"),
                            dependency: ItemIdentity {
                                kind: ItemKind::Type,
                                stable_id: StableItemId(identity.rust_name.into()),
                                display_name: identity.rust_name.into(),
                            },
                        }])
                        .expect("one context issue is nonempty"),
                    );
                }
            } else if let CapabilityTargetId::Definition(identity) = &registration.target {
                let name = identity.display_name.as_ref();
                let registered = type_definitions
                    .values()
                    .any(|definition| definition.name == name || definition.path == name)
                    || trait_definitions
                        .values()
                        .any(|definition| definition.name == name || definition.path == name)
                    || impl_definitions.values().any(|definition| {
                        format!("{:?}", definition.id) == identity.stable_id.0.as_ref()
                    });
                if !registered {
                    return Err(
                        ContextError::from_issues(vec![ContextIssue::MissingDependency {
                            item: identity.clone(),
                            dependency: item_identity(
                                identity.kind,
                                identity.stable_id.0.as_ref(),
                                identity.display_name.as_ref(),
                            ),
                        }])
                        .expect("one context issue is nonempty"),
                    );
                }
            }
            grouped_capabilities
                .entry(registration.target.clone())
                .or_default()
                .push(registration);
        }
        let capabilities = grouped_capabilities
            .into_iter()
            .map(|(target, registrations)| {
                Capabilities::try_new(target.clone(), registrations)
                    .map(|capabilities| (target, Arc::new(capabilities)))
                    .map_err(capability_context_error)
            })
            .collect::<Result<HashMap<_, _>, _>>()?;

        let type_instances = type_instances
            .into_iter()
            .map(|(definition, instances)| (definition, instances.into_boxed_slice()))
            .collect();
        let trait_applications = trait_applications
            .into_iter()
            .map(|(definition, applications)| (definition, applications.into_boxed_slice()))
            .collect();
        let impl_applications = impl_applications
            .into_iter()
            .map(|(definition, applications)| (definition, applications.into_boxed_slice()))
            .collect();
        let construction_plans = construction_plans
            .into_iter()
            .map(|(target, plans)| (target, plans.into_boxed_slice()))
            .collect();

        Ok(Self {
            types,
            indexes,
            shapes,
            methods,
            type_definitions,
            trait_definitions,
            impl_definitions,
            capabilities,
            type_instances,
            trait_applications,
            impl_applications,
            construction_plans,
        })
    }

    pub(crate) fn type_by_id(&self, type_id: TypeId) -> Option<TypeRecord> {
        self.indexes
            .types_by_id
            .get(&type_id)
            .map(|index| self.types[*index])
    }

    pub(crate) fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.indexes.types_by_id.contains_key(&type_id)
    }

    pub(crate) fn shape(&self, type_id: TypeId) -> Option<&'static TypeKindMetadata> {
        self.shapes.get(&type_id).copied()
    }

    pub(crate) fn methods(&self, type_id: TypeId) -> &'static [EffectiveMethod] {
        self.methods.get(&type_id).copied().unwrap_or(&[])
    }

    pub(crate) fn type_definition(
        &self,
        id: TypeDefinitionId,
    ) -> Option<&'static TypeDefinitionMetadata> {
        self.type_definitions.get(&id).copied()
    }

    pub(crate) fn trait_definition(
        &self,
        id: TraitDefinitionId,
    ) -> Option<&'static TraitDefinitionMetadata> {
        self.trait_definitions.get(&id).copied()
    }

    pub(crate) fn impl_definition(
        &self,
        id: ImplDefinitionId,
    ) -> Option<&'static ImplDefinitionMetadata> {
        self.impl_definitions.get(&id).copied()
    }

    pub(crate) fn capabilities(&self, target: &CapabilityTargetId) -> Option<Arc<Capabilities>> {
        self.capabilities.get(target).cloned()
    }

    pub(crate) fn type_instances(&self, definition: TypeDefinitionId) -> &[TypeId] {
        self.type_instances
            .get(&definition)
            .map(Box::as_ref)
            .unwrap_or(&[])
    }

    pub(crate) fn trait_applications(
        &self,
        definition: TraitDefinitionId,
    ) -> &[&'static TraitApplicationMetadata] {
        self.trait_applications
            .get(&definition)
            .map(Box::as_ref)
            .unwrap_or(&[])
    }

    pub(crate) fn impl_applications(
        &self,
        definition: ImplDefinitionId,
    ) -> &[&'static ImplApplicationMetadata] {
        self.impl_applications
            .get(&definition)
            .map(Box::as_ref)
            .unwrap_or(&[])
    }

    pub(crate) fn construction_plans(&self, target: TypeId) -> &[&'static ConstructionPlan] {
        self.construction_plans
            .get(&target)
            .map(Box::as_ref)
            .unwrap_or(&[])
    }
}

fn item_identity(kind: ItemKind, stable_id: &str, display_name: &str) -> ItemIdentity {
    ItemIdentity {
        kind,
        stable_id: StableItemId(stable_id.into()),
        display_name: display_name.into(),
    }
}

fn type_identity(type_id: TypeId) -> ItemIdentity {
    let display_name = format!("{type_id:?}");
    item_identity(ItemKind::Type, &display_name, &display_name)
}

fn capability_context_error(error: CapabilityError) -> ContextError {
    let issue = match error {
        CapabilityError::Conflict {
            target: CapabilityTargetId::Type(target),
            capability_id,
            registrations,
        } => ContextIssue::ConflictingCapability {
            target,
            capability: ItemIdentity {
                kind: ItemKind::Capability,
                stable_id: StableItemId(capability_id.as_str().into()),
                display_name: capability_id.as_str().into(),
            },
            registrations: registrations
                .iter()
                .map(capability_origin_registration)
                .collect(),
        },
        other => ContextIssue::InvalidAdapter {
            item: item_identity(ItemKind::Capability, "capability", "capability"),
            reason: AdapterValidationError::new(other.to_string()),
        },
    };
    ContextError::from_issues(vec![issue]).expect("one context issue is nonempty")
}

fn capability_origin_registration(origin: &CapabilityOrigin) -> RegistrationOrigin {
    match origin {
        CapabilityOrigin::Linked(origin) | CapabilityOrigin::Explicit(origin) => origin.clone(),
        CapabilityOrigin::Intrinsic => RegistrationOrigin {
            crate_name: "intrinsic",
            crate_version: None,
            module_path: "qubit_reflect::runtime::capability",
            file: None,
            line: None,
            column: None,
        },
    }
}
