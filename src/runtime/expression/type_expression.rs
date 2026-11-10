// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

use super::AssociatedTypeExpression;
use super::FunctionPointerExpression;
use super::NamedTypeExpression;
use super::OpaqueTypeExpression;
use super::TraitExpression;
use super::TraitObjectExpression;
use super::TypeExpressionKind;
use super::data::ExpressionData;
use super::internal::expression_display::ExpressionDisplay;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::GenericParameterDescriptor;

/// A read-only, context-bound view of source-level Rust type syntax.
#[derive(Clone, Copy)]
pub struct TypeExpression<'ctx> {
    /// Static structure emitted from the source type syntax.
    pub(in crate::runtime::expression) data: &'static ExpressionData,
    /// Context used to resolve links while preserving source-level structure.
    pub(in crate::runtime::expression) context: &'ctx ReflectionContext,
}

impl<'ctx> TypeExpression<'ctx> {
    /// Binds static expression metadata to the supplied context.
    ///
    /// # Parameters
    ///
    /// - `data`: Static expression node retained by generated metadata.
    /// - `context`: Runtime context used by nested descriptor views.
    pub(crate) const fn new(data: &'static ExpressionData, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested descriptors.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the structural kind without resolving generic parameters.
    pub fn kind(self) -> TypeExpressionKind<'ctx> {
        match self.data {
            ExpressionData::Named {
                path,
                definition,
                arguments,
            } => TypeExpressionKind::Named(NamedTypeExpression {
                path,
                definition: *definition,
                arguments,
                context: self.context,
            }),
            ExpressionData::Parameter(parameter) => {
                TypeExpressionKind::Parameter(GenericParameterDescriptor::new(parameter, self.context))
            }
            ExpressionData::ParameterName(name) => TypeExpressionKind::ParameterName(name),
            ExpressionData::SelfType => TypeExpressionKind::SelfType,
            ExpressionData::AssociatedType {
                self_type,
                trait_,
                name,
                arguments,
            } => TypeExpressionKind::AssociatedType(AssociatedTypeExpression {
                self_type: Self::new(self_type, self.context),
                trait_: trait_.map(|data| TraitExpression {
                    data,
                    context: self.context,
                }),
                name,
                arguments,
            }),
            ExpressionData::Reference {
                lifetime,
                mutable,
                target,
            } => TypeExpressionKind::Reference {
                lifetime: *lifetime,
                mutable: *mutable,
                target: Self::new(target, self.context),
            },
            ExpressionData::RawPointer { mutable, target } => TypeExpressionKind::RawPointer {
                mutable: *mutable,
                target: Self::new(target, self.context),
            },
            ExpressionData::Slice(element) => TypeExpressionKind::Slice(Self::new(element, self.context)),
            ExpressionData::Array { element, length } => TypeExpressionKind::Array {
                element: Self::new(element, self.context),
                length: *length,
            },
            ExpressionData::Tuple(elements) => TypeExpressionKind::Tuple(
                elements
                    .iter()
                    .map(|element| Self::new(element, self.context))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
            ExpressionData::FunctionPointer(data) => TypeExpressionKind::FunctionPointer(FunctionPointerExpression {
                data,
                context: self.context,
            }),
            ExpressionData::TraitObject(data) => TypeExpressionKind::TraitObject(TraitObjectExpression {
                data,
                context: self.context,
            }),
            ExpressionData::Opaque(data) => TypeExpressionKind::Opaque(OpaqueTypeExpression {
                data,
                context: self.context,
            }),
            ExpressionData::Never => TypeExpressionKind::Never,
        }
    }

    /// Returns tuple elements as a context-bound borrowed view.
    ///
    /// Unlike extracting [`TypeExpressionKind::Tuple`], this does not allocate
    /// an owned element collection. Empty tuples return `Some` with length 0.
    pub fn tuple_elements(self) -> Option<super::TupleTypeExpressions<'ctx>> {
        match self.data {
            ExpressionData::Tuple(elements) => Some(super::TupleTypeExpressions {
                elements,
                context: self.context,
            }),
            _ => None,
        }
    }

    /// Formats source facts for logs and diagnostics only.
    ///
    /// The formatted text is not a type identity; use
    /// [`structurally_eq`](Self::structurally_eq) when comparing retained
    /// source structure.
    pub fn display(self) -> impl fmt::Display + 'ctx {
        ExpressionDisplay { expression: self }
    }

    /// Compares the retained source structure of two type expressions.
    ///
    /// This compares source facts only. It does not compare contexts, resolve
    /// aliases, evaluate constants, or establish Rust type identity. Parameter
    /// references compare by name, and binder names and bound order are
    /// significant. Use this for consistency checks within the same declaration
    /// environment, not for registration identity or access control.
    ///
    /// # Parameters
    ///
    /// - `self`: The first type expression.
    /// - `other`: The second type expression.
    ///
    /// # Returns
    ///
    /// Returns `true` when the retained source structures match.
    pub fn structurally_eq(self, other: TypeExpression<'_>) -> bool {
        super::structural_eq::expression_eq(self.data, other.data)
    }
}
