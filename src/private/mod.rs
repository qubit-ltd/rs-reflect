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
