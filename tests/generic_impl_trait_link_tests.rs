//! Standalone integration target for generic impl trait-link regressions.

#![cfg(feature = "derive")]

#[path = "registry/generic_impl_trait_link_tests.rs"]
mod registry_generic_impl_trait_link_tests;
