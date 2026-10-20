// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::ReflectionContext;

const MAX_INPUT_BYTES: usize = 4_096;

// The vNext facade no longer exposes standalone identity parsers. Exercise its
// public name-lookup boundary with arbitrary lossy UTF-8 input instead.
fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let query = String::from_utf8_lossy(data);
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("the linked reflection context must initialize");

    let lookup = context
        .lookup_type_by_short_name(&query)
        .expect("a frozen context must answer short-name lookups");
    assert_eq!(lookup.is_unique(), lookup.len() == 1);
    for descriptor in lookup.candidates() {
        assert_eq!(descriptor.short_name(), query.as_ref());
    }
});
