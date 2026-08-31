//! Standalone integration target for ambiguous generic trait-alias linking.

#![cfg(feature = "derive")]

#[path = "registry/generic_impl_trait_link_ambiguity_tests.rs"]
mod registry_generic_impl_trait_link_ambiguity_tests;
