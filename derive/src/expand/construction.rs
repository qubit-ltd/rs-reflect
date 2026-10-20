// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Token contract tests for safe struct and enum construction codegen.

#[cfg(test)]
mod tests {
    use quote::quote;

    use crate::expand::ExpansionContext;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::parse::parse_declaration;

    #[test]
    fn test_struct_construction_emits_codegen_v4_only() {
        let parsed = parse_declaration(
            MacroKind::Derive,
            quote!(),
            quote! {
                struct Fixture {
                    value: u8,
                }
            },
        )
        .expect("the construction fixture must parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the construction fixture must produce a type declaration");
        };
        let generated = crate::expand::structs::expand(declaration, &ExpansionContext::for_tests())
            .expect("the struct should expand")
            .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "struct construction must use the v4 protocol"
        );
        assert!(
            !generated.contains(concat!("codegen_", "v3")),
            "struct construction must not use the v3 protocol"
        );
        assert!(
            generated.contains("ConstructProvider"),
            "struct construction must emit a typed provider"
        );
        assert!(
            generated.contains("ConstructionPlan :: for_type"),
            "struct construction must emit a plan"
        );
    }

    #[test]
    fn test_variant_construction_emits_codegen_v4_only() {
        let parsed = parse_declaration(
            MacroKind::Derive,
            quote!(),
            quote! {
                enum Fixture {
                    Value(u8),
                }
            },
        )
        .expect("the construction fixture must parse");
        let DeclarationIr::Type(declaration) = parsed.declaration else {
            panic!("the construction fixture must produce a type declaration");
        };
        let generated = crate::expand::enums::expand(declaration, &ExpansionContext::for_tests())
            .expect("the enum should expand")
            .to_string();

        assert!(
            generated.contains("codegen_v4"),
            "variant construction must use the v4 protocol"
        );
        assert!(
            !generated.contains(concat!("codegen_", "v3")),
            "variant construction must not use the v3 protocol"
        );
        assert!(
            generated.contains("ConstructProvider"),
            "variant construction must emit a typed provider"
        );
        assert!(
            generated.contains("ConstructionPlan :: for_variant"),
            "variant construction must emit a plan"
        );
    }
}
