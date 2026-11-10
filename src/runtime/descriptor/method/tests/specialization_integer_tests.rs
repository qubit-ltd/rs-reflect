// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::method::MethodSpecializationArgument;
use crate::runtime::descriptor::method::MethodSpecializationSpec;
use crate::runtime::descriptor::method::NO_ADAPTERS;
use crate::runtime::descriptor::method::imports::ExpressionData;
use crate::runtime::descriptor::method::imports::GenericArgumentIdentity;
use crate::runtime::descriptor::method::imports::GenericParameterMetadata;

static U8_EXPRESSION: ExpressionData = ExpressionData::Named {
    path: "u8",
    definition: None,
    arguments: &[],
};
static U8_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::const_parameter("N", 0, &U8_EXPRESSION, None);
static PARAMETERS: [GenericParameterMetadata; 1] = [U8_PARAMETER];
static MAX_ARGUMENT: [MethodSpecializationArgument; 1] =
    [MethodSpecializationArgument::const_integer_literal(false, 255)];
static OVERFLOW_ARGUMENT: [MethodSpecializationArgument; 1] =
    [MethodSpecializationArgument::const_integer_literal(false, 256)];

#[test]
fn test_normalizes_untyped_integer_literal_using_declared_const_type() {
    let resolved = MethodSpecializationSpec::new(&MAX_ARGUMENT, NO_ADAPTERS)
        .resolve(&PARAMETERS)
        .expect("255 fits in u8");
    assert_eq!(
        resolved.arguments.as_slice(),
        &[GenericArgumentIdentity::Const("u8:255".into())]
    );
}

#[test]
fn test_rejects_untyped_integer_literal_outside_declared_const_type() {
    assert!(
        MethodSpecializationSpec::new(&OVERFLOW_ARGUMENT, NO_ADAPTERS)
            .resolve(&PARAMETERS)
            .is_err()
    );
}
