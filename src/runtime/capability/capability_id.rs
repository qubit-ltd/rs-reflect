pub(crate) use crate::identity::CapabilityId as CapabilityIdentifier;

pub(crate) fn validate_capability_id(
    value: &'static str,
) -> Result<CapabilityIdentifier, crate::error::IdError> {
    CapabilityIdentifier::new(value)
}
