use std::any::TypeId;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::capability::capabilities::Capabilities;
use crate::runtime::capability::capability_target::CapabilityTargetId;
use crate::runtime::construct::ConstructionDescriptor;
use crate::runtime::descriptor::TypeDescriptor;
use crate::runtime::descriptor::generic::{TypeDefinitionDescriptor, TypeDefinitionId};
use crate::runtime::descriptor::impl_descriptor::{
    ImplDefinitionDescriptor, ImplDefinitionId, ImplDescriptor,
};
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::trait_descriptor::{
    TraitDefinitionDescriptor, TraitDefinitionId, TraitDescriptor,
};
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ReflectionContextId(u64);

impl ReflectionContextId {
    fn next() -> Self {
        Self(NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub(crate) struct ReflectionContext {
    id: ReflectionContextId,
    state: ContextState,
}

enum ContextState {
    Explicit(Arc<ContextData>),
    Global(&'static GlobalContext),
}

impl ReflectionContext {
    pub(crate) fn global() -> &'static Self {
        global::global_context()
    }

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

    pub(crate) fn initialize(&self) -> Result<&Self, ContextError> {
        if let ContextState::Global(state) = &self.state {
            state.data()?;
        }
        Ok(self)
    }

    pub(crate) fn is_initialized(&self) -> bool {
        match &self.state {
            ContextState::Explicit(_) => true,
            ContextState::Global(state) => state.is_initialized(),
        }
    }

    pub(crate) fn id(&self) -> ReflectionContextId {
        self.id
    }

    pub(crate) fn data(&self) -> Result<Arc<ContextData>, ContextError> {
        match &self.state {
            ContextState::Explicit(data) => Ok(Arc::clone(data)),
            ContextState::Global(state) => state.data(),
        }
    }

    pub(crate) fn find_type_by_id(
        &self,
        id: TypeId,
    ) -> Result<Option<TypeDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .type_by_id(id)
            .map(|type_record| type_record.descriptor(self)))
    }

    pub(crate) fn contains_type_id(&self, id: TypeId) -> Result<bool, ContextError> {
        Ok(self.data()?.contains_type_id(id))
    }

    pub(crate) fn lookup_type_by_name(
        &self,
        name: &str,
    ) -> Result<Lookup<TypeDescriptor<'_>>, ContextError> {
        let data = self.data()?;
        Ok(self.lookup_by_indexes(&data, data.indexes.types_by_name.get(name)))
    }

    pub(crate) fn lookup_type_by_short_name(
        &self,
        name: &str,
    ) -> Result<Lookup<TypeDescriptor<'_>>, ContextError> {
        let data = self.data()?;
        Ok(self.lookup_by_indexes(&data, data.indexes.types_by_short_name.get(name)))
    }

    pub(crate) fn type_by_id(&self, id: TypeId) -> Result<TypeDescriptor<'_>, LookupError> {
        self.find_type_by_id(id)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Type,
                query: LookupQuery::TypeId(id),
            })
    }

    pub(crate) fn type_by_name(&self, name: &str) -> Result<TypeDescriptor<'_>, LookupError> {
        self.lookup_type_by_name(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Type, LookupQuery::Name(name.into()))
    }

    pub(crate) fn type_by_short_name(&self, name: &str) -> Result<TypeDescriptor<'_>, LookupError> {
        self.lookup_type_by_short_name(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Type, LookupQuery::ShortName(name.into()))
    }

    pub(crate) fn types(&self) -> Result<Box<[TypeDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .types
            .iter()
            .map(|type_record| type_record.descriptor(self))
            .collect())
    }

    fn lookup_by_indexes(
        &self,
        data: &ContextData,
        indexes: Option<&Box<[usize]>>,
    ) -> Lookup<TypeDescriptor<'_>> {
        let Some(indexes) = indexes else {
            return Lookup::Missing;
        };
        let mut descriptors = indexes
            .iter()
            .map(|index| data.types[*index].descriptor(self));
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

    pub(crate) fn registered_type_count(&self) -> Result<usize, ContextError> {
        Ok(self.data()?.types.len())
    }

    pub(crate) fn shape_for(
        &self,
        id: TypeId,
    ) -> Result<Option<&'static TypeKindMetadata>, ContextError> {
        Ok(self.data()?.shape(id))
    }

    pub(crate) fn methods_for(
        &self,
        id: TypeId,
    ) -> Result<&'static [EffectiveMethod], ContextError> {
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

    pub(crate) fn trait_definition(
        &self,
        id: TraitDefinitionId,
    ) -> Result<Option<TraitDefinitionDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .trait_definition(id)
            .map(|metadata| TraitDefinitionDescriptor::new(metadata, self)))
    }

    pub(crate) fn impl_definition(
        &self,
        id: ImplDefinitionId,
    ) -> Result<Option<ImplDefinitionDescriptor<'_>>, ContextError> {
        Ok(self
            .data()?
            .impl_definition(id)
            .map(|metadata| ImplDefinitionDescriptor::new(metadata, self)))
    }

    pub(crate) fn capabilities_for_type(
        &self,
        type_id: TypeId,
        rust_name: &'static str,
    ) -> Result<Option<Arc<Capabilities>>, ContextError> {
        Ok(self
            .data()?
            .capabilities(&CapabilityTargetId::Type(TypeIdentity {
                type_id,
                rust_name,
            })))
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

    pub(crate) fn trait_applications(
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

    pub(crate) fn construction_plans(
        &self,
        target: TypeId,
    ) -> Result<Box<[ConstructionDescriptor<'_>]>, ContextError> {
        let data = self.data()?;
        Ok(data
            .construction_plans(target)
            .iter()
            .map(|plan| ConstructionDescriptor::new(*plan))
            .collect())
    }

    pub(crate) fn handle(&self) -> ContextHandle<'_> {
        ContextHandle(self)
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

#[derive(Clone, Copy)]
pub(crate) struct ContextHandle<'ctx>(&'ctx ReflectionContext);

impl<'ctx> ContextHandle<'ctx> {
    pub(crate) fn context(self) -> &'ctx ReflectionContext {
        self.0
    }
}
