// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Lookup errors retain the query and subject as structured data.

use qubit_reflect::ReflectionContext;
use qubit_reflect::error::LookupError;
use qubit_reflect::error::LookupQuery;
use qubit_reflect::error::LookupSubject;

#[test]
fn missing_type_error_keeps_the_exact_query() {
    let context = ReflectionContext::builder().build().expect("an empty context is valid");
    let result = context.type_by_name("missing::Type");
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("an absent type must fail lookup"),
    };

    assert!(matches!(
        error,
        LookupError::Missing {
            subject: LookupSubject::Type,
            query: LookupQuery::Name(name),
        } if name.as_ref() == "missing::Type"
    ));
}
