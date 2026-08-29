//! Concrete generic-instance navigation shared by root descriptors.

use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;

/// A concrete instance's link to its declaration-level generic definition and
/// arguments.
#[derive(Debug)]
pub struct ConcreteGenericDescriptor {
    definition: &'static GenericDefinitionDescriptor,
    arguments: &'static [GenericArgument],
}

impl ConcreteGenericDescriptor {
    /// Creates an immutable concrete generic view.
    #[doc(hidden)]
    pub const fn new(definition: &'static GenericDefinitionDescriptor, arguments: &'static [GenericArgument]) -> Self {
        Self { definition, arguments }
    }

    /// Returns the declaration shared by every concrete instance.
    pub const fn definition(&self) -> &'static GenericDefinitionDescriptor {
        self.definition
    }

    /// Returns concrete arguments in declaration order.
    pub const fn arguments(&self) -> &'static [GenericArgument] {
        self.arguments
    }
}
