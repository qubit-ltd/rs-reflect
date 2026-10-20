// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::runtime::capability::capabilities::Capabilities;
use crate::runtime::capability::capability_target_id::CapabilityTargetId;
use crate::runtime::construct::ConstructionDescriptor;
use crate::runtime::context::collections::TraitDefinitions;
use crate::runtime::context::collections::Traits;
use crate::runtime::context::collections::TypeDefinitions;
use crate::runtime::context::collections::Types;
use crate::runtime::context::handle::registration::TraitObjectRegistration;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
use crate::runtime::descriptor::generic::TypeDefinitionId;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitionDescriptor;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitionId;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitions;
use crate::runtime::descriptor::impl_descriptor::ImplDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionId;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitDescriptor;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::error::ContextError;
use crate::runtime::error::LookupError;
use crate::runtime::error::LookupQuery;
use crate::runtime::error::LookupSubject;
use crate::runtime::identity::IntoLookupCandidate;
use crate::runtime::identity::ItemIdentity;
use crate::runtime::identity::ItemKind;
use crate::runtime::identity::LookupCandidate;
use crate::runtime::identity::StableItemId;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::lookup::Lookup;

#[path = "builder.rs"]
pub(crate) mod builder;
#[path = "data.rs"]
pub(crate) mod data;
#[path = "global.rs"]
pub(crate) mod global;
#[path = "indexes.rs"]
pub(crate) mod indexes;
#[path = "registration.rs"]
pub(crate) mod registration;
use data::ContextData;
use global::GlobalContext;

static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Unique process-local identity of a reflection context instance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReflectionContextId(u64);

impl ReflectionContextId {
    fn next() -> Self {
        Self(NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A context containing the registrations and indexes used for reflection
/// queries.
pub struct ReflectionContext {
    id: ReflectionContextId,
    state: ContextState,
}

/// Context-bound description of one registered Rust trait-object type.
#[derive(Clone, Copy)]
pub struct TraitObjectDescriptor<'ctx> {
    registration: TraitObjectRegistration,
    _context: std::marker::PhantomData<&'ctx ReflectionContext>,
}

/// Context-bound declaration facts for one marker-identified trait.
#[derive(Clone, Copy)]
pub struct RegisteredTraitDescriptor<'ctx> {
    metadata: &'static TraitDefinitionMetadata,
    _context: std::marker::PhantomData<&'ctx ReflectionContext>,
}

impl RegisteredTraitDescriptor<'_> {
    /// Returns the source trait path retained by the registration.
    pub const fn path(self) -> &'static str {
        self.metadata.path
    }

    /// Returns the declared trait name retained by the registration.
    pub const fn name(self) -> &'static str {
        self.metadata.name
    }

    /// Returns whether complete source metadata is available.
    pub const fn completeness(self) -> TraitCompleteness {
        self.metadata.completeness
    }
}

impl TraitObjectDescriptor<'_> {
    /// Returns the source path supplied when the trait object was registered.
    pub const fn trait_path(self) -> &'static str {
        self.registration.trait_path()
    }

    /// Returns Rust's compiler-provided type name for the registered object.
    pub fn rust_type_name(self) -> &'static str {
        self.registration.rust_name()
    }
}

enum ContextState {
    Explicit(Arc<ContextData>),
    Global(&'static GlobalContext),
}

impl ReflectionContext {
    /// Looks up a trait declaration by its opaque marker identity.
    pub fn trait_definition_for<D: 'static>(&self) -> Result<Option<RegisteredTraitDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .trait_definition(TraitDefinitionId::of::<D>())
            .map(|metadata| RegisteredTraitDescriptor {
                metadata,
                _context: std::marker::PhantomData,
            }))
    }

    /// Looks up a uniquely registered incomplete external trait by source path.
    pub fn external_trait_definition(&self, path: &str) -> Result<Option<RegisteredTraitDescriptor<'_>>, ContextError> {
        Ok(self
            .external_trait_definition_by_path(path)?
            .map(|metadata| RegisteredTraitDescriptor {
                metadata,
                _context: std::marker::PhantomData,
            }))
    }

    /// Returns retained trait applications whose original source path matches
    /// exactly.
    ///
    /// Applications without a registered identity handle remain queryable and
    /// report [`TraitCompleteness::ExternalIncomplete`] through their
    /// descriptors.
    pub fn trait_applications_by_path(&self, path: &str) -> Result<Box<[TraitDescriptor<'_>]>, ContextError> {
        Ok(self
            .data()?
            .trait_applications_by_path(path)
            .iter()
            .map(|metadata| TraitDescriptor::new(metadata, self))
            .collect())
    }

    /// Returns all registered concrete trait applications in registration
    /// order.
    pub fn trait_applications(&self) -> Result<Traits<'_>, ContextError> {
        let data = self.data()?;
        Ok(Traits::new(
            data.trait_applications_all()
                .iter()
                .map(|metadata| TraitDescriptor::new(metadata, self))
                .collect(),
        ))
    }

    /// Looks up a compiler-checked registered trait-object type.
    pub fn trait_object<T: ?Sized + 'static>(&self) -> Result<Option<TraitObjectDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .trait_object(TypeId::of::<T>())
            .map(|registration| TraitObjectDescriptor {
                registration,
                _context: std::marker::PhantomData,
            }))
    }

    /// Returns the process-wide lazily initialized reflection context.
    pub fn global() -> &'static Self {
        global::global_context()
    }

    #[cfg(test)]
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self::from_data(ContextData::empty())
    }

    pub(crate) fn from_data(data: ContextData) -> Self {
        Self {
            id: ReflectionContextId::next(),
            state: ContextState::Explicit(Arc::new(data)),
        }
    }

    fn new_global() -> Self {
        static GLOBAL_STATE: std::sync::OnceLock<GlobalContext> = std::sync::OnceLock::new();
        Self {
            id: ReflectionContextId::next(),
            state: ContextState::Global(GLOBAL_STATE.get_or_init(GlobalContext::new)),
        }
    }

    /// Initializes the global context if needed and returns this context.
    ///
    /// Returns an error if linked registration materialization or validation
    /// fails.
    pub fn initialize(&self) -> Result<&Self, ContextError> {
        if let ContextState::Global(state) = &self.state {
            state.data()?;
        }
        Ok(self)
    }

    /// Reports whether this context's registrations have been initialized.
    pub fn is_initialized(&self) -> bool {
        match &self.state {
            ContextState::Explicit(_) => true,
            ContextState::Global(state) => state.is_initialized(),
        }
    }

    /// Returns this context's unique process-local identity.
    pub fn id(&self) -> ReflectionContextId {
        self.id
    }

    pub(crate) fn data(&self) -> Result<Arc<ContextData>, ContextError> {
        match &self.state {
            ContextState::Explicit(data) => Ok(Arc::clone(data)),
            ContextState::Global(state) => state.data(),
        }
    }

    /// Finds a registered concrete type by its Rust `TypeId`.
    ///
    /// Returns `None` when the context has no registration for `id`.
    pub fn find_type_by_id(&self, id: TypeId) -> Result<Option<TypeDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .type_by_id(id)
            .map(|type_record| type_record.descriptor(self)))
    }

    pub(crate) fn contains_type_id(&self, id: TypeId) -> Result<bool, ContextError> {
        Ok(self.data()?.contains_type_id(id))
    }

    /// Looks up concrete types by their full reflected name.
    pub fn lookup_type_by_name(&self, name: &str) -> Result<Lookup<TypeDescriptor<'_>>, ContextError> {
        let data = self.data()?;
        Ok(self.lookup_by_indexes(&data, data.indexes.types_by_name.get(name).map(Box::as_ref)))
    }

    /// Looks up concrete types by their final reflected name segment.
    pub fn lookup_type_by_short_name(&self, name: &str) -> Result<Lookup<TypeDescriptor<'_>>, ContextError> {
        let data = self.data()?;
        Ok(self.lookup_by_indexes(&data, data.indexes.types_by_short_name.get(name).map(Box::as_ref)))
    }

    /// Requires a registered concrete type with the given Rust `TypeId`.
    pub fn type_by_id(&self, id: TypeId) -> Result<TypeDescriptor<'_>, LookupError> {
        self.find_type_by_id(id)
            .map_err(LookupError::Context)?
            .ok_or(LookupError::Missing {
                subject: LookupSubject::Type,
                query: LookupQuery::TypeId(id),
            })
    }

    /// Requires exactly one concrete type with the given full reflected name.
    pub fn type_by_name(&self, name: &str) -> Result<TypeDescriptor<'_>, LookupError> {
        self.lookup_type_by_name(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Type, LookupQuery::Name(name.into()))
    }

    /// Requires exactly one concrete type with the given short name.
    pub fn type_by_short_name(&self, name: &str) -> Result<TypeDescriptor<'_>, LookupError> {
        self.lookup_type_by_short_name(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Type, LookupQuery::ShortName(name.into()))
    }

    /// Returns all registered concrete types in context order.
    pub fn types(&self) -> Result<Types<'_>, ContextError> {
        let data = self.data()?;
        Ok(Types::new(
            data.types
                .iter()
                .map(|type_record| type_record.descriptor(self))
                .collect(),
        ))
    }

    /// Returns all registered source type definitions in path order.
    pub fn type_definitions(&self) -> Result<TypeDefinitions<'_>, ContextError> {
        let data = self.data()?;
        Ok(TypeDefinitions::new(
            data.type_definitions()
                .into_iter()
                .map(|metadata| TypeDefinitionDescriptor::new(metadata, self))
                .collect(),
        ))
    }

    /// Looks up a source type definition by its full reflected path.
    pub fn lookup_type_definition_by_name(
        &self,
        path: &str,
    ) -> Result<Lookup<TypeDefinitionDescriptor<'_>>, ContextError> {
        let data = self.data()?;
        let candidates = data
            .indexes
            .type_definitions_by_path
            .get(path)
            .into_iter()
            .flatten()
            .filter_map(|id| data.type_definition(*id))
            .map(|metadata| TypeDefinitionDescriptor::new(metadata, self))
            .collect::<Vec<_>>();
        Ok(match candidates.as_slice() {
            [] => Lookup::Missing,
            [only] => Lookup::Unique(*only),
            _ => Lookup::Ambiguous(candidates.into_boxed_slice()),
        })
    }

    /// Requires exactly one source type definition with the given full path.
    pub fn type_definition_by_name(&self, path: &str) -> Result<TypeDefinitionDescriptor<'_>, LookupError> {
        self.lookup_type_definition_by_name(path)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::TypeDefinition, LookupQuery::Name(path.into()))
    }

    /// Finds a source type definition by its opaque process-local identity.
    pub fn find_type_definition_by_id(
        &self,
        id: TypeDefinitionId,
    ) -> Result<Option<TypeDefinitionDescriptor<'_>>, ContextError> {
        self.type_definition(id)
    }

    /// Requires a source type definition with the given opaque identity.
    pub fn type_definition_by_id(&self, id: TypeDefinitionId) -> Result<TypeDefinitionDescriptor<'_>, LookupError> {
        self.find_type_definition_by_id(id)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::TypeDefinition,
                query: LookupQuery::ItemId(StableItemId(format!("{id:?}").into())),
            })
    }

    /// Returns source type definitions whose final path segment matches `name`.
    pub fn type_definitions_by_short_name(&self, name: &str) -> Result<TypeDefinitions<'_>, ContextError> {
        let data = self.data()?;
        let definitions = data
            .indexes
            .type_definitions_by_short_name
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|id| data.type_definition(*id))
            .map(|metadata| TypeDefinitionDescriptor::new(metadata, self))
            .collect();
        Ok(TypeDefinitions::new(definitions))
    }

    /// Returns all registered source trait definitions in path order.
    pub fn trait_definitions(&self) -> Result<TraitDefinitions<'_>, ContextError> {
        let data = self.data()?;
        Ok(TraitDefinitions::new(
            data.trait_definitions()
                .into_iter()
                .map(|metadata| TraitDefinitionDescriptor::new(metadata, self))
                .collect(),
        ))
    }

    /// Returns all registered source implementation declarations in
    /// registration order.
    pub fn impl_definitions(&self) -> Result<ImplDefinitions<'_>, ContextError> {
        let definitions = self
            .data()?
            .impl_definitions()
            .iter()
            .map(|metadata| ImplDefinitionDescriptor::new(metadata, self))
            .collect();
        Ok(ImplDefinitions::new(definitions, self))
    }

    /// Looks up a source trait definition by its full reflected path.
    pub fn lookup_trait_definition_by_name(
        &self,
        path: &str,
    ) -> Result<Lookup<TraitDefinitionDescriptor<'_>>, ContextError> {
        let data = self.data()?;
        let candidates = data
            .indexes
            .trait_definitions_by_path
            .get(path)
            .into_iter()
            .flatten()
            .filter_map(|id| data.trait_definition(*id))
            .map(|metadata| TraitDefinitionDescriptor::new(metadata, self))
            .collect::<Vec<_>>();
        Ok(match candidates.as_slice() {
            [] => Lookup::Missing,
            [only] => Lookup::Unique(*only),
            _ => Lookup::Ambiguous(candidates.into_boxed_slice()),
        })
    }

    /// Requires exactly one source trait definition with the given full path.
    pub fn trait_definition_by_name(&self, path: &str) -> Result<TraitDefinitionDescriptor<'_>, LookupError> {
        self.lookup_trait_definition_by_name(path)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::TraitDefinition, LookupQuery::Name(path.into()))
    }

    /// Returns source trait definitions whose final path segment matches
    /// `name`.
    pub fn trait_definitions_by_short_name(&self, name: &str) -> Result<TraitDefinitions<'_>, ContextError> {
        let data = self.data()?;
        let definitions = data
            .indexes
            .trait_definitions_by_short_name
            .get(name)
            .into_iter()
            .flatten()
            .filter_map(|id| data.trait_definition(*id))
            .map(|metadata| TraitDefinitionDescriptor::new(metadata, self))
            .collect();
        Ok(TraitDefinitions::new(definitions))
    }

    fn lookup_by_indexes(&self, data: &ContextData, indexes: Option<&[usize]>) -> Lookup<TypeDescriptor<'_>> {
        let Some(indexes) = indexes else {
            return Lookup::Missing;
        };
        let mut descriptors = indexes.iter().map(|index| data.types[*index].descriptor(self));
        let Some(first) = descriptors.next() else {
            return Lookup::Missing;
        };
        match descriptors.next() {
            None => Lookup::Unique(first),
            Some(second) => Lookup::Ambiguous(
                std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(descriptors)
                    .collect(),
            ),
        }
    }

    #[cfg(test)]
    pub(crate) fn registered_type_count(&self) -> Result<usize, ContextError> {
        Ok(self.data()?.types.len())
    }

    pub(crate) fn shape_for(&self, id: TypeId) -> Result<Option<&'static TypeKindMetadata>, ContextError> {
        Ok(self.data()?.shape(id))
    }

    pub(crate) fn methods_for(
        &self,
        id: TypeId,
    ) -> Result<Arc<[crate::runtime::descriptor::method::ContextEffectiveMethod]>, ContextError> {
        Ok(self.data()?.methods(id))
    }

    pub(crate) fn type_definition(
        &self,
        id: TypeDefinitionId,
    ) -> Result<Option<TypeDefinitionDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .type_definition(id)
            .map(|metadata| TypeDefinitionDescriptor::new(metadata, self)))
    }

    pub(crate) fn external_trait_definition_by_path(
        &self,
        path: &str,
    ) -> Result<Option<&'static TraitDefinitionMetadata>, ContextError> {
        Ok(self
            .data()?
            .trait_definition_by_path(path)
            .filter(|metadata| metadata.completeness == TraitCompleteness::ExternalIncomplete))
    }

    pub(crate) fn capabilities_for_type(
        &self,
        type_id: TypeId,
        rust_name: &'static str,
    ) -> Result<Option<Arc<Capabilities>>, ContextError> {
        Ok(self
            .data()?
            .capabilities(&CapabilityTargetId::Type(TypeIdentity { type_id, rust_name })))
    }

    pub(crate) fn capabilities_for_definition(
        &self,
        definition: TypeDefinitionId,
    ) -> Result<Option<Arc<Capabilities>>, ContextError> {
        Ok(self.data()?.capabilities(&CapabilityTargetId::Definition(definition)))
    }

    pub(crate) fn type_instances(
        &self,
        definition: TypeDefinitionId,
    ) -> Result<Box<[TypeDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .type_instances(definition)
            .iter()
            .filter_map(|type_id| data.type_by_id(*type_id))
            .map(|record| record.descriptor(self))
            .collect())
    }

    pub(crate) fn trait_applications_for_definition(
        &self,
        definition: TraitDefinitionId,
    ) -> Result<Box<[TraitDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .trait_applications(definition)
            .iter()
            .map(|metadata| TraitDescriptor::new(metadata, self))
            .collect())
    }

    pub(crate) fn impl_applications(
        &self,
        definition: ImplDefinitionId,
    ) -> Result<Box<[ImplDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .impl_applications(definition)
            .iter()
            .map(|metadata| ImplDescriptor::new(metadata, self))
            .collect())
    }

    pub(crate) fn impl_applications_for_target(
        &self,
        target: TypeId,
    ) -> Result<Box<[ImplDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .impl_applications_for_target(target)
            .iter()
            .map(|metadata| ImplDescriptor::new(metadata, self))
            .collect())
    }

    pub(crate) fn construction_plans(&self, target: TypeId) -> Result<Box<[ConstructionDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .construction_plans(target)
            .iter()
            .map(|plan| ConstructionDescriptor::new(plan))
            .collect())
    }
}

impl IntoLookupCandidate for TypeDescriptor<'_> {
    fn into_lookup_candidate(self) -> LookupCandidate {
        let identity = self.type_identity();
        let display_name: Box<str> = identity.rust_name.into();
        LookupCandidate {
            identity: ItemIdentity {
                kind: ItemKind::Type,
                stable_id: StableItemId(display_name.clone()),
                display_name: display_name.clone(),
            },
            display_name,
            origin: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReflectionContext;
    use crate::runtime::context::handle::registration::Registration;
    use crate::runtime::context::handle::registration::TraitObjectRegistration;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::error::LookupError;
    use crate::runtime::error::LookupSubject;
    use crate::runtime::lookup::Lookup;

    struct Marker;

    static NO_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
    static TRAIT_DEFINITION: TraitDefinitionMetadata = TraitDefinitionMetadata::new::<Marker>(
        "DisplayLike",
        "tests::DisplayLike",
        Visibility::Public,
        TraitCompleteness::Complete,
        &NO_GENERICS,
        &[],
        &[],
        &[],
        &[],
    );

    #[test]
    fn empty_context_lookups_return_precise_missing_results_and_remain_usable() {
        let context = ReflectionContext::empty();

        assert!(
            context
                .find_type_by_id(std::any::TypeId::of::<u8>())
                .expect("empty context lookup succeeds")
                .is_none()
        );
        assert!(matches!(
            context.lookup_type_by_name("missing::Type").unwrap(),
            Lookup::Missing
        ));
        assert!(matches!(
            context.lookup_type_by_short_name("Missing").unwrap(),
            Lookup::Missing
        ));
        assert!(matches!(
            context.lookup_type_definition_by_name("missing::Definition").unwrap(),
            Lookup::Missing
        ));
        assert!(
            context
                .trait_definition_for::<Marker>()
                .expect("typed trait lookup succeeds")
                .is_none()
        );
        assert!(
            context
                .external_trait_definition("external::Trait")
                .expect("external trait lookup succeeds")
                .is_none()
        );
        assert!(
            context
                .trait_object::<dyn std::fmt::Debug>()
                .expect("trait object lookup succeeds")
                .is_none()
        );
        assert!(matches!(
            context.type_by_id(std::any::TypeId::of::<u8>()),
            Err(LookupError::Missing {
                subject: LookupSubject::Type,
                ..
            })
        ));
        assert!(std::ptr::eq(
            context.initialize().expect("explicit context initializes"),
            &context
        ));
        assert!(context.is_initialized());
    }

    #[test]
    fn strict_lookup_failure_does_not_prevent_later_successful_lookup() {
        struct Registered;
        impl crate::runtime::descriptor::Reflect for Registered {
            fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
                static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Registered> =
                    crate::runtime::descriptor::TypeRegistration::new();
                &REGISTRATION
            }
        }

        let context = ReflectionContext::builder()
            .include_type::<Registered>()
            .build()
            .expect("fixture context builds");
        assert!(matches!(
            context.type_by_name("missing::Type"),
            Err(LookupError::Missing {
                subject: LookupSubject::Type,
                ..
            })
        ));
        let recovered = context
            .type_by_id(std::any::TypeId::of::<Registered>())
            .expect("a failed lookup leaves the context available");
        assert_eq!(recovered.type_id(), std::any::TypeId::of::<Registered>());
    }

    #[test]
    fn registered_trait_handle_getters_preserve_metadata_and_object_path() {
        let context = ReflectionContext::builder()
            .register(
                Registration::metadata_only()
                    .with_trait_definition(&TRAIT_DEFINITION)
                    .with_trait_object(TraitObjectRegistration::of::<dyn std::fmt::Display>(
                        "std::fmt::Display",
                    )),
            )
            .build()
            .expect("trait metadata context builds");

        let definition = context
            .trait_definition_for::<Marker>()
            .expect("trait marker lookup succeeds")
            .expect("trait definition is registered");
        assert_eq!(definition.path(), "tests::DisplayLike");
        assert_eq!(definition.name(), "DisplayLike");
        assert_eq!(definition.completeness(), TraitCompleteness::Complete);

        let object = context
            .trait_object::<dyn std::fmt::Display>()
            .expect("trait object lookup succeeds")
            .expect("object registration is present");
        assert_eq!(object.trait_path(), "std::fmt::Display");
        assert_eq!(object.rust_type_name(), std::any::type_name::<dyn std::fmt::Display>());
    }
}
