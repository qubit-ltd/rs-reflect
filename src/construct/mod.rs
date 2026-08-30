//! Safe two-phase construction and owned-update runtime contracts.
//!
//! Construction first validates shape, binding, policy, and exact Rust type
//! identity without extracting any owned input. Only a
//! [`ValidatedConstructionInput`](crate::construct::ValidatedConstructionInput)
//! or [`ValidatedUpdateInput`](crate::construct::ValidatedUpdateInput) can
//! cross the generated adapter boundary. Validation failures retain every
//! caller-owned
//! value in [`ConstructionRecovery`](crate::construct::ConstructionRecovery).

mod error;
mod input;
mod recovery;
mod struct_construction_descriptor;
mod struct_constructor;
mod update;
mod validated;
mod variant_construction_descriptor;
mod variant_constructor;

pub use error::ConstructionError;
pub use error::ConstructionFieldId;
pub use error::ConstructionShape;
pub use error::ConstructionUnavailableReason;
pub use input::ConstructionDefaultProvider;
pub use input::ConstructionField;
pub use input::ConstructionFieldPolicy;
pub use input::NamedConstructionInput;
pub use input::StructUpdateInput;
pub use input::TupleConstructionInput;
pub use input::UpdateField;
pub use input::UpdateFieldPolicy;
pub use recovery::ConstructionRecovery;
pub use recovery::RecoveredConstructionValue;
pub use struct_construction_descriptor::StructConstructionDescriptor;
pub use struct_constructor::StructConstructionAdapter;
pub use struct_constructor::StructConstructor;
pub use update::StructUpdateAdapter;
pub use update::StructUpdater;
pub use validated::ValidatedConstructionInput;
pub use validated::ValidatedOverride;
pub use validated::ValidatedUpdateInput;
pub use variant_construction_descriptor::VariantConstructionDescriptor;
pub use variant_constructor::VariantConstructionAdapter;
pub use variant_constructor::VariantConstructor;
