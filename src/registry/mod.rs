//! Reflection registry APIs.

mod builder;
pub(crate) mod fragment;
mod indexes;
#[allow(
    clippy::module_inception,
    reason = "the planned file layout names the central registry type registry.rs"
)]
mod registry;

pub use registry::{ReflectRegistry, TraitCandidates, TypeCandidates};

pub(crate) use builder::{build_registry, initialize_registry};
