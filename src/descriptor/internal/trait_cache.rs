// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Per-key initialization cells for externally mapped supertraits.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::descriptor::TraitDescriptor;
use crate::expression::GenericArgument;
use crate::identity::ExternalTraitId;

pub(in crate::descriptor) type ExternalSupertraitKey = (TypeId, ExternalTraitId, Box<[GenericArgument]>);
pub(in crate::descriptor) type ExternalSupertraitCell = Arc<OnceLock<&'static TraitDescriptor>>;

static EXTERNAL_SUPERTRAITS: LazyLock<Mutex<HashMap<ExternalSupertraitKey, ExternalSupertraitCell>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Returns the shared one-time initializer for one exact external application.
pub(in crate::descriptor) fn external_supertrait_cell(key: ExternalSupertraitKey) -> ExternalSupertraitCell {
    let mut cache = EXTERNAL_SUPERTRAITS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.entry(key).or_insert_with(|| Arc::new(OnceLock::new())).clone()
}
