//! Dynamic operations supported by reflected field adapters.

use std::fmt;

/// The dynamic field operation that was requested.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FieldAccessOperation {
    /// Shared field access.
    Get,
    /// Mutable field access.
    GetMut,
    /// Whole-value field replacement.
    Set,
}

impl fmt::Display for FieldAccessOperation {
    /// Formats the operation using its public API spelling.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Get => formatter.write_str("get"),
            Self::GetMut => formatter.write_str("get_mut"),
            Self::Set => formatter.write_str("set"),
        }
    }
}
