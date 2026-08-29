//! Internal dependencies used by generated reflection code.

/// Hidden compile-time assertion helpers used by generated code.
#[doc(hidden)]
pub mod assertions;
/// Hidden static descriptor factories shared with generated reflection code.
#[doc(hidden)]
pub mod descriptor;
/// Hidden distributed-registration protocol used by generated code.
#[doc(hidden)]
pub mod registration;

pub use inventory;
#[doc(hidden)]
pub use crate::descriptor::TraitImplPayload;
#[doc(hidden)]
pub use descriptor::intern_type;
#[doc(hidden)]
pub use crate::descriptor::external_supertrait;
#[doc(hidden)]
pub use registration::{
    FragmentKind, FragmentPayload, RegistrationFragment, RuntimeIdentity, StaticFragmentIdentity,
};
