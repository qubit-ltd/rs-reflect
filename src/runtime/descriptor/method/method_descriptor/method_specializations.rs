// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A view over the concrete implementations registered for one method.
pub struct MethodSpecializations<'ctx> {
    /// Returns the unique method descriptor for the requested name.
    method: MethodDescriptor<'ctx>,
}

impl<'ctx> MethodSpecializations<'ctx> {
    /// Iterates over the registered specializations in registration order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = MethodSpecializationDescriptor<'ctx>> + 'ctx {
        let method = self.method;
        let specializations = method.specializations.clone();
        (0..specializations.len()).map(move |index| MethodSpecializationDescriptor {
            method: method.clone(),
            specializations: specializations.clone(),
            index,
        })
    }
    /// Returns the number of registered specializations.
    pub fn len(&self) -> usize {
        self.method.specializations.len()
    }

    /// Returns whether this method has no concrete specializations.
    pub fn is_empty(&self) -> bool {
        self.method.specializations.is_empty()
    }
}
