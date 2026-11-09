// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::TypeDescriptor;

#[test]
fn test_reports_diagnostic_names_for_reflected_type_shapes() {
    let nominal = TypeDescriptor::of::<String>();
    assert_eq!(nominal.short_name(), "String");
    let names = [
        TypeDescriptor::of::<Vec<String>>(),
        TypeDescriptor::of::<Option<Vec<String>>>(),
        TypeDescriptor::of::<&'static String>(),
        TypeDescriptor::of::<[String; 2]>(),
        TypeDescriptor::of::<[String]>(),
        TypeDescriptor::of::<(String, u32)>(),
        TypeDescriptor::of::<fn(String) -> u32>(),
        TypeDescriptor::of::<unsafe fn(String) -> u32>(),
        TypeDescriptor::of::<extern "C" fn(String) -> u32>(),
    ];
    assert_eq!(names[0].short_name(), "Vec");
    assert_eq!(names[1].short_name(), "Option");
    for descriptor in &names[2..] {
        assert_eq!(descriptor.short_name(), descriptor.rust_type_name());
    }
}
