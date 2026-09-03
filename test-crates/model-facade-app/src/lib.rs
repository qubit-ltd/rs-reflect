// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Terminal fixture: it depends only on the two downstream facade crates.

mod facade_envelope;
mod facade_event;
mod facade_service;
mod facade_service_impl;
mod facade_user;

pub use facade_envelope::FacadeEnvelope;
pub use facade_event::FacadeEvent;
pub use facade_service::FacadeService;
pub use facade_user::FacadeUser;

#[cfg(test)]
mod tests {
    use model_facade_runtime::Reflect;
    use model_facade_runtime::ReflectRegistry;

    use super::FacadeEnvelope;
    use super::FacadeEvent;
    use super::FacadeService;
    use super::FacadeUser;

    #[test]
    fn downstream_facade_delegates_reflect_derive() {
        let descriptor = FacadeUser::type_descriptor();
        assert!(descriptor.type_name().ends_with("FacadeUser"));

        let registry = ReflectRegistry::initialize().expect("facade fragment registers");
        assert!(registry.get(descriptor.type_id()).is_some());
        assert!(descriptor.struct_construction().is_some());
        assert!(FacadeEnvelope::<String>::type_descriptor()
            .concrete_generic()
            .is_some());
        assert!(FacadeEvent::type_descriptor()
            .variants()
            .iter()
            .all(|variant| variant.construction().is_some()));
        assert_eq!(FacadeUser { id: 7 }.label(), "facade");
    }
}
