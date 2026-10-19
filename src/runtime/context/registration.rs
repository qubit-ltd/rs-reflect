use std::any::TypeId;

use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::{TypeDefinitionId, TypeDefinitionMetadata};
use crate::runtime::descriptor::impl_descriptor::{
    ImplApplicationMetadata, ImplDefinitionMetadata,
};
use crate::runtime::descriptor::trait_descriptor::{
    TraitApplicationMetadata, TraitDefinitionMetadata,
};
use crate::runtime::descriptor::{Reflect, TypeDescriptor, TypeRegistration};
use crate::runtime::error::{ContextError, ContextIssue, MetadataValidationError};
use crate::runtime::identity::RegistrationOrigin;
use crate::runtime::identity::{ItemIdentity, ItemKind, StableItemId};

pub type RegistrationSortKey = (
    &'static str,
    Option<&'static str>,
    &'static str,
    Option<&'static str>,
    Option<u32>,
    Option<u32>,
    &'static str,
);

#[derive(Clone, Copy)]
pub(crate) struct TypeRecord {
    type_id: fn() -> TypeId,
    rust_name: fn() -> &'static str,
    descriptor: for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>,
}

impl TypeRecord {
    fn of<T: Reflect>() -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            descriptor: descriptor_for::<T>,
        }
    }

    pub(crate) fn type_id(self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn rust_name(self) -> &'static str {
        (self.rust_name)()
    }

    pub(crate) fn descriptor(self, context: &ReflectionContext) -> TypeDescriptor<'_> {
        (self.descriptor)(context)
    }
}

fn descriptor_for<'ctx, T: Reflect>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::new(T::__reflect_registration().metadata(), context)
}

pub struct Registration {
    type_record: TypeRecord,
    origin: RegistrationOrigin,
    type_definitions: Vec<&'static TypeDefinitionMetadata>,
    trait_definitions: Vec<&'static TraitDefinitionMetadata>,
    impl_definitions: Vec<&'static ImplDefinitionMetadata>,
    capabilities: Vec<CapabilityRegistration>,
    type_instance_of: Option<TypeDefinitionId>,
    trait_applications: Vec<&'static TraitApplicationMetadata>,
    impl_applications: Vec<&'static ImplApplicationMetadata>,
    construction_plans: Vec<&'static ConstructionPlan>,
}

impl Registration {
    pub fn of<T: Reflect>() -> Self {
        Self::new::<T>()
    }

    fn new<T: Reflect>() -> Self {
        let type_record = TypeRecord::of::<T>();
        Self {
            type_record,
            origin: RegistrationOrigin {
                crate_name: "explicit",
                crate_version: None,
                module_path: type_record.rust_name(),
                file: None,
                line: None,
                column: None,
            },
            type_definitions: Vec::new(),
            trait_definitions: Vec::new(),
            impl_definitions: Vec::new(),
            capabilities: Vec::new(),
            type_instance_of: None,
            trait_applications: Vec::new(),
            impl_applications: Vec::new(),
            construction_plans: Vec::new(),
        }
    }

    pub(crate) fn with_type_definition(
        mut self,
        metadata: &'static TypeDefinitionMetadata,
    ) -> Self {
        self.type_definitions.push(metadata);
        self
    }

    pub(crate) fn with_trait_definition(
        mut self,
        metadata: &'static TraitDefinitionMetadata,
    ) -> Self {
        self.trait_definitions.push(metadata);
        self
    }

    pub(crate) fn with_impl_definition(
        mut self,
        metadata: &'static ImplDefinitionMetadata,
    ) -> Self {
        self.impl_definitions.push(metadata);
        self
    }

    pub(crate) fn type_definitions(&self) -> &[&'static TypeDefinitionMetadata] {
        &self.type_definitions
    }

    pub(crate) fn trait_definitions(&self) -> &[&'static TraitDefinitionMetadata] {
        &self.trait_definitions
    }

    pub(crate) fn impl_definitions(&self) -> &[&'static ImplDefinitionMetadata] {
        &self.impl_definitions
    }

    pub(crate) fn with_capability(mut self, registration: CapabilityRegistration) -> Self {
        self.capabilities.push(registration);
        self
    }

    pub(crate) fn take_capabilities(&mut self) -> Vec<CapabilityRegistration> {
        std::mem::take(&mut self.capabilities)
    }

    pub(crate) fn with_type_instance(mut self, definition: TypeDefinitionId) -> Self {
        self.type_instance_of = Some(definition);
        self
    }

    pub(crate) fn type_instance_of(&self) -> Option<TypeDefinitionId> {
        self.type_instance_of
    }

    pub(crate) fn with_trait_application(
        mut self,
        application: &'static TraitApplicationMetadata,
    ) -> Self {
        self.trait_applications.push(application);
        self
    }

    pub(crate) fn trait_applications(&self) -> &[&'static TraitApplicationMetadata] {
        &self.trait_applications
    }

    pub(crate) fn with_impl_application(
        mut self,
        application: &'static ImplApplicationMetadata,
    ) -> Self {
        self.impl_applications.push(application);
        self
    }

    pub(crate) fn impl_applications(&self) -> &[&'static ImplApplicationMetadata] {
        &self.impl_applications
    }

    pub(crate) fn with_construction_plan(mut self, plan: &'static ConstructionPlan) -> Self {
        self.construction_plans.push(plan);
        self
    }

    pub(crate) fn construction_plans(&self) -> &[&'static ConstructionPlan] {
        &self.construction_plans
    }

    pub(crate) const fn type_record(&self) -> TypeRecord {
        self.type_record
    }

    pub(crate) fn origin(&self) -> &RegistrationOrigin {
        &self.origin
    }

    pub(crate) fn sort_key(&self) -> RegistrationSortKey {
        (
            self.origin.crate_name,
            self.origin.crate_version,
            self.origin.module_path,
            self.origin.file,
            self.origin.line,
            self.origin.column,
            self.type_record.rust_name(),
        )
    }
}

impl<T: Reflect> From<&'static TypeRegistration<T>> for Registration {
    fn from(_registration: &'static TypeRegistration<T>) -> Self {
        Self::new::<T>()
    }
}

/// A statically linked type registration consumed while building a context.
#[derive(Clone, Copy)]
pub struct RegistrationFragment {
    sort_key: RegistrationSortKey,
    target_type_id: Option<fn() -> TypeId>,
    dependencies: fn() -> &'static [TypeId],
    includes_type: bool,
    registration: fn() -> Registration,
}

impl RegistrationFragment {
    /// Creates a type fragment with a runtime controlled stable origin.
    pub const fn for_reflected_type<T: Reflect>(label: &'static str) -> Self {
        Self {
            sort_key: ("generated", None, "generated", Some(label), None, None, label),
            target_type_id: Some(TypeId::of::<T>),
            dependencies: no_dependencies,
            includes_type: true,
            registration: Registration::of::<T>,
        }
    }

    pub(crate) const fn new(
        sort_key: RegistrationSortKey,
        registration: fn() -> Registration,
    ) -> Self {
        Self {
            sort_key,
            target_type_id: None,
            dependencies: no_dependencies,
            includes_type: true,
            registration,
        }
    }

    pub(crate) const fn for_target<T: 'static>(
        sort_key: RegistrationSortKey,
        dependencies: fn() -> &'static [TypeId],
        registration: fn() -> Registration,
    ) -> Self {
        Self {
            sort_key,
            target_type_id: Some(TypeId::of::<T>),
            dependencies,
            includes_type: false,
            registration,
        }
    }

    pub(crate) const fn for_type<T: Reflect>(
        sort_key: RegistrationSortKey,
        dependencies: fn() -> &'static [TypeId],
        registration: fn() -> Registration,
    ) -> Self {
        Self {
            sort_key,
            target_type_id: Some(TypeId::of::<T>),
            dependencies,
            includes_type: true,
            registration,
        }
    }

    pub(crate) fn materialize(self) -> Result<Registration, ContextError> {
        let registration = (self.registration)();
        if let Some(target) = self.target_type_id() {
            if self.includes_type && registration.type_record().type_id() != target {
                let name = format!("{target:?}");
                return Err(
                    ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                        item: ItemIdentity {
                            kind: ItemKind::Type,
                            stable_id: StableItemId(name.clone().into()),
                            display_name: name.into(),
                        },
                        reason: MetadataValidationError::new(
                            "type fragment target does not match its registration",
                        ),
                    }])
                    .expect("one context issue is nonempty"),
                );
            }
        }
        Ok(registration)
    }

    pub(crate) const fn sort_key(&self) -> RegistrationSortKey {
        self.sort_key
    }

    pub(crate) fn target_type_id(&self) -> Option<TypeId> {
        self.target_type_id.map(|identity| identity())
    }

    pub(crate) fn dependencies(&self) -> &'static [TypeId] {
        (self.dependencies)()
    }

    pub(crate) const fn includes_type(&self) -> bool {
        self.includes_type
    }
}

fn no_dependencies() -> &'static [TypeId] {
    &[]
}

inventory::collect!(RegistrationFragment);
