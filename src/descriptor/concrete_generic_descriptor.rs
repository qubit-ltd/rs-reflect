// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Concrete generic-instance navigation shared by root descriptors.

use crate::__private::LazyTypeRef;
use crate::descriptor::TypeDescriptor;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::value::ReflectedOwned;

/// Creates one fresh local owned value for a concrete const argument.
///
/// The factory is called only when the corresponding runtime const value is
/// explicitly requested.
type ConstArgumentValueFactory = fn() -> ReflectedOwned;

/// A concrete instance's link to its declaration-level generic definition and
/// arguments.
///
/// # Examples
///
/// ```
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// struct Envelope<T>(T);
///
/// let generic = TypeDescriptor::of::<Envelope<String>>()
///     .concrete_generic()
///     .expect("generic instance metadata");
/// assert_eq!(generic.arguments().len(), 1);
/// assert!(generic.type_argument(0).is_some());
/// ```
#[derive(Debug)]
pub struct ConcreteGenericDescriptor {
    /// Declaration-level generic definition shared by all instances.
    definition: &'static GenericDefinitionDescriptor,
    /// Concrete type and const expressions in runtime argument order.
    arguments: &'static [GenericArgument],
    /// Runtime-to-declaration parameter index mapping.
    definition_indices: &'static [usize],
    /// Lazy resolvers for concrete type arguments, indexed like `arguments`.
    type_arguments: &'static [Option<&'static LazyTypeRef>],
    /// Factories for concrete const values, indexed like `arguments`.
    const_argument_values: &'static [Option<ConstArgumentValueFactory>],
}

impl ConcreteGenericDescriptor {
    /// Creates an immutable concrete generic view without runtime argument
    /// navigation.
    ///
    /// This compatibility constructor is retained for generated code that
    /// only supplies structural arguments.
    ///
    /// # Parameters
    ///
    /// - `definition`: The source-level generic declaration.
    /// - `arguments`: The structural arguments supplied by this instance.
    ///
    /// # Returns
    ///
    /// Returns a descriptor without runtime argument resolvers.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new(definition: &'static GenericDefinitionDescriptor, arguments: &'static [GenericArgument]) -> Self {
        Self {
            definition,
            arguments,
            definition_indices: &[],
            type_arguments: &[],
            const_argument_values: &[],
        }
    }

    /// Creates an immutable concrete generic view with lazy type navigation
    /// and owned const-value factories.
    ///
    /// All runtime slices use the same indices as `arguments`.
    /// `definition_indices` maps those positions back to declaration
    /// parameters; a missing resolver or factory means the corresponding
    /// structural argument cannot enter that runtime boundary.
    ///
    /// # Parameters
    ///
    /// - `definition`: The source-level generic declaration.
    /// - `arguments`: Structural arguments in runtime argument order.
    /// - `definition_indices`: Mapping to source parameter indices.
    /// - `type_arguments`: Optional lazy concrete type resolvers.
    /// - `const_argument_values`: Optional owned const-value factories.
    ///
    /// # Returns
    ///
    /// Returns a descriptor linking structural and available runtime arguments.
    #[doc(hidden)]
    #[must_use]
    #[inline]
    pub const fn new_with_runtime_arguments(
        definition: &'static GenericDefinitionDescriptor,
        arguments: &'static [GenericArgument],
        definition_indices: &'static [usize],
        type_arguments: &'static [Option<&'static LazyTypeRef>],
        const_argument_values: &'static [Option<ConstArgumentValueFactory>],
    ) -> Self {
        Self {
            definition,
            arguments,
            definition_indices,
            type_arguments,
            const_argument_values,
        }
    }

    /// Returns the declaration shared by every concrete instance.
    ///
    /// # Returns
    ///
    /// Returns the static source-level generic definition.
    #[must_use]
    #[inline]
    pub const fn definition(&self) -> &'static GenericDefinitionDescriptor {
        self.definition
    }

    /// Returns runtime type and const argument records in declaration order.
    ///
    /// Lifetime parameters are definition-only and are omitted from this
    /// slice. A type argument preserves its declaration-level structural
    /// expression; use [`Self::type_argument`] for its exact concrete root.
    /// Use [`Self::definition_index`] or [`Self::argument_index`] when
    /// declaration-level parameter positions are required.
    ///
    /// # Returns
    ///
    /// Returns runtime type and const arguments in declaration order.
    #[must_use]
    #[inline]
    pub const fn arguments(&self) -> &'static [GenericArgument] {
        self.arguments
    }

    /// Returns the definition parameter index represented by one runtime
    /// argument index.
    ///
    /// `None` means `argument_index` is out of range or index metadata is not
    /// available for a compatibility-constructed descriptor.
    ///
    /// # Parameters
    ///
    /// - `argument_index`: The runtime argument position to map.
    ///
    /// # Returns
    ///
    /// Returns its declaration parameter index, or `None` when no mapping is
    /// available.
    #[must_use]
    #[inline]
    pub const fn definition_index(&self, argument_index: usize) -> Option<usize> {
        if argument_index >= self.definition_indices.len() {
            return None;
        }
        Some(self.definition_indices[argument_index])
    }

    /// Returns the runtime argument index for a definition parameter index.
    ///
    /// `None` means the parameter is a lifetime, is absent, or index metadata
    /// is unavailable for a compatibility-constructed descriptor.
    ///
    /// # Parameters
    ///
    /// - `definition_index`: The source parameter position to find.
    ///
    /// # Returns
    ///
    /// Returns its runtime argument position, or `None` when it has none.
    #[must_use]
    pub fn argument_index(&self, definition_index: usize) -> Option<usize> {
        self.definition_indices
            .iter()
            .position(|candidate| *candidate == definition_index)
    }

    /// Returns the structural runtime argument for a definition parameter.
    ///
    /// `None` means the definition parameter is a lifetime or is absent.
    ///
    /// # Parameters
    ///
    /// - `definition_index`: The source parameter position to retrieve.
    ///
    /// # Returns
    ///
    /// Returns its structural argument, or `None` for lifetimes and absent
    /// parameters.
    #[must_use]
    #[inline]
    pub fn argument_for_definition(&self, definition_index: usize) -> Option<&GenericArgument> {
        self.argument_index(definition_index)
            .and_then(|argument_index| self.arguments.get(argument_index))
    }

    /// Returns the exact root descriptor for the concrete type argument at
    /// `index`.
    ///
    /// Resolution is deferred until this method is called, then cached for
    /// the process lifetime. `None` means the index is absent, does not name a
    /// type argument, or the concrete type is intentionally unavailable at
    /// the reflected root boundary.
    ///
    /// Nested arguments are inferred only through syntactically unambiguous
    /// standard container paths and builtin type forms. A container alias or
    /// an unqualified/custom container name requires an explicit runtime
    /// `Reflect` bound on the type parameter before this method exposes its
    /// concrete root.
    ///
    /// # Parameters
    ///
    /// - `index`: The runtime argument position to resolve.
    ///
    /// # Returns
    ///
    /// Returns the cached concrete root descriptor for a resolvable type
    /// argument, or `None` otherwise.
    #[must_use]
    pub fn type_argument(&self, index: usize) -> Option<&'static TypeDescriptor> {
        if !matches!(self.arguments.get(index), Some(GenericArgument::Type(_))) {
            return None;
        }
        self.type_arguments
            .get(index)
            .copied()
            .flatten()
            .and_then(|argument| argument.get().as_resolved())
    }

    /// Returns the exact root descriptor for a definition-level type
    /// parameter index.
    ///
    /// `None` has the same meaning as [`Self::type_argument`] and also covers
    /// lifetime and const definition parameters.
    ///
    /// # Parameters
    ///
    /// - `definition_index`: The source type parameter position to resolve.
    ///
    /// # Returns
    ///
    /// Returns the concrete root descriptor, or `None` when unavailable.
    #[must_use]
    #[inline]
    pub fn type_argument_for_definition(&self, definition_index: usize) -> Option<&'static TypeDescriptor> {
        self.argument_index(definition_index)
            .and_then(|argument_index| self.type_argument(argument_index))
    }

    /// Creates a reflected-owned copy of the concrete const argument at
    /// `index`.
    ///
    /// `None` means the index is absent, does not name a const argument, or
    /// its declaration type cannot be represented safely by the local owned
    /// dynamic-value boundary. Each successful call allocates a fresh wrapper
    /// containing the const value with its exact Rust declaration type.
    ///
    /// # Parameters
    ///
    /// - `index`: The runtime const argument position to materialize.
    ///
    /// # Returns
    ///
    /// Returns a fresh exact-type owned value, or `None` when unavailable.
    #[must_use]
    pub fn const_argument_value(&self, index: usize) -> Option<ReflectedOwned> {
        if !matches!(self.arguments.get(index), Some(GenericArgument::Const(_))) {
            return None;
        }
        self.const_argument_values
            .get(index)
            .copied()
            .flatten()
            .map(|factory| factory())
    }

    /// Creates a reflected-owned copy for a definition-level const parameter
    /// index.
    ///
    /// `None` has the same meaning as [`Self::const_argument_value`] and also
    /// covers lifetime and type definition parameters.
    ///
    /// # Parameters
    ///
    /// - `definition_index`: The source const parameter position to
    ///   materialize.
    ///
    /// # Returns
    ///
    /// Returns a fresh owned wrapper, or `None` when no safe factory exists.
    #[must_use]
    #[inline]
    pub fn const_argument_value_for_definition(&self, definition_index: usize) -> Option<ReflectedOwned> {
        self.argument_index(definition_index)
            .and_then(|argument_index| self.const_argument_value(argument_index))
    }
}
