// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Static source metadata for a target's effective method catalog.
pub struct MethodCatalogRegistration {
    /// Returns the target type identity used to match catalog registrations.
    type_id: fn() -> TypeId,
    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    methods: &'static [EffectiveMethod],
    /// Calls the registration provider to obtain method metadata for the
    /// target.
    provider: Option<fn() -> &'static [EffectiveMethod]>,
}

impl MethodCatalogRegistration {
    /// Creates a catalog registration for a reflected target type.
    ///
    /// # Parameters
    ///
    /// - `methods`: Method records registered for the target type.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `methods`: Method records registered for the target type.
    pub const fn new<T: crate::runtime::descriptor::Reflect>(methods: &'static [EffectiveMethod]) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            methods,
            provider: None,
        }
    }

    /// Creates a registration whose method slice is initialized by a typed
    /// provider.
    ///
    /// # Parameters
    ///
    /// - `provider`: Function that supplies the target type’s static method
    ///   records.
    ///
    ///
    /// # Type Parameters
    ///
    /// - `T`: Concrete Rust type whose metadata or value is used by the
    ///   operation.
    ///
    ///
    /// # Parameters
    ///
    /// - `provider`: Function that supplies the target type’s static method
    ///   records.
    pub const fn with_provider<T: crate::runtime::descriptor::Reflect>(
        provider: fn() -> &'static [EffectiveMethod],
    ) -> Self {
        Self {
            type_id: TypeId::of::<T>,
            methods: &[],
            provider: Some(provider),
        }
    }

    /// Returns the target type identity used to match catalog registrations.
    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    pub(crate) fn methods(&self) -> &'static [EffectiveMethod] {
        match self.provider {
            Some(provider) => provider(),
            None => self.methods,
        }
    }
}

inventory::collect!(MethodCatalogRegistration);

impl<'ctx> MethodCatalog<'ctx> {
    /// Creates a catalog borrowing the static method records for `target`.
    ///
    /// # Parameters
    ///
    /// - `target`: Type descriptor whose effective methods this catalog
    ///   exposes.
    /// - `methods`: Static method records registered for the target type.
    ///
    /// # Returns
    ///
    /// Returns a catalog that borrows `methods` for the context lifetime.
    #[allow(dead_code)] // Used by crate-level tests for hand-written method metadata.
    pub(crate) fn new(target: TypeDescriptor<'ctx>, methods: &'ctx [EffectiveMethod]) -> Self {
        Self {
            target,
            methods: MethodCatalogMethods::Borrowed(methods),
        }
    }

    /// Creates a catalog sharing context-resolved method records through an
    /// `Arc`.
    ///
    /// # Parameters
    ///
    /// - `target`: Type descriptor whose effective methods this catalog
    ///   exposes.
    /// - `methods`: Context-bound method records resolved for this reflection
    ///   context.
    ///
    /// # Returns
    ///
    /// Returns a catalog sharing ownership of `methods`.
    pub(crate) fn from_shared(target: TypeDescriptor<'ctx>, methods: std::sync::Arc<[ContextEffectiveMethod]>) -> Self {
        Self {
            target,
            methods: MethodCatalogMethods::Shared(methods),
        }
    }

    /// Returns the static method slice, resolving it through the provider when
    /// configured.
    fn methods(&self) -> MethodCatalogMethodIter<'_> {
        match &self.methods {
            MethodCatalogMethods::Borrowed(methods) => MethodCatalogMethodIter::Borrowed(methods.iter()),
            MethodCatalogMethods::Shared(methods) => MethodCatalogMethodIter::Shared(methods.iter()),
        }
    }
    /// Looks up methods by source name in the catalog's effective method set.
    ///
    /// # Parameters
    ///
    /// - `name`: Method name to match against the target type's methods.
    ///
    /// # Returns
    ///
    /// Returns the lookup result, preserving missing and ambiguous cases.
    pub(crate) fn lookup_method(&self, name: &str) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(self.lookup(|method| method.metadata.name == name))
    }
    /// Looks up a method by name and requires exactly one effective match.
    ///
    /// # Parameters
    ///
    /// - `name`: Method name that must identify a unique catalog entry.
    ///
    /// # Returns
    ///
    /// Returns the unique descriptor or an error for missing or ambiguous
    /// names.
    #[allow(dead_code)] // Used by crate-level tests for hand-written method metadata.
    pub(crate) fn method(&self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.lookup_method(name)
            .map_err(LookupError::Context)?
            .into_unique(LookupSubject::Method, LookupQuery::Name(name.into()))
    }
    #[cfg(test)]
    pub(crate) fn find_inherent_method(&self, name: &str) -> Result<Option<MethodDescriptor<'ctx>>, ContextError> {
        match self.lookup(|method| {
            method.metadata.name == name
                && matches!(method.metadata.namespace, MethodNamespaceMetadata::Inherent { .. })
        }) {
            Lookup::Missing => Ok(None),
            Lookup::Unique(method) => Ok(Some(method)),
            Lookup::Ambiguous(_) => {
                unreachable!("inherent method names are unique within one namespace")
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn inherent_method(&self, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.find_inherent_method(name)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name.into()),
            })
    }
    #[cfg(test)]
    pub(crate) fn lookup_trait_method(
        &self,
        trait_: &str,
        name: &str,
    ) -> Result<Lookup<MethodDescriptor<'ctx>>, ContextError> {
        Ok(self.lookup(|method| method.metadata.name == name && matches!(method.metadata.namespace, MethodNamespaceMetadata::Trait { trait_: candidate, .. } if candidate == trait_)))
    }
    #[cfg(test)]
    pub(crate) fn find_trait_method(
        &self,
        trait_: &str,
        name: &str,
    ) -> Result<Option<MethodDescriptor<'ctx>>, ContextError> {
        match self.lookup_trait_method(trait_, name)? {
            Lookup::Missing => Ok(None),
            Lookup::Unique(method) => Ok(Some(method)),
            Lookup::Ambiguous(_) => {
                unreachable!("method names are unique within one trait namespace")
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn trait_method(&self, trait_: &str, name: &str) -> Result<MethodDescriptor<'ctx>, LookupError> {
        self.find_trait_method(trait_, name)
            .map_err(LookupError::Context)?
            .ok_or_else(|| LookupError::Missing {
                subject: LookupSubject::Method,
                query: LookupQuery::Name(name.into()),
            })
    }
    /// Provides the lookup operation used to resolve or dispatch reflected
    /// methods. # Parameters
    ///
    /// - `predicate`: Condition used to select matching effective method
    ///   entries.
    /// # Parameters
    ///
    /// - `predicate`: Condition used to filter the catalog’s effective methods.
    fn lookup(&self, predicate: impl Fn(&EffectiveMethod) -> bool) -> Lookup<MethodDescriptor<'ctx>> {
        let mut matches = self
            .methods()
            .filter(|method| {
                (method.effective().target)().type_id() == self.target.type_id() && predicate(method.effective())
            })
            .map(|method| match method {
                MethodCatalogMethod::Static(method) => MethodDescriptor::new(*method, self.target.context()),
                MethodCatalogMethod::Context(method) => MethodDescriptor::with_resolved_specializations(
                    method.effective,
                    method.specializations.clone(),
                    method.implementation,
                    self.target.context(),
                ),
            });
        let Some(first) = matches.next() else {
            return Lookup::Missing;
        };
        match matches.next() {
            None => Lookup::Unique(first),
            Some(second) => Lookup::Ambiguous(
                std::iter::once(first)
                    .chain(std::iter::once(second))
                    .chain(matches)
                    .collect(),
            ),
        }
    }
}
