// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::TypeDescriptor;
// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

#[qubit_reflect::Reflect]
struct Page<T>(PhantomData<T>);

#[qubit_reflect::ReflectImpl]
impl<T> Page<T> {
    fn reflected_count() -> usize {
        1
    }
}

fn main() {
    assert_eq!(TypeDescriptor::of::<Page<u8>>().name(), "Page");
}
