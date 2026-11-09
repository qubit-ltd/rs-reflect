// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed construction and update adapter contracts emitted by derive.
//!
//! Generated constructors need safe entry points for struct, tuple, unit, and
//! enum-variant construction, plus field update. The matching runtime pieces
//! are `ConstructionDescriptor`, `ConstructField`, `ConstructionProviders`,
//! `ConstructionTarget`, `ConstructionInput`, and the validated adapter
//! contracts under `runtime::construct`. They are currently crate-private;
//! generated code must not construct or mutate validated inputs directly.

use super::value::DynamicOwned;
use super::value::Local;
use super::value::ThreadSafe;
use super::value::ValueFor;
use super::value::ValueMode;
pub use crate::runtime::construct::ConstructField;
use crate::runtime::construct::ConstructProvider as RuntimeConstructProvider;
pub use crate::runtime::construct::ConstructionAvailability;
pub use crate::runtime::construct::ConstructionFieldPolicy;
pub use crate::runtime::construct::ConstructionPlan;
pub use crate::runtime::construct::ConstructionProviders;
pub use crate::runtime::construct::ConstructionShape;
pub use crate::runtime::construct::ConstructionUnavailableReason;
pub use crate::runtime::construct::UpdateFieldPolicy;
pub use crate::runtime::construct::UpdateUnavailableReason;
use crate::runtime::value::DynamicOwned as RuntimeDynamicOwned;
use crate::runtime::value::Local as RuntimeLocal;

/// Typed construction and update callbacks using only codegen-v4 dynamic
/// values.
///
/// # Type Parameters
///
/// * `T` — concrete target type produced by construction and update.
/// * `M` — value mode accepted by the callbacks.
pub struct ConstructProvider<T: 'static, M: ValueMode> {
    /// Callback that builds a target from its ordered field inputs.
    construct: fn(Vec<Option<DynamicOwned<M>>>) -> T,
    /// Callback that updates an existing target from replacement inputs.
    update: fn(T, Vec<Option<DynamicOwned<M>>>) -> T,
    /// Associates the callback pair with its target type without storing it.
    _type: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static, M: ValueMode> ConstructProvider<T, M> {
    /// Creates typed callbacks for constructing and updating one concrete
    /// target type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — target type returned by both callbacks.
    /// * `M` — ownership mode used for dynamic field values.
    ///
    /// # Parameters
    ///
    /// * `construct` — function that creates a target from field slots.
    /// * `update` — function that applies replacement slots to an existing
    ///   target.
    pub const fn new(
        construct: fn(Vec<Option<DynamicOwned<M>>>) -> T,
        update: fn(T, Vec<Option<DynamicOwned<M>>>) -> T,
    ) -> Self {
        Self {
            construct,
            update,
            _type: std::marker::PhantomData,
        }
    }
}

impl<T> RuntimeConstructProvider<RuntimeLocal> for ConstructProvider<T, Local>
where
    T: ValueFor<Local>,
{
    /// Converts runtime field values, calls the typed constructor, and erases
    /// its result.
    ///
    /// # Parameters
    ///
    /// * `fields` — ordered field slots supplied by the runtime constructor.
    fn construct(&self, fields: Vec<Option<RuntimeDynamicOwned<RuntimeLocal>>>) -> RuntimeDynamicOwned<RuntimeLocal> {
        let fields = fields
            .into_iter()
            .map(|field| field.map(DynamicOwned::<Local>::from_runtime))
            .collect();
        DynamicOwned::<Local>::new((self.construct)(fields)).into_runtime()
    }

    /// Recovers the typed base, converts replacements, then erases the updated
    /// value.
    ///
    /// # Parameters
    ///
    /// * `base` — erased value previously created by this typed provider.
    /// * `replacements` — ordered slots that replace selected fields.
    fn update(
        &self,
        base: RuntimeDynamicOwned<RuntimeLocal>,
        replacements: Vec<Option<RuntimeDynamicOwned<RuntimeLocal>>>,
    ) -> RuntimeDynamicOwned<RuntimeLocal> {
        let base = match DynamicOwned::<Local>::from_runtime(base).downcast::<T>() {
            Ok(value) => value,
            Err(_) => panic!("validated update base must match its typed provider"),
        };
        let replacements = replacements
            .into_iter()
            .map(|field| field.map(DynamicOwned::<Local>::from_runtime))
            .collect();
        DynamicOwned::<Local>::new((self.update)(base, replacements)).into_runtime()
    }
}

impl<T> RuntimeConstructProvider<crate::runtime::value::ThreadSafe> for ConstructProvider<T, ThreadSafe>
where
    T: ValueFor<ThreadSafe>,
{
    /// Converts thread-safe field values, invokes the typed constructor, and
    /// erases its result.
    ///
    /// # Parameters
    ///
    /// * `fields` — ordered field slots supplied by the runtime constructor.
    fn construct(
        &self,
        fields: Vec<Option<RuntimeDynamicOwned<crate::runtime::value::ThreadSafe>>>,
    ) -> RuntimeDynamicOwned<crate::runtime::value::ThreadSafe> {
        let fields = fields
            .into_iter()
            .map(|field| field.map(DynamicOwned::<ThreadSafe>::from_runtime))
            .collect();
        DynamicOwned::<ThreadSafe>::new((self.construct)(fields)).into_runtime()
    }

    /// Recovers the typed base and replacements before returning an erased
    /// updated value.
    ///
    /// # Parameters
    ///
    /// * `base` — erased value previously created by this typed provider.
    /// * `replacements` — ordered slots that replace selected fields.
    fn update(
        &self,
        base: RuntimeDynamicOwned<crate::runtime::value::ThreadSafe>,
        replacements: Vec<Option<RuntimeDynamicOwned<crate::runtime::value::ThreadSafe>>>,
    ) -> RuntimeDynamicOwned<crate::runtime::value::ThreadSafe> {
        let base = match DynamicOwned::<ThreadSafe>::from_runtime(base).downcast::<T>() {
            Ok(value) => value,
            Err(_) => panic!("validated update base must match its typed provider"),
        };
        let replacements = replacements
            .into_iter()
            .map(|field| field.map(DynamicOwned::<ThreadSafe>::from_runtime))
            .collect();
        DynamicOwned::<ThreadSafe>::new((self.update)(base, replacements)).into_runtime()
    }
}

impl crate::runtime::construct::ConstructionProviders {
    /// Creates providers with the local mode, which works for all reflected
    /// values.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected target type served by the local provider.
    ///
    /// # Parameters
    ///
    /// * `local` — optional constructor/update callbacks for local values.
    pub const fn new<T: ValueFor<Local>>(local: Option<&'static ConstructProvider<T, Local>>) -> Self {
        let local = match local {
            Some(provider) => Some(provider as &dyn RuntimeConstructProvider<RuntimeLocal>),
            None => None,
        };
        Self::from_runtime_providers(local, None)
    }

    /// Adds a thread-safe provider when the target and its fields are `Send +
    /// Sync`.
    ///
    /// # Type Parameters
    ///
    /// * `T` — reflected target type served by the thread-safe provider.
    ///
    /// # Parameters
    ///
    /// * `provider` — constructor/update callbacks for thread-safe values.
    pub const fn with_thread_safe<T: ValueFor<ThreadSafe>>(
        self,
        provider: &'static ConstructProvider<T, ThreadSafe>,
    ) -> Self {
        self.with_runtime_thread_safe(provider as &dyn RuntimeConstructProvider<crate::runtime::value::ThreadSafe>)
    }
}

#[cfg(test)]
mod tests {
    use super::ConstructProvider;
    use super::ConstructionProviders;
    use super::DynamicOwned;
    use super::Local;
    use super::RuntimeConstructProvider;
    use super::RuntimeDynamicOwned;
    use super::RuntimeLocal;
    use super::ThreadSafe;

    fn construct_local(fields: Vec<Option<DynamicOwned<Local>>>) -> u8 {
        fields
            .into_iter()
            .next()
            .flatten()
            .and_then(|field| field.downcast::<u8>().ok())
            .unwrap_or_default()
    }

    fn update_local(base: u8, replacements: Vec<Option<DynamicOwned<Local>>>) -> u8 {
        replacements
            .into_iter()
            .next()
            .flatten()
            .and_then(|field| field.downcast::<u8>().ok())
            .unwrap_or(base)
    }

    fn construct_thread_safe(fields: Vec<Option<DynamicOwned<ThreadSafe>>>) -> u32 {
        fields
            .into_iter()
            .next()
            .flatten()
            .and_then(|field| field.downcast::<u32>().ok())
            .unwrap_or_default()
    }

    fn update_thread_safe(base: u32, replacements: Vec<Option<DynamicOwned<ThreadSafe>>>) -> u32 {
        replacements
            .into_iter()
            .next()
            .flatten()
            .and_then(|field| field.downcast::<u32>().ok())
            .unwrap_or(base)
    }

    #[test]
    fn test_local_provider_converts_construct_and_update_values() {
        let provider = ConstructProvider::<u8, Local>::new(construct_local, update_local);
        let fields = vec![Some(RuntimeDynamicOwned::<RuntimeLocal>::new(17_u8))];
        let constructed =
            <ConstructProvider<u8, Local> as RuntimeConstructProvider<RuntimeLocal>>::construct(&provider, fields);
        assert_eq!(
            DynamicOwned::<Local>::from_runtime(constructed).downcast::<u8>().ok(),
            Some(17)
        );

        let base = RuntimeDynamicOwned::<RuntimeLocal>::new(4_u8);
        let replacements = vec![Some(RuntimeDynamicOwned::<RuntimeLocal>::new(29_u8))];
        let updated = <ConstructProvider<u8, Local> as RuntimeConstructProvider<RuntimeLocal>>::update(
            &provider,
            base,
            replacements,
        );
        assert_eq!(
            DynamicOwned::<Local>::from_runtime(updated).downcast::<u8>().ok(),
            Some(29)
        );
    }

    #[test]
    fn test_thread_safe_provider_converts_construct_and_update_values() {
        let provider = ConstructProvider::<u32, ThreadSafe>::new(construct_thread_safe, update_thread_safe);
        let fields = vec![Some(RuntimeDynamicOwned::<crate::runtime::value::ThreadSafe>::new(
            23_u32,
        ))];
        let constructed = <ConstructProvider<u32, ThreadSafe> as RuntimeConstructProvider<
            crate::runtime::value::ThreadSafe,
        >>::construct(&provider, fields);
        assert_eq!(
            DynamicOwned::<ThreadSafe>::from_runtime(constructed)
                .downcast::<u32>()
                .ok(),
            Some(23)
        );

        let base = RuntimeDynamicOwned::<crate::runtime::value::ThreadSafe>::new(5_u32);
        let replacements = vec![Some(RuntimeDynamicOwned::<crate::runtime::value::ThreadSafe>::new(
            31_u32,
        ))];
        let updated = <ConstructProvider<u32, ThreadSafe> as RuntimeConstructProvider<
            crate::runtime::value::ThreadSafe,
        >>::update(&provider, base, replacements);
        assert_eq!(
            DynamicOwned::<ThreadSafe>::from_runtime(updated).downcast::<u32>().ok(),
            Some(31)
        );
    }

    #[test]
    fn test_providers_can_be_registered_for_both_modes() {
        static LOCAL: ConstructProvider<u8, Local> = ConstructProvider::new(construct_local, update_local);
        static THREAD_SAFE: ConstructProvider<u32, ThreadSafe> =
            ConstructProvider::new(construct_thread_safe, update_thread_safe);

        let providers = ConstructionProviders::new(Some(&LOCAL)).with_thread_safe(&THREAD_SAFE);
        assert!(providers.for_mode::<Local>().is_some());
        assert!(providers.for_mode::<ThreadSafe>().is_some());
    }
}
