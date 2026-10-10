// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A zero-sized typed view returned for a root descriptor whose type is marked
/// opaque.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect, opaque)]
/// struct Hidden;
/// let opaque = TypeDescriptor::of::<Hidden>().as_opaque().expect("opaque root");
/// assert_eq!(std::mem::size_of_val(opaque), 0);
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct OpaqueTypeView;
