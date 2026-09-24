// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Process-lifetime interning for immutable concrete type descriptors.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::descriptor::TypeDescriptor;

type DescriptorCell = OnceLock<TypeDescriptor>;

static INTERNER: OnceLock<Mutex<HashMap<TypeId, &'static DescriptorCell>>> = OnceLock::new();

/// Interns the descriptor built for `T` and returns its unique root.
///
/// The factory runs without the interner map lock, so it may safely query other
/// descriptors. A factory panic propagates unchanged and leaves the cell
/// uninitialized for a later retry. Descriptor cells are intentionally leaked
/// because descriptors are process-lifetime immutable data.
pub(crate) fn intern<T: ?Sized + 'static>(build: fn() -> TypeDescriptor) -> &'static TypeDescriptor {
    let interner = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut descriptors = match interner.lock() {
        Ok(descriptors) => descriptors,
        Err(error) => error.into_inner(),
    };
    let cell = *descriptors
        .entry(TypeId::of::<T>())
        .or_insert_with(|| Box::leak(Box::new(DescriptorCell::new())));
    drop(descriptors);
    cell.get_or_init(build)
}
