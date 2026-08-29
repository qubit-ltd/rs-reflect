//! Safe two-phase construction and owned-update runtime contracts.
//!
//! Construction first validates shape, binding, policy, and exact Rust type
//! identity without extracting any owned input. Only a
//! [`ValidatedConstructionInput`](crate::construct::ValidatedConstructionInput)
//! or [`ValidatedUpdateInput`](crate::construct::ValidatedUpdateInput) can cross the
//! generated adapter boundary. Validation failures retain every caller-owned
//! value in [`ConstructionRecovery`](crate::construct::ConstructionRecovery).

mod error;
mod input;
mod recovery;
mod struct_constructor;
mod update;
mod validated;
mod variant_constructor;

pub use error::{
    ConstructionError, ConstructionFieldId, ConstructionShape, ConstructionUnavailableReason,
};
pub use input::{
    ConstructionDefaultProvider, ConstructionField, ConstructionFieldPolicy,
    NamedConstructionInput, StructUpdateInput, TupleConstructionInput, UpdateField,
    UpdateFieldPolicy,
};
pub use recovery::{ConstructionRecovery, RecoveredConstructionValue};
pub use struct_constructor::{StructConstructionAdapter, StructConstructor};
pub use update::{StructUpdateAdapter, StructUpdater};
pub use validated::{ValidatedConstructionInput, ValidatedOverride, ValidatedUpdateInput};
pub use variant_constructor::{VariantConstructionAdapter, VariantConstructor};
