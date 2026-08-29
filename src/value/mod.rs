//! Dynamic value APIs.

mod dynamic_mut;
mod dynamic_owned;
mod dynamic_ref;
mod mode;
mod storage;

pub use dynamic_mut::DynamicMut;
pub use dynamic_owned::DynamicOwned;
pub use dynamic_ref::DynamicRef;
pub use mode::Local;
pub use mode::Mode;
pub use mode::ThreadSafe;

/// A locally scoped shared dynamic value borrow.
pub type ReflectedRef<'a> = DynamicRef<'a, Local>;
/// A locally scoped mutable dynamic value borrow.
pub type ReflectedMut<'a> = DynamicMut<'a, Local>;
/// A locally scoped owned dynamic value.
pub type ReflectedOwned = DynamicOwned<Local>;
/// A thread-safe shared dynamic value borrow.
pub type SendReflectedRef<'a> = DynamicRef<'a, ThreadSafe>;
/// A thread-safe mutable dynamic value borrow.
pub type SendReflectedMut<'a> = DynamicMut<'a, ThreadSafe>;
/// A thread-safe owned dynamic value.
pub type SendReflectedOwned = DynamicOwned<ThreadSafe>;
