// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod type_expression_model_tests;

#[test]
fn test_function_pointer_parameter_and_result_edges_are_reachable() {
    // Roots 0..5 retain all five derive-generated fields; root 8 is fn(u16) ->
    // usize.
    let parameter = type_expression_model_tests::check(&[8, 0]);
    assert_eq!(parameter.function_parameter_edges, 1);
    assert_eq!(parameter.function_result_edges, 0);
    assert!(parameter.visited_u16);
    assert!(!parameter.visited_usize);

    let result = type_expression_model_tests::check(&[8, 1]);
    assert_eq!(result.function_parameter_edges, 0);
    assert!(result.function_result_edges > 0);
    assert!(!result.visited_u16);
    assert!(result.visited_usize);
}

#[test]
fn test_all_added_static_roots_are_actually_visited() {
    let mut associated_types = 0;
    let mut trait_objects = 0;
    let mut opaque_types = 0;
    // Bounded exhaustive root and child-byte selections over the static corpus.
    for root in 0..12_u8 {
        for selection in 0..=u8::MAX {
            let report = type_expression_model_tests::check(&[root, selection]);
            associated_types += report.associated_types;
            trait_objects += report.trait_objects;
            opaque_types += report.opaque_types;
        }
    }
    assert!(associated_types > 0, "associated-type root branch was never visited");
    assert!(trait_objects > 0, "trait-object root branch was never visited");
    assert!(opaque_types > 0, "opaque root branch was never visited");
}

#[test]
fn test_function_signature_boundaries_stop_or_visit_the_only_child() {
    // Root 9 has no parameters and a result; root 10 has a parameter and no result;
    // root 11 is an empty signature.
    let no_arguments = type_expression_model_tests::check(&[9, 255]);
    assert_eq!(no_arguments.function_parameter_edges, 0);
    assert_eq!(no_arguments.function_result_edges, 1);
    assert!(no_arguments.visited_usize);

    let no_result = type_expression_model_tests::check(&[10, 255]);
    assert_eq!(no_result.function_parameter_edges, 1);
    assert_eq!(no_result.function_result_edges, 0);
    assert!(no_result.visited_u16);

    let empty = type_expression_model_tests::check(&[11, 255]);
    assert_eq!(empty.function_parameter_edges, 0);
    assert_eq!(empty.function_result_edges, 0);
}

#[test]
fn test_empty_extreme_and_oversized_inputs_are_bounded() {
    let empty = type_expression_model_tests::check(&[]);
    assert_eq!(empty.function_parameter_edges, 0);
    let extreme = type_expression_model_tests::check(&[8, 255]);
    assert!(extreme.function_result_edges > 0);

    let mut long = vec![8, 1];
    long.resize(4_097, 255);
    let oversized = type_expression_model_tests::check(&long);
    assert_eq!(oversized.function_result_edges, 1);
    assert!(oversized.visited_usize);
}
