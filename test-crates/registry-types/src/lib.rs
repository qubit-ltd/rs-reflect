//! Shared reflected root used by the cross-crate registry fixture.

#[derive(qubit_reflect::Reflect)]
/// A reflected type shared by the cross-crate registry fixtures.
pub struct RegistryUser {
    /// The identifier used by registry aggregation assertions.
    pub id: u8,
}

#[qubit_reflect::reflect]
/// A dependency-owned dyn-compatible trait with an inherited binding.
pub trait RegistryDynParent {
    /// The value exposed by the parent trait.
    type Item;

    /// Returns the parent value.
    fn parent(&self) -> Self::Item;
}
