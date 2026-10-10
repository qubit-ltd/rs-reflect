// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Concrete trait implementation facts shared through the hidden hook.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::descriptor::AppliedTraitId;
use crate::descriptor::AssociatedConstReader;
use crate::descriptor::InvocationAdapter;
use crate::descriptor::InvocationUnavailableReason;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TraitDescriptor;
use crate::descriptor::TraitDescriptorBuildError;
use crate::descriptor::TypeDescriptorResolver;
use crate::expression::GenericArgument;

/// Cache for trait hook payloads keyed by receiver type and applied identity.
type AppliedTraitCache = HashMap<(TypeId, AppliedTraitId), Arc<OnceLock<TraitImplPayload>>>;

/// Concrete trait facts supplied by a reflected trait's hidden implementation
/// hook.
///
/// The hook carries declaration identity without imposing object-safety
/// requirements on the reflected trait. Implementation expansion enriches this
/// payload with concrete application details before it becomes part of an
/// implementation descriptor.
///
/// # Examples
///
/// ```
/// use std::any::TypeId;
/// use std::sync::LazyLock;
/// use qubit_reflect::descriptor::TraitCompleteness;
/// use qubit_reflect::descriptor::TraitDefinitionDescriptor;
/// use qubit_reflect::descriptor::TraitDescriptor;
/// use qubit_reflect::descriptor::TraitId;
/// use qubit_reflect::descriptor::TraitImplPayload;
/// use qubit_reflect::expression::GenericDefinitionDescriptor;
///
/// struct Marker;
/// static GENERICS: LazyLock<GenericDefinitionDescriptor> =
///     LazyLock::new(|| GenericDefinitionDescriptor::new([], []));
/// static DEFINITION: LazyLock<TraitDefinitionDescriptor> = LazyLock::new(|| {
///     TraitDefinitionDescriptor::new(
///         TraitId::Reflected(TypeId::of::<Marker>()), "Example", "example::Example", "Example",
///         TraitCompleteness::Complete, &GENERICS,
///     )
/// });
/// static APPLIED: LazyLock<TraitDescriptor> = LazyLock::new(|| {
///     TraitDescriptor::builder(&DEFINITION).build().expect("valid application")
/// });
/// let payload = TraitImplPayload::new(&DEFINITION, &APPLIED);
/// assert_eq!(payload.applied().rust_name(), "Example");
/// ```
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct TraitImplPayload {
    /// Declaration shared by every concrete application of this trait.
    definition: &'static TraitDefinitionDescriptor,
    /// Concrete trait application for the hook receiver.
    applied: &'static TraitDescriptor,
    /// Generated default-method adapters in declaration order.
    default_method_adapters: &'static [Option<&'static InvocationAdapter>],
    /// Reasons generated default-method adapters are unavailable.
    default_method_unavailable_reasons: &'static [&'static [InvocationUnavailableReason]],
    /// Resolvers for proven concrete associated types.
    associated_type_resolvers: &'static [Option<TypeDescriptorResolver>],
    /// Safe generated readers for associated constants.
    associated_const_readers: &'static [Option<&'static AssociatedConstReader>],
}

impl TraitImplPayload {
    /// Creates a payload for one reflected trait declaration.
    ///
    /// # Parameters
    ///
    /// - `definition`: Trait declaration shared by implementations.
    /// - `applied`: Concrete trait application for this hook.
    ///
    /// # Returns
    ///
    /// Returns a payload with no generated implementation adapters attached.
    #[doc(hidden)]
    #[inline]
    pub const fn new(definition: &'static TraitDefinitionDescriptor, applied: &'static TraitDescriptor) -> Self {
        Self {
            definition,
            applied,
            default_method_adapters: &[],
            default_method_unavailable_reasons: &[],
            associated_type_resolvers: &[],
            associated_const_readers: &[],
        }
    }

    /// Returns the complete trait declaration shared by every implementation.
    ///
    /// # Returns
    ///
    /// Returns the source declaration descriptor.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn definition(self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the concrete applied trait descriptor for the hook receiver.
    ///
    /// # Returns
    ///
    /// Returns the concrete trait application.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn applied(self) -> &'static TraitDescriptor {
        self.applied
    }

    /// Returns concrete adapters for default methods in declaration order.
    ///
    /// # Returns
    ///
    /// Returns one optional adapter slot for each default method.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn default_method_adapters(self) -> &'static [Option<&'static InvocationAdapter>] {
        self.default_method_adapters
    }

    /// Returns unavailable-reason sets for default methods in declaration
    /// order.
    ///
    /// # Returns
    ///
    /// Returns unavailable reasons paired with default methods.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn default_method_unavailable_reasons(self) -> &'static [&'static [InvocationUnavailableReason]] {
        self.default_method_unavailable_reasons
    }

    /// Returns proven concrete associated-type resolvers in declaration order.
    ///
    /// # Returns
    ///
    /// Returns one optional resolver slot for each associated type.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn associated_type_resolvers(self) -> &'static [Option<TypeDescriptorResolver>] {
        self.associated_type_resolvers
    }

    /// Returns safe associated-constant readers in declaration order.
    ///
    /// # Returns
    ///
    /// Returns one optional reader slot for each associated constant.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn associated_const_readers(self) -> &'static [Option<&'static AssociatedConstReader>] {
        self.associated_const_readers
    }

    /// Reuses a cached payload for one receiver type and trait application.
    ///
    /// Concurrent callers share one initialization cell for the key. The cache
    /// lock is released before descriptor and adapter builders run; a later
    /// call can retry initialization if a builder panics.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete receiver type used as part of the cache key.
    ///
    /// # Parameters
    ///
    /// - `definition`: Trait declaration shared by all applications.
    /// - `arguments`: Concrete generic arguments for the trait application.
    /// - `build`: Constructs the applied descriptor from `arguments`.
    /// - `build_default_method_adapters`: Builds generated adapters for default
    ///   methods.
    /// - `build_default_method_unavailable_reasons`: Builds reasons for
    ///   unavailable default methods.
    /// - `build_associated_type_resolvers`: Builds resolvers for concrete
    ///   associated types.
    /// - `build_associated_const_readers`: Builds readers for associated
    ///   constants.
    ///
    /// # Returns
    ///
    /// Returns the cached payload for this receiver and trait application.
    ///
    /// # Panics
    ///
    /// Panics if the cache mutex is poisoned, if `build` returns an error, or
    /// if any supplied builder panics.
    #[doc(hidden)]
    pub fn cached_with_arguments<T: ?Sized + 'static>(
        definition: &'static TraitDefinitionDescriptor,
        arguments: Vec<GenericArgument>,
        build: impl FnOnce(Vec<GenericArgument>) -> Result<TraitDescriptor, TraitDescriptorBuildError>,
        build_default_method_adapters: impl FnOnce() -> Vec<Option<&'static InvocationAdapter>>,
        build_default_method_unavailable_reasons: impl FnOnce() -> Vec<&'static [InvocationUnavailableReason]>,
        build_associated_type_resolvers: impl FnOnce() -> Vec<Option<TypeDescriptorResolver>>,
        build_associated_const_readers: impl FnOnce() -> Vec<Option<&'static AssociatedConstReader>>,
    ) -> Self {
        static CACHE: LazyLock<Mutex<AppliedTraitCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));
        let identity = AppliedTraitId {
            definition: definition.trait_id().clone(),
            arguments: arguments.clone().into_boxed_slice(),
            associated_type_arguments: Box::new([]),
        };
        let key = (TypeId::of::<T>(), identity);
        let mut cache = CACHE.lock().expect("trait payload cache mutex must not be poisoned");
        let cell = cache.entry(key).or_insert_with(|| Arc::new(OnceLock::new())).clone();
        drop(cache);
        *cell.get_or_init(|| {
            let applied = Box::leak(Box::new(
                build(arguments).expect("a reflected trait must build a valid applied descriptor"),
            ));
            let default_method_adapters = Box::leak(build_default_method_adapters().into_boxed_slice());
            let default_method_unavailable_reasons =
                Box::leak(build_default_method_unavailable_reasons().into_boxed_slice());
            let associated_type_resolvers = Box::leak(build_associated_type_resolvers().into_boxed_slice());
            let associated_const_readers = Box::leak(build_associated_const_readers().into_boxed_slice());
            Self {
                definition,
                applied,
                default_method_adapters,
                default_method_unavailable_reasons,
                associated_type_resolvers,
                associated_const_readers,
            }
        })
    }
}
