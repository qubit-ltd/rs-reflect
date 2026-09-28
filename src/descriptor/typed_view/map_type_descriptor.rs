// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::MapKind;
use crate::__private::LazyTypeRef;
use crate::__private::TypeRefSource;
use crate::descriptor::TypeRef;

/// The typed view of a key-value map descriptor.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use qubit_reflect::TypeDescriptor;
/// let map = TypeDescriptor::of::<BTreeMap<u8, bool>>().as_map().expect("map type");
/// assert_eq!(map.kind(), qubit_reflect::descriptor::MapKind::BTreeMap);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MapTypeDescriptor {
    /// Standard-library map family.
    kind: MapKind,
    /// Eager or lazily resolved key type.
    key: TypeRefSource,
    /// Eager or lazily resolved value type.
    value: TypeRefSource,
}

impl MapTypeDescriptor {
    /// Creates a map view for internal descriptor construction.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library map family.
    /// - `key`: Eager key type reference.
    /// - `value`: Eager value type reference.
    ///
    /// # Returns
    ///
    /// Returns a map view backed by the supplied type references.
    pub(crate) const fn new(kind: MapKind, key: &'static TypeRef, value: &'static TypeRef) -> Self {
        Self {
            kind,
            key: TypeRefSource::Eager(key),
            value: TypeRefSource::Eager(value),
        }
    }

    /// Creates a map view whose key and value resolve independently on first
    /// navigation.
    ///
    /// # Parameters
    ///
    /// - `kind`: Standard-library map family.
    /// - `key`: Lazy key type source.
    /// - `value`: Lazy value type source.
    ///
    /// # Returns
    ///
    /// Returns a map view backed by the lazy type sources.
    pub(crate) const fn new_lazy(kind: MapKind, key: &'static LazyTypeRef, value: &'static LazyTypeRef) -> Self {
        Self {
            kind,
            key: TypeRefSource::Lazy(key),
            value: TypeRefSource::Lazy(value),
        }
    }

    /// Returns the concrete standard-library map family.
    ///
    /// # Returns
    ///
    /// Returns the map family represented by this view.
    #[must_use]
    #[inline]
    pub const fn kind(&self) -> MapKind {
        self.kind
    }

    /// Returns the map key type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic key type, initializing a lazy
    /// reference on first access.
    #[must_use]
    #[inline]
    pub fn key_type(&self) -> &'static TypeRef {
        self.key.get()
    }

    /// Returns the map value type.
    ///
    /// # Returns
    ///
    /// Returns the static resolved or symbolic value type, initializing a lazy
    /// reference on first access.
    #[must_use]
    #[inline]
    pub fn value_type(&self) -> &'static TypeRef {
        self.value.get()
    }
}
