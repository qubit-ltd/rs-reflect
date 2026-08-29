//! Common immutable naming facts for reflected traits.

/// The base description of a reflected trait declaration.
///
/// Supertraits, methods, and associated items are added by the trait-descriptor
/// layer.
#[derive(Debug)]
pub struct TraitDescriptor {
    rust_name: &'static str,
    rust_path: &'static str,
    query_name: &'static str,
}

impl TraitDescriptor {
    /// Creates immutable base trait facts for generated descriptor data.
    #[doc(hidden)]
    pub const fn new(rust_name: &'static str, rust_path: &'static str, query_name: &'static str) -> Self {
        Self {
            rust_name,
            rust_path,
            query_name,
        }
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the diagnostic fully qualified Rust path.
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the lookup name.
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }
}
