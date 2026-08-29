//! Common immutable facts for reflected implementation blocks.

use std::fmt;

use crate::descriptor::TraitDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;

/// Whether an implementation is inherent or implements a trait.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplKind {
    /// An inherent implementation block.
    Inherent,
    /// A trait implementation block.
    Trait,
}

/// The base description of a reflected implementation block.
///
/// Fragment identity and method collections are added by the impl-descriptor
/// layer.
pub struct ImplDescriptor {
    target_type: TypeDescriptorResolver,
    kind: ImplKind,
    implemented_trait: Option<&'static TraitDescriptor>,
}

impl ImplDescriptor {
    /// Creates immutable base impl facts for generated descriptor data.
    ///
    /// Inherent implementations pass `None` for `implemented_trait`; trait
    /// implementations pass the exact declaration descriptor.
    #[doc(hidden)]
    pub const fn new(
        target_type: TypeDescriptorResolver,
        kind: ImplKind,
        implemented_trait: Option<&'static TraitDescriptor>,
    ) -> Self {
        Self {
            target_type,
            kind,
            implemented_trait,
        }
    }

    /// Returns the reflected root targeted by this implementation.
    pub fn target_type(&self) -> &'static TypeDescriptor {
        (self.target_type)()
    }

    /// Returns whether this is an inherent or trait implementation.
    pub const fn kind(&self) -> ImplKind {
        self.kind
    }

    /// Returns the implemented trait declaration.
    ///
    /// `None` identifies an inherent implementation.
    pub const fn implemented_trait(&self) -> Option<&'static TraitDescriptor> {
        self.implemented_trait
    }
}

impl fmt::Debug for ImplDescriptor {
    /// Formats local facts without following the target descriptor recursively.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImplDescriptor")
            .field("target_type", &"<resolver>")
            .field("kind", &self.kind)
            .field("implemented_trait", &self.implemented_trait)
            .finish()
    }
}
