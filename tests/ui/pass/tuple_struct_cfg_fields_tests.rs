// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[qubit_reflect::Reflect]
struct Conditional(
    #[cfg(any())] u8,
    #[cfg_attr(all(), cfg(any()))] u16,
    pub u32,
);

#[qubit_reflect::Reflect]
struct ConditionalNested(
    #[cfg_attr(all(), cfg_attr(all(), cfg(any())))] u8,
    #[cfg(feature = "derive")]
    #[cfg_attr(all(), cfg(any()))]
    u16,
    pub u32,
);

fn main() {
    let _ = Conditional(7);
    let _ = ConditionalNested(9);
}
