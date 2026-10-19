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
use crate::runtime::descriptor::method::{
    ContextEffectiveMethod, EffectiveMethod, MethodCatalogRegistration, MethodNamespaceMetadata,
};
use crate::runtime::descriptor::trait_descriptor::{
    TraitApplicationMetadata, TraitArgumentIdentity, TraitDefinitionId, TraitDefinitionMetadata,
};
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::error::{
    AdapterValidationError, ContextError, ContextIssue, MetadataValidationError,
};
use crate::runtime::identity::{
    ItemIdentity, ItemKind, RegistrationOrigin, StableItemId, TypeIdentity,
};

use super::indexes::ContextIndexes;
use super::registration::{Registration, TraitObjectRegistration, TypeRecord};

pub(crate) struct ContextData {
    pub(crate) types: Box<[TypeRecord]>,
    pub(crate) indexes: ContextIndexes,
    shapes: HashMap<TypeId, &'static TypeKindMetadata>,
    methods: HashMap<TypeId, Arc<[ContextEffectiveMethod]>>,
    type_definitions: HashMap<TypeDefinitionId, &'static TypeDefinitionMetadata>,
    trait_definitions: HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    impl_definitions: HashMap<ImplDefinitionId, &'static ImplDefinitionMetadata>,
    capabilities: HashMap<CapabilityTargetId, Arc<Capabilities>>,
    type_instances: HashMap<TypeDefinitionId, Box<[TypeId]>>,
    trait_applications: HashMap<TraitDefinitionId, Box<[&'static TraitApplicationMetadata]>>,
    trait_applications_by_path: HashMap<&'static str, Box<[&'static TraitApplicationMetadata]>>,
    impl_applications: HashMap<ImplDefinitionId, Box<[&'static ImplApplicationMetadata]>>,
    construction_plans: HashMap<TypeId, Box<[&'static ConstructionPlan]>>,
    trait_objects: HashMap<TypeId, TraitObjectRegistration>,
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
            trait_applications_by_path: HashMap::new(),
            impl_applications: HashMap::new(),
            construction_plans: HashMap::new(),
            trait_objects: HashMap::new(),
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
        let mut trait_applications_pending = Vec::<&'static TraitApplicationMetadata>::new();
        let mut trait_applications =
            HashMap::<TraitDefinitionId, Vec<&'static TraitApplicationMetadata>>::new();
        let mut trait_applications_by_path =
            HashMap::<&'static str, Vec<&'static TraitApplicationMetadata>>::new();
        let mut impl_applications =
            HashMap::<ImplDefinitionId, Vec<&'static ImplApplicationMetadata>>::new();
        let mut impl_application_origins =
            Vec::<(&'static ImplApplicationMetadata, RegistrationOrigin)>::new();
        let mut registered_shapes = HashMap::<TypeId, &'static TypeKindMetadata>::new();
        let mut construction_plans = HashMap::<TypeId, Vec<&'static ConstructionPlan>>::new();
        let mut trait_objects = HashMap::<TypeId, TraitObjectRegistration>::new();
        let mut type_definition_origins =
            HashMap::<TypeDefinitionId, Vec<RegistrationOrigin>>::new();
        let mut trait_definition_origins =
            HashMap::<TraitDefinitionId, Vec<RegistrationOrigin>>::new();
        let mut trait_definition_ids_by_path = HashMap::<&'static str, TraitDefinitionId>::new();
        let mut impl_definition_origins =
            HashMap::<ImplDefinitionId, Vec<RegistrationOrigin>>::new();
        for mut registration in registrations {
            for trait_object in registration.trait_objects() {
                let type_id = trait_object.type_id();
                if let Some(previous) = trait_objects.insert(type_id, *trait_object) {
                    if previous.trait_path() != trait_object.trait_path() {
                        return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                            item: item_identity(ItemKind::Type, trait_object.rust_name(), "trait object"),
                            reason: MetadataValidationError::new(
                                "trait object type has conflicting registered trait paths",
                            ),
                        }])
                        .expect("one context issue is nonempty"));
                    }
                }
            }
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
                if let Some(previous) = trait_definition_ids_by_path
                    .insert(metadata.path, metadata.id)
                {
                    if previous != metadata.id {
                        return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                            item: item_identity(ItemKind::TraitDefinition, metadata.path, metadata.name),
                            reason: MetadataValidationError::new(
                                "multiple trait identity handles use the same source path",
                            ),
                        }])
                        .expect("one context issue is nonempty"));
                    }
                }
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
            for application in registration.trait_applications() {
                trait_applications_pending.push(*application);
            }
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
            for application in registration.impl_applications() {
                let applications = impl_applications
                    .entry(application.definition.id)
                    .or_default();
                if !applications
                    .iter()
                    .any(|candidate| std::ptr::eq(*candidate, *application))
                {
                    applications.push(*application);
                    impl_application_origins.push((*application, registration.origin().clone()));
                }
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

        for application in trait_applications_pending {
            let definition = resolve_trait_application_definition(
                application,
                &trait_definitions,
                &trait_definition_ids_by_path,
            )?;
            let path = trait_application_path(application).or_else(|| definition.map(|value| value.path));
            if let Some(path) = path {
                trait_applications_by_path.entry(path).or_default().push(application);
            }
            if let Some(definition) = definition {
                trait_applications.entry(definition.id).or_default().push(application);
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
                    let supertrait_definition = resolve_trait_application_definition(
                        supertrait,
                        &trait_definitions,
                        &trait_definition_ids_by_path,
                    )?;
                    if let Some(supertrait_definition) = supertrait_definition {
                    if !trait_definitions.contains_key(&supertrait_definition.id) {
                        return Err(ContextError::from_issues(vec![
                            ContextIssue::MissingDependency {
                                item: item_identity(
                                    ItemKind::TraitApplication,
                                    format!("{definition:?}").as_str(),
                                    "trait application",
                                ),
                                dependency: item_identity(
                                    ItemKind::TraitDefinition,
                                    format!("{:?}", supertrait_definition.id).as_str(),
                                    supertrait_definition.name,
                                ),
                            },
                        ])
                        .expect("one context issue is nonempty"));
                    }
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
                .find(|application| !included_types.contains(&(application.target)().type_id()))
            {
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Impl,
                            format!("{:?}", application.definition.id).as_str(),
                            "impl application",
                        ),
                        dependency: type_identity((application.target)().type_id()),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
            for application in applications {
                if let Some(trait_application) = application.implemented_trait {
                    resolve_trait_application_definition(
                        trait_application,
                        &trait_definitions,
                        &trait_definition_ids_by_path,
                    )?;
                }
            }
            if let Some(application) = applications.iter().find(|application| {
                application.implemented_trait.is_some_and(|trait_application| {
                    resolve_trait_application_definition(
                        trait_application,
                        &trait_definitions,
                        &trait_definition_ids_by_path,
                    )
                    .is_err()
                })
            }) {
                let trait_application = application.implemented_trait.expect("checked above");
                let trait_path = trait_application.expression.map_or("<unresolved trait>", |expr| expr.path);
                return Err(
                    ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(
                            ItemKind::Impl,
                            format!("{:?}", application.definition.id).as_str(),
                            "impl application",
                        ),
                        dependency: item_identity(
                            ItemKind::TraitDefinition,
                            trait_path,
                            trait_path,
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
        validate_trait_impl_uniqueness(
            &impl_application_origins,
            &trait_definitions,
            &trait_definition_ids_by_path,
        )?;
        let method_catalog_registrations: Vec<_> =
            inventory::iter::<MethodCatalogRegistration>().collect();
        let methods = aggregate_method_catalogs(
            &included_types,
            &impl_applications,
            &method_catalog_registrations,
            &trait_definitions,
            &trait_definition_ids_by_path,
        )?;
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
        let trait_applications_by_path = trait_applications_by_path
            .into_iter()
            .map(|(path, applications)| (path, applications.into_boxed_slice()))
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
            trait_applications_by_path,
            impl_applications,
            construction_plans,
            trait_objects,
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

    pub(crate) fn methods(&self, type_id: TypeId) -> Arc<[ContextEffectiveMethod]> {
        self.methods
            .get(&type_id)
            .cloned()
            .unwrap_or_else(|| Arc::from([]))
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

    pub(crate) fn trait_definition_by_path(
        &self,
        path: &str,
    ) -> Option<&'static TraitDefinitionMetadata> {
        let mut matches = self
            .trait_definitions
            .values()
            .filter(|metadata| metadata.path == path);
        let definition = matches.next()?;
        matches.next().is_none().then_some(definition)
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

    pub(crate) fn trait_applications_by_path(
        &self,
        path: &str,
    ) -> &[&'static TraitApplicationMetadata] {
        self.trait_applications_by_path
            .get(path)
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

    pub(crate) fn trait_object(&self, type_id: TypeId) -> Option<TraitObjectRegistration> {
        self.trait_objects.get(&type_id).copied()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MethodNamespaceKey {
    Inherent,
    TraitApplication {
        definition: TraitDefinitionId,
        identity: &'static [TraitArgumentIdentity],
    },
    TraitPath(&'static str),
}

struct MethodEntry {
    namespace: MethodNamespaceKey,
    method: EffectiveMethod,
}

fn validate_trait_impl_uniqueness(
    registrations: &[(&'static ImplApplicationMetadata, RegistrationOrigin)],
    definitions: &HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    definitions_by_path: &HashMap<&'static str, TraitDefinitionId>,
) -> Result<(), ContextError> {
    for (index, (application, origin)) in registrations.iter().enumerate() {
        let Some(trait_application) = application.implemented_trait else {
            continue;
        };
        let target = (application.target)();
        let definition = resolve_trait_application_definition(
            trait_application,
            definitions,
            definitions_by_path,
        )?;
        let duplicate_origins: Vec<_> = registrations[..index]
            .iter()
            .filter_map(|(previous, previous_origin)| {
                let previous_trait = previous.implemented_trait?;
                let same_target = (previous.target)().type_id() == target.type_id();
                let previous_definition = resolve_trait_application_definition(
                    previous_trait,
                    definitions,
                    definitions_by_path,
                ).ok()?;
                let same_trait = match (previous_definition, definition) {
                    (Some(previous), Some(current)) => previous.id == current.id,
                    (None, None) => trait_application_path(previous_trait)
                        == trait_application_path(trait_application),
                    _ => false,
                };
                let same_application = same_trait
                    && previous_trait.identity == trait_application.identity;
                (same_target && same_application).then(|| previous_origin.clone())
            })
            .chain(std::iter::once(origin.clone()))
            .collect();
        if duplicate_origins.len() > 1 {
            let path = definition.map_or_else(
                || trait_application_path(trait_application).unwrap_or("<unknown trait path>"),
                |definition| definition.path,
            );
            let name = definition.map_or(path, |definition| definition.name);
            return Err(
                ContextError::from_issues(vec![ContextIssue::DuplicateTraitImpl {
                    target: TypeIdentity {
                        type_id: target.type_id(),
                        rust_name: target.rust_name(),
                    },
                    trait_: item_identity(
                        ItemKind::TraitDefinition,
                        path,
                        name,
                    ),
                    registrations: duplicate_origins.into_boxed_slice(),
                }])
                .expect("one context issue is nonempty"),
            );
        }
    }
    Ok(())
}

fn resolve_trait_application_definition(
    application: &TraitApplicationMetadata,
    definitions: &HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    definitions_by_path: &HashMap<&'static str, TraitDefinitionId>,
) -> Result<Option<&'static TraitDefinitionMetadata>, ContextError> {
    let definition = application
        .expression
        .and_then(|expression| definitions_by_path.get(expression.path).copied())
        .or_else(|| application.definition.map(|definition| definition.id))
        .and_then(|id| definitions.get(&id).copied());
    if definition.is_none() && application.definition.is_some() {
        let definition = application.definition.expect("checked above");
        return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
            item: item_identity(ItemKind::TraitApplication, definition.path, "trait application"),
            dependency: item_identity(ItemKind::TraitDefinition, definition.path, definition.name),
        }])
        .expect("one context issue is nonempty"));
    }
    Ok(definition)
}

fn trait_application_path(application: &TraitApplicationMetadata) -> Option<&'static str> {
    application
        .expression
        .map(|expression| expression.path)
        .or_else(|| application.definition.map(|definition| definition.path))
}

fn aggregate_method_catalogs(
    included_types: &std::collections::HashSet<TypeId>,
    impl_applications: &HashMap<ImplDefinitionId, Vec<&'static ImplApplicationMetadata>>,
    catalog_registrations: &[&MethodCatalogRegistration],
    definitions: &HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    definitions_by_path: &HashMap<&'static str, TraitDefinitionId>,
) -> Result<HashMap<TypeId, Arc<[ContextEffectiveMethod]>>, ContextError> {
    let mut applications_by_target =
        HashMap::<TypeId, Vec<&'static ImplApplicationMetadata>>::new();
    let mut entries_by_target = HashMap::<TypeId, Vec<MethodEntry>>::new();

    for application in impl_applications.values().flatten() {
        let target = (application.target)();
        let type_id = target.type_id();
        if !included_types.contains(&type_id) {
            continue;
        }
        applications_by_target
            .entry(type_id)
            .or_default()
            .push(application);
        let namespace = match application.implemented_trait {
            Some(trait_application) => {
                let definition = resolve_trait_application_definition(
                    trait_application,
                    definitions,
                    definitions_by_path,
                )?;
                match definition {
                    Some(definition) => MethodNamespaceKey::TraitApplication {
                        definition: definition.id,
                        identity: trait_application.identity,
                    },
                    None => MethodNamespaceKey::TraitPath(
                        trait_application_path(trait_application)
                            .expect("source trait application has a path"),
                    ),
                }
            }
            None => MethodNamespaceKey::Inherent,
        };
        for method in application.methods {
            if method.target_type_id() != type_id {
                return Err(invalid_method_metadata(
                    *method,
                    "impl method target mismatch",
                ));
            }
            let namespace_matches = match (namespace, method.metadata.namespace) {
                (MethodNamespaceKey::Inherent, MethodNamespaceMetadata::Inherent { .. }) => true,
                (
                    MethodNamespaceKey::TraitApplication { .. } | MethodNamespaceKey::TraitPath(_),
                    MethodNamespaceMetadata::Trait { trait_, .. },
                ) => application
                    .implemented_trait
                    .and_then(trait_application_path)
                    .is_some_and(|path| path == trait_),
                _ => false,
            };
            if !namespace_matches {
                return Err(invalid_method_metadata(
                    *method,
                    "impl method namespace does not match its implementation",
                ));
            }
            push_method_entry(
                &mut entries_by_target,
                type_id,
                MethodEntry {
                    namespace,
                    method: *method,
                },
            )?;
        }
    }

    for registration in catalog_registrations {
        let type_id = registration.type_id();
        if !included_types.contains(&type_id) {
            continue;
        }
        for method in registration.methods() {
            if method.target_type_id() != type_id {
                return Err(invalid_method_metadata(
                    *method,
                    "catalog method target mismatch",
                ));
            }
            let namespace = match method.metadata.namespace {
                MethodNamespaceMetadata::Inherent { .. } => MethodNamespaceKey::Inherent,
                MethodNamespaceMetadata::Trait { trait_, .. } => {
                    let applications = applications_by_target
                        .get(&type_id)
                        .into_iter()
                        .flatten()
                        .filter_map(|application| {
                            let trait_application = application.implemented_trait?;
                            (trait_application_path(trait_application) == Some(trait_))
                                .then_some((application, trait_application))
                        })
                        .collect::<Vec<_>>();
                    let exact_matches = applications
                        .iter()
                        .filter(|(application, _)| {
                            application
                                .methods
                                .iter()
                                .any(|candidate| candidate.same_registration(method))
                        })
                        .copied()
                        .collect::<Vec<_>>();
                    let matches = if exact_matches.is_empty() {
                        applications
                    } else {
                        exact_matches
                    };
                    match matches.as_slice() {
                        [] => MethodNamespaceKey::TraitPath(trait_),
                        [(_, trait_application)] => match resolve_trait_application_definition(
                                trait_application,
                                definitions,
                                definitions_by_path,
                            )? {
                                Some(definition) => MethodNamespaceKey::TraitApplication {
                                    definition: definition.id,
                                    identity: trait_application.identity,
                                },
                                None => MethodNamespaceKey::TraitPath(
                                    trait_application_path(trait_application)
                                        .expect("source trait application has a path"),
                                ),
                            },
                        _ => {
                            return Err(invalid_method_metadata(
                                *method,
                                "catalog trait method matches multiple generic trait applications",
                            ));
                        }
                    }
                }
            };
            push_method_entry(
                &mut entries_by_target,
                type_id,
                MethodEntry {
                    namespace,
                    method: *method,
                },
            )?;
        }
    }

    entries_by_target
        .into_iter()
        .map(|(type_id, entries)| {
            let methods = entries
                .into_iter()
                .map(|entry| {
                    ContextEffectiveMethod::new(entry.method)
                        .map_err(|reason| invalid_method_metadata(entry.method, reason))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok((type_id, Arc::from(methods.into_boxed_slice())))
        })
        .collect()
}

fn push_method_entry(
    entries_by_target: &mut HashMap<TypeId, Vec<MethodEntry>>,
    type_id: TypeId,
    entry: MethodEntry,
) -> Result<(), ContextError> {
    let entries = entries_by_target.entry(type_id).or_default();
    if let Some(previous) = entries.iter().find(|previous| {
        previous.namespace == entry.namespace
            && previous.method.metadata.name == entry.method.metadata.name
    }) {
        if previous.method.same_registration(&entry.method) {
            return Ok(());
        }
        let target = (entry.method.target)();
        return Err(
            ContextError::from_issues(vec![ContextIssue::ConflictingMethod {
                target: TypeIdentity {
                    type_id,
                    rust_name: target.rust_name(),
                },
                namespace: method_namespace_description(entry.namespace),
                method: entry.method.metadata.name,
            }])
            .expect("one context issue is nonempty"),
        );
    }
    entries.push(entry);
    Ok(())
}

fn method_namespace_description(namespace: MethodNamespaceKey) -> Box<str> {
    match namespace {
        MethodNamespaceKey::Inherent => "inherent".into(),
        MethodNamespaceKey::TraitPath(path) => format!("trait {path}").into(),
        MethodNamespaceKey::TraitApplication {
            definition,
            identity,
        } => {
            let arguments = identity
                .iter()
                .map(|identity| match identity {
                    TraitArgumentIdentity::Type(type_id) => format!("{:?}", type_id()),
                    TraitArgumentIdentity::Const(value) => format!("{value:?}"),
                    TraitArgumentIdentity::Lifetime(value)
                    | TraitArgumentIdentity::Expression(value) => (*value).to_owned(),
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("trait {definition:?}<{arguments}>").into()
        }
    }
}

fn invalid_method_metadata(method: EffectiveMethod, reason: &'static str) -> ContextError {
    let target = (method.target)();
    ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
        item: item_identity(ItemKind::Method, target.rust_name(), method.metadata.name),
        reason: MetadataValidationError::new(reason),
    }])
    .expect("one context issue is nonempty")
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::runtime::context::ReflectionContext;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::impl_descriptor::{ImplKind, ImplQualifiers};
    use crate::runtime::descriptor::metadata::TypeMetadata;
    use crate::runtime::descriptor::method::MethodCatalog;
    use crate::runtime::descriptor::method::{
        InvocationAdapters, MethodCatalogRegistration, MethodMetadata, MethodNamespaceMetadata,
        MethodQualifiers, MethodVisibility, ReturnKind, ReturnMetadata,
    };
    use crate::runtime::descriptor::trait_descriptor::{
        TraitApplicationMetadata, TraitCompleteness, TraitDefinitionMetadata, Visibility,
    };
    use crate::runtime::expression::{ExpressionData, TraitExpressionData};
    use crate::runtime::invoke::{
        FunctionInvocationAdapter, Invocation, InvocationOutput, Local, ValidatedInvocation,
    };
    use std::sync::OnceLock;

    struct Target;
    struct ImplOne;
    struct ImplTwo;
    struct TraitMarker;
    struct ExternalTraitMarker;

    impl crate::runtime::descriptor::Reflect for Target {
        fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
            static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Target> =
                crate::runtime::descriptor::TypeRegistration::new();
            &REGISTRATION
        }
    }

    static TARGET_METADATA: TypeMetadata = TypeMetadata::new::<Target>();
    fn target_metadata() -> &'static TypeMetadata {
        &TARGET_METADATA
    }

    #[test]
    fn resolves_unlinked_trait_application_by_exact_registered_path() {
        let registration = Registration::metadata_only()
            .with_trait_definition(&EXTERNAL_TRAIT_DEFINITION)
            .with_trait_application(&EXTERNAL_TRAIT_APPLICATION);
        let context = ContextData::build(vec![registration])
            .expect("an unresolved application links to the exact trait handle path");

        let applications = context.trait_applications(TraitDefinitionId::of::<ExternalTraitMarker>());
        assert_eq!(applications.len(), 1);
        assert!(std::ptr::eq(applications[0], &EXTERNAL_TRAIT_APPLICATION));
    }

    static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &[],
        predicates: &[],
    };
    static EXTERNAL_TRAIT_DEFINITION: TraitDefinitionMetadata =
        TraitDefinitionMetadata::new::<ExternalTraitMarker>(
            "ExternalTrait",
            "example :: ExternalTrait",
            Visibility::Public,
            TraitCompleteness::ExternalIncomplete,
            &EMPTY_GENERICS,
            &[],
            &[],
            &[],
            &[],
        );
    static EXTERNAL_TRAIT_EXPRESSION: TraitExpressionData =
        TraitExpressionData::new("example :: ExternalTrait", &[], &[]);
    static EXTERNAL_TRAIT_APPLICATION: TraitApplicationMetadata =
        TraitApplicationMetadata::unresolved(&EXTERNAL_TRAIT_EXPRESSION, &[], &[]);
    static SELF_EXPRESSION: ExpressionData = ExpressionData::SelfType;
    static EMPTY_RETURN: ReturnMetadata = ReturnMetadata {
        kind: ReturnKind::Unit,
        expression: None,
        concrete_type: None,
    };

    const fn inherent_method(name: &'static str, implementation: &'static str) -> MethodMetadata {
        MethodMetadata {
            name,
            namespace: MethodNamespaceMetadata::Inherent { implementation },
            visibility: MethodVisibility::Public,
            receiver: None,
            parameters: &[],
            output: &EMPTY_RETURN,
            qualifiers: MethodQualifiers {
                is_const: false,
                is_async: false,
                is_unsafe: false,
                abi: None,
                is_variadic: false,
            },
            generics: &EMPTY_GENERICS,
            is_generic: false,
            default_body: false,
        }
    }

    const fn trait_method(name: &'static str, trait_: &'static str) -> MethodMetadata {
        MethodMetadata {
            name,
            namespace: MethodNamespaceMetadata::Trait {
                trait_,
                implementation: "fixture::Target",
            },
            ..inherent_method(name, "fixture::Target")
        }
    }

    static RESET_METADATA: MethodMetadata = inherent_method("reset", "fixture::ImplOne");
    static CLEAR_METADATA: MethodMetadata = inherent_method("clear", "fixture::ImplTwo");
    static CONFLICTING_RESET_METADATA: MethodMetadata =
        inherent_method("reset", "fixture::ImplTwo");
    static RESET_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&RESET_METADATA);
    static CLEAR_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&CLEAR_METADATA);
    static CONFLICTING_RESET_METHOD: EffectiveMethod =
        EffectiveMethod::for_type::<Target>(&CONFLICTING_RESET_METADATA);
    static IMPL_ONE_METHODS: &[EffectiveMethod] = &[RESET_METHOD];
    static IMPL_TWO_METHODS: &[EffectiveMethod] = &[CLEAR_METHOD];
    static CONFLICTING_METHODS: &[EffectiveMethod] = &[CONFLICTING_RESET_METHOD];
    static CATALOG_WITH_RESET: &[EffectiveMethod] = &[RESET_METHOD];
    static TRAIT_A_METADATA: MethodMetadata = trait_method("same", "fixture::TraitA");
    static TRAIT_B_METADATA: MethodMetadata = trait_method("same", "fixture::TraitB");
    static TRAIT_A_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&TRAIT_A_METADATA);
    static TRAIT_B_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&TRAIT_B_METADATA);
    static TRAIT_METHODS: &[EffectiveMethod] = &[TRAIT_A_METHOD, TRAIT_B_METHOD];
    static AMBIGUOUS_TRAIT_METHOD_METADATA: MethodMetadata =
        trait_method("catalog_only", "fixture::GenericTrait");
    static AMBIGUOUS_TRAIT_METHOD: EffectiveMethod =
        EffectiveMethod::for_type::<Target>(&AMBIGUOUS_TRAIT_METHOD_METADATA);
    static AMBIGUOUS_TRAIT_METHODS: &[EffectiveMethod] = &[AMBIGUOUS_TRAIT_METHOD];

    fn provider_adapter<'call>(
        _invocation: ValidatedInvocation<'call, Local>,
    ) -> InvocationOutput<'call, Local> {
        InvocationOutput::unit()
    }

    static PROVIDER_ADAPTER: FunctionInvocationAdapter<Local> =
        FunctionInvocationAdapter::new(provider_adapter);
    static PROVIDER_ADAPTERS: InvocationAdapters = InvocationAdapters::builder()
        .local(&PROVIDER_ADAPTER)
        .build();
    static PROVIDER_METADATA: MethodMetadata =
        MethodMetadata::inherent("provided", "fixture::Target");
    static PROVIDER_METHOD: EffectiveMethod =
        EffectiveMethod::for_type::<Target>(&PROVIDER_METADATA).with_adapters(PROVIDER_ADAPTERS);
    static PROVIDED_METHODS: OnceLock<Box<[EffectiveMethod]>> = OnceLock::new();

    fn provided_methods() -> &'static [EffectiveMethod] {
        PROVIDED_METHODS.get_or_init(|| vec![PROVIDER_METHOD].into_boxed_slice())
    }

    static PROVIDER_CATALOG: MethodCatalogRegistration =
        MethodCatalogRegistration::with_provider::<Target>(provided_methods);

    static IMPL_ONE_DEFINITION: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<ImplOne>(
        ImplKind::Inherent,
        &SELF_EXPRESSION,
        None,
        &EMPTY_GENERICS,
        ImplQualifiers::new(false),
        &[],
        &[],
        &[],
    );
    static IMPL_TWO_DEFINITION: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<ImplTwo>(
        ImplKind::Inherent,
        &SELF_EXPRESSION,
        None,
        &EMPTY_GENERICS,
        ImplQualifiers::new(false),
        &[],
        &[],
        &[],
    );
    static IMPL_ONE: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_ONE_DEFINITION,
        None,
        &[],
        &[],
        IMPL_ONE_METHODS,
        &[],
        &[],
    );
    static IMPL_TWO: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_TWO_DEFINITION,
        None,
        &[],
        &[],
        IMPL_TWO_METHODS,
        &[],
        &[],
    );
    static CONFLICTING_IMPL: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_TWO_DEFINITION,
        None,
        &[],
        &[],
        CONFLICTING_METHODS,
        &[],
        &[],
    );

    static GENERIC_TRAIT: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<TraitMarker>(
        "GenericTrait",
        "fixture::GenericTrait",
        Visibility::Public,
        TraitCompleteness::Complete,
        &EMPTY_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static GENERIC_TRAIT_A: TraitApplicationMetadata = TraitApplicationMetadata::new(
        &GENERIC_TRAIT,
        &[],
        &[TraitArgumentIdentity::expression("A")],
        &[],
        &[],
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
    );
    static GENERIC_TRAIT_B: TraitApplicationMetadata = TraitApplicationMetadata::new(
        &GENERIC_TRAIT,
        &[],
        &[TraitArgumentIdentity::expression("B")],
        &[],
        &[],
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
    );
    static GENERIC_IMPL_ONE: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_ONE_DEFINITION,
        Some(&GENERIC_TRAIT_A),
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    static GENERIC_IMPL_TWO: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_TWO_DEFINITION,
        Some(&GENERIC_TRAIT_B),
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    static DUPLICATE_GENERIC_IMPL: ImplApplicationMetadata =
        ImplApplicationMetadata::for_target::<Target>(
            &IMPL_TWO_DEFINITION,
            Some(&GENERIC_TRAIT_A),
            &[],
            &[],
            &[],
            &[],
            &[],
        );

    fn included_types() -> std::collections::HashSet<TypeId> {
        [TypeId::of::<Target>()].into_iter().collect()
    }

    fn application_map(
        applications: &[&'static ImplApplicationMetadata],
    ) -> HashMap<ImplDefinitionId, Vec<&'static ImplApplicationMetadata>> {
        applications
            .iter()
            .fold(HashMap::new(), |mut grouped, application| {
                grouped
                    .entry(application.definition.id)
                    .or_default()
                    .push(application);
                grouped
            })
    }

    fn catalog(methods: &'static [EffectiveMethod]) -> MethodCatalogRegistration {
        MethodCatalogRegistration::new::<Target>(methods)
    }

    fn origin(module_path: &'static str) -> RegistrationOrigin {
        RegistrationOrigin {
            crate_name: "fixture",
            crate_version: Some("1.0.0"),
            module_path,
            file: None,
            line: None,
            column: None,
        }
    }

    #[test]
    fn aggregates_inherent_methods_from_separate_impl_applications() {
        let applications = application_map(&[&IMPL_ONE, &IMPL_TWO]);
        let catalog = catalog(CATALOG_WITH_RESET);
        let methods = aggregate_method_catalogs(&included_types(), &applications, &[&catalog], &HashMap::new(), &HashMap::new())
            .expect("distinct inherent methods aggregate");
        let methods = methods
            .get(&TypeId::of::<Target>())
            .expect("target catalog");
        assert_eq!(methods.len(), 2);
        assert_eq!(
            methods
                .iter()
                .filter(|method| method.effective.metadata.name == "reset")
                .count(),
            1
        );
        assert!(methods
            .iter()
            .any(|method| method.effective.metadata.name == "clear"));
    }

    #[test]
    fn provider_catalog_is_frozen_and_its_adapter_remains_invokable() {
        let catalogs = [&PROVIDER_CATALOG];
        let methods = aggregate_method_catalogs(&included_types(), &HashMap::new(), &catalogs, &HashMap::new(), &HashMap::new())
            .expect("provider catalog is valid");
        let target_methods = methods
            .get(&TypeId::of::<Target>())
            .expect("provider target catalog");
        assert_eq!(target_methods.len(), 1);
        assert!(target_methods[0].effective.adapters.is_some());

        let context = ReflectionContext::from_data(ContextData::empty());
        let descriptor =
            crate::runtime::descriptor::TypeDescriptor::new(&TARGET_METADATA, &context);
        let catalog =
            MethodCatalog::from_shared(descriptor, methods[&TypeId::of::<Target>()].clone());
        let method = catalog.method("provided").expect("provider method lookup");
        let output = match method.invoke(Invocation::<Local>::associated()) {
            Ok(output) => output,
            Err(_) => panic!("provider adapter invocation"),
        };
        assert!(output.into_unit().is_ok());
    }

    #[test]
    fn allows_same_method_name_in_distinct_trait_namespaces() {
        let catalog = catalog(TRAIT_METHODS);
        let methods = aggregate_method_catalogs(&included_types(), &HashMap::new(), &[&catalog], &HashMap::new(), &HashMap::new())
            .expect("trait namespaces are distinct");
        assert_eq!(methods[&TypeId::of::<Target>()].len(), 2);
    }

    #[test]
    fn rejects_duplicate_method_name_in_one_inherent_namespace() {
        let applications = application_map(&[&IMPL_ONE, &CONFLICTING_IMPL]);
        let error = match aggregate_method_catalogs(&included_types(), &applications, &[], &HashMap::new(), &HashMap::new()) {
            Ok(_) => panic!("duplicate inherent names reject the context"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::ConflictingMethod {
                namespace,
                method: "reset",
                ..
            } if namespace.as_ref() == "inherent"
        ));
    }

    #[test]
    fn rejects_catalog_method_with_ambiguous_generic_trait_path() {
        let applications = application_map(&[&GENERIC_IMPL_ONE, &GENERIC_IMPL_TWO]);
        let catalog = catalog(AMBIGUOUS_TRAIT_METHODS);
        let error = match aggregate_method_catalogs(&included_types(), &applications, &[&catalog], &HashMap::new(), &HashMap::new()) {
            Ok(_) => panic!("one trait path cannot select between generic applications"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::InvalidMetadata { reason, .. }
                if reason.to_string().contains("multiple generic trait applications")
        ));
    }

    #[test]
    fn allows_distinct_generic_trait_applications_but_rejects_duplicate_application() {
        let distinct = [
            (&GENERIC_IMPL_ONE, origin("impl_a")),
            (&GENERIC_IMPL_TWO, origin("impl_b")),
        ];
        validate_trait_impl_uniqueness(&distinct, &HashMap::new(), &HashMap::new())
            .expect("distinct trait arguments are distinct impls");

        let duplicate = [
            (&GENERIC_IMPL_ONE, origin("impl_a")),
            (&DUPLICATE_GENERIC_IMPL, origin("impl_b")),
        ];
        let error = match validate_trait_impl_uniqueness(&duplicate, &HashMap::new(), &HashMap::new()) {
            Ok(()) => panic!("duplicate trait application rejects the context"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::DuplicateTraitImpl { registrations, .. } if registrations.len() == 2
        ));
    }
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
