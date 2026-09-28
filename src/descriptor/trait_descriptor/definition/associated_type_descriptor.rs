// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Declaration facts for an associated type.

use std::sync::LazyLock;

use crate::descriptor::trait_descriptor::TraitApplicationSubstitutions;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;

/// One associated type declaration.
///
/// # Examples
///
/// ```
/// use qubit_reflect::descriptor::AssociatedTypeDescriptor;
///
/// let item = AssociatedTypeDescriptor::new(0, "Item", "item", Box::new([]), None);
/// assert_eq!(item.query_name(), "item");
/// ```
#[derive(Clone, Debug)]
pub struct AssociatedTypeDescriptor {
    /// Zero-based source position among associated types.
    index: usize,
    /// Rust declaration identifier.
    rust_name: &'static str,
    /// Reflection lookup name.
    query_name: &'static str,
    /// Declared bounds in source order.
    bounds: Box<[PredicateDescriptor]>,
    /// Optional symbolic default type.
    default: Option<TypeExpression>,
    /// Optional GAT parameters and predicates.
    generic_definition: Option<Box<GenericDefinitionDescriptor>>,
}

impl AssociatedTypeDescriptor {
    /// Creates associated type facts in declaration order.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `bounds`: Declared bounds in source order.
    /// - `default`: Optional symbolic default type.
    ///
    /// # Returns
    ///
    /// Returns associated-type facts without GAT parameters.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
            generic_definition: None,
        }
    }

    /// Creates associated type facts with GAT parameters and predicates.
    ///
    /// # Parameters
    ///
    /// - `index`: Zero-based source declaration position.
    /// - `rust_name`: Source Rust identifier.
    /// - `query_name`: Reflection lookup name.
    /// - `bounds`: Declared bounds in source order.
    /// - `default`: Optional symbolic default type.
    /// - `generic_definition`: GAT parameters and where predicates.
    ///
    /// # Returns
    ///
    /// Returns associated-type facts with the supplied GAT definition.
    #[doc(hidden)]
    #[must_use]
    pub fn new_with_generic_definition(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
        generic_definition: GenericDefinitionDescriptor,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
            generic_definition: Some(Box::new(generic_definition)),
        }
    }

    /// Returns the source declaration index.
    ///
    /// # Returns
    ///
    /// Returns the zero-based position among associated types.
    #[must_use]
    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    ///
    /// # Returns
    ///
    /// Returns the source identifier.
    #[must_use]
    #[inline]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    ///
    /// # Returns
    ///
    /// Returns the reflection query name.
    #[must_use]
    #[inline]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns declared bounds in source order.
    ///
    /// # Returns
    ///
    /// Returns the declared predicate bounds.
    #[must_use]
    #[inline]
    pub const fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }

    /// Returns GAT parameters and where predicates in declaration order.
    ///
    /// # Returns
    ///
    /// Returns the GAT definition, or an empty definition for an ordinary
    /// associated type.
    #[must_use]
    pub fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        static EMPTY: LazyLock<GenericDefinitionDescriptor> = LazyLock::new(|| GenericDefinitionDescriptor {
            parameters: Box::new([]),
            predicates: Box::new([]),
            diagnostic: crate::expression::DiagnosticText::default(),
        });
        self.generic_definition.as_deref().unwrap_or(&EMPTY)
    }

    /// Returns the symbolic default type.
    ///
    /// `None` means the trait requires implementations to provide the binding.
    ///
    /// # Returns
    ///
    /// Returns the symbolic default type, or `None` when no default exists.
    #[must_use]
    #[inline]
    pub const fn default(&self) -> Option<&TypeExpression> {
        self.default.as_ref()
    }

    /// Applies one concrete trait application to this declaration.
    ///
    /// # Parameters
    ///
    /// - `substitutions`: Concrete generic and associated-type substitutions.
    ///
    /// # Returns
    ///
    /// Returns this declaration with its type expressions substituted.
    pub(in crate::descriptor::trait_descriptor) fn substituted(
        self,
        substitutions: &TraitApplicationSubstitutions,
    ) -> Self {
        Self {
            bounds: self
                .bounds
                .iter()
                .map(|predicate| substitutions.predicate(predicate))
                .collect(),
            default: self
                .default
                .as_ref()
                .map(|expression| substitutions.type_expression(expression)),
            generic_definition: self
                .generic_definition
                .as_ref()
                .map(|definition| Box::new(substitutions.generic_definition(definition))),
            ..self
        }
    }
}
