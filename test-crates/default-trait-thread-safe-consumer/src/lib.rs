// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Consumer-side fixtures for reflected default trait methods across crates.

#![forbid(unsafe_code)]

mod local_target;
mod shared_target;

pub use local_target::LocalTarget;
pub use local_target::local_marker_count;
pub use local_target::new_local_target;
pub use shared_target::SharedTarget;
