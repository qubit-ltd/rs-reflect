// qubit-style: allow public-type-layout
//! Source policies controlling reflected field access.

/// The source policy controlling which field adapters callers may use.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FieldAccessPolicy {
    /// Shared reads, mutable reads, and replacement may be exposed.
    ReadWrite,
    /// Shared reads may be exposed, but mutation is forbidden.
    ReadOnly,
    /// Structural facts remain visible, but every dynamic access is forbidden.
    Skipped,
}
