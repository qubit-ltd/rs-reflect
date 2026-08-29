//! Shared reflected root used by the cross-crate registry fixture.

#[derive(qubit_reflect::Reflect)]
pub struct RegistryUser {
    pub id: u8,
}
