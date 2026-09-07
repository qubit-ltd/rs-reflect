// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Associated-constant shape analysis used by trait expansion.

use crate::ir::TypeIr;

/// Proves that an associated constant's type has no unresolved lifetime.
pub(super) fn has_proven_static_shape(ty: &TypeIr) -> bool {
    super::associated_const_type_has_proven_static_shape_impl(ty)
}

#[cfg(test)]
mod tests {
    use super::has_proven_static_shape;

    #[test]
    fn helper_is_available_as_a_narrow_parent_api() {
        let ty = crate::parse::convert_type(
            &syn::parse_str::<syn::Type>("u32").expect("a primitive type must parse"),
        );
        assert!(has_proven_static_shape(&ty));
    }
}
