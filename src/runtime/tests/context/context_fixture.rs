// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A distinct zero-sized type used as a context-test registration fixture.
#[derive(Clone)]
pub(super) struct ContextFixture<const ID: u8>;
