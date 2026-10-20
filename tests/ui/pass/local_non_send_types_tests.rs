// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::rc::Rc;

#[qubit_reflect::Reflect]
struct LocalOnly {
    #[reflect(skip)]
    value: Rc<u8>,
}

#[qubit_reflect::ReflectImpl]
impl LocalOnly {
    fn borrow(value: &Rc<u8>) -> u8 {
        **value
    }

    fn mutate(value: &mut Rc<u8>) {
        *value = Rc::new(1);
    }

    fn own(value: Rc<u8>) -> Rc<u8> {
        value
    }

    async fn future() -> Rc<u8> {
        Rc::new(1)
    }
}

fn main() {}
