// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::marker::PhantomData;

use crate::runtime::capability::capability_registration::CapabilityRegistration;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::TypeRegistration;
use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::impl_descriptor::ImplApplicationMetadata;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitionMetadata;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::error::ContextError;
use crate::runtime::error::ContextIssue;
use crate::runtime::error::MetadataValidationError;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::RegistrationOrigin;
use crate::runtime::identity::StableItemId;

/// Stable source-order key used to order registration fragments.
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
    fn of<T: Reflect + ?Sized>() -> Self {
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

fn descriptor_for<'ctx, T: Reflect + ?Sized>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::new(T::__reflect_registration().metadata(), context)
}

/// A bundle of type, definition, application, and capability metadata for a
/// context.
pub struct Registration {
    type_record: Option<TypeRecord>,
    shape: Option<&'static TypeKindMetadata>,
    origin: RegistrationOrigin,
    type_definitions: Vec<&'static TypeDefinitionMetadata>,
    trait_definitions: Vec<&'static TraitDefinitionMetadata>,
    impl_definitions: Vec<&'static ImplDefinitionMetadata>,
    capabilities: Vec<CapabilityRegistration>,
    type_instance_of: Option<TypeDefinitionId>,
    trait_applications: Vec<&'static TraitApplicationMetadata>,
    impl_applications: Vec<&'static ImplApplicationMetadata>,
    construction_plans: Vec<&'static ConstructionPlan>,
    trait_objects: Vec<TraitObjectRegistration>,
    method_bindings: Vec<EffectiveMethod>,
    method_binding_targets: Vec<TypeRecord>,
    issues: Vec<ContextIssue>,
}

/// A compiler-checked trait-object type recorded in a reflection context.
#[derive(Clone, Copy)]
pub struct TraitObjectRegistration {
    type_id: fn() -> TypeId,
    rust_name: fn() -> &'static str,
    trait_path: &'static str,
}

impl TraitObjectRegistration {
    /// Creates a registration bound to the exact Rust trait-object type `T`.
    pub const fn of<T: ?Sized + 'static>(trait_path: &'static str) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            trait_path,
        }
    }

    pub(crate) fn type_id(self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn rust_name(self) -> &'static str {
        (self.rust_name)()
    }

    pub(crate) const fn trait_path(self) -> &'static str {
        self.trait_path
    }
}

/// A generic definition registration whose process-local identity is bound to
/// `T`.
pub struct TypeDefinitionRegistration<T: 'static> {
    metadata: TypeDefinitionMetadata,
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> TypeDefinitionRegistration<T> {
    /// Creates source metadata for a generic definition, binding identity to
    /// its marker type.
    pub const fn new(
        name: &'static str,
        path: &'static str,
        kind: crate::runtime::descriptor::generic::TypeDefinitionKind,
        generics: &'static crate::runtime::descriptor::generic::GenericDefinitionMetadata,
    ) -> Self {
        Self {
            metadata: TypeDefinitionMetadata::new::<T>(name, path, kind, generics),
            _marker: PhantomData,
        }
    }

    pub(crate) const fn metadata(&'static self) -> &'static TypeDefinitionMetadata {
        &self.metadata
    }
}

impl Registration {
    /// Creates a registration containing the reflected metadata for `T`.
    pub fn of<T: Reflect + ?Sized>() -> Self {
        Self::new::<T>()
    }

    fn new<T: Reflect + ?Sized>() -> Self {
        let type_record = TypeRecord::of::<T>();
        let reflect_registration = T::__reflect_registration();
        Self {
            type_record: Some(type_record),
            shape: reflect_registration.shape(),
            origin: RegistrationOrigin {
                crate_name: "explicit",
                crate_version: None,
                module_path: type_record.rust_name(),
                file: None,
                line: None,
                column: None,
            },
            type_definitions: reflect_registration.type_definition().into_iter().collect(),
            trait_definitions: Vec::new(),
            impl_definitions: Vec::new(),
            capabilities: Vec::new(),
            type_instance_of: reflect_registration.type_definition().map(|definition| definition.id),
            trait_applications: Vec::new(),
            impl_applications: Vec::new(),
            construction_plans: reflect_registration.construction_plans().iter().collect(),
            trait_objects: Vec::new(),
            method_bindings: Vec::new(),
            method_binding_targets: Vec::new(),
            issues: Vec::new(),
        }
    }

    /// Creates a registration that contributes definitions or applications
    /// without adding a type.
    pub fn metadata_only() -> Self {
        Self {
            type_record: None,
            origin: RegistrationOrigin {
                crate_name: "generated",
                crate_version: None,
                module_path: "generated metadata",
                file: None,
                line: None,
                column: None,
            },
            shape: None,
            type_definitions: Vec::new(),
            trait_definitions: Vec::new(),
            impl_definitions: Vec::new(),
            capabilities: Vec::new(),
            type_instance_of: None,
            trait_applications: Vec::new(),
            impl_applications: Vec::new(),
            construction_plans: Vec::new(),
            trait_objects: Vec::new(),
            method_bindings: Vec::new(),
            method_binding_targets: Vec::new(),
            issues: Vec::new(),
        }
    }

    /// Creates a metadata-only registration for a generic source definition.
    pub fn type_definition<T: 'static>(definition: &'static TypeDefinitionRegistration<T>) -> Self {
        Self::metadata_only().with_type_definition(definition)
    }

    #[cfg(test)]
    pub(crate) fn with_type_definition_metadata(mut self, metadata: &'static TypeDefinitionMetadata) -> Self {
        self.type_definitions.push(metadata);
        self
    }

    /// Adds a generic definition without exposing its process-local identity.
    pub fn with_type_definition<T: 'static>(mut self, registration: &'static TypeDefinitionRegistration<T>) -> Self {
        self.type_definitions.push(registration.metadata());
        self
    }

    /// Adds source metadata for a reflected trait definition.
    pub fn with_trait_definition(mut self, metadata: &'static TraitDefinitionMetadata) -> Self {
        self.trait_definitions.push(metadata);
        self
    }

    /// Adds source metadata for a reflected impl definition.
    pub fn with_impl_definition(mut self, metadata: &'static ImplDefinitionMetadata) -> Self {
        self.impl_definitions.push(metadata);
        self
    }

    /// Associates a compiler-checked trait object with this registration.
    pub fn with_trait_object(mut self, registration: TraitObjectRegistration) -> Self {
        self.trait_objects.push(registration);
        self
    }

    pub(crate) fn type_definitions(&self) -> &[&'static TypeDefinitionMetadata] {
        &self.type_definitions
    }

    pub(crate) const fn shape(&self) -> Option<&'static TypeKindMetadata> {
        self.shape
    }

    pub(crate) fn trait_definitions(&self) -> &[&'static TraitDefinitionMetadata] {
        &self.trait_definitions
    }

    pub(crate) fn impl_definitions(&self) -> &[&'static ImplDefinitionMetadata] {
        &self.impl_definitions
    }

    pub(crate) fn trait_objects(&self) -> &[TraitObjectRegistration] {
        &self.trait_objects
    }

    pub(crate) fn with_capability(mut self, mut registration: CapabilityRegistration) -> Self {
        match registration.validation_error.take() {
            Some(error) => self.issues.push(ContextIssue::InvalidMetadata {
                item: ItemIdentity {
                    kind: ItemKind::Capability,
                    stable_id: StableItemId("invalid capability registration".into()),
                    display_name: "invalid capability registration".into(),
                },
                reason: MetadataValidationError::new(error.to_string()),
            }),
            None => self.capabilities.push(registration),
        }
        self
    }

    pub(crate) fn take_issues(&mut self) -> Vec<ContextIssue> {
        std::mem::take(&mut self.issues)
    }

    pub(crate) fn take_capabilities(&mut self) -> Vec<CapabilityRegistration> {
        std::mem::take(&mut self.capabilities)
    }

    #[cfg(test)]
    pub(crate) fn with_type_instance_id(mut self, definition: TypeDefinitionId) -> Self {
        self.type_instance_of = Some(definition);
        self
    }

    /// Associates the concrete type registration with a generic definition.
    pub fn with_type_instance<T: 'static>(mut self) -> Self {
        self.type_instance_of = Some(TypeDefinitionId::of::<T>());
        self
    }

    pub(crate) fn type_instance_of(&self) -> Option<TypeDefinitionId> {
        self.type_instance_of
    }

    /// Adds metadata for one concrete trait application.
    pub fn with_trait_application(mut self, application: &'static TraitApplicationMetadata) -> Self {
        self.trait_applications.push(application);
        self
    }

    pub(crate) fn trait_applications(&self) -> &[&'static TraitApplicationMetadata] {
        &self.trait_applications
    }

    /// Adds metadata for one concrete impl application.
    pub fn with_impl_application(mut self, application: &'static ImplApplicationMetadata) -> Self {
        self.impl_applications.push(application);
        self
    }

    pub(crate) fn impl_applications(&self) -> &[&'static ImplApplicationMetadata] {
        &self.impl_applications
    }

    #[cfg(test)]
    pub(crate) fn with_construction_plan(mut self, plan: &'static ConstructionPlan) -> Self {
        self.construction_plans.push(plan);
        self
    }

    pub(crate) fn construction_plans(&self) -> &[&'static ConstructionPlan] {
        &self.construction_plans
    }

    pub(crate) fn with_method_binding<T: Reflect>(mut self, method: EffectiveMethod) -> Self {
        self.method_bindings.push(method);
        self.method_binding_targets.push(TypeRecord::of::<T>());
        self
    }

    pub(crate) fn method_bindings(&self) -> &[EffectiveMethod] {
        &self.method_bindings
    }

    pub(crate) fn method_binding_targets(&self) -> &[TypeRecord] {
        &self.method_binding_targets
    }

    pub(crate) const fn type_record(&self) -> Option<TypeRecord> {
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
            self.type_record.map_or("metadata", TypeRecord::rust_name),
        )
    }
}

impl<T: Reflect + ?Sized> From<&'static TypeRegistration<T>> for Registration {
    fn from(_registration: &'static TypeRegistration<T>) -> Self {
        Self::new::<T>()
    }
}

impl From<CapabilityRegistration> for Registration {
    fn from(registration: CapabilityRegistration) -> Self {
        Self::metadata_only().with_capability(registration)
    }
}

/// A statically linked registration fragment materialized during context
/// construction.
#[derive(Clone, Copy)]
pub struct RegistrationFragment {
    sort_key: RegistrationSortKey,
    target_type_id: Option<fn() -> TypeId>,
    dependencies: fn() -> &'static [TypeId],
    includes_type: bool,
    include_in_linked_context: bool,
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
            include_in_linked_context: false,
            registration: Registration::of::<T>,
        }
    }

    /// Creates a metadata-only fragment with a typed registration factory.
    pub const fn for_global_registration(label: &'static str, registration: fn() -> Registration) -> Self {
        Self {
            sort_key: ("generated", None, "generated", Some(label), None, None, label),
            target_type_id: None,
            dependencies: no_dependencies,
            includes_type: false,
            include_in_linked_context: false,
            registration,
        }
    }

    /// Creates a global definition fragment that is also visible in linked
    /// contexts.
    pub const fn for_type_definition(label: &'static str, registration: fn() -> Registration) -> Self {
        Self {
            sort_key: ("generated", None, "generated", Some(label), None, None, label),
            target_type_id: None,
            dependencies: no_dependencies,
            includes_type: false,
            include_in_linked_context: true,
            registration,
        }
    }

    /// Creates a capability-only fragment included by linked contexts.
    pub const fn for_linked_capability(label: &'static str, registration: fn() -> Registration) -> Self {
        Self {
            sort_key: ("generated", None, "generated", Some(label), None, None, label),
            target_type_id: None,
            dependencies: no_dependencies,
            includes_type: false,
            include_in_linked_context: true,
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
            include_in_linked_context: false,
            registration,
        }
    }

    #[cfg(test)]
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
            include_in_linked_context: false,
            registration,
        }
    }

    pub(crate) fn materialize(self) -> Result<Registration, ContextError> {
        let registration = (self.registration)();
        if let Some(target) = self.target_type_id()
            && self.includes_type
            && registration.type_record().map(TypeRecord::type_id) != Some(target)
        {
            let name = format!("{target:?}");
            return Err(ContextError::from_issues(vec![ContextIssue::InvalidMetadata {
                item: ItemIdentity {
                    kind: ItemKind::Type,
                    stable_id: StableItemId(name.clone().into()),
                    display_name: name.into(),
                },
                reason: MetadataValidationError::new("type fragment target does not match its registration"),
            }])
            .expect("one context issue is nonempty"));
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

    pub(crate) const fn include_in_linked_context(&self) -> bool {
        self.include_in_linked_context
    }
}

fn no_dependencies() -> &'static [TypeId] {
    &[]
}

inventory::collect!(RegistrationFragment);

#[cfg(test)]
mod tests {
    use super::Registration;
    use super::RegistrationFragment;
    use super::TraitObjectRegistration;
    use super::TypeDefinitionRegistration;
    use crate::runtime::capability::CapabilitySpec;
    use crate::runtime::capability::capability_registration::CapabilityRegistration;
    use crate::runtime::capability::capability_registration_origin::CapabilityRegistrationOrigin;
    use crate::runtime::construct::ConstructionAvailability;
    use crate::runtime::construct::ConstructionPlan;
    use crate::runtime::construct::ConstructionProviders;
    use crate::runtime::construct::ConstructionShape;
    use crate::runtime::descriptor::Reflect;
    use crate::runtime::descriptor::TypeRegistration;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::generic::TypeDefinitionId;
    use crate::runtime::descriptor::generic::TypeDefinitionKind;
    use crate::runtime::descriptor::impl_descriptor::ImplApplicationMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplDefinitionMetadata;
    use crate::runtime::descriptor::impl_descriptor::ImplKind;
    use crate::runtime::descriptor::impl_descriptor::ImplQualifiers;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::TraitExpressionData;

    #[derive(Clone)]
    struct Reflected;

    struct Other;

    impl Reflect for Reflected {
        fn __reflect_registration() -> &'static TypeRegistration<Self> {
            static REGISTRATION: TypeRegistration<Reflected> = TypeRegistration::new();
            &REGISTRATION
        }
    }

    impl Reflect for Other {
        fn __reflect_registration() -> &'static TypeRegistration<Self> {
            static REGISTRATION: TypeRegistration<Other> = TypeRegistration::new();
            &REGISTRATION
        }
    }

    struct DefinitionMarker;
    struct TraitMarker;
    struct ImplMarker;
    struct InvalidCapability;

    impl CapabilitySpec for InvalidCapability {
        type Adapter = ();

        const ID: &'static str = "invalid/capability-id";
    }

    static EMPTY_GENERIC_DEFINITION: GenericDefinitionMetadata = GenericDefinitionMetadata {
        parameters: &[],
        predicates: &[],
    };
    static TYPE_DEFINITION: TypeDefinitionRegistration<DefinitionMarker> = TypeDefinitionRegistration::new(
        "Reflected",
        "registration_tests::Reflected",
        TypeDefinitionKind::Struct(crate::runtime::descriptor::generic::StructKind::Unit),
        &EMPTY_GENERIC_DEFINITION,
    );
    static TRAIT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<TraitMarker>(
        "Marker",
        "registration_tests::Marker",
        Visibility::Public,
        TraitCompleteness::Complete,
        &EMPTY_GENERIC_DEFINITION,
        &[],
        &[],
        &[],
        &[],
    );
    static TRAIT_EXPRESSION: TraitExpressionData = TraitExpressionData::new("External", &[], &[]);
    static TRAIT_APPLICATION: TraitApplicationMetadata =
        TraitApplicationMetadata::unresolved(&TRAIT_EXPRESSION, &[], &[]);
    static TARGET_EXPRESSION: ExpressionData = ExpressionData::SelfType;
    static IMPL_DEFINITION: ImplDefinitionMetadata = ImplDefinitionMetadata::new::<ImplMarker>(
        ImplKind::Inherent,
        &TARGET_EXPRESSION,
        None,
        &EMPTY_GENERIC_DEFINITION,
        ImplQualifiers::new(false),
        &[],
        &[],
        &[],
    );
    static IMPL_APPLICATION: ImplApplicationMetadata =
        ImplApplicationMetadata::for_target::<Reflected>(&IMPL_DEFINITION, None, &[], &[], &[], &[], &[]);
    static EMPTY_DEPENDENCIES: [std::any::TypeId; 0] = [];
    static REFLECTED_DEPENDENCIES: [std::any::TypeId; 1] = [std::any::TypeId::of::<Reflected>()];
    static EMPTY_PLAN: ConstructionPlan = ConstructionPlan::for_type::<Reflected>(
        ConstructionShape::Unit,
        &[],
        ConstructionProviders::from_runtime_providers(None, None),
        ConstructionAvailability::Available,
    );

    fn metadata_registration() -> Registration {
        Registration::metadata_only()
    }

    fn reflected_registration() -> Registration {
        Registration::of::<Reflected>()
    }

    fn reflected_dependencies() -> &'static [std::any::TypeId] {
        &REFLECTED_DEPENDENCIES
    }

    fn no_dependencies() -> &'static [std::any::TypeId] {
        &EMPTY_DEPENDENCIES
    }

    #[test]
    fn registration_builders_preserve_metadata_and_explicit_defaults() {
        let empty = Registration::metadata_only();
        assert!(empty.type_record().is_none());
        assert!(empty.shape().is_none());
        assert!(empty.type_definitions().is_empty());
        assert!(empty.trait_definitions().is_empty());
        assert!(empty.impl_definitions().is_empty());
        assert!(empty.trait_applications().is_empty());
        assert!(empty.impl_applications().is_empty());
        assert!(empty.construction_plans().is_empty());
        assert!(empty.trait_objects().is_empty());
        assert!(empty.method_bindings().is_empty());
        assert!(empty.method_binding_targets().is_empty());
        assert_eq!(
            empty.origin(),
            &crate::runtime::identity::RegistrationOrigin {
                crate_name: "generated",
                crate_version: None,
                module_path: "generated metadata",
                file: None,
                line: None,
                column: None,
            }
        );
        assert_eq!(
            empty.sort_key(),
            ("generated", None, "generated metadata", None, None, None, "metadata")
        );

        let explicit = Registration::of::<Reflected>();
        let type_record = explicit.type_record().expect("typed registration has a type record");
        assert_eq!(type_record.type_id(), std::any::TypeId::of::<Reflected>());
        assert_eq!(type_record.rust_name(), std::any::type_name::<Reflected>());
        assert_eq!(
            explicit.origin(),
            &crate::runtime::identity::RegistrationOrigin {
                crate_name: "explicit",
                crate_version: None,
                module_path: std::any::type_name::<Reflected>(),
                file: None,
                line: None,
                column: None,
            }
        );
        let from_type_registration = Registration::from(Reflected::__reflect_registration());
        assert_eq!(
            from_type_registration.type_record().unwrap().type_id(),
            std::any::TypeId::of::<Reflected>()
        );

        let with_definition = Registration::type_definition(&TYPE_DEFINITION);
        assert!(std::ptr::eq(
            with_definition.type_definitions()[0],
            TYPE_DEFINITION.metadata()
        ));
        assert_eq!(with_definition.type_instance_of(), None);

        let chained = Registration::of::<Reflected>()
            .with_type_definition(&TYPE_DEFINITION)
            .with_type_instance::<DefinitionMarker>()
            .with_trait_definition(&TRAIT_DEFINITION)
            .with_impl_definition(&IMPL_DEFINITION)
            .with_trait_application(&TRAIT_APPLICATION)
            .with_impl_application(&IMPL_APPLICATION)
            .with_construction_plan(&EMPTY_PLAN)
            .with_trait_object(TraitObjectRegistration::of::<dyn Send>("dyn Send"));
        assert!(std::ptr::eq(chained.type_definitions()[0], TYPE_DEFINITION.metadata()));
        assert_eq!(
            chained.type_instance_of(),
            Some(TypeDefinitionId::of::<DefinitionMarker>())
        );
        assert!(std::ptr::eq(chained.trait_definitions()[0], &TRAIT_DEFINITION));
        assert!(std::ptr::eq(chained.impl_definitions()[0], &IMPL_DEFINITION));
        assert!(std::ptr::eq(chained.trait_applications()[0], &TRAIT_APPLICATION));
        assert!(std::ptr::eq(chained.impl_applications()[0], &IMPL_APPLICATION));
        assert_eq!(chained.construction_plans().len(), 1);
        assert_eq!(chained.trait_objects().len(), 1);
        assert_eq!(chained.trait_objects()[0].trait_path(), "dyn Send");
        assert_eq!(chained.trait_objects()[0].type_id(), std::any::TypeId::of::<dyn Send>());
        assert_eq!(
            chained.trait_objects()[0].rust_name(),
            std::any::type_name::<dyn Send>()
        );
    }

    #[test]
    fn capability_builder_transfers_and_drains_registration_state() {
        let capability = CapabilityRegistration::clone::<Reflected>(CapabilityRegistrationOrigin::Explicit(
            crate::runtime::identity::RegistrationOrigin {
                crate_name: "registration-tests",
                crate_version: None,
                module_path: "capability_builder_transfers_and_drains_registration_state",
                file: None,
                line: None,
                column: None,
            },
        ))
        .expect("clone capability is valid for this fixture");
        let mut registration = Registration::of::<Reflected>().with_capability(capability);
        let capabilities = registration.take_capabilities();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].id_str, "qubit.reflect.clone");
        assert!(registration.take_capabilities().is_empty());
        assert!(registration.take_issues().is_empty());
    }

    #[test]
    fn invalid_capability_registration_becomes_a_context_issue() {
        let invalid = CapabilityRegistration::for_type::<Reflected, InvalidCapability>(());
        let mut registration = Registration::metadata_only().with_capability(invalid);
        assert!(registration.capabilities.is_empty());
        let issues = registration.take_issues();
        assert_eq!(issues.len(), 1);
        assert!(matches!(
            &issues[0],
            crate::runtime::error::ContextIssue::InvalidMetadata { item, reason }
                if item.kind == crate::runtime::identity::ItemKind::Capability
                    && reason.to_string().contains("invalid")
        ));
        assert!(registration.take_issues().is_empty());
    }

    #[test]
    fn registration_fragments_retain_targets_linkage_and_snapshot_each_factory_result() {
        let reflected = RegistrationFragment::for_reflected_type::<Reflected>("reflected");
        assert_eq!(reflected.target_type_id(), Some(std::any::TypeId::of::<Reflected>()));
        assert!(reflected.includes_type());
        assert!(!reflected.include_in_linked_context());
        assert_eq!(reflected.dependencies(), &[]);
        assert_eq!(
            reflected.sort_key(),
            (
                "generated",
                None,
                "generated",
                Some("reflected"),
                None,
                None,
                "reflected"
            )
        );
        let first = reflected
            .materialize()
            .expect("matching reflected fragment materializes");
        let second = reflected
            .materialize()
            .expect("copied fragment materializes independently");
        assert_eq!(
            first.type_record().unwrap().type_id(),
            second.type_record().unwrap().type_id()
        );
        assert!(!std::ptr::eq(&first, &second));

        let global = RegistrationFragment::for_global_registration("metadata", metadata_registration);
        assert_eq!(global.target_type_id(), None);
        assert!(!global.includes_type());
        assert!(!global.include_in_linked_context());
        assert!(global.dependencies().is_empty());
        assert!(global.materialize().unwrap().type_record().is_none());

        let definition = RegistrationFragment::for_type_definition("definition", metadata_registration);
        assert!(!definition.includes_type());
        assert!(definition.include_in_linked_context());
        assert!(definition.materialize().unwrap().type_record().is_none());

        let target = RegistrationFragment::for_target::<Reflected>(
            ("tests", None, "registration_tests", None, None, None, "Reflected"),
            reflected_dependencies,
            metadata_registration,
        );
        assert_eq!(target.target_type_id(), Some(std::any::TypeId::of::<Reflected>()));
        assert_eq!(target.dependencies(), &REFLECTED_DEPENDENCIES);
        assert!(!target.includes_type());
        assert!(target.materialize().unwrap().type_record().is_none());
    }

    #[test]
    fn typed_fragment_validation_reports_a_mismatched_type_identity() {
        let matching = RegistrationFragment::for_type::<Reflected>(
            ("tests", None, "registration_tests", None, None, None, "Reflected"),
            no_dependencies,
            reflected_registration,
        );
        assert_eq!(
            matching.materialize().unwrap().type_record().unwrap().type_id(),
            std::any::TypeId::of::<Reflected>()
        );

        let mismatched = RegistrationFragment::for_type::<Reflected>(
            ("tests", None, "registration_tests", None, None, None, "Other"),
            no_dependencies,
            Registration::of::<Other>,
        );
        let error = match mismatched.materialize() {
            Ok(_) => panic!("a typed fragment rejects the wrong registration target"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("type fragment target does not match its registration")
        );
        assert_eq!(error.issues().len(), 1);
        assert!(matches!(
            error.issues()[0],
            crate::runtime::error::ContextIssue::InvalidMetadata { .. }
        ));
    }
}
