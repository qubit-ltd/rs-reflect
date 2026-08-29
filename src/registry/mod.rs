//! Reflection registry APIs.

mod builder;
pub(crate) mod fragment;
mod indexes;
#[allow(
    clippy::module_inception,
    reason = "the planned file layout names the central registry type registry.rs"
)]
mod registry;

pub(crate) use builder::build_registry;
pub(crate) use builder::initialize_registry;
pub use registry::ReflectRegistry;
pub use registry::TraitCandidates;
pub use registry::TypeCandidates;
