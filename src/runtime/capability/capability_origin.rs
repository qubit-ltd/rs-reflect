use crate::runtime::identity::RegistrationOrigin;

/// Records whether a capability belongs to a type or a context registration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapabilityOrigin {
    Intrinsic,
    Linked(RegistrationOrigin),
    Explicit(RegistrationOrigin),
}
