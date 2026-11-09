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

use super::ReflectionContext;
use super::builder::ReflectionContextBuilder;
use super::data::ContextData;
#[path = "initialization_state.rs"]
mod initialization_state;
use self::initialization_state::InitializationState;
use crate::runtime::error::ContextError;

/// Lazily initializes and shares the process-wide reflection context data.
pub(crate) struct GlobalContext {
    /// Initialization state guarded by the condition variable.
    state: Mutex<InitializationState>,
    /// Wakes threads waiting for initialization to complete.
    changed: Condvar,
    /// Wakes test code waiting for worker threads to reach the wait state.
    #[cfg(test)]
    waiter_changed: Condvar,
}

impl GlobalContext {
    /// Creates an uninitialized global context coordinator.
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

    /// Returns global context data, building it on the first call.
    pub(crate) fn data(&self) -> Result<Arc<ContextData>, ContextError> {
        self.get_or_initialize(|| ReflectionContextBuilder::new().include_linked().build_data())
    }

    /// Returns cached data or runs `initialize` exactly once.
    ///
    /// Concurrent callers wait for the active initializer. A panic clears the
    /// in-progress state and resumes unwinding so a later caller can retry.
    ///
    /// # Parameters
    /// - `initialize`: operation that builds the context data when needed.
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

    /// Reports whether initialization has completed successfully or failed.
    pub(crate) fn is_initialized(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .result
            .is_some()
    }

    /// Checks whether the state mutex can be acquired without waiting.
    #[cfg(test)]
    pub(crate) fn state_lock_is_available(&self) -> bool {
        self.state.try_lock().is_ok()
    }

    /// Waits until at least `count` callers are blocked on initialization.
    ///
    /// # Parameters
    /// - `count`: minimum number of waiting callers.
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

/// Returns the process-wide reflection context.
pub(crate) fn global_context() -> &'static ReflectionContext {
    static GLOBAL_CONTEXT: std::sync::OnceLock<ReflectionContext> = std::sync::OnceLock::new();
    GLOBAL_CONTEXT.get_or_init(ReflectionContext::new_global)
}
