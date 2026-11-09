// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// First concrete type with the shared short name in the ambiguity fixture.
pub(super) struct Thing;

impl crate::runtime::descriptor::Reflect for Thing {
    /// Returns the static registration for the first path fixture.
    fn __reflect_registration() -> &'static crate::runtime::descriptor::TypeRegistration<Self> {
        static REGISTRATION: crate::runtime::descriptor::TypeRegistration<Thing> =
            crate::runtime::descriptor::TypeRegistration::new();
        &REGISTRATION
    }
}
