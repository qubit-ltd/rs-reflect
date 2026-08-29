//! Structural representations of Rust lifetime syntax.

/// A lifetime that appears in a type, bound, or generic declaration.
///
/// Named lifetimes omit the leading apostrophe so they can be compared and displayed without
/// retaining parser tokens.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LifetimeExpression {
    /// The distinguished `'static` lifetime.
    Static,
    /// A named lifetime such as `'a`, stored as `a`.
    Named(Box<str>),
    /// An elided lifetime supplied by Rust's lifetime elision rules.
    Elided,
    /// The anonymous placeholder lifetime `'_`.
    Placeholder,
}
