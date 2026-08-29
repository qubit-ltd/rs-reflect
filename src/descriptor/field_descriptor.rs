//! Immutable structural facts about reflected fields.

use std::fmt;

use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::identity::Visibility;

/// A function that resolves a field or variant's declaring root descriptor.
///
/// Resolver indirection permits immutable static descriptor graphs to contain
/// cycles.
pub type TypeDescriptorResolver = fn() -> &'static TypeDescriptor;

/// The immutable structural description of one reflected field.
pub struct FieldDescriptor {
    declaring_type: TypeDescriptorResolver,
    index: usize,
    rust_name: Option<&'static str>,
    query_name: Option<&'static str>,
    field_type: &'static TypeRef,
    visibility: Visibility,
}

impl FieldDescriptor {
    /// Creates a frozen field descriptor for generated or hand-written
    /// descriptor data.
    ///
    /// The resolver must return the root containing this field. Positional
    /// fields use `None` for both names. This constructor performs no
    /// allocation and never calls the resolver.
    #[doc(hidden)]
    pub const fn new(
        declaring_type: TypeDescriptorResolver,
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
        field_type: &'static TypeRef,
        visibility: Visibility,
    ) -> Self {
        Self {
            declaring_type,
            index,
            rust_name,
            query_name,
            field_type,
            visibility,
        }
    }

    /// Returns the root type that contains this field.
    pub fn declaring_type(&self) -> &'static TypeDescriptor {
        (self.declaring_type)()
    }

    /// Returns the zero-based source declaration index.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust field name, or `None` for tuple and newtype fields.
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the lookup name, or `None` for tuple and newtype fields.
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the resolved, explicitly opaque, or symbolic field type.
    pub const fn field_type(&self) -> &'static TypeRef {
        self.field_type
    }

    /// Returns the normalized source visibility.
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }
}

impl fmt::Debug for FieldDescriptor {
    /// Formats local facts without following declaring or field-type
    /// relationships recursively.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldDescriptor")
            .field("declaring_type", &"<resolver>")
            .field("index", &self.index)
            .field("rust_name", &self.rust_name)
            .field("query_name", &self.query_name)
            .field("field_type", &self.field_type)
            .field("visibility", &self.visibility)
            .finish()
    }
}
