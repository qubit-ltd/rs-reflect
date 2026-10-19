// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::marker::PhantomData;

use qubit_reflect::TypeDescriptor;
// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[qubit_reflect::Reflect]
struct Borrowed<'scope> {
    marker: PhantomData<&'scope ()>,
}

fn main() {
    let _ = Borrowed {
        marker: PhantomData,
    };
    assert_eq!(TypeDescriptor::of::<Borrowed<'static>>().name(), "Borrowed");
}
