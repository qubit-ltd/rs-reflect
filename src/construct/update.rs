// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Atomic owned-struct update validation and adapter dispatch.

use std::any::TypeId;
use std::fmt;

use crate::construct::ConstructionRecovery;
use crate::construct::StructUpdateInput;
use crate::construct::UpdateField;
use crate::construct::ValidatedUpdateInput;
use crate::construct::struct_constructor::local_type_id;
use crate::construct::struct_constructor::thread_safe_type_id;
use crate::descriptor::TypeDescriptor;
use crate::value::DynamicOwned;
use crate::value::Local;
use crate::value::Mode;
use crate::value::ThreadSafe;

/// A mode-specific safe adapter generated inside the declaring struct module.
pub type StructUpdateAdapter<M> =
    fn(ValidatedUpdateInput<M>) -> DynamicOwned<M>;

/// A descriptor-bound atomic updater for one concrete struct root.
pub struct StructUpdater<M: Mode + 'static> {
    descriptor: &'static TypeDescriptor,
    fields: &'static [UpdateField],
    adapter: StructUpdateAdapter<M>,
}

impl<M: Mode + 'static> StructUpdater<M> {
    /// Creates an updater from generated immutable descriptor data.
    ///
    /// `fields` must correspond to every direct struct field in source order,
    /// and `adapter` must return the descriptor's exact root type.
    #[doc(hidden)]
    pub const fn new(
        descriptor: &'static TypeDescriptor,
        fields: &'static [UpdateField],
        adapter: StructUpdateAdapter<M>,
    ) -> Self {
        Self {
            descriptor,
            fields,
            adapter,
        }
    }

    /// Returns the concrete struct root descriptor.
    #[must_use]
    #[inline(always)]
    pub const fn descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor
    }

    /// Returns update policies in source declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn fields(&self) -> &'static [UpdateField] {
        self.fields
    }

    /// Validates the base and all overrides before invoking generated code.
    fn update_with(
        &self,
        input: StructUpdateInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        let validated = crate::construct::validated::validate_update(
            input,
            self.descriptor.type_id(),
            self.fields,
            value_type_id,
        )?;
        let output = (self.adapter)(validated);
        assert_eq!(
            value_type_id(&output),
            self.descriptor.type_id(),
            "a struct update adapter must return its exact declared root type"
        );
        Ok(output)
    }

    /// Enforces generated descriptor/field alignment before accepting input.
    fn assert_descriptor_contract(&self) {
        assert!(
            self.descriptor.as_struct().is_some(),
            "a StructUpdater requires a struct descriptor"
        );
        assert_eq!(
            self.descriptor.fields().len(),
            self.fields.len(),
            "update policy must cover every direct struct field"
        );
        for (descriptor_field, construction_field) in
            self.descriptor.fields().iter().zip(self.fields)
        {
            assert!(
                std::ptr::eq(descriptor_field, construction_field.descriptor()),
                "update policy fields must be the descriptor's own fields"
            );
        }
    }
}

impl StructUpdater<Local> {
    /// Atomically validates and updates a local owned struct value.
    pub fn update(
        &self,
        input: StructUpdateInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.update_with(input, local_type_id)
    }
}

impl StructUpdater<ThreadSafe> {
    /// Atomically validates and updates a thread-safe owned struct value.
    pub fn update(
        &self,
        input: StructUpdateInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>>
    {
        self.update_with(input, thread_safe_type_id)
    }
}

impl<M: Mode + 'static> fmt::Debug for StructUpdater<M> {
    /// Formats descriptor and policy facts without exposing adapter addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StructUpdater")
            .field("descriptor", &self.descriptor.type_name())
            .field("fields", &self.fields)
            .finish()
    }
}
