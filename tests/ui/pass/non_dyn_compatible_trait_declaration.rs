// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use qubit_reflect::reflect;

#[reflect]
trait Service {
    const LIMIT: usize;

    fn transform<T>(&self, value: T) -> T;
}

#[reflect(external_trait(Sized, id = "core.marker.Sized"))]
trait SizedService: Sized {
    fn value(&self) -> usize;
}

#[reflect]
trait StaticService {
    fn create() -> usize;
}

#[reflect]
trait ReturnsSelf {
    fn duplicate(&self) -> Self;
}

#[reflect]
trait OpaqueMethodService {
    fn opaque(&self) -> impl std::fmt::Debug;

    fn takes_opaque(&self, value: impl std::fmt::Debug);
}

#[reflect]
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
