// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed linked-registration contracts consumed by reflection contexts.
//!
//! The current runtime stores `RegistrationFragment` and its inventory
//! collector under `runtime::context`; both are crate-private. Derive needs a
//! safe submit macro or typed registration constructor that binds a concrete
//! `Reflect` type to its generated registration and source origin. It must not
//! expose caller-constructed fragment identities, raw payload variants, or the
//! inventory macro as part of this protocol.
