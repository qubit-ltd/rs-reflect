// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::data::GenericExpressionArgument;
use super::generic_expression_arguments::GenericExpressionArguments;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;

/// A named path that may resolve to a known generic definition.
#[derive(Clone, Copy)]
pub struct NamedTypeExpression<'ctx> {
    /// Complete source path, including qualification.
    pub(super) path: &'static str,
    /// Definition metadata when code generation established an explicit link.
    pub(super) definition: Option<&'static crate::runtime::descriptor::generic::TypeDefinitionMetadata>,
    /// Generic arguments in source order.
    pub(super) arguments: &'static [GenericExpressionArgument],
    /// Context used for nested generic-argument views.
    pub(super) context: &'ctx ReflectionContext,
}

impl<'ctx> NamedTypeExpression<'ctx> {
    /// Returns the context used by the optional definition descriptor.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns a definition only when metadata explicitly links one.
    pub fn find_definition(self) -> Option<TypeDefinitionDescriptor<'ctx>> {
        self.definition
            .map(|metadata| TypeDefinitionDescriptor::new(metadata, self.context))
    }

    /// Returns the complete source path, whether reflection knows it or not.
    pub const fn path(self) -> &'static str {
        self.path
    }

    /// Returns generic arguments in the original source order.
    pub fn arguments(self) -> GenericExpressionArguments<'ctx> {
        GenericExpressionArguments::new(self.arguments, self.context)
    }
}
