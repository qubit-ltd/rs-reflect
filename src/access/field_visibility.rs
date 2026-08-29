//! Visibility origin of reflected struct and enum-variant fields.

use crate::identity::Visibility;

/// The source visibility fact recorded for a reflected field.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FieldVisibility<'a> {
    /// A struct field's explicitly declared Rust visibility.
    Declared(&'a Visibility),
    /// An enum-variant field inherits the enum and variant access boundary.
    VariantInherited,
}

impl<'a> FieldVisibility<'a> {
    /// Returns the explicitly declared visibility of a struct field.
    ///
    /// `None` means this is an enum-variant field with inherited visibility.
    pub const fn as_declared(self) -> Option<&'a Visibility> {
        match self {
            Self::Declared(visibility) => Some(visibility),
            Self::VariantInherited => None,
        }
    }

    /// Returns whether this field inherits an enum variant's access boundary.
    pub const fn is_variant_inherited(self) -> bool {
        matches!(self, Self::VariantInherited)
    }
}
