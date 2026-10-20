// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use super::indexes::ContextIndexes;
use super::registration::Registration;
use super::registration::TraitObjectRegistration;
use super::registration::TypeRecord;
use crate::runtime::capability::capabilities::Capabilities;
use crate::runtime::capability::capability_error::CapabilityError;
use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::capability::capability_registration_origin::CapabilityRegistrationOrigin;
use crate::runtime::capability::capability_target_id::CapabilityTargetId;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::descriptor::collections::TypeShapeRegistration;
use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::impl_descriptor::ImplApplicationMetadata;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitionId;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitionMetadata;
use crate::runtime::descriptor::method::ContextEffectiveMethod;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::method::MethodCatalogRegistration;
use crate::runtime::descriptor::method::MethodNamespaceMetadata;
use crate::runtime::descriptor::method::ResolvedMethodSpecialization;
use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionId;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::error::AdapterValidationError;
use crate::runtime::error::ContextError;
use crate::runtime::error::ContextIssue;
use crate::runtime::error::MetadataValidationError;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::RegistrationOrigin;
use crate::runtime::identity::StableItemId;
use crate::runtime::identity::TypeIdentity;

pub(crate) struct ContextData {
    pub(crate) types: Box<[TypeRecord]>,
    pub(crate) indexes: ContextIndexes,
    shapes: HashMap<TypeId, &'static TypeKindMetadata>,
    methods: HashMap<TypeId, Arc<[ContextEffectiveMethod]>>,
    type_definitions: HashMap<TypeDefinitionId, &'static TypeDefinitionMetadata>,
    trait_definitions: HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    registered_impl_definitions: Box<[&'static ImplDefinitionMetadata]>,
    capabilities: HashMap<CapabilityTargetId, Arc<Capabilities>>,
    type_instances: HashMap<TypeDefinitionId, Box<[TypeId]>>,
    all_trait_applications: Box<[&'static TraitApplicationMetadata]>,
    trait_applications: HashMap<TraitDefinitionId, Box<[&'static TraitApplicationMetadata]>>,
    trait_applications_by_path: HashMap<&'static str, Box<[&'static TraitApplicationMetadata]>>,
    impl_applications: HashMap<ImplDefinitionId, Box<[&'static ImplApplicationMetadata]>>,
    impl_applications_by_target: HashMap<TypeId, Box<[&'static ImplApplicationMetadata]>>,
    construction_plans: HashMap<TypeId, Box<[&'static ConstructionPlan]>>,
    trait_objects: HashMap<TypeId, TraitObjectRegistration>,
}

impl ContextData {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            types: Box::new([]),
            indexes: ContextIndexes::default(),
            shapes: HashMap::new(),
            methods: HashMap::new(),
            type_definitions: HashMap::new(),
            trait_definitions: HashMap::new(),
            registered_impl_definitions: Box::new([]),
            capabilities: HashMap::new(),
            type_instances: HashMap::new(),
            all_trait_applications: Box::new([]),
            trait_applications: HashMap::new(),
            trait_applications_by_path: HashMap::new(),
            impl_applications: HashMap::new(),
            impl_applications_by_target: HashMap::new(),
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
        let mut registered_impl_definitions = Vec::new();
        let mut capability_registrations = Vec::<CapabilityRegistration>::new();
        let mut method_bindings = Vec::<EffectiveMethod>::new();
        let mut method_binding_targets = Vec::<TypeRecord>::new();
        let mut type_instances = HashMap::<TypeDefinitionId, Vec<TypeId>>::new();
        let mut trait_applications_pending = Vec::<&'static TraitApplicationMetadata>::new();
        let mut trait_applications = HashMap::<TraitDefinitionId, Vec<&'static TraitApplicationMetadata>>::new();
        let mut trait_applications_by_path = HashMap::<&'static str, Vec<&'static TraitApplicationMetadata>>::new();
        let mut impl_applications = HashMap::<ImplDefinitionId, Vec<&'static ImplApplicationMetadata>>::new();
        let mut impl_application_origins = Vec::<(&'static ImplApplicationMetadata, RegistrationOrigin)>::new();
        let mut registered_shapes = HashMap::<TypeId, &'static TypeKindMetadata>::new();
        let mut construction_plans = HashMap::<TypeId, Vec<&'static ConstructionPlan>>::new();
        let mut trait_objects = HashMap::<TypeId, TraitObjectRegistration>::new();
        let mut type_definition_origins = HashMap::<TypeDefinitionId, Vec<RegistrationOrigin>>::new();
        let mut trait_definition_origins = HashMap::<TraitDefinitionId, Vec<RegistrationOrigin>>::new();
        let mut trait_definition_ids_by_path = HashMap::<&'static str, TraitDefinitionId>::new();
        let mut impl_definition_origins = HashMap::<ImplDefinitionId, Vec<RegistrationOrigin>>::new();
        for mut registration in registrations {
            method_bindings.extend_from_slice(registration.method_bindings());
            method_binding_targets.extend_from_slice(registration.method_binding_targets());
            for trait_object in registration.trait_objects() {
                let type_id = trait_object.type_id();
                if let Some(previous) = trait_objects.insert(type_id, *trait_object)
                    && previous.trait_path() != trait_object.trait_path()
                {
                    return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                        item: item_identity(ItemKind::Type, trait_object.rust_name(), "trait object"),
                        reason: MetadataValidationError::new(
                            "trait object type has conflicting registered trait paths",
                        ),
                    }])
                    .expect("one context issue is nonempty"));
                }
            }
            for metadata in registration.type_definitions() {
                let origins = type_definition_origins.entry(metadata.id).or_default();
                origins.push(registration.origin().clone());
                if type_definitions.insert(metadata.id, *metadata).is_some() {
                    return Err(ContextError::from_issues(vec![ContextIssue::ConflictingTypeDefinition {
                        identity: item_identity(ItemKind::TypeDefinition, metadata.path, metadata.name),
                        registrations: origins.clone().into_boxed_slice(),
                    }])
                    .expect("one context issue is nonempty"));
                }
            }
            for metadata in registration.trait_definitions() {
                // Source paths are lookup candidates and display metadata. Multiple
                // typed trait handles may legitimately retain the same spelling.
                trait_definition_ids_by_path.insert(metadata.path, metadata.id);
                let origins = trait_definition_origins.entry(metadata.id).or_default();
                origins.push(registration.origin().clone());
                if trait_definitions.insert(metadata.id, *metadata).is_some() {
                    return Err(
                        ContextError::from_issues(vec![ContextIssue::ConflictingTraitDefinition {
                            identity: item_identity(ItemKind::TraitDefinition, metadata.path, metadata.name),
                            registrations: origins.clone().into_boxed_slice(),
                        }])
                        .expect("one context issue is nonempty"),
                    );
                }
            }
            for metadata in registration.impl_definitions() {
                let origins = impl_definition_origins.entry(metadata.id).or_default();
                origins.push(registration.origin().clone());
                if impl_definitions.insert(metadata.id, *metadata).is_some() {
                    return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                        item: item_identity(ItemKind::Impl, format!("{:?}", metadata.id).as_str(), "impl definition"),
                        reason: MetadataValidationError::new("duplicate impl definition identity"),
                    }])
                    .expect("one context issue is nonempty"));
                }
                registered_impl_definitions.push(*metadata);
            }
            capability_registrations.extend(registration.take_capabilities());
            for application in registration.trait_applications() {
                trait_applications_pending.push(*application);
            }
            for application in registration.impl_applications() {
                let applications = impl_applications.entry(application.definition.id).or_default();
                if !applications
                    .iter()
                    .any(|candidate| std::ptr::eq(*candidate, *application))
                {
                    applications.push(*application);
                    impl_application_origins.push((*application, registration.origin().clone()));
                }
            }
            let Some(type_record) = registration.type_record() else {
                continue;
            };
            let type_id = type_record.type_id();
            if let Some(shape) = registration.shape()
                && registered_shapes.insert(type_id, shape).is_some()
            {
                return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                    item: type_identity(type_id),
                    reason: MetadataValidationError::new("multiple type shape registrations"),
                }])
                .expect("one context issue is nonempty"));
            }
            if let Some(definition) = registration.type_instance_of() {
                type_instances.entry(definition).or_default().push(type_id);
            }
            for plan in registration.construction_plans() {
                if plan.target.identity().type_id != type_id {
                    let item = type_identity(type_id);
                    return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                        item,
                        reason: MetadataValidationError::new(
                            "construction plan target does not match its registration",
                        ),
                    }])
                    .expect("one context issue is nonempty"));
                }
                construction_plans
                    .entry(plan.target.identity().type_id)
                    .or_default()
                    .push(*plan);
            }
            let index = *registration_group_by_type.entry(type_id).or_insert_with(|| {
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

        let mut included_type_ids: std::collections::HashSet<_> = types.iter().map(|record| record.type_id()).collect();
        for target in method_binding_targets {
            if included_type_ids.insert(target.type_id()) {
                types.push(target);
            }
        }

        let mut all_trait_applications =
            Vec::<&'static TraitApplicationMetadata>::with_capacity(trait_applications_pending.len());
        for application in trait_applications_pending {
            if all_trait_applications
                .iter()
                .any(|registered| std::ptr::eq::<TraitApplicationMetadata>(*registered, application))
            {
                continue;
            }
            let definition =
                resolve_trait_application_definition(application, &trait_definitions, &trait_definition_ids_by_path)?;
            let path = trait_application_path(application).or_else(|| definition.map(|value| value.path));
            if let Some(path) = path {
                trait_applications_by_path.entry(path).or_default().push(application);
            }
            all_trait_applications.push(application);
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
        let indexes = ContextIndexes::build(
            &types,
            type_definitions.values().copied(),
            trait_definitions.values().copied(),
        );
        let registered_type_ids: std::collections::HashSet<_> = types.iter().map(|record| record.type_id()).collect();
        let mut included_types = registered_type_ids.clone();
        let method_catalog_registrations: Vec<_> = inventory::iter::<MethodCatalogRegistration>().collect();
        included_types.extend(
            method_catalog_registrations
                .iter()
                .map(|registration| registration.type_id()),
        );
        for (definition, instances) in &type_instances {
            if !type_definitions.contains_key(definition) {
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
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
                .expect("one context issue is nonempty"));
            }
            if let Some(missing) = instances.iter().find(|instance| !included_types.contains(instance)) {
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                    item: item_identity(ItemKind::TypeDefinition, "generic application", "generic application"),
                    dependency: type_identity(*missing),
                }])
                .expect("one context issue is nonempty"));
            }
        }
        for (definition, applications) in &trait_applications {
            if !trait_definitions.contains_key(definition) {
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
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
                .expect("one context issue is nonempty"));
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
                    if let Some(supertrait_definition) = supertrait_definition
                        && !trait_definitions.contains_key(&supertrait_definition.id)
                    {
                        return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
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
                        }])
                        .expect("one context issue is nonempty"));
                    }
                }
            }
        }
        for (definition, applications) in &impl_applications {
            if !impl_definitions.contains_key(definition) {
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                    item: item_identity(ItemKind::Impl, format!("{definition:?}").as_str(), "impl application"),
                    dependency: item_identity(ItemKind::Impl, format!("{definition:?}").as_str(), "impl definition"),
                }])
                .expect("one context issue is nonempty"));
            }
            if let Some(application) = applications
                .iter()
                .find(|application| !registered_type_ids.contains(&(application.target)().type_id()))
            {
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                    item: item_identity(
                        ItemKind::Impl,
                        format!("{:?}", application.definition.id).as_str(),
                        "impl application",
                    ),
                    dependency: type_identity((application.target)().type_id()),
                }])
                .expect("one context issue is nonempty"));
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
                let trait_path = trait_application
                    .expression
                    .map_or("<unresolved trait>", |expr| expr.path);
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                    item: item_identity(
                        ItemKind::Impl,
                        format!("{:?}", application.definition.id).as_str(),
                        "impl application",
                    ),
                    dependency: item_identity(ItemKind::TraitDefinition, trait_path, trait_path),
                }])
                .expect("one context issue is nonempty"));
            }
        }
        for target in construction_plans.keys() {
            if !included_types.contains(target) {
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                    item: item_identity(ItemKind::Impl, "construction plan", "construction plan"),
                    dependency: type_identity(*target),
                }])
                .expect("one context issue is nonempty"));
            }
        }
        let mut shapes = registered_shapes;
        for registration in inventory::iter::<TypeShapeRegistration> {
            let type_id = registration.type_id();
            if !included_types.contains(&type_id) {
                continue;
            }
            if shapes.insert(type_id, registration.metadata()).is_some() {
                return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                    item: type_identity(type_id),
                    reason: MetadataValidationError::new("multiple type shape registrations"),
                }])
                .expect("one context issue is nonempty"));
            }
        }
        validate_trait_impl_uniqueness(
            &impl_application_origins,
            &trait_definitions,
            &trait_definition_ids_by_path,
        )?;
        let methods = aggregate_method_catalogs(
            &included_types,
            &impl_applications,
            &method_catalog_registrations,
            &method_bindings,
            &trait_definitions,
            &trait_definition_ids_by_path,
        )?;
        let mut grouped_capabilities = HashMap::<CapabilityTargetId, Vec<CapabilityRegistration>>::new();
        for registration in capability_registrations {
            if let CapabilityTargetId::Type(identity) = &registration.target {
                if !registered_type_ids.contains(&identity.type_id) {
                    return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                        item: item_identity(ItemKind::Capability, "capability", "capability"),
                        dependency: ItemIdentity {
                            kind: ItemKind::Type,
                            stable_id: StableItemId(identity.rust_name.into()),
                            display_name: identity.rust_name.into(),
                        },
                    }])
                    .expect("one context issue is nonempty"));
                }
            } else if let CapabilityTargetId::Definition(identity) = &registration.target
                && !type_definitions.contains_key(identity)
            {
                let diagnostic = format!("{identity:?}");
                return Err(ContextError::from_issues(vec![ContextIssue::MissingDependency {
                    item: item_identity(ItemKind::Capability, "capability", "capability"),
                    dependency: item_identity(ItemKind::TypeDefinition, &diagnostic, &diagnostic),
                }])
                .expect("one context issue is nonempty"));
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
        let impl_applications: HashMap<ImplDefinitionId, Box<[&'static ImplApplicationMetadata]>> = impl_applications
            .into_iter()
            .map(|(definition, applications)| (definition, applications.into_boxed_slice()))
            .collect();
        let mut impl_applications_by_target = HashMap::<TypeId, Vec<&'static ImplApplicationMetadata>>::new();
        for application in impl_applications.values().flatten() {
            impl_applications_by_target
                .entry((application.target)().type_id())
                .or_default()
                .push(*application);
        }
        let impl_applications_by_target = impl_applications_by_target
            .into_iter()
            .map(|(target, applications)| (target, applications.into_boxed_slice()))
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
            registered_impl_definitions: registered_impl_definitions.into_boxed_slice(),
            capabilities,
            type_instances,
            all_trait_applications: all_trait_applications.into_boxed_slice(),
            trait_applications,
            trait_applications_by_path,
            impl_applications,
            impl_applications_by_target,
            construction_plans,
            trait_objects,
        })
    }

    pub(crate) fn type_by_id(&self, type_id: TypeId) -> Option<TypeRecord> {
        self.indexes.types_by_id.get(&type_id).map(|index| self.types[*index])
    }

    pub(crate) fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.indexes.types_by_id.contains_key(&type_id)
    }

    pub(crate) fn shape(&self, type_id: TypeId) -> Option<&'static TypeKindMetadata> {
        self.shapes.get(&type_id).copied()
    }

    pub(crate) fn methods(&self, type_id: TypeId) -> Arc<[ContextEffectiveMethod]> {
        self.methods.get(&type_id).cloned().unwrap_or_else(|| Arc::from([]))
    }

    pub(crate) fn type_definition(&self, id: TypeDefinitionId) -> Option<&'static TypeDefinitionMetadata> {
        self.type_definitions.get(&id).copied()
    }

    pub(crate) fn type_definitions(&self) -> Vec<&'static TypeDefinitionMetadata> {
        let mut definitions = self.type_definitions.values().copied().collect::<Vec<_>>();
        definitions.sort_by(|left, right| left.path.cmp(right.path));
        definitions
    }

    pub(crate) fn trait_definition(&self, id: TraitDefinitionId) -> Option<&'static TraitDefinitionMetadata> {
        self.trait_definitions.get(&id).copied()
    }

    pub(crate) fn trait_definitions(&self) -> Vec<&'static TraitDefinitionMetadata> {
        let mut definitions = self.trait_definitions.values().copied().collect::<Vec<_>>();
        definitions.sort_by(|left, right| left.path.cmp(right.path));
        definitions
    }

    pub(crate) fn impl_definitions(&self) -> &[&'static ImplDefinitionMetadata] {
        &self.registered_impl_definitions
    }

    pub(crate) fn trait_applications_all(&self) -> &[&'static TraitApplicationMetadata] {
        &self.all_trait_applications
    }

    pub(crate) fn trait_definition_by_path(&self, path: &str) -> Option<&'static TraitDefinitionMetadata> {
        let mut matches = self.trait_definitions.values().filter(|metadata| metadata.path == path);
        let definition = matches.next()?;
        matches.next().is_none().then_some(definition)
    }

    pub(crate) fn capabilities(&self, target: &CapabilityTargetId) -> Option<Arc<Capabilities>> {
        self.capabilities.get(target).cloned()
    }

    pub(crate) fn type_instances(&self, definition: TypeDefinitionId) -> &[TypeId] {
        self.type_instances.get(&definition).map(Box::as_ref).unwrap_or(&[])
    }

    pub(crate) fn trait_applications(&self, definition: TraitDefinitionId) -> &[&'static TraitApplicationMetadata] {
        self.trait_applications.get(&definition).map(Box::as_ref).unwrap_or(&[])
    }

    pub(crate) fn trait_applications_by_path(&self, path: &str) -> &[&'static TraitApplicationMetadata] {
        self.trait_applications_by_path
            .get(path)
            .map(Box::as_ref)
            .unwrap_or(&[])
    }

    pub(crate) fn impl_applications(&self, definition: ImplDefinitionId) -> &[&'static ImplApplicationMetadata] {
        self.impl_applications.get(&definition).map(Box::as_ref).unwrap_or(&[])
    }

    pub(crate) fn impl_applications_for_target(&self, target: TypeId) -> &[&'static ImplApplicationMetadata] {
        self.impl_applications_by_target
            .get(&target)
            .map(Box::as_ref)
            .unwrap_or(&[])
    }

    pub(crate) fn construction_plans(&self, target: TypeId) -> &[&'static ConstructionPlan] {
        self.construction_plans.get(&target).map(Box::as_ref).unwrap_or(&[])
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
    TraitPath {
        path: &'static str,
        identity: &'static [TraitArgumentIdentity],
        // Unlinked source paths are not proof of Rust trait identity. Keep
        // independently registered applications in separate method scopes.
        application: Option<usize>,
    },
}

struct MethodEntry {
    namespace: MethodNamespaceKey,
    method: EffectiveMethod,
    implementation: Option<&'static ImplApplicationMetadata>,
    specializations: Vec<ResolvedMethodSpecialization>,
    pending_specs: bool,
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
        let definition = resolve_trait_application_definition(trait_application, definitions, definitions_by_path)?;
        let duplicate_origins: Vec<_> = registrations[..index]
            .iter()
            .filter_map(|(previous, previous_origin)| {
                let previous_trait = previous.implemented_trait?;
                let same_target = (previous.target)().type_id() == target.type_id();
                let previous_definition =
                    resolve_trait_application_definition(previous_trait, definitions, definitions_by_path).ok()?;
                let same_trait = match (previous_definition, definition) {
                    (Some(previous), Some(current)) => previous.id == current.id,
                    // Source paths are display metadata, so unresolved
                    // applications cannot establish duplicate trait identity.
                    _ => false,
                };
                let same_application = same_trait && previous_trait.identity == trait_application.identity;
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
            return Err(ContextError::from_issues(vec![ContextIssue::DuplicateTraitImpl {
                target: TypeIdentity {
                    type_id: target.type_id(),
                    rust_name: target.rust_name(),
                },
                trait_: item_identity(ItemKind::TraitDefinition, path, name),
                registrations: duplicate_origins.into_boxed_slice(),
            }])
            .expect("one context issue is nonempty"));
        }
    }
    Ok(())
}

fn resolve_trait_application_definition(
    application: &TraitApplicationMetadata,
    definitions: &HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    _definitions_by_path: &HashMap<&'static str, TraitDefinitionId>,
) -> Result<Option<&'static TraitDefinitionMetadata>, ContextError> {
    // A source path is display metadata, not proof that two trait paths name
    // the same Rust trait. Only a typed marker link may resolve an application
    // to a definition; unresolved applications remain external/incomplete.
    let Some(link) = application.definition else {
        return Ok(None);
    };
    let definition = definitions.get(&link.id).copied().ok_or_else(|| {
        ContextError::from_issues(vec![ContextIssue::MissingDependency {
            item: item_identity(ItemKind::TraitApplication, link.path, "trait application"),
            dependency: item_identity(ItemKind::TraitDefinition, link.path, link.name),
        }])
        .expect("one context issue is nonempty")
    })?;
    Ok(Some(definition))
}

fn trait_application_path(application: &TraitApplicationMetadata) -> Option<&'static str> {
    application
        .expression
        .map(|expression| expression.path)
        .or_else(|| application.definition.map(|definition| definition.path))
}

fn trait_application_path_matches(application: &TraitApplicationMetadata, path: &str) -> bool {
    trait_application_path(application) == Some(path)
}

fn trait_application_matches_binding(
    application: &TraitApplicationMetadata,
    _path: &str,
    identity: &[crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
    definition: Option<&TraitDefinitionMetadata>,
) -> bool {
    if application.identity != identity {
        return false;
    }
    match definition {
        Some(expected) => application.definition.is_some_and(|actual| actual.id == expected.id),
        // Source spelling is useful for diagnostics and lookup, but cannot
        // establish the identity needed by a method binding. Unlinked
        // applications therefore cannot satisfy an application-specific
        // binding, even when their path and generic arguments happen to match.
        None => false,
    }
}

fn trait_application_namespace(
    application: &TraitApplicationMetadata,
    definitions: &HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    definitions_by_path: &HashMap<&'static str, TraitDefinitionId>,
) -> Result<MethodNamespaceKey, ContextError> {
    match resolve_trait_application_definition(application, definitions, definitions_by_path)? {
        Some(definition) => Ok(MethodNamespaceKey::TraitApplication {
            definition: definition.id,
            identity: application.identity,
        }),
        None => Ok(MethodNamespaceKey::TraitPath {
            path: trait_application_path(application).expect("source trait application has a path"),
            identity: application.identity,
            application: Some(std::ptr::from_ref(application) as usize),
        }),
    }
}

fn aggregate_method_catalogs(
    included_types: &std::collections::HashSet<TypeId>,
    impl_applications: &HashMap<ImplDefinitionId, Vec<&'static ImplApplicationMetadata>>,
    catalog_registrations: &[&MethodCatalogRegistration],
    method_bindings: &[EffectiveMethod],
    definitions: &HashMap<TraitDefinitionId, &'static TraitDefinitionMetadata>,
    definitions_by_path: &HashMap<&'static str, TraitDefinitionId>,
) -> Result<HashMap<TypeId, Arc<[ContextEffectiveMethod]>>, ContextError> {
    let mut applications_by_target = HashMap::<TypeId, Vec<&'static ImplApplicationMetadata>>::new();
    let mut entries_by_target = HashMap::<TypeId, Vec<MethodEntry>>::new();

    for application in impl_applications.values().flatten() {
        let target = (application.target)();
        let type_id = target.type_id();
        if !included_types.contains(&type_id) {
            continue;
        }
        applications_by_target.entry(type_id).or_default().push(application);
        let namespace = match application.implemented_trait {
            Some(trait_application) => {
                let definition =
                    resolve_trait_application_definition(trait_application, definitions, definitions_by_path)?;
                match definition {
                    Some(definition) => MethodNamespaceKey::TraitApplication {
                        definition: definition.id,
                        identity: trait_application.identity,
                    },
                    None => MethodNamespaceKey::TraitPath {
                        path: trait_application_path(trait_application).expect("source trait application has a path"),
                        identity: trait_application.identity,
                        application: Some(std::ptr::from_ref(trait_application) as usize),
                    },
                }
            }
            None => MethodNamespaceKey::Inherent,
        };
        for method in application.methods {
            if method.target_type_id() != type_id {
                return Err(invalid_method_metadata(*method, "impl method target mismatch"));
            }
            let namespace_matches = match (namespace, method.metadata.namespace) {
                (MethodNamespaceKey::Inherent, MethodNamespaceMetadata::Inherent { .. }) => true,
                (
                    MethodNamespaceKey::TraitApplication { .. } | MethodNamespaceKey::TraitPath { .. },
                    MethodNamespaceMetadata::Trait { trait_, .. },
                ) => application.implemented_trait.is_some_and(|trait_application| {
                    trait_application_path_matches(trait_application, trait_)
                        || trait_application
                            .definition
                            .is_some_and(|definition| definition.path == trait_)
                }),
                (
                    MethodNamespaceKey::TraitApplication { .. } | MethodNamespaceKey::TraitPath { .. },
                    MethodNamespaceMetadata::TraitApplication { trait_, .. },
                ) => application.implemented_trait.is_some_and(|trait_application| {
                    trait_application_path_matches(trait_application, trait_)
                        || trait_application
                            .definition
                            .is_some_and(|definition| definition.path == trait_)
                }),
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
                method_entry(namespace, *method, Some(application))?,
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
                return Err(invalid_method_metadata(*method, "catalog method target mismatch"));
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
                        [] => MethodNamespaceKey::TraitPath {
                            path: trait_,
                            identity: &[],
                            application: None,
                        },
                        [(_, trait_application)] => {
                            trait_application_namespace(trait_application, definitions, definitions_by_path)?
                        }
                        _ => {
                            return Err(invalid_method_metadata(
                                *method,
                                "catalog trait method matches multiple generic trait applications",
                            ));
                        }
                    }
                }
                MethodNamespaceMetadata::TraitApplication {
                    trait_,
                    identity,
                    definition,
                    ..
                } => {
                    let matches = applications_by_target
                        .get(&type_id)
                        .into_iter()
                        .flatten()
                        .filter_map(|application| {
                            let trait_application = application.implemented_trait?;
                            trait_application_matches_binding(trait_application, trait_, identity, definition)
                                .then_some(trait_application)
                        })
                        .collect::<Vec<_>>();
                    match matches.as_slice() {
                        [trait_application] => {
                            trait_application_namespace(trait_application, definitions, definitions_by_path)?
                        }
                        [] => {
                            return Err(invalid_method_metadata(
                                *method,
                                "method binding trait application is not registered",
                            ));
                        }
                        _ => {
                            return Err(invalid_method_metadata(
                                *method,
                                "method binding matches multiple identical trait applications",
                            ));
                        }
                    }
                }
            };
            push_method_entry(&mut entries_by_target, type_id, method_entry(namespace, *method, None)?)?;
        }
    }

    for method in method_bindings {
        let type_id = method.target_type_id();
        if !included_types.contains(&type_id) {
            return Err(invalid_method_metadata(
                *method,
                "method binding target is not included in the context",
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
                        (trait_application_path(trait_application) == Some(trait_)).then_some(trait_application)
                    })
                    .collect::<Vec<_>>();
                match applications.as_slice() {
                    [] => MethodNamespaceKey::TraitPath {
                        path: trait_,
                        identity: &[],
                        application: None,
                    },
                    [trait_application] => {
                        trait_application_namespace(trait_application, definitions, definitions_by_path)?
                    }
                    _ => {
                        return Err(invalid_method_metadata(
                            *method,
                            "method binding matches multiple generic trait applications",
                        ));
                    }
                }
            }
            MethodNamespaceMetadata::TraitApplication {
                trait_,
                identity,
                definition,
                ..
            } => {
                let applications = applications_by_target
                    .get(&type_id)
                    .into_iter()
                    .flatten()
                    .filter_map(|application| {
                        let trait_application = application.implemented_trait?;
                        trait_application_matches_binding(trait_application, trait_, identity, definition)
                            .then_some(trait_application)
                    })
                    .collect::<Vec<_>>();
                match applications.as_slice() {
                    [trait_application] => {
                        trait_application_namespace(trait_application, definitions, definitions_by_path)?
                    }
                    [] => {
                        return Err(invalid_method_metadata(
                            *method,
                            "method binding trait application is not registered",
                        ));
                    }
                    _ => {
                        return Err(invalid_method_metadata(
                            *method,
                            "method binding matches multiple identical trait applications",
                        ));
                    }
                }
            }
        };
        push_method_entry(&mut entries_by_target, type_id, method_entry(namespace, *method, None)?)?;
    }

    entries_by_target
        .into_iter()
        .map(|(type_id, entries)| {
            let methods = entries
                .into_iter()
                .map(|entry| {
                    if entry.pending_specs {
                        return Err(invalid_method_metadata(
                            entry.method,
                            "method binding has no matching generic declaration",
                        ));
                    }
                    Ok(ContextEffectiveMethod {
                        effective: entry.method,
                        specializations: Arc::from(entry.specializations.into_boxed_slice()),
                        implementation: entry.implementation,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok((type_id, Arc::from(methods.into_boxed_slice())))
        })
        .collect()
}

fn method_entry(
    namespace: MethodNamespaceKey,
    method: EffectiveMethod,
    implementation: Option<&'static ImplApplicationMetadata>,
) -> Result<MethodEntry, ContextError> {
    let (specializations, pending_specs) = match method.resolve_specializations() {
        Ok(specializations) => (specializations.into_vec(), false),
        Err(_) if !method.specialization_specs.is_empty() => (Vec::new(), true),
        Err(reason) => return Err(invalid_method_metadata(method, reason)),
    };
    for (index, specialization) in specializations.iter().enumerate() {
        if specializations[..index]
            .iter()
            .any(|previous| previous.arguments.same_selection(&specialization.arguments))
        {
            return Err(duplicate_method_specialization(method, specialization));
        }
    }
    Ok(MethodEntry {
        namespace,
        method,
        implementation,
        specializations,
        pending_specs,
    })
}

fn push_method_entry(
    entries_by_target: &mut HashMap<TypeId, Vec<MethodEntry>>,
    type_id: TypeId,
    mut entry: MethodEntry,
) -> Result<(), ContextError> {
    let entries = entries_by_target.entry(type_id).or_default();
    if let Some(previous) = entries.iter_mut().find(|previous| {
        previous.namespace == entry.namespace && previous.method.metadata.name == entry.method.metadata.name
    }) {
        if same_method_declaration(previous.method, entry.method) {
            if entry.pending_specs {
                let mut declaration_bound = entry.method;
                declaration_bound.metadata = previous.method.metadata;
                entry.specializations = declaration_bound
                    .resolve_specializations()
                    .map_err(|reason| invalid_method_metadata(entry.method, reason))?
                    .into_vec();
                entry.pending_specs = false;
            }
            for specialization in &entry.specializations {
                if previous
                    .specializations
                    .iter()
                    .any(|existing| existing.arguments.same_selection(&specialization.arguments))
                {
                    return Err(duplicate_method_specialization(entry.method, specialization));
                }
            }
            previous.specializations.append(&mut entry.specializations);
            if previous.implementation.is_none() {
                previous.implementation = entry.implementation;
            }
            return Ok(());
        }
        if previous.method.same_registration(&entry.method) {
            return Ok(());
        }
        let target = (entry.method.target)();
        return Err(ContextError::from_issues(vec![ContextIssue::ConflictingMethod {
            target: TypeIdentity {
                type_id,
                rust_name: target.rust_name(),
            },
            namespace: method_namespace_description(entry.namespace),
            method: entry.method.metadata.name,
        }])
        .expect("one context issue is nonempty"));
    }
    entries.push(entry);
    Ok(())
}

fn same_method_declaration(left: EffectiveMethod, right: EffectiveMethod) -> bool {
    if !left.metadata.is_generic
        || !right.metadata.is_generic
        || left.metadata.name != right.metadata.name
        || left.metadata.parameters.len() != right.metadata.parameters.len()
        || (left.metadata.generics.parameters.len() != right.metadata.generics.parameters.len()
            && !left.metadata.generics.parameters.is_empty()
            && !right.metadata.generics.parameters.is_empty())
    {
        return false;
    }
    // Namespace identity has already been resolved before entries are merged.
    // Comparing the original path spelling here would reject bindings whose
    // trait alias or qualification differs from the impl declaration.
    true
}

fn duplicate_method_specialization(
    method: EffectiveMethod,
    specialization: &ResolvedMethodSpecialization,
) -> ContextError {
    let target = (method.target)();
    let identity = item_identity(
        ItemKind::Method,
        format!("{}::{}", target.rust_name(), method.metadata.name).as_str(),
        method.metadata.name,
    );
    ContextError::from_issues(vec![ContextIssue::DuplicateMethodSpecialization {
        method: identity,
        arguments: specialization.arguments.as_slice().to_vec().into_boxed_slice(),
        registrations: Box::new([]),
    }])
    .expect("one context issue is nonempty")
}

fn method_namespace_description(namespace: MethodNamespaceKey) -> Box<str> {
    match namespace {
        MethodNamespaceKey::Inherent => "inherent".into(),
        MethodNamespaceKey::TraitPath {
            path,
            identity: _identity,
            application: _application,
        } => format!("trait {path}").into(),
        MethodNamespaceKey::TraitApplication { definition, identity } => {
            let arguments = identity
                .iter()
                .map(|identity| match identity {
                    TraitArgumentIdentity::Type(type_id) => format!("{:?}", type_id()),
                    TraitArgumentIdentity::Const(value) => format!("{value:?}"),
                    TraitArgumentIdentity::Lifetime(value) | TraitArgumentIdentity::Expression(value) => {
                        (*value).to_owned()
                    }
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
            target,
            capability_id,
            registrations,
        } => ContextIssue::ConflictingCapability {
            target,
            capability: ItemIdentity {
                kind: ItemKind::Capability,
                stable_id: StableItemId(capability_id.as_str().into()),
                display_name: capability_id.as_str().into(),
            },
            registrations: registrations.iter().map(capability_origin_registration).collect(),
        },
        other => ContextIssue::InvalidAdapter {
            item: item_identity(ItemKind::Capability, "capability", "capability"),
            reason: AdapterValidationError::new(other.to_string()),
        },
    };
    ContextError::from_issues(vec![issue]).expect("one context issue is nonempty")
}

fn capability_origin_registration(origin: &CapabilityRegistrationOrigin) -> RegistrationOrigin {
    match origin {
        CapabilityRegistrationOrigin::Linked(origin) | CapabilityRegistrationOrigin::Explicit(origin) => origin.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::ContextData;
    use super::ContextIssue;
    use super::EffectiveMethod;
    use super::HashMap;
    use super::ImplApplicationMetadata;
    use super::ImplDefinitionId;
    use super::MethodNamespaceKey;
    use super::Registration;
    use super::RegistrationOrigin;
    use super::TraitArgumentIdentity;
    use super::TraitDefinitionId;
    use super::TraitObjectRegistration;
    use super::TypeId;
    use super::aggregate_method_catalogs;
    use super::capability_origin_registration;
    use super::invalid_method_metadata;
    use super::method_namespace_description;
    use super::trait_application_matches_binding;
    use super::trait_application_namespace;
    use super::trait_application_path;
    use super::trait_application_path_matches;
    use super::validate_trait_impl_uniqueness;
    use crate::runtime::capability::capability_registration_origin::CapabilityRegistrationOrigin;
    use crate::runtime::construct::ConstructionAvailability;
    use crate::runtime::construct::ConstructionPlan;
    use crate::runtime::construct::ConstructionProviders;
    use crate::runtime::construct::ConstructionShape;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::generic::StructKind;
    use crate::runtime::descriptor::generic::TypeDefinitionKind;
    use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplDefinitionMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplKind;
    use crate::runtime::descriptor::impl_descriptor::ImplQualifiers;
    use crate::runtime::descriptor::metadata::TypeMetadata;
    use crate::runtime::descriptor::method::InvocationAdapters;
    use crate::runtime::descriptor::method::MethodCatalog;
    use crate::runtime::descriptor::method::MethodCatalogRegistration;
    use crate::runtime::descriptor::method::MethodMetadata;
    use crate::runtime::descriptor::method::MethodNamespaceMetadata;
    use crate::runtime::descriptor::method::MethodQualifiers;
    use crate::runtime::descriptor::method::MethodVisibility;
    use crate::runtime::descriptor::method::ReturnKind;
    use crate::runtime::descriptor::method::ReturnMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::TraitExpressionData;
    use crate::runtime::invoke::FunctionInvocationAdapter;
    use crate::runtime::invoke::Invocation;
    use crate::runtime::invoke::InvocationOutput;
    use crate::runtime::invoke::Local;
    use crate::runtime::invoke::ValidatedInvocation;

    struct Target;
    struct ImplOne;
    struct ImplTwo;
    struct TraitMarker;
    struct SamePathTraitMarker;
    struct ExternalTraitMarker;
    struct MissingTraitMarker;
    struct TypeDefinitionMarker;

    impl crate::runtime::descriptor::Reflect for Target {
        fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
            static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Target> =
                crate::runtime::descriptor::TypeRegistration::new();
            &REGISTRATION
        }
    }

    static TARGET_METADATA: TypeMetadata = TypeMetadata::new::<Target>();
    static TARGET_CONSTRUCTION_PLAN: ConstructionPlan = ConstructionPlan::for_type::<Target>(
        ConstructionShape::Unit,
        &[],
        ConstructionProviders::from_runtime_providers(None, None),
        ConstructionAvailability::Available,
    );
    #[test]
    fn keeps_unlinked_trait_application_unresolved_despite_matching_path() {
        let registration = Registration::metadata_only()
            .with_trait_definition(&EXTERNAL_TRAIT_DEFINITION)
            .with_trait_application(&EXTERNAL_TRAIT_APPLICATION);
        let context = ContextData::build(vec![registration])
            .expect("an unresolved application does not link by matching source path");

        let applications = context.trait_applications(TraitDefinitionId::of::<ExternalTraitMarker>());
        assert!(applications.is_empty());
        assert!(std::ptr::eq(
            context.trait_applications_by_path("example :: ExternalTrait")[0],
            &EXTERNAL_TRAIT_APPLICATION
        ));
    }

    static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &[],
        predicates: &[],
    };
    static TYPE_DEFINITION: TypeDefinitionMetadata = TypeDefinitionMetadata::new::<TypeDefinitionMarker>(
        "TypeDefinitionMarker",
        "fixture::TypeDefinitionMarker",
        TypeDefinitionKind::Struct(StructKind::Named),
        &EMPTY_GENERICS,
    );
    static EXTERNAL_TRAIT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<ExternalTraitMarker>(
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
    static CONFLICTING_RESET_METADATA: MethodMetadata = inherent_method("reset", "fixture::ImplTwo");
    static RESET_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&RESET_METADATA);
    static CLEAR_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&CLEAR_METADATA);
    static CONFLICTING_RESET_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&CONFLICTING_RESET_METADATA);
    static IMPL_ONE_METHODS: &[EffectiveMethod] = &[RESET_METHOD];
    static IMPL_TWO_METHODS: &[EffectiveMethod] = &[CLEAR_METHOD];
    static CONFLICTING_METHODS: &[EffectiveMethod] = &[CONFLICTING_RESET_METHOD];
    static CATALOG_WITH_RESET: &[EffectiveMethod] = &[RESET_METHOD];
    static TRAIT_A_METADATA: MethodMetadata = trait_method("same", "fixture::TraitA");
    static TRAIT_B_METADATA: MethodMetadata = trait_method("same", "fixture::TraitB");
    static TRAIT_A_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&TRAIT_A_METADATA);
    static TRAIT_B_METHOD: EffectiveMethod = EffectiveMethod::for_type::<Target>(&TRAIT_B_METADATA);
    static TRAIT_METHODS: &[EffectiveMethod] = &[TRAIT_A_METHOD, TRAIT_B_METHOD];
    static AMBIGUOUS_TRAIT_METHOD_METADATA: MethodMetadata = trait_method("catalog_only", "fixture::GenericTrait");
    static AMBIGUOUS_TRAIT_METHOD: EffectiveMethod =
        EffectiveMethod::for_type::<Target>(&AMBIGUOUS_TRAIT_METHOD_METADATA);
    static AMBIGUOUS_TRAIT_METHODS: &[EffectiveMethod] = &[AMBIGUOUS_TRAIT_METHOD];
    static UNREGISTERED_TRAIT_APPLICATION_IDENTITY: &[TraitArgumentIdentity] =
        &[TraitArgumentIdentity::expression("C")];
    static UNREGISTERED_TRAIT_APPLICATION_METHOD_METADATA: MethodMetadata = MethodMetadata {
        namespace: MethodNamespaceMetadata::TraitApplication {
            trait_: "fixture::GenericTrait",
            implementation: "fixture::Target",
            identity: UNREGISTERED_TRAIT_APPLICATION_IDENTITY,
            definition: None,
        },
        ..trait_method("missing_application", "fixture::GenericTrait")
    };
    static UNREGISTERED_TRAIT_APPLICATION_METHOD: EffectiveMethod =
        EffectiveMethod::for_type::<Target>(&UNREGISTERED_TRAIT_APPLICATION_METHOD_METADATA);

    fn provider_adapter<'call>(_invocation: ValidatedInvocation<'call, Local>) -> InvocationOutput<'call, Local> {
        InvocationOutput::unit()
    }

    static PROVIDER_ADAPTER: FunctionInvocationAdapter<Local> = FunctionInvocationAdapter::new(provider_adapter);
    static PROVIDER_ADAPTERS: InvocationAdapters = InvocationAdapters::builder().local(&PROVIDER_ADAPTER).build();
    static PROVIDER_METADATA: MethodMetadata = MethodMetadata::inherent("provided", "fixture::Target");
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
    static IMPL_ONE: ImplApplicationMetadata =
        ImplApplicationMetadata::for_target::<Target>(&IMPL_ONE_DEFINITION, None, &[], &[], IMPL_ONE_METHODS, &[], &[]);
    static IMPL_TWO: ImplApplicationMetadata =
        ImplApplicationMetadata::for_target::<Target>(&IMPL_TWO_DEFINITION, None, &[], &[], IMPL_TWO_METHODS, &[], &[]);
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
    static SAME_PATH_GENERIC_TRAIT: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<SamePathTraitMarker>(
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
    static SAME_PATH_GENERIC_TRAIT_A: TraitApplicationMetadata = TraitApplicationMetadata::new(
        &SAME_PATH_GENERIC_TRAIT,
        &[],
        &[TraitArgumentIdentity::expression("A")],
        &[],
        &[],
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
    );
    static MISSING_TRAIT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<MissingTraitMarker>(
        "MissingTrait",
        "fixture::MissingTrait",
        Visibility::Public,
        TraitCompleteness::Complete,
        &EMPTY_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );
    static MISSING_TRAIT_APPLICATION: TraitApplicationMetadata = TraitApplicationMetadata::new(
        &MISSING_TRAIT_DEFINITION,
        &[],
        &[],
        &[],
        &[],
        crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
    );
    static APPLICATION_WITH_MISSING_SUPERTRAIT: TraitApplicationMetadata = TraitApplicationMetadata::new(
        &GENERIC_TRAIT,
        &[],
        &[],
        &[&MISSING_TRAIT_APPLICATION],
        &[&MISSING_TRAIT_APPLICATION],
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
    static DUPLICATE_GENERIC_IMPL: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_TWO_DEFINITION,
        Some(&GENERIC_TRAIT_A),
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    static UNRESOLVED_IMPL_ONE: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_ONE_DEFINITION,
        Some(&EXTERNAL_TRAIT_APPLICATION),
        &[],
        &[],
        &[],
        &[],
        &[],
    );
    static UNRESOLVED_IMPL_TWO: ImplApplicationMetadata = ImplApplicationMetadata::for_target::<Target>(
        &IMPL_TWO_DEFINITION,
        Some(&EXTERNAL_TRAIT_APPLICATION),
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
        applications.iter().fold(HashMap::new(), |mut grouped, application| {
            grouped.entry(application.definition.id).or_default().push(application);
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
        let methods = aggregate_method_catalogs(
            &included_types(),
            &applications,
            &[&catalog],
            &[],
            &HashMap::new(),
            &HashMap::new(),
        )
        .expect("distinct inherent methods aggregate");
        let methods = methods.get(&TypeId::of::<Target>()).expect("target catalog");
        assert_eq!(methods.len(), 2);
        assert_eq!(
            methods
                .iter()
                .filter(|method| method.effective.metadata.name == "reset")
                .count(),
            1
        );
        assert!(methods.iter().any(|method| method.effective.metadata.name == "clear"));
    }

    #[test]
    fn provider_catalog_is_frozen_and_its_adapter_remains_invokable() {
        let catalogs = [&PROVIDER_CATALOG];
        let methods = aggregate_method_catalogs(
            &included_types(),
            &HashMap::new(),
            &catalogs,
            &[],
            &HashMap::new(),
            &HashMap::new(),
        )
        .expect("provider catalog is valid");
        let target_methods = methods.get(&TypeId::of::<Target>()).expect("provider target catalog");
        assert_eq!(target_methods.len(), 1);
        assert!(target_methods[0].effective.adapters.is_some());

        let context = ReflectionContext::from_data(ContextData::empty());
        let descriptor = crate::runtime::descriptor::TypeDescriptor::new(&TARGET_METADATA, &context);
        let catalog = MethodCatalog::from_shared(descriptor, methods[&TypeId::of::<Target>()].clone());
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
        let methods = aggregate_method_catalogs(
            &included_types(),
            &HashMap::new(),
            &[&catalog],
            &[],
            &HashMap::new(),
            &HashMap::new(),
        )
        .expect("trait namespaces are distinct");
        assert_eq!(methods[&TypeId::of::<Target>()].len(), 2);
    }

    #[test]
    fn rejects_duplicate_method_name_in_one_inherent_namespace() {
        let applications = application_map(&[&IMPL_ONE, &CONFLICTING_IMPL]);
        let error = match aggregate_method_catalogs(
            &included_types(),
            &applications,
            &[],
            &[],
            &HashMap::new(),
            &HashMap::new(),
        ) {
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
        let definitions = HashMap::from([(GENERIC_TRAIT.id, &GENERIC_TRAIT)]);
        let definitions_by_path = HashMap::from([(GENERIC_TRAIT.path, GENERIC_TRAIT.id)]);
        let error = match aggregate_method_catalogs(
            &included_types(),
            &applications,
            &[&catalog],
            &[],
            &definitions,
            &definitions_by_path,
        ) {
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
    fn rejects_binding_for_unregistered_generic_trait_application() {
        let applications = application_map(&[&GENERIC_IMPL_ONE, &GENERIC_IMPL_TWO]);
        let definitions = HashMap::from([(GENERIC_TRAIT.id, &GENERIC_TRAIT)]);
        let definitions_by_path = HashMap::from([(GENERIC_TRAIT.path, GENERIC_TRAIT.id)]);
        let error = match aggregate_method_catalogs(
            &included_types(),
            &applications,
            &[],
            &[UNREGISTERED_TRAIT_APPLICATION_METHOD],
            &definitions,
            &definitions_by_path,
        ) {
            Ok(_) => panic!("explicit typed identity must match a registered application"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::InvalidMetadata { reason, .. }
                if reason.to_string().contains("trait application is not registered")
        ));
    }

    #[test]
    fn typed_application_binding_uses_definition_and_argument_identity() {
        assert!(trait_application_matches_binding(
            &GENERIC_TRAIT_A,
            "an::unrelated::Alias<A>",
            GENERIC_TRAIT_A.identity,
            Some(&GENERIC_TRAIT),
        ));
        assert!(!trait_application_matches_binding(
            &GENERIC_TRAIT_A,
            GENERIC_TRAIT.path,
            GENERIC_TRAIT_B.identity,
            Some(&GENERIC_TRAIT),
        ));
        assert!(!trait_application_matches_binding(
            &SAME_PATH_GENERIC_TRAIT_A,
            GENERIC_TRAIT.path,
            GENERIC_TRAIT_A.identity,
            Some(&GENERIC_TRAIT),
        ));
        assert!(!trait_application_matches_binding(
            &EXTERNAL_TRAIT_APPLICATION,
            "example :: ExternalTrait",
            &[],
            None,
        ));
    }

    #[test]
    fn allows_distinct_generic_trait_applications_but_rejects_duplicate_application() {
        let distinct = [
            (&GENERIC_IMPL_ONE, origin("impl_a")),
            (&GENERIC_IMPL_TWO, origin("impl_b")),
        ];
        let definitions = HashMap::from([(GENERIC_TRAIT.id, &GENERIC_TRAIT)]);
        validate_trait_impl_uniqueness(&distinct, &definitions, &HashMap::new())
            .expect("distinct trait arguments are distinct impls");

        let duplicate = [
            (&GENERIC_IMPL_ONE, origin("impl_a")),
            (&DUPLICATE_GENERIC_IMPL, origin("impl_b")),
        ];
        let error = match validate_trait_impl_uniqueness(&duplicate, &definitions, &HashMap::new()) {
            Ok(()) => panic!("duplicate trait application rejects the context"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::DuplicateTraitImpl { registrations, .. } if registrations.len() == 2
        ));
    }

    #[test]
    fn unresolved_source_path_does_not_create_trait_impl_identity_conflict() {
        let registrations = [
            (&UNRESOLVED_IMPL_ONE, origin("unresolved_one")),
            (&UNRESOLVED_IMPL_TWO, origin("unresolved_two")),
        ];

        validate_trait_impl_uniqueness(&registrations, &HashMap::new(), &HashMap::new())
            .expect("matching display paths cannot establish typed trait identity");
    }

    #[test]
    fn links_same_path_trait_applications_by_typed_definition_identity() {
        let context = ContextData::build(vec![
            Registration::metadata_only()
                .with_trait_definition(&GENERIC_TRAIT)
                .with_trait_application(&GENERIC_TRAIT_A),
            Registration::metadata_only()
                .with_trait_definition(&SAME_PATH_GENERIC_TRAIT)
                .with_trait_application(&SAME_PATH_GENERIC_TRAIT_A),
        ])
        .expect("distinct typed trait definitions can share a source path");

        assert_eq!(context.trait_definitions().len(), 2);
        let generic_applications = context.trait_applications(GENERIC_TRAIT.id);
        assert_eq!(generic_applications.len(), 1);
        assert!(std::ptr::eq(generic_applications[0], &GENERIC_TRAIT_A));
        let same_path_applications = context.trait_applications(SAME_PATH_GENERIC_TRAIT.id);
        assert_eq!(same_path_applications.len(), 1);
        assert!(std::ptr::eq(same_path_applications[0], &SAME_PATH_GENERIC_TRAIT_A));
        assert_eq!(
            context.trait_applications_by_path(GENERIC_TRAIT.path).len(),
            2,
            "path lookup retains both candidates when spelling is ambiguous"
        );
        assert!(context.trait_definition_by_path(GENERIC_TRAIT.path).is_none());
    }

    #[test]
    fn rejects_trait_application_when_typed_definition_is_unregistered() {
        let error = match ContextData::build(vec![
            Registration::metadata_only().with_trait_application(&MISSING_TRAIT_APPLICATION),
        ]) {
            Ok(_) => panic!("linked applications require their typed trait definition"),
            Err(error) => error,
        };

        assert!(matches!(
            error.primary_issue(),
            ContextIssue::MissingDependency { dependency, .. }
                if dependency.display_name.as_ref() == "MissingTrait"
        ));

        let context = ContextData::build(vec![
            Registration::metadata_only()
                .with_trait_definition(&GENERIC_TRAIT)
                .with_trait_application(&GENERIC_TRAIT_A),
        ])
        .expect("a failed build does not contaminate a subsequent context");
        let applications = context.trait_applications(GENERIC_TRAIT.id);
        assert_eq!(applications.len(), 1);
        assert!(std::ptr::eq(applications[0], &GENERIC_TRAIT_A));
    }

    #[test]
    fn rejects_registered_application_with_unregistered_supertrait() {
        let error = match ContextData::build(vec![
            Registration::metadata_only()
                .with_trait_definition(&GENERIC_TRAIT)
                .with_trait_application(&APPLICATION_WITH_MISSING_SUPERTRAIT),
        ]) {
            Ok(_) => panic!("typed supertrait links require their definition metadata"),
            Err(error) => error,
        };

        assert!(matches!(
            error.primary_issue(),
            ContextIssue::MissingDependency { dependency, .. }
                if dependency.display_name.as_ref() == "MissingTrait"
        ));
    }

    #[test]
    fn rejects_duplicate_typed_trait_definition_and_recovers_on_next_build() {
        let error = match ContextData::build(vec![
            Registration::metadata_only().with_trait_definition(&GENERIC_TRAIT),
            Registration::metadata_only().with_trait_definition(&GENERIC_TRAIT),
        ]) {
            Ok(_) => panic!("one typed trait definition cannot be registered twice"),
            Err(error) => error,
        };
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::ConflictingTraitDefinition { .. }
        ));

        let context = ContextData::build(vec![
            Registration::metadata_only().with_trait_definition(&GENERIC_TRAIT),
        ])
        .expect("a duplicate-definition error does not affect later builds");
        assert_eq!(
            context.trait_definition(GENERIC_TRAIT.id).map(|metadata| metadata.name),
            Some("GenericTrait")
        );
    }

    #[test]
    fn preserves_target_indexes_and_registered_metadata_accessors() {
        let trait_object = TraitObjectRegistration::of::<dyn std::fmt::Debug>("std::fmt::Debug");
        let context = ContextData::build(vec![
            Registration::of::<Target>()
                .with_impl_definition(&IMPL_ONE_DEFINITION)
                .with_impl_application(&IMPL_ONE)
                .with_construction_plan(&TARGET_CONSTRUCTION_PLAN)
                .with_trait_object(trait_object),
        ])
        .expect("well-formed registration populates context indexes");

        assert!(context.contains_type_id(TypeId::of::<Target>()));
        assert!(!context.contains_type_id(TypeId::of::<u8>()));
        assert!(context.type_by_id(TypeId::of::<Target>()).is_some());
        assert!(context.type_by_id(TypeId::of::<u8>()).is_none());
        assert!(context.shape(TypeId::of::<Target>()).is_none());
        let methods = context.methods(TypeId::of::<Target>());
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].effective.metadata.name, "reset");
        assert_eq!(context.impl_definitions().len(), 1);
        assert_eq!(context.impl_applications(IMPL_ONE_DEFINITION.id).len(), 1);
        assert_eq!(context.impl_applications_for_target(TypeId::of::<Target>()).len(), 1);
        assert_eq!(context.construction_plans(TypeId::of::<Target>()).len(), 1);
        assert_eq!(
            context
                .trait_object(TypeId::of::<dyn std::fmt::Debug>())
                .map(TraitObjectRegistration::trait_path),
            Some("std::fmt::Debug")
        );
        assert!(context.trait_object(TypeId::of::<dyn std::fmt::Display>()).is_none());
    }

    #[test]
    fn rejects_conflicting_trait_object_paths_for_one_compiler_type() {
        let error = match ContextData::build(vec![
            Registration::metadata_only()
                .with_trait_object(TraitObjectRegistration::of::<dyn std::fmt::Debug>("first::Debug")),
            Registration::metadata_only()
                .with_trait_object(TraitObjectRegistration::of::<dyn std::fmt::Debug>("second::Debug")),
        ]) {
            Ok(_) => panic!("one trait object type cannot have two registered trait paths"),
            Err(error) => error,
        };

        assert!(matches!(
            error.primary_issue(),
            ContextIssue::InvalidMetadata { reason, .. }
                if reason.to_string().contains("conflicting registered trait paths")
        ));
    }

    #[test]
    fn deduplicates_repeated_application_pointer_and_indexes_type_instances() {
        let context = ContextData::build(vec![
            Registration::metadata_only()
                .with_trait_definition(&GENERIC_TRAIT)
                .with_trait_application(&GENERIC_TRAIT_A)
                .with_trait_application(&GENERIC_TRAIT_A),
            Registration::of::<Target>()
                .with_type_definition_metadata(&TYPE_DEFINITION)
                .with_type_instance::<TypeDefinitionMarker>(),
        ])
        .expect("repeated pointers deduplicate while a registered type instance is retained");

        let applications = context.trait_applications_all();
        assert_eq!(applications.len(), 1);
        assert!(std::ptr::eq(applications[0], &GENERIC_TRAIT_A));
        assert_eq!(
            context.type_definition(TYPE_DEFINITION.id).map(|item| item.name),
            Some("TypeDefinitionMarker")
        );
        assert_eq!(context.type_instances(TYPE_DEFINITION.id), &[TypeId::of::<Target>()]);
    }

    #[test]
    fn rejects_type_instance_without_registered_definition() {
        let error = match ContextData::build(vec![
            Registration::of::<Target>().with_type_instance::<TypeDefinitionMarker>(),
        ]) {
            Ok(_) => panic!("type instances require their source definition"),
            Err(error) => error,
        };

        assert!(matches!(error.primary_issue(), ContextIssue::MissingDependency { .. }));
    }

    #[test]
    fn rejects_duplicate_type_definition_identity() {
        let error = match ContextData::build(vec![
            Registration::metadata_only().with_type_definition_metadata(&TYPE_DEFINITION),
            Registration::metadata_only().with_type_definition_metadata(&TYPE_DEFINITION),
        ]) {
            Ok(_) => panic!("one typed generic definition cannot be registered twice"),
            Err(error) => error,
        };

        assert!(matches!(
            error.primary_issue(),
            ContextIssue::ConflictingTypeDefinition { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_impl_definition_identity() {
        let error = match ContextData::build(vec![
            Registration::metadata_only().with_impl_definition(&IMPL_ONE_DEFINITION),
            Registration::metadata_only().with_impl_definition(&IMPL_ONE_DEFINITION),
        ]) {
            Ok(_) => panic!("one impl definition identity cannot be registered twice"),
            Err(error) => error,
        };

        assert!(matches!(
            error.primary_issue(),
            ContextIssue::InvalidMetadata { reason, .. }
                if reason.to_string().contains("duplicate impl definition identity")
        ));
    }

    #[test]
    fn rejects_impl_application_without_definition_and_without_registered_target() {
        let missing_definition =
            match ContextData::build(vec![Registration::metadata_only().with_impl_application(&IMPL_ONE)]) {
                Ok(_) => panic!("an impl application requires its definition"),
                Err(error) => error,
            };
        assert!(matches!(
            missing_definition.primary_issue(),
            ContextIssue::MissingDependency { item, .. }
                if item.display_name.as_ref() == "impl application"
        ));

        let missing_target = match ContextData::build(vec![
            Registration::metadata_only()
                .with_impl_definition(&IMPL_ONE_DEFINITION)
                .with_impl_application(&IMPL_ONE),
        ]) {
            Ok(_) => panic!("an impl application requires its target type registration"),
            Err(error) => error,
        };
        assert!(matches!(
            missing_target.primary_issue(),
            ContextIssue::MissingDependency { item, .. }
                if item.display_name.as_ref() == "impl application"
        ));
    }

    #[test]
    fn reports_duplicate_target_registration_origins() {
        let error = match ContextData::build(vec![Registration::of::<Target>(), Registration::of::<Target>()]) {
            Ok(_) => panic!("duplicate reflected type registrations must be reported"),
            Err(error) => error,
        };

        assert!(matches!(
            error.primary_issue(),
            ContextIssue::DuplicateTypeIdentity { registrations, .. }
                if registrations.len() == 2
        ));
    }

    #[test]
    fn describes_unlinked_trait_method_namespace_and_reports_invalid_metadata() {
        let definitions = HashMap::new();
        let definitions_by_path = HashMap::new();
        let namespace = trait_application_namespace(&EXTERNAL_TRAIT_APPLICATION, &definitions, &definitions_by_path)
            .expect("an unlinked source application has a path namespace");

        assert!(matches!(
            namespace,
            MethodNamespaceKey::TraitPath {
                path: "example :: ExternalTrait",
                ..
            }
        ));
        assert_eq!(
            trait_application_path(&EXTERNAL_TRAIT_APPLICATION),
            Some("example :: ExternalTrait")
        );
        assert!(trait_application_path_matches(
            &EXTERNAL_TRAIT_APPLICATION,
            "example :: ExternalTrait"
        ));
        assert_eq!(
            method_namespace_description(namespace).as_ref(),
            "trait example :: ExternalTrait"
        );

        let error = invalid_method_metadata(RESET_METHOD, "test invalid declaration");
        assert!(matches!(
            error.primary_issue(),
            ContextIssue::InvalidMetadata { reason, .. }
                if reason.to_string() == "test invalid declaration"
        ));
    }

    #[test]
    fn converts_both_capability_origins_to_registration_origins() {
        let linked = CapabilityRegistrationOrigin::Linked(origin("fixture::linked"));
        let explicit = CapabilityRegistrationOrigin::Explicit(origin("fixture::explicit"));

        assert_eq!(capability_origin_registration(&linked), origin("fixture::linked"));
        assert_eq!(capability_origin_registration(&explicit), origin("fixture::explicit"));
    }
}
