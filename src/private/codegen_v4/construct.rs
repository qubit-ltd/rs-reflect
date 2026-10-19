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

use crate::runtime::construct::ConstructProvider as RuntimeConstructProvider;
use crate::runtime::value::{DynamicOwned as RuntimeDynamicOwned, Local as RuntimeLocal};

use super::value::{DynamicOwned, Local, ThreadSafe, ValueFor, ValueMode};

pub use crate::runtime::construct::{
    ConstructField, ConstructionAvailability, ConstructionFieldPolicy, ConstructionPlan,
    ConstructionProviders, ConstructionShape, ConstructionUnavailableReason, UpdateFieldPolicy,
    UpdateUnavailableReason,
};

/// Typed construction and update callbacks using only codegen-v4 dynamic values.
pub struct ConstructProvider<T: 'static, M: ValueMode> {
    construct: fn(Vec<Option<DynamicOwned<M>>>) -> T,
    update: fn(T, Vec<Option<DynamicOwned<M>>>) -> T,
    _type: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static, M: ValueMode> ConstructProvider<T, M> {
    /// Creates typed callbacks for constructing and updating one concrete target type.
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
    fn construct(
        &self,
        fields: Vec<Option<RuntimeDynamicOwned<RuntimeLocal>>>,
    ) -> RuntimeDynamicOwned<RuntimeLocal> {
        let fields = fields
            .into_iter()
            .map(|field| field.map(DynamicOwned::<Local>::from_runtime))
            .collect();
        DynamicOwned::<Local>::new((self.construct)(fields)).into_runtime()
    }

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

impl<T> RuntimeConstructProvider<crate::runtime::value::ThreadSafe>
    for ConstructProvider<T, ThreadSafe>
where
    T: ValueFor<ThreadSafe>,
{
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
    /// Creates providers with the local mode, which works for all reflected values.
    pub const fn new<T: ValueFor<Local>>(
        local: Option<&'static ConstructProvider<T, Local>>,
    ) -> Self {
        let local = match local {
            Some(provider) => Some(provider as &dyn RuntimeConstructProvider<RuntimeLocal>),
            None => None,
        };
        Self::from_runtime_providers(local, None)
    }

    /// Adds a thread-safe provider when the target and its fields are `Send + Sync`.
    pub const fn with_thread_safe<T: ValueFor<ThreadSafe>>(
        self,
        provider: &'static ConstructProvider<T, ThreadSafe>,
    ) -> Self {
        self.with_runtime_thread_safe(
            provider as &dyn RuntimeConstructProvider<crate::runtime::value::ThreadSafe>,
        )
    }
}
