// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compile-time handshake between generated code and its runtime.
//!
//! Derive output should reference this constant through
//! `qubit_reflect::__private::codegen_v4::protocol`. A macro can compare the
//! expected value in a const context so incompatible generated code fails at
//! compile time.

/// Version of the generated-code/runtime protocol.
pub const PROTOCOL_VERSION: u32 = 4;
