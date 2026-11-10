// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Static namespace identity used before it is bound to a reflection context.
#[derive(Clone, Copy)]
pub enum MethodNamespaceMetadata {
    /// An inherent implementation namespace.
    Inherent {
        /// Stable source description of the providing inherent impl.
        implementation: &'static str,
    },
    /// A trait namespace identified by source paths.
    Trait {
        /// Source path of the trait providing this method.
        trait_: &'static str,
        /// Source description of the providing impl.
        implementation: &'static str,
    },
    /// A trait namespace with the exact generic arguments of its application.
    TraitApplication {
        /// Source path of the trait providing this method.
        trait_: &'static str,
        /// Source description of the providing impl.
        implementation: &'static str,
        /// Exact generic arguments identifying the trait application.
        identity: &'static [crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity],
        /// Reflected trait declaration when one is known.
        definition: Option<&'static crate::runtime::descriptor::trait_descriptor::TraitDefinitionMetadata>,
    },
}

static EMPTY_GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata {
    parameters: &[],
    predicates: &[],
};
static NO_METHOD_ADAPTER: [InvocationUnavailableReason; 1] = [InvocationUnavailableReason::AdapterUnavailable];
static UNIT_OUTPUT: ReturnMetadata = ReturnMetadata {
    kind: ReturnKind::Unit,
    expression: None,
    concrete_type: None,
};

impl MethodMetadata {
    /// Creates method metadata from a complete typed specification.
    ///
    /// # Parameters
    ///
    /// - `spec`: Static specialization selector being registered.
    pub const fn from_spec(spec: MethodMetadataSpec) -> Self {
        Self {
            name: spec.name,
            namespace: spec.namespace,
            visibility: spec.visibility,
            receiver: spec.receiver,
            parameters: spec.parameters,
            output: spec.output,
            qualifiers: spec.qualifiers,
            generics: spec.generics,
            is_generic: spec.is_generic,
            default_body: spec.default_body,
        }
    }

    /// Creates a zero-argument inherent method declaration with unit output.
    ///
    /// # Parameters
    ///
    /// - `name`: Reflected field name to match against the target.
    /// - `implementation`: Implementation block or function that supplies the
    ///   method.
    pub const fn inherent(name: &'static str, implementation: &'static str) -> Self {
        Self {
            name,
            namespace: MethodNamespaceMetadata::Inherent { implementation },
            visibility: MethodVisibility::Public,
            receiver: None,
            parameters: &[],
            output: &UNIT_OUTPUT,
            qualifiers: MethodQualifiers {
                is_const: false,
                is_async: false,
                is_unsafe: false,
                abi: None,
                is_variadic: false,
            },
            generics: &EMPTY_GENERICS,
            is_generic: false,
            default_body: false,
        }
    }
}
