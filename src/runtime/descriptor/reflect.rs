// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::any::Any;
use std::any::TypeId;
use std::any::type_name;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::metadata::TypeMetadata;
use crate::runtime::construct::ConstructionPlan;
use crate::runtime::construct::ConstructionTargetKind;
use crate::runtime::descriptor::generic::TypeDefinitionMetadata;
use crate::runtime::descriptor::type_kind::TypeKindMetadata;
use crate::runtime::descriptor::variant::VariantMetadata;

/// Supplies the static registration used to reflect a Rust type.
pub trait Reflect: 'static {
    /// Returns the shared registration for this concrete type.
    fn __reflect_registration() -> &'static TypeRegistration<Self> {
        TypeRegistration::get_or_init_with(TypeRegistration::new)
    }
}

/// Static type metadata retained by the reflection runtime.
///
/// # Type Parameters
/// - `T`: the Rust type represented by this registration.
pub struct TypeRegistration<T: ?Sized + 'static> {
    /// Shared identity and generic metadata for the registered Rust type.
    metadata: TypeMetadata,
    /// Optional structural metadata for this type.
    shape: Option<&'static TypeKindMetadata>,
    /// Optional source type definition represented by this registration.
    type_definition: Option<&'static TypeDefinitionMetadata>,
    /// Generated construction plans associated with this type.
    construction_plans: &'static [ConstructionPlan],
    /// Associates this registration with its Rust type without storing a value.
    _type: PhantomData<fn() -> T>,
}

impl<T: ?Sized + 'static> TypeRegistration<T> {
    /// Creates a registration containing only the concrete type identity.
    ///
    /// # Type Parameters
    /// - `T`: the Rust type represented by the registration.
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
    ///
    /// # Type Parameters
    /// - `T`: the Rust type represented by the registration.
    ///
    /// # Parameters
    /// - `shape`: structural metadata describing the type.
    /// - `type_definition`: optional source definition metadata.
    /// - `construction_plans`: generated construction plans for the type.
    pub fn with_shape(
        shape: &'static TypeKindMetadata,
        type_definition: Option<&'static TypeDefinitionMetadata>,
        construction_plans: &'static [ConstructionPlan],
    ) -> Self {
        let construction_shape = construction_plans
            .iter()
            .find(|plan| plan.target.kind == ConstructionTargetKind::Struct)
            .map(|plan| plan.shape);
        Self {
            metadata: TypeMetadata::with_shape::<T>(shape, construction_shape, type_definition),
            shape: Some(shape),
            type_definition,
            construction_plans,
            _type: PhantomData,
        }
    }

    /// Returns the stable registration for this concrete type, initializing it
    /// once.
    ///
    /// # Type Parameters
    /// - `T`: the Rust type whose registration is requested.
    ///
    /// # Parameters
    /// - `initialize`: creates the registration when the cache has no entry.
    ///
    /// # Panics
    /// Panics if initialization directly or indirectly requests this type
    /// again on the same thread. Registration initializers may depend on other
    /// types as long as that dependency chain does not return to a type whose
    /// initialization is already in progress on this thread. Cycles formed
    /// across multiple threads are not detected and may block.
    ///
    /// If `initialize` panics, the registration remains uninitialized and a
    /// later call may retry it.
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
        if let Some(value) = cell.get() {
            return value
                .downcast_ref::<Self>()
                .expect("registration cache is keyed by exact TypeId");
        }

        let _guard = RegistrationInitializationGuard::enter::<T>();
        cell.get_or_init(|| Box::new(initialize()) as Box<dyn Any + Send + Sync>)
            .downcast_ref::<Self>()
            .expect("registration cache is keyed by exact TypeId")
    }

    /// Returns the shared metadata associated with this registration.
    pub(crate) fn metadata(&'static self) -> &'static TypeMetadata {
        &self.metadata
    }

    /// Adds concrete arguments retained for this compiled type application.
    ///
    /// # Type Parameters
    /// - `T`: the Rust type represented by the registration.
    ///
    /// # Parameters
    /// - `arguments`: generic arguments to retain in the registration.
    #[doc(hidden)]
    pub fn with_generic_arguments(
        mut self,
        arguments: &'static [crate::runtime::descriptor::generic::GenericArgumentMetadata],
    ) -> Self {
        self.metadata = self.metadata.with_generic_arguments(arguments);
        self
    }

    /// Returns the optional structural metadata for this type.
    pub(crate) fn shape(&'static self) -> Option<&'static TypeKindMetadata> {
        self.shape
    }

    /// Returns the optional source definition for this type.
    pub(crate) fn type_definition(&'static self) -> Option<&'static TypeDefinitionMetadata> {
        self.type_definition
    }

    /// Returns the generated construction plans for this type.
    pub(crate) fn construction_plans(&'static self) -> &'static [ConstructionPlan] {
        self.construction_plans
    }

    /// Returns a source variant by index when this registration has enum shape.
    ///
    /// # Parameters
    /// - `index`: zero-based variant index in the enum metadata.
    pub fn variant(&'static self, index: usize) -> Option<&'static VariantMetadata> {
        match self.shape? {
            TypeKindMetadata::Enum(variants) => variants.get(index),
            _ => None,
        }
    }
}

impl<T: ?Sized + 'static> Default for TypeRegistration<T> {
    /// Creates the default registration for `T`.
    fn default() -> Self {
        Self::new()
    }
}

/// Cache key and registration cell for each concrete reflected type.
type RegistrationCache = HashMap<TypeId, &'static OnceLock<Box<dyn Any + Send + Sync>>>;

/// Returns the process-wide cache of type registrations.
fn registration_cache() -> &'static Mutex<RegistrationCache> {
    static REGISTRATIONS: OnceLock<Mutex<RegistrationCache>> = OnceLock::new();
    REGISTRATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

thread_local! {
    /// Types currently being initialized by this thread.
    static INITIALIZING_REGISTRATIONS: RefCell<HashSet<TypeId>> = RefCell::new(HashSet::new());
}

/// Removes this type from the current thread's active initialization set.
struct RegistrationInitializationGuard {
    /// Type whose initialization this guard tracks.
    type_id: TypeId,
}

impl RegistrationInitializationGuard {
    /// Marks `T` as initializing or panics if it is already active on this
    /// thread.
    ///
    /// # Type Parameters
    /// - `T`: the Rust type whose registration is being initialized.
    fn enter<T: ?Sized + 'static>() -> Self {
        let type_id = TypeId::of::<T>();
        let inserted = INITIALIZING_REGISTRATIONS.with(|initializing| initializing.borrow_mut().insert(type_id));
        if !inserted {
            panic!("recursive type registration initialization for {}", type_name::<T>());
        }
        Self { type_id }
    }
}

impl Drop for RegistrationInitializationGuard {
    /// Removes the type from active initialization tracking.
    fn drop(&mut self) {
        INITIALIZING_REGISTRATIONS.with(|initializing| {
            initializing.borrow_mut().remove(&self.type_id);
        });
    }
}
