// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Conservative dyn-compatibility analysis helpers.

use syn::ItemTrait;

use crate::ir::TraitDeclarationIr;

pub(super) fn is_provably_dyn_compatible(
    item: &ItemTrait,
    declaration: &TraitDeclarationIr,
) -> bool {
    super::is_provably_dyn_compatible(item, declaration)
}

#[cfg(test)]
mod tests {
    #[test]
    fn dyn_analysis_module_is_loaded() {
        let _entry = super::is_provably_dyn_compatible;
    }
}
