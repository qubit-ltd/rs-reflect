use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Mutex, OnceLock};

use super::metadata::TypeMetadata;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::descriptor::variant::VariantMetadata;

pub trait Reflect: 'static {
    fn __reflect_registration() -> &'static TypeRegistration<Self>
    where
        Self: Sized;
}

pub struct TypeRegistration<T: 'static> {
    metadata: TypeMetadata,
    shape: Option<&'static TypeKindMetadata>,
    type_definition: Option<&'static TypeDefinitionMetadata>,
    construction_plans: &'static [ConstructionPlan],
    _type: PhantomData<fn() -> T>,
}

impl<T: 'static> TypeRegistration<T> {
    pub const fn new() -> Self {
        Self {
            metadata: TypeMetadata::new::<T>(),
            shape: None,
            type_definition: None,
            construction_plans: &[],
            _type: PhantomData,
        }
    }

    /// Creates a registration with static source shape and construction plans.
    pub fn with_shape(
        shape: &'static TypeKindMetadata,
        type_definition: Option<&'static TypeDefinitionMetadata>,
        construction_plans: &'static [ConstructionPlan],
    ) -> Self {
        Self {
            metadata: TypeMetadata::new::<T>(),
            shape: Some(shape),
            type_definition,
            construction_plans,
            _type: PhantomData,
        }
    }

    /// Returns the stable registration for this concrete type, initializing it once.
    pub fn get_or_init_with(initialize: impl FnOnce() -> Self) -> &'static Self {
        let type_id = TypeId::of::<T>();
        let cell = {
            let mut registrations = registration_cache()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *registrations
                .entry(type_id)
                .or_insert_with(|| Box::leak(Box::new(OnceLock::new())))
        };
        cell.get_or_init(|| Box::new(initialize()) as Box<dyn Any + Send + Sync>)
            .downcast_ref::<Self>()
            .expect("registration cache is keyed by exact TypeId")
    }

    pub(crate) fn metadata(&'static self) -> &'static TypeMetadata {
        &self.metadata
    }

    pub(crate) fn shape(&'static self) -> Option<&'static TypeKindMetadata> {
        self.shape
    }

    pub(crate) fn type_definition(&'static self) -> Option<&'static TypeDefinitionMetadata> {
        self.type_definition
    }

    pub(crate) fn construction_plans(&'static self) -> &'static [ConstructionPlan] {
        self.construction_plans
    }

    /// Returns a source variant by index when this registration has enum shape.
    pub fn variant(&'static self, index: usize) -> Option<&'static VariantMetadata> {
        match self.shape? {
            TypeKindMetadata::Enum(variants) => variants.get(index),
            _ => None,
        }
    }
}

type RegistrationCache = HashMap<TypeId, &'static OnceLock<Box<dyn Any + Send + Sync>>>;

fn registration_cache() -> &'static Mutex<RegistrationCache> {
    static REGISTRATIONS: OnceLock<Mutex<RegistrationCache>> = OnceLock::new();
    REGISTRATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
