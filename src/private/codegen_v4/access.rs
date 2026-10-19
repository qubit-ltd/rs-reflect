// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Typed field and variant access adapters emitted by derive.
//!
//! The runtime counterparts are `FieldMetadata`, `TypeShapeRegistration`,
//! `FieldAdapter`, `DynamicFieldAdapter`, `VariantMetadata`, and
//! `VariantAdapter` in `runtime::descriptor` and `runtime::access`. Derive needs
//! safe typed constructors that keep each adapter tied to the owner and value
//! types. These internal traits and metadata are currently crate-private.
