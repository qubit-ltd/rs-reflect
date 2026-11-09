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

/// Runtime getters that bind a reflected type to its ID, name, and descriptor.
#[derive(Clone, Copy)]
pub(crate) struct TypeRecord {
    /// Function returning the concrete Rust `TypeId`.
    type_id: fn() -> TypeId,
    /// Function returning the compiler's fully qualified type name.
    rust_name: fn() -> &'static str,
    /// Function resolving the type descriptor in a supplied context.
    descriptor: for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>,
}

impl TypeRecord {
    /// Captures compiler-derived identity functions for a reflected type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete or dynamically sized type.
    fn of<T: Reflect + ?Sized>() -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            descriptor: descriptor_for::<T>,
        }
    }

    /// Returns the concrete type ID represented by this record.
    pub(crate) fn type_id(self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the compiler's fully qualified type name.
    pub(crate) fn rust_name(self) -> &'static str {
        (self.rust_name)()
    }

    /// Resolves this record's type descriptor in `context`.
    ///
    /// # Parameters
    ///
    /// * `context` — context used to resolve the registered type metadata.
    pub(crate) fn descriptor(self, context: &ReflectionContext) -> TypeDescriptor<'_> {
        (self.descriptor)(context)
    }
}

/// Resolves the reflected registration metadata for `T` in a context.
///
/// # Type Parameters
///
/// * `T` — reflected concrete or dynamically sized type.
///
/// # Parameters
///
/// * `context` — context that owns the returned descriptor.
fn descriptor_for<'ctx, T: Reflect + ?Sized>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::new(T::__reflect_registration().metadata(), context)
}

/// A bundle of type, definition, application, and capability metadata for a
/// context.
pub struct Registration {
    /// Optional concrete type contributed by this registration.
    type_record: Option<TypeRecord>,
    /// Optional generated source shape for the concrete type.
    shape: Option<&'static TypeKindMetadata>,
    /// Provenance used to order and explain this registration.
    origin: RegistrationOrigin,
    /// Source generic type definitions contributed by the registration.
    type_definitions: Vec<&'static TypeDefinitionMetadata>,
    /// Source trait definitions contributed by the registration.
    trait_definitions: Vec<&'static TraitDefinitionMetadata>,
    /// Impl declarations contributed by the registration.
    impl_definitions: Vec<&'static ImplDefinitionMetadata>,
    /// Typed capability registrations to validate and index.
    capabilities: Vec<CapabilityRegistration>,
    /// Generic definition instantiated by the concrete type, if any.
    type_instance_of: Option<TypeDefinitionId>,
    /// Concrete trait applications contributed by the registration.
    trait_applications: Vec<&'static TraitApplicationMetadata>,
    /// Concrete impl applications contributed by the registration.
    impl_applications: Vec<&'static ImplApplicationMetadata>,
    /// Construction and update plans associated with the concrete type.
    construction_plans: Vec<&'static ConstructionPlan>,
    /// Compiler-checked trait-object registrations.
    trait_objects: Vec<TraitObjectRegistration>,
    /// Effective method bindings contributed by generated metadata.
    method_bindings: Vec<EffectiveMethod>,
    /// Concrete target types corresponding to `method_bindings`.
    method_binding_targets: Vec<TypeRecord>,
    /// Validation issues retained until context construction.
    issues: Vec<ContextIssue>,
}

/// A compiler-checked trait-object type recorded in a reflection context.
#[derive(Clone, Copy)]
pub struct TraitObjectRegistration {
    /// Function returning the compiler identity of the exact object type.
    type_id: fn() -> TypeId,
    /// Function returning the compiler type name for diagnostics.
    rust_name: fn() -> &'static str,
    /// Source trait path associated with the object type.
    trait_path: &'static str,
}

impl TraitObjectRegistration {
    /// Creates a registration bound to the exact Rust trait-object type `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — exact trait-object type whose identity is recorded.
    ///
    /// # Parameters
    ///
    /// * `trait_path` — source path describing the object's principal trait.
    pub const fn of<T: ?Sized + 'static>(trait_path: &'static str) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            trait_path,
        }
    }

    /// Returns the compiler identity of the registered trait-object type.
    pub(crate) fn type_id(self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the compiler type name used in validation diagnostics.
    pub(crate) fn rust_name(self) -> &'static str {
        (self.rust_name)()
    }

    /// Returns the source trait path recorded for this object type.
    pub(crate) const fn trait_path(self) -> &'static str {
        self.trait_path
    }
}

/// A generic definition registration whose process-local identity is bound to
/// `T`.
///
/// # Type Parameters
///
/// * `T` — marker type that supplies the process-local definition identity.
pub struct TypeDefinitionRegistration<T: 'static> {
    /// Static source metadata for the generic definition.
    metadata: TypeDefinitionMetadata,
    /// Associates the definition with its marker without storing a value.
    _marker: PhantomData<fn() -> T>,
}

impl<T: 'static> TypeDefinitionRegistration<T> {
    /// Creates source metadata for a generic definition, binding identity to
    /// its marker type.
    ///
    /// # Parameters
    ///
    /// * `name` — source identifier of the generic definition.
    /// * `path` — fully qualified source path of the definition.
    /// * `kind` — source kind and visibility metadata.
    /// * `generics` — generic parameters and predicates declared by the item.
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

    /// Borrows the static metadata retained by this registration.
    pub(crate) const fn metadata(&'static self) -> &'static TypeDefinitionMetadata {
        &self.metadata
    }
}

impl Registration {
    /// Creates a registration containing the reflected metadata for `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete or dynamically sized type.
    pub fn of<T: Reflect + ?Sized>() -> Self {
        Self::new::<T>()
    }

    /// Builds the type registration from `T`'s generated reflection metadata.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected concrete or dynamically sized type.
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
    ///
    /// # Type Parameters
    ///
    /// * `T` — marker type identifying the definition.
    ///
    /// # Parameters
    ///
    /// * `definition` — static registration containing the definition metadata.
    pub fn type_definition<T: 'static>(definition: &'static TypeDefinitionRegistration<T>) -> Self {
        Self::metadata_only().with_type_definition(definition)
    }

    #[cfg(test)]
    /// Adds raw definition metadata for tests that do not use a marker wrapper.
    ///
    /// # Parameters
    ///
    /// * `metadata` — static metadata describing the generic definition.
    pub(crate) fn with_type_definition_metadata(mut self, metadata: &'static TypeDefinitionMetadata) -> Self {
        self.type_definitions.push(metadata);
        self
    }

    /// Adds a generic definition without exposing its process-local identity.
    ///
    /// # Type Parameters
    ///
    /// * `T` — marker type that owns the registered definition identity.
    ///
    /// # Parameters
    ///
    /// * `registration` — static source definition to add.
    pub fn with_type_definition<T: 'static>(mut self, registration: &'static TypeDefinitionRegistration<T>) -> Self {
        self.type_definitions.push(registration.metadata());
        self
    }

    /// Adds source metadata for a reflected trait definition.
    ///
    /// # Parameters
    ///
    /// * `metadata` — static trait definition metadata to include.
    pub fn with_trait_definition(mut self, metadata: &'static TraitDefinitionMetadata) -> Self {
        self.trait_definitions.push(metadata);
        self
    }

    /// Adds source metadata for a reflected impl definition.
    ///
    /// # Parameters
    ///
    /// * `metadata` — static impl definition metadata to include.
    pub fn with_impl_definition(mut self, metadata: &'static ImplDefinitionMetadata) -> Self {
        self.impl_definitions.push(metadata);
        self
    }

    /// Associates a compiler-checked trait object with this registration.
    ///
    /// # Parameters
    ///
    /// * `registration` — exact object type and principal trait path to add.
    pub fn with_trait_object(mut self, registration: TraitObjectRegistration) -> Self {
        self.trait_objects.push(registration);
        self
    }

    /// Returns the generic definitions carried by this registration.
    pub(crate) fn type_definitions(&self) -> &[&'static TypeDefinitionMetadata] {
        &self.type_definitions
    }

    /// Returns the generated shape metadata for the registered type, if
    /// present.
    pub(crate) const fn shape(&self) -> Option<&'static TypeKindMetadata> {
        self.shape
    }

    /// Returns the trait definitions carried by this registration.
    pub(crate) fn trait_definitions(&self) -> &[&'static TraitDefinitionMetadata] {
        &self.trait_definitions
    }

    /// Returns the impl definitions carried by this registration.
    pub(crate) fn impl_definitions(&self) -> &[&'static ImplDefinitionMetadata] {
        &self.impl_definitions
    }

    /// Returns compiler-checked trait-object metadata carried by this
    /// registration.
    pub(crate) fn trait_objects(&self) -> &[TraitObjectRegistration] {
        &self.trait_objects
    }

    /// Adds a capability registration or retains its validation error as an
    /// issue.
    ///
    /// # Parameters
    ///
    /// * `registration` — capability metadata and any validation failure to
    ///   record.
    pub(crate) fn with_capability(mut self, mut registration: CapabilityRegistration) -> Self {
        match registration.validation_error.take() {
            Some(error) => self.issues.push(ContextIssue::InvalidMetadata {
                item: ItemIdentity {
                    kind: ItemKind::Capability,
                    stable_id: StableItemId(registration.id_str.into()),
                    display_name: registration.id_str.into(),
                },
                reason: MetadataValidationError::new(error.to_string()),
            }),
            None => self.capabilities.push(registration),
        }
        self
    }

    /// Drains issues collected while assembling this registration.
    pub(crate) fn take_issues(&mut self) -> Vec<ContextIssue> {
        std::mem::take(&mut self.issues)
    }

    /// Drains capability registrations for context-wide validation.
    pub(crate) fn take_capabilities(&mut self) -> Vec<CapabilityRegistration> {
        std::mem::take(&mut self.capabilities)
    }

    #[cfg(test)]
    /// Associates this type with a generic definition using its internal
    /// identity.
    ///
    /// # Parameters
    ///
    /// * `definition` — identity of the generic definition instantiated by this
    ///   type.
    pub(crate) fn with_type_instance_id(mut self, definition: TypeDefinitionId) -> Self {
        self.type_instance_of = Some(definition);
        self
    }

    /// Associates the concrete type registration with a generic definition.
    ///
    /// # Type Parameters
    ///
    /// * `T` — marker type identifying the generic definition.
    pub fn with_type_instance<T: 'static>(mut self) -> Self {
        self.type_instance_of = Some(TypeDefinitionId::of::<T>());
        self
    }

    /// Returns the generic definition instantiated by the registered type, if
    /// any.
    pub(crate) fn type_instance_of(&self) -> Option<TypeDefinitionId> {
        self.type_instance_of
    }

    /// Adds metadata for one concrete trait application.
    ///
    /// # Parameters
    ///
    /// * `application` — metadata describing the concrete trait application.
    pub fn with_trait_application(mut self, application: &'static TraitApplicationMetadata) -> Self {
        self.trait_applications.push(application);
        self
    }

    /// Returns trait applications carried by this registration.
    pub(crate) fn trait_applications(&self) -> &[&'static TraitApplicationMetadata] {
        &self.trait_applications
    }

    /// Adds metadata for one concrete impl application.
    ///
    /// # Parameters
    ///
    /// * `application` — metadata describing the concrete impl application.
    pub fn with_impl_application(mut self, application: &'static ImplApplicationMetadata) -> Self {
        self.impl_applications.push(application);
        self
    }

    /// Returns impl applications carried by this registration.
    pub(crate) fn impl_applications(&self) -> &[&'static ImplApplicationMetadata] {
        &self.impl_applications
    }

    #[cfg(test)]
    /// Adds a generated construction plan for tests.
    ///
    /// # Parameters
    ///
    /// * `plan` — static construction plan associated with this registration.
    pub(crate) fn with_construction_plan(mut self, plan: &'static ConstructionPlan) -> Self {
        self.construction_plans.push(plan);
        self
    }

    /// Returns construction plans carried by this registration.
    pub(crate) fn construction_plans(&self) -> &[&'static ConstructionPlan] {
        &self.construction_plans
    }

    /// Adds an effective method binding for a reflected target type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected target type for which the binding is effective.
    ///
    /// # Parameters
    ///
    /// * `method` — effective method metadata to attach to this registration.
    pub(crate) fn with_method_binding<T: Reflect>(mut self, method: EffectiveMethod) -> Self {
        self.method_bindings.push(method);
        self.method_binding_targets.push(TypeRecord::of::<T>());
        self
    }

    /// Returns generated effective method bindings carried by this
    /// registration.
    pub(crate) fn method_bindings(&self) -> &[EffectiveMethod] {
        &self.method_bindings
    }

    /// Returns the target type records aligned with the method bindings.
    pub(crate) fn method_binding_targets(&self) -> &[TypeRecord] {
        &self.method_binding_targets
    }

    /// Returns the concrete type record contributed by this registration.
    pub(crate) const fn type_record(&self) -> Option<TypeRecord> {
        self.type_record
    }

    /// Returns source provenance used to order and diagnose this registration.
    pub(crate) fn origin(&self) -> &RegistrationOrigin {
        &self.origin
    }

    /// Returns the stable ordering key derived from source provenance and type
    /// name.
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
    /// Converts a static type registration into context registration data.
    ///
    /// # Parameters
    ///
    /// * `_registration` — static registration whose type metadata is copied.
    fn from(_registration: &'static TypeRegistration<T>) -> Self {
        Self::new::<T>()
    }
}

impl From<CapabilityRegistration> for Registration {
    /// Converts a typed capability registration into context registration data.
    ///
    /// # Parameters
    ///
    /// * `registration` — capability registration to include.
    fn from(registration: CapabilityRegistration) -> Self {
        Self::metadata_only().with_capability(registration)
    }
}

/// A statically linked registration fragment materialized during context
/// construction.
#[derive(Clone, Copy)]
pub struct RegistrationFragment {
    /// Stable source-order key used before materialization.
    sort_key: RegistrationSortKey,
    /// Optional target identity used to validate the materialized payload.
    target_type_id: Option<fn() -> TypeId>,
    /// Dependency IDs required before the fragment can be included.
    dependencies: fn() -> &'static [TypeId],
    /// Whether materialization is expected to contribute the target type.
    includes_type: bool,
    /// Whether global linked-context construction includes this fragment.
    include_in_linked_context: bool,
    /// Callback that creates the registration payload.
    registration: fn() -> Registration,
}

impl RegistrationFragment {
    /// Creates a type fragment with a runtime controlled stable origin.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected type contributed by this fragment.
    ///
    /// # Parameters
    ///
    /// * `label` — stable label used as the fragment source location.
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
    ///
    /// # Parameters
    ///
    /// * `label` — stable label used to order the fragment.
    /// * `registration` — callback that creates its metadata-only payload.
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
    ///
    /// # Parameters
    ///
    /// * `label` — stable source label for the definition fragment.
    /// * `registration` — callback that creates the definition metadata.
    pub const fn for_type_definition(label: &'static str, registration: fn() -> Registration) -> Self {
        Self::for_linked_definition(label, registration)
    }

    /// Creates a global definition fragment included by linked contexts.
    ///
    /// # Parameters
    ///
    /// * `label` — stable source label for the definition fragment.
    /// * `registration` — callback that creates the definition metadata.
    pub(crate) const fn for_linked_definition(label: &'static str, registration: fn() -> Registration) -> Self {
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
    ///
    /// # Parameters
    ///
    /// * `label` — stable source label for the capability fragment.
    /// * `registration` — callback that creates the capability payload.
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

    /// Creates a metadata fragment associated with a concrete target identity.
    ///
    /// # Type Parameters
    ///
    /// * `T` — target type whose identity controls fragment ordering.
    ///
    /// # Parameters
    ///
    /// * `sort_key` — source-order key assigned to the fragment.
    /// * `dependencies` — function returning prerequisite type IDs.
    /// * `registration` — callback that materializes the metadata payload.
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
    /// Creates a test fragment that contributes its target type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected target type contributed by the fragment.
    ///
    /// # Parameters
    ///
    /// * `sort_key` — source-order key assigned to the fragment.
    /// * `dependencies` — function returning prerequisite type IDs.
    /// * `registration` — callback that materializes the type payload.
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

    /// Runs the payload callback and checks any declared target type.
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

    /// Returns the stable source-order key used to sort this fragment.
    pub(crate) const fn sort_key(&self) -> RegistrationSortKey {
        self.sort_key
    }

    /// Returns the target type ID, if this fragment is associated with a type.
    pub(crate) fn target_type_id(&self) -> Option<TypeId> {
        self.target_type_id.map(|identity| identity())
    }

    /// Returns prerequisite type IDs declared for this fragment.
    pub(crate) fn dependencies(&self) -> &'static [TypeId] {
        (self.dependencies)()
    }

    /// Reports whether the fragment is expected to contribute its target type.
    pub(crate) const fn includes_type(&self) -> bool {
        self.includes_type
    }

    /// Reports whether global linked contexts include this fragment.
    pub(crate) const fn include_in_linked_context(&self) -> bool {
        self.include_in_linked_context
    }
}

/// Returns the empty dependency list for fragments without prerequisites.
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
    fn test_registration_builders_preserve_metadata_and_explicit_defaults() {
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
    fn test_capability_builder_transfers_and_drains_registration_state() {
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
    fn test_invalid_capability_registration_becomes_a_context_issue() {
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
    fn test_registration_fragments_retain_targets_linkage_and_snapshot_each_factory_result() {
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
    fn test_typed_fragment_validation_reports_a_mismatched_type_identity() {
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
