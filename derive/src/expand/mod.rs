// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Token expansion for validated reflection declarations.

pub(crate) mod bind_method;
mod construction;
mod dispatcher;
mod enum_repr_ir;
mod enums;
mod expansion_context;
mod expression_codegen;
mod generic_environment;
mod generics;
mod impls;
mod invocation;
mod structs;
mod traits;

pub(crate) use dispatcher::dispatch;
pub(crate) use expansion_context::ExpansionContext;
pub(crate) use expansion_context::fingerprint;
