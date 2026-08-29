//! Common immutable naming facts for reflected methods.

/// The base description of a reflected method declaration.
///
/// Signature, invocation, and specialization data are added by the
/// method-descriptor layer.
#[derive(Debug)]
pub struct MethodDescriptor {
    rust_name: &'static str,
    query_name: &'static str,
}

impl MethodDescriptor {
    /// Creates immutable base method facts for generated descriptor data.
    #[doc(hidden)]
    pub const fn new(rust_name: &'static str, query_name: &'static str) -> Self {
        Self { rust_name, query_name }
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }
}
