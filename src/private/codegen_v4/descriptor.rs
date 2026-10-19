// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Type registration and descriptor contracts for generated code.
//!
//! The final façade needs a typed `Reflect` registration hook and a
//! `TypeRegistration<T>` constructor that binds metadata to `T`. Runtime
//! counterparts currently live in `runtime::descriptor::{Reflect,
//! TypeRegistration, TypeMetadata}`; all are crate-private. Generated code
//! must not choose a raw `TypeIdentity` or metadata `TypeId` itself.
