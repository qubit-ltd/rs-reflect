//! Fixture proving that reflected members follow a Cargo feature.

use qubit_reflect::reflect;

/// A signature type available only when the fixture feature is enabled.
#[cfg(feature = "conditional-extra")]
pub struct FeatureOnlyType;

/// A trait whose members and helper validation follow the selected feature.
#[reflect]
pub trait FeatureConditional {
    #[cfg(target_pointer_width = "64")]
    /// Exists on 64-bit targets.
    fn pointer_width_64(&self);

    #[cfg(not(target_pointer_width = "64"))]
    /// Exists on targets that are not 64-bit.
    fn other_pointer_width(&self);

    #[cfg(feature = "conditional-extra")]
    #[reflect(rename = "enabled_feature_method")]
    /// Returns the feature-gated signature type.
    fn feature_method(&self) -> FeatureOnlyType;
}
