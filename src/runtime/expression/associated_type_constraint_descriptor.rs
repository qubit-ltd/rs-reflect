// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::associated_type_constraint_kind::AssociatedTypeConstraintKind;

/// One associated-type bound retained from a trait expression.
#[derive(Clone, Copy)]
pub struct AssociatedTypeConstraintDescriptor<'ctx> {
    /// Associated type identifier constrained by this entry.
    pub(super) name: &'static str,
    /// Equality assignment or bound predicates retained from the source.
    pub(super) kind: AssociatedTypeConstraintKind<'ctx>,
}

impl<'ctx> AssociatedTypeConstraintDescriptor<'ctx> {
    /// Returns the associated-type name.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns the source form and value or predicates for this constraint.
    pub fn kind(self) -> AssociatedTypeConstraintKind<'ctx> {
        self.kind
    }
}
