// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Token expansion for validated reflection declarations.

mod construction;
mod context;
mod dispatcher;
mod enums;
mod expression_codegen;
mod generics;
mod impls;
mod invocation;
mod structs;
mod traits;

pub(crate) use context::ExpansionContext;
pub(crate) use dispatcher::dispatch;
