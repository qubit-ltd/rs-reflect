// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Lazily resolved relationships between immutable type descriptors.

use std::fmt;
use std::sync::OnceLock;

use crate::descriptor::Reflect;
use crate::descriptor::TypeRef;

/// A process-lifetime relationship that resolves its target on first
/// navigation.
///
/// Root descriptor construction stores this handle without invoking the
/// target type's [`Reflect`] implementation. The cached [`TypeRef`] is
/// immutable and shared by every subsequent query.
#[doc(hidden)]
pub struct LazyTypeRef {
    resolver: fn() -> TypeRef,
    resolved: OnceLock<TypeRef>,
}

impl LazyTypeRef {
    /// Creates a deferred resolved relationship to `T`.
    ///
    /// This constructor does not query `T`; resolution occurs only when
    /// [`Self::get`] is called.
    #[must_use]
    pub(crate) const fn resolved<T: Reflect + ?Sized>() -> Self {
        Self {
            resolver: resolve::<T>,
            resolved: OnceLock::new(),
        }
    }

    /// Returns the relationship target, resolving and caching it on the first
    /// navigation.
    ///
    /// Concurrent callers receive the same cached object. A resolver panic
    /// propagates unchanged and leaves the handle available for a later retry.
    #[must_use]
    #[inline]
    pub fn get(&'static self) -> &'static TypeRef {
        self.resolved.get_or_init(self.resolver)
    }
}

impl fmt::Debug for LazyTypeRef {
    /// Formats cached state without forcing relation resolution.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.resolved.get() {
            Some(reference) => formatter.debug_tuple("LazyTypeRef").field(reference).finish(),
            None => formatter.debug_tuple("LazyTypeRef").field(&"<unresolved>").finish(),
        }
    }
}

/// Resolves `T` through its unique interned root descriptor.
fn resolve<T: Reflect + ?Sized>() -> TypeRef {
    TypeRef::Resolved(T::type_descriptor())
}

#[cfg(test)]
mod tests {
    use std::sync::Barrier;
    use std::sync::OnceLock;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use super::LazyTypeRef;
    use crate::descriptor::Reflect;
    use crate::descriptor::TypeDescriptor;

    static RESOLUTIONS: AtomicUsize = AtomicUsize::new(0);

    struct CountedTarget;

    impl Reflect for CountedTarget {
        fn type_descriptor() -> &'static TypeDescriptor {
            static DESCRIPTOR: OnceLock<TypeDescriptor> = OnceLock::new();
            RESOLUTIONS.fetch_add(1, Ordering::SeqCst);
            DESCRIPTOR.get_or_init(|| TypeDescriptor::new_opaque::<Self>("CountedTarget"))
        }
    }

    #[test]
    fn test_concurrent_first_navigation_executes_resolver_once() {
        static RELATION: LazyTypeRef = LazyTypeRef::resolved::<CountedTarget>();
        let barrier = std::sync::Arc::new(Barrier::new(32));
        let handles: Vec<_> = (0..32)
            .map(|_| {
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    RELATION.get() as *const _ as usize
                })
            })
            .collect();
        let pointers: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("resolver thread must finish"))
            .collect();

        assert!(pointers.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(RESOLUTIONS.load(Ordering::SeqCst), 1);
    }
}
