// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread::ThreadId;

use super::ReflectionContext;
use super::builder::ReflectionContextBuilder;
use super::data::ContextData;
use crate::runtime::error::ContextError;

#[derive(Default)]
struct InitializationState {
    building: bool,
    owner: Option<ThreadId>,
    result: Option<Result<Arc<ContextData>, ContextError>>,
    #[cfg(test)]
    waiters: usize,
}

pub(crate) struct GlobalContext {
    state: Mutex<InitializationState>,
    changed: Condvar,
    #[cfg(test)]
    waiter_changed: Condvar,
}

impl GlobalContext {
    pub(crate) const fn new() -> Self {
        Self {
            state: Mutex::new(InitializationState {
                building: false,
                owner: None,
                result: None,
                #[cfg(test)]
                waiters: 0,
            }),
            changed: Condvar::new(),
            #[cfg(test)]
            waiter_changed: Condvar::new(),
        }
    }

    pub(crate) fn data(&self) -> Result<Arc<ContextData>, ContextError> {
        self.get_or_initialize(|| ReflectionContextBuilder::new().include_linked().build_data())
    }

    pub(crate) fn get_or_initialize(
        &self,
        initialize: impl FnOnce() -> Result<ContextData, ContextError>,
    ) -> Result<Arc<ContextData>, ContextError> {
        let mut initialize = Some(initialize);
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(result) = &state.result {
                return result.clone();
            }
            if !state.building {
                state.building = true;
                state.owner = Some(std::thread::current().id());
                drop(state);

                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    initialize.take().expect("initializer is consumed once")().map(Arc::new)
                }));

                let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                state.owner = None;
                state.building = false;
                self.changed.notify_all();
                match result {
                    Ok(result) => {
                        state.result = Some(result.clone());
                        return result;
                    }
                    Err(panic) => {
                        drop(state);
                        std::panic::resume_unwind(panic);
                    }
                }
            }
            if state.owner.as_ref() == Some(&std::thread::current().id()) {
                drop(state);
                panic!("recursive global reflection context initialization");
            }
            #[cfg(test)]
            {
                state.waiters += 1;
                self.waiter_changed.notify_all();
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            #[cfg(test)]
            {
                state.waiters -= 1;
            }
        }
    }

    pub(crate) fn is_initialized(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .result
            .is_some()
    }

    #[cfg(test)]
    pub(crate) fn state_lock_is_available(&self) -> bool {
        self.state.try_lock().is_ok()
    }

    #[cfg(test)]
    pub(crate) fn wait_for_waiters(&self, count: usize) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        while state.waiters < count {
            state = self
                .waiter_changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

pub(crate) fn global_context() -> &'static ReflectionContext {
    static GLOBAL_CONTEXT: std::sync::OnceLock<ReflectionContext> = std::sync::OnceLock::new();
    GLOBAL_CONTEXT.get_or_init(ReflectionContext::new_global)
}
