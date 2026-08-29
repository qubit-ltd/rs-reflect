// qubit-style: allow public-type-layout
//! Stable identities for distributed registration fragments.

/// The source and content identity of one registration fragment.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FragmentIdentity {
    declaring_crate: Box<str>,
    module_path: Box<str>,
    line: u32,
    column: u32,
    member_kind: Box<str>,
    content_fingerprint: u64,
}

impl FragmentIdentity {
    /// Creates a fragment identity from its stable source and content facts.
    pub fn new(
        declaring_crate: &str,
        module_path: &str,
        line: u32,
        column: u32,
        member_kind: &str,
        content_fingerprint: u64,
    ) -> Self {
        Self {
            declaring_crate: declaring_crate.into(),
            module_path: module_path.into(),
            line,
            column,
            member_kind: member_kind.into(),
            content_fingerprint,
        }
    }

    /// Returns the crate that declared this fragment.
    pub fn declaring_crate(&self) -> &str {
        &self.declaring_crate
    }
    /// Returns the declaring module path.
    pub fn module_path(&self) -> &str {
        &self.module_path
    }
    /// Returns the declaration line number.
    pub fn line(&self) -> u32 {
        self.line
    }
    /// Returns the declaration column number.
    pub fn column(&self) -> u32 {
        self.column
    }
    /// Returns the category of members declared by this fragment.
    pub fn member_kind(&self) -> &str {
        &self.member_kind
    }
    /// Returns the deterministic fingerprint of normalized macro input.
    pub fn content_fingerprint(&self) -> u64 {
        self.content_fingerprint
    }
}
