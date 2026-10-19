// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[qubit_reflect::Reflect]
trait Service {
    const LIMIT: usize;

    fn transform<T>(&self, value: T) -> T;
}

#[qubit_reflect::Reflect]
trait SizedService {
    fn value(&self) -> usize
    where
        Self: Sized;
}

#[qubit_reflect::Reflect]
trait StaticService {
    fn create() -> usize;
}

#[qubit_reflect::Reflect]
trait ReturnsSelf {
    fn duplicate(&self) -> Self;
}

#[qubit_reflect::Reflect]
trait OpaqueMethodService {
    fn opaque(&self) -> impl std::fmt::Debug;

    fn takes_opaque(&self, value: impl std::fmt::Debug);
}

#[qubit_reflect::Reflect]
trait AsyncService {
    async fn run(&self);
}

struct Implementation;

impl Service for Implementation {
    const LIMIT: usize = 8;

    fn transform<T>(&self, value: T) -> T {
        value
    }
}

fn main() {
    assert_eq!(
        Service::transform(&Implementation, 8),
        <Implementation as Service>::LIMIT
    );
}
