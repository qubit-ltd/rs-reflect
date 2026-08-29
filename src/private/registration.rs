//! Hidden distributed-registration protocol for generated macro output.

use std::sync::OnceLock;

use crate::error::RegistryError;
use crate::registry::ReflectRegistry;

#[doc(hidden)]
pub use crate::registry::fragment::{
    CapabilityRegistration, FragmentKind, FragmentPayload, RegistrationFragment, RuntimeIdentity,
    StaticFragmentIdentity,
};

/// Builds and validates an isolated registry snapshot from static fragments.
///
/// This entry point exists for generated-code integration and conformance
/// fixtures. Ordinary applications should call [`ReflectRegistry::initialize`].
/// Returns [`RegistryError`] only after checking the complete fragment set.
#[doc(hidden)]
pub fn build_registry(
    fragments: &[&'static RegistrationFragment],
) -> Result<ReflectRegistry, RegistryError> {
    crate::registry::build_registry(fragments)
}

/// Initializes a caller-supplied cache from static fragments.
///
/// The first complete success or failure is retained in `cache`; later calls
/// return the same registry reference or a clone of the cached error even when
/// supplied a different fragment slice.
#[doc(hidden)]
pub fn initialize_registry(
    cache: &'static OnceLock<Result<ReflectRegistry, RegistryError>>,
    fragments: &'static [&'static RegistrationFragment],
) -> Result<&'static ReflectRegistry, RegistryError> {
    crate::registry::initialize_registry(cache, fragments)
}
