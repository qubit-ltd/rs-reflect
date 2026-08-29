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

#[doc(hidden)]
pub use descriptor::intern_type;
pub use inventory;
#[doc(hidden)]
pub use registration::FragmentKind;
#[doc(hidden)]
pub use registration::FragmentPayload;
#[doc(hidden)]
pub use registration::RegistrationFragment;
#[doc(hidden)]
pub use registration::RuntimeIdentity;
#[doc(hidden)]
pub use registration::StaticFragmentIdentity;

#[doc(hidden)]
pub use crate::descriptor::TraitImplPayload;
#[doc(hidden)]
pub use crate::descriptor::external_supertrait;
