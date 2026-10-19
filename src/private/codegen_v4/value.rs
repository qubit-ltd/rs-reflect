// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Dynamic-value and mode contracts used by generated adapters.
//!
//! Derive output needs the `ValueMode`, `Local`, `ThreadSafe`, `DynamicOwned`,
//! `DynamicRef`, `DynamicMut`, `ValueFor`, `ValueForRef`, and `ValueForMut`
//! contracts. Their current definitions are crate-private under
//! `runtime::value`; `Reflect` is also required by conversion bounds. The
//! exported façade must preserve those trait bounds so generated code cannot
//! claim thread safety without Rust proving it.
