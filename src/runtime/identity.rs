use std::any::TypeId;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TypeIdentity {
    pub type_id: TypeId,
    pub rust_name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ItemKind {
    Type,
    TypeDefinition,
    TraitDefinition,
    TraitApplication,
    Impl,
    Field,
    Variant,
    Method,
    MethodSpecialization,
    Parameter,
    AssociatedType,
    AssociatedConst,
    Capability,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StableItemId(pub Box<str>);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ItemIdentity {
    pub kind: ItemKind,
    pub stable_id: StableItemId,
    pub display_name: Box<str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistrationOrigin {
    pub crate_name: &'static str,
    pub crate_version: Option<&'static str>,
    pub module_path: &'static str,
    pub file: Option<&'static str>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LookupCandidate {
    pub identity: ItemIdentity,
    pub display_name: Box<str>,
    pub origin: Option<RegistrationOrigin>,
}

pub trait IntoLookupCandidate {
    fn into_lookup_candidate(self) -> LookupCandidate;
}

impl IntoLookupCandidate for LookupCandidate {
    fn into_lookup_candidate(self) -> LookupCandidate { self }
}
