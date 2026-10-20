// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// =============================================================================

//! Typed implementation declaration and application metadata for generated
//! code.

use std::marker::PhantomData;

use crate::runtime::descriptor::generic::GenericArgumentMetadata;
use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
use crate::runtime::descriptor::generic::GenericParameterMetadata;
use crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata;
pub use crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata as AssociatedConstBinding;
pub use crate::runtime::descriptor::impl_descriptor::AssociatedItemSource as ItemSource;
use crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata;
pub use crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata as AssociatedTypeBinding;
use crate::runtime::descriptor::impl_descriptor::ImplApplicationMetadata;
use crate::runtime::descriptor::impl_descriptor::ImplDefinitionMetadata;
use crate::runtime::descriptor::impl_descriptor::ImplKind;
pub use crate::runtime::descriptor::impl_descriptor::ImplKind as Kind;
use crate::runtime::descriptor::impl_descriptor::ImplQualifiers;
pub use crate::runtime::descriptor::impl_descriptor::ImplQualifiers as Qualifiers;
use crate::runtime::descriptor::method::EffectiveMethod;
use crate::runtime::descriptor::method::MethodMetadata;
use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata;
pub use crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata as AssociatedConst;
use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata;
pub use crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata as AssociatedType;
use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
use crate::runtime::descriptor::trait_descriptor::TraitArgumentIdentity;
use crate::runtime::descriptor::trait_descriptor::TraitExpressionMetadata;
use crate::runtime::expression::ExpressionData;

/// Marker-bound static metadata for one source impl block.
pub struct ImplDefinitionSpec<D: 'static> {
    metadata: ImplDefinitionMetadata,
    _marker: PhantomData<fn() -> D>,
}

impl<D: 'static> ImplDefinitionSpec<D> {
    /// Creates definition metadata whose identity is bound to marker type `D`.
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        kind: ImplKind,
        target: &'static ExpressionData,
        implemented_trait: Option<&'static TraitExpressionMetadata>,
        generics: &'static GenericDefinitionMetadata,
        qualifiers: ImplQualifiers,
        methods: &'static [MethodMetadata],
        associated_types: &'static [AssociatedTypeDefinitionMetadata],
        associated_consts: &'static [AssociatedConstDefinitionMetadata],
    ) -> Self {
        Self {
            metadata: ImplDefinitionMetadata::new::<D>(
                kind,
                target,
                implemented_trait,
                generics,
                qualifiers,
                methods,
                associated_types,
                associated_consts,
            ),
            _marker: PhantomData,
        }
    }

    /// Borrows this static definition for a typed application.
    pub const fn metadata(&'static self) -> &'static ImplDefinitionMetadata {
        &self.metadata
    }

    /// Returns an impl generic parameter by its source-order index.
    pub const fn generic_parameter_at(&'static self, index: usize) -> &'static GenericParameterMetadata {
        if index >= self.metadata.generics.parameters.len() {
            panic!("impl generic parameter index is out of range");
        }
        &self.metadata.generics.parameters[index]
    }
}

/// Static application metadata for one concrete target type.
pub struct ImplApplicationSpec<T: 'static> {
    metadata: ImplApplicationMetadata,
    _target: PhantomData<fn() -> T>,
}

impl<T: crate::runtime::descriptor::Reflect + 'static> ImplApplicationSpec<T> {
    /// Creates a concrete application linked to marker-bound impl metadata.
    #[allow(clippy::too_many_arguments)]
    pub const fn for_target<D: 'static>(
        definition: &'static ImplDefinitionSpec<D>,
        implemented_trait: Option<&'static TraitApplicationMetadata>,
        arguments: &'static [GenericArgumentMetadata],
        identity: &'static [TraitArgumentIdentity],
        methods: &'static [EffectiveMethod],
        associated_types: &'static [AssociatedTypeBindingMetadata],
        associated_consts: &'static [AssociatedConstBindingMetadata],
    ) -> Self {
        Self {
            metadata: ImplApplicationMetadata::for_target::<T>(
                definition.metadata(),
                implemented_trait,
                arguments,
                identity,
                methods,
                associated_types,
                associated_consts,
            ),
            _target: PhantomData,
        }
    }

    /// Borrows this static application for the runtime registration closure.
    pub const fn metadata(&'static self) -> &'static ImplApplicationMetadata {
        &self.metadata
    }
}

/// Creates a typed associated type binding from its trait declaration item.
pub const fn associated_type_binding(
    definition: &'static AssociatedType,
    expression: &'static ExpressionData,
    source: crate::runtime::descriptor::impl_descriptor::AssociatedItemSource,
) -> AssociatedTypeBindingMetadata {
    AssociatedTypeBindingMetadata::new(definition, expression, None, source)
}

/// Creates an associated type binding linked to its exact reflected Rust type.
pub const fn associated_type_binding_for<T: 'static>(
    definition: &'static AssociatedType,
    expression: &'static ExpressionData,
    source: crate::runtime::descriptor::impl_descriptor::AssociatedItemSource,
) -> AssociatedTypeBindingMetadata {
    AssociatedTypeBindingMetadata::new(
        definition,
        expression,
        Some(crate::runtime::descriptor::metadata::TypeMetadata::of::<T>),
        source,
    )
}

/// Merges explicit associated type bindings with trait defaults.
#[doc(hidden)]
pub fn associated_type_bindings_with_defaults<D: 'static>(
    definition: &'static crate::__private::codegen_v4::traits::TraitDefinitionSpec<D>,
    application: &'static TraitApplicationMetadata,
    declared: &[&'static str],
    bindings: &'static [crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata],
) -> &'static [crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata] {
    let metadata = definition.metadata();
    let mut values = Vec::with_capacity(bindings.len() + metadata.associated_types.len());
    values.extend(bindings.iter().copied().map(|mut binding| {
        binding.source = if binding.definition.default.is_some() {
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Overridden
        } else {
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Declared
        };
        binding
    }));
    for associated_type in metadata.associated_types {
        if declared.contains(&associated_type.name) {
            continue;
        }
        if let Some(default) = associated_type.default {
            let concrete = type_argument_concrete(default, application.arguments);
            let default = substitute_type_expression(default, application.arguments);
            values.push(
                crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata::new(
                    associated_type,
                    default,
                    concrete,
                    crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Defaulted,
                ),
            );
        }
    }
    Box::leak(values.into_boxed_slice())
}

fn type_argument_concrete(
    expression: &'static ExpressionData,
    arguments: &[GenericArgumentMetadata],
) -> Option<fn() -> &'static crate::runtime::descriptor::metadata::TypeMetadata> {
    use crate::runtime::descriptor::generic::GenericArgumentMetadataKind;
    use crate::runtime::expression::ExpressionData;

    let name = match expression {
        ExpressionData::Parameter(parameter) => parameter.name,
        ExpressionData::ParameterName(name) => name,
        _ => return None,
    };
    arguments
        .iter()
        .find(|argument| argument.parameter.is_some_and(|parameter| parameter.name == name))
        .and_then(|argument| match argument.kind {
            GenericArgumentMetadataKind::Type { concrete, .. } => concrete,
            GenericArgumentMetadataKind::Const(_) => None,
        })
}

/// Merges explicit associated constant bindings with trait defaults.
#[doc(hidden)]
pub fn associated_const_bindings_with_defaults<D: 'static>(
    definition: &'static crate::__private::codegen_v4::traits::TraitDefinitionSpec<D>,
    application: &'static TraitApplicationMetadata,
    overridden: &[&'static str],
    bindings: &'static [crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata],
) -> &'static [crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata] {
    let mut values = Vec::with_capacity(bindings.len() + definition.metadata().associated_consts.len());
    values.extend(bindings.iter().copied().map(|mut binding| {
        binding.source = if binding.definition.default.is_some() {
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Overridden
        } else {
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Declared
        };
        binding
    }));
    for associated_const in definition.metadata().associated_consts {
        if overridden.contains(&associated_const.name) {
            continue;
        }
        if let Some(default) = associated_const.default {
            let default = substitute_const_expression(default, application.arguments);
            values.push(
                crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata::new(
                    associated_const,
                    default,
                    crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Defaulted,
                ),
            );
        }
    }
    Box::leak(values.into_boxed_slice())
}

fn substitute_const_expression(
    expression: crate::runtime::expression::ConstExpression,
    arguments: &[GenericArgumentMetadata],
) -> crate::runtime::expression::ConstExpression {
    use crate::runtime::descriptor::generic::GenericArgumentMetadataKind;
    use crate::runtime::expression::ConstExpression;

    let ConstExpression::Parameter(name) = expression else {
        return expression;
    };
    arguments
        .iter()
        .find(|argument| argument.parameter.is_some_and(|parameter| parameter.name == name))
        .and_then(|argument| match argument.kind {
            GenericArgumentMetadataKind::Const(value) => Some(value),
            GenericArgumentMetadataKind::Type { .. } => None,
        })
        .unwrap_or(expression)
}

fn substitute_type_expression(
    expression: &'static ExpressionData,
    arguments: &[GenericArgumentMetadata],
) -> &'static ExpressionData {
    use crate::runtime::descriptor::generic::GenericArgumentMetadataKind;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::GenericExpressionArgument;

    let substitute = |value| substitute_type_expression(value, arguments);
    let substitute_arguments = |values: &[GenericExpressionArgument]| {
        Box::leak(
            values
                .iter()
                .map(|argument| match argument {
                    GenericExpressionArgument::Type(value) => GenericExpressionArgument::Type(substitute(value)),
                    GenericExpressionArgument::Lifetime(value) => GenericExpressionArgument::Lifetime(*value),
                    GenericExpressionArgument::Const(value) => {
                        GenericExpressionArgument::Const(substitute_const_expression(*value, arguments))
                    }
                    GenericExpressionArgument::AssociatedType { name, value } => {
                        GenericExpressionArgument::AssociatedType {
                            name,
                            value: substitute(value),
                        }
                    }
                    GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                        GenericExpressionArgument::AssociatedTypeBound {
                            name,
                            bounds: Box::leak(
                                bounds
                                    .iter()
                                    .map(|predicate| substitute_predicate(predicate, arguments))
                                    .collect::<Vec<_>>()
                                    .into_boxed_slice(),
                            ),
                        }
                    }
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    };

    let result = match expression {
        ExpressionData::Parameter(parameter) => {
            return arguments
                .iter()
                .find(|argument| {
                    argument
                        .parameter
                        .is_some_and(|argument_parameter| argument_parameter.name == parameter.name)
                })
                .and_then(|argument| match argument.kind {
                    GenericArgumentMetadataKind::Type { expression, .. } => Some(expression),
                    GenericArgumentMetadataKind::Const(_) => None,
                })
                .unwrap_or(expression);
        }
        ExpressionData::ParameterName(name) => {
            return arguments
                .iter()
                .find(|argument| argument.parameter.is_some_and(|parameter| parameter.name == *name))
                .and_then(|argument| match argument.kind {
                    GenericArgumentMetadataKind::Type { expression, .. } => Some(expression),
                    GenericArgumentMetadataKind::Const(_) => None,
                })
                .unwrap_or(expression);
        }
        ExpressionData::Named {
            path,
            definition,
            arguments: values,
        } => ExpressionData::Named {
            path,
            definition: *definition,
            arguments: substitute_arguments(values),
        },
        ExpressionData::SelfType => ExpressionData::SelfType,
        ExpressionData::AssociatedType {
            self_type,
            trait_,
            name,
            arguments: values,
        } => ExpressionData::AssociatedType {
            self_type: substitute(self_type),
            trait_: trait_.map(|trait_| &*Box::leak(Box::new(substitute_trait_expression(trait_, arguments)))),
            name,
            arguments: substitute_arguments(values),
        },
        ExpressionData::Reference {
            lifetime,
            mutable,
            target,
        } => ExpressionData::Reference {
            lifetime: *lifetime,
            mutable: *mutable,
            target: substitute(target),
        },
        ExpressionData::RawPointer { mutable, target } => ExpressionData::RawPointer {
            mutable: *mutable,
            target: substitute(target),
        },
        ExpressionData::Slice(element) => ExpressionData::Slice(substitute(element)),
        ExpressionData::Array { element, length } => ExpressionData::Array {
            element: substitute(element),
            length: substitute_const_expression(*length, arguments),
        },
        ExpressionData::Tuple(elements) => ExpressionData::Tuple(Box::leak(
            elements
                .iter()
                .map(|element| substitute(element))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )),
        ExpressionData::FunctionPointer(data) => {
            let arguments = Box::leak(
                data.arguments
                    .iter()
                    .map(|value| substitute(value))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            );
            let result = data.result.map(substitute);
            ExpressionData::FunctionPointer(Box::leak(Box::new(
                crate::runtime::expression::FunctionPointerData::new(
                    arguments,
                    result,
                    data.unsafe_,
                    data.abi,
                    data.variadic,
                    data.lifetimes,
                ),
            )))
        }
        ExpressionData::TraitObject(data) => {
            ExpressionData::TraitObject(Box::leak(Box::new(crate::runtime::expression::TraitObjectData::new(
                Box::leak(
                    data.traits
                        .iter()
                        .map(|trait_| substitute_trait_expression(trait_, arguments))
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
                data.lifetime,
            ))))
        }
        ExpressionData::Opaque(data) => ExpressionData::Opaque(Box::leak(Box::new(
            crate::runtime::expression::OpaqueTypeData::new(Box::leak(
                data.bounds
                    .iter()
                    .map(|predicate| substitute_predicate(predicate, arguments))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )),
        ))),
        ExpressionData::Never => ExpressionData::Never,
    };
    Box::leak(Box::new(result))
}

fn substitute_trait_expression(
    expression: &crate::runtime::expression::TraitExpressionData,
    arguments: &[GenericArgumentMetadata],
) -> crate::runtime::expression::TraitExpressionData {
    let generic_arguments = expression
        .arguments
        .iter()
        .map(|argument| match argument {
            crate::runtime::expression::GenericExpressionArgument::Type(value) => {
                crate::runtime::expression::GenericExpressionArgument::Type(substitute_type_expression(
                    value, arguments,
                ))
            }
            crate::runtime::expression::GenericExpressionArgument::Lifetime(value) => {
                crate::runtime::expression::GenericExpressionArgument::Lifetime(*value)
            }
            crate::runtime::expression::GenericExpressionArgument::Const(value) => {
                crate::runtime::expression::GenericExpressionArgument::Const(substitute_const_expression(
                    *value, arguments,
                ))
            }
            crate::runtime::expression::GenericExpressionArgument::AssociatedType { name, value } => {
                crate::runtime::expression::GenericExpressionArgument::AssociatedType {
                    name,
                    value: substitute_type_expression(value, arguments),
                }
            }
            crate::runtime::expression::GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                crate::runtime::expression::GenericExpressionArgument::AssociatedTypeBound {
                    name,
                    bounds: Box::leak(
                        bounds
                            .iter()
                            .map(|predicate| substitute_predicate(predicate, arguments))
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    ),
                }
            }
        })
        .collect::<Vec<_>>();
    crate::runtime::expression::TraitExpressionData::new(
        expression.path,
        Box::leak(generic_arguments.into_boxed_slice()),
        expression.lifetimes,
    )
}

fn substitute_predicate(
    predicate: &crate::runtime::expression::PredicateData,
    arguments: &[GenericArgumentMetadata],
) -> crate::runtime::expression::PredicateData {
    use crate::runtime::expression::PredicateData;

    match predicate {
        PredicateData::TraitBound { ty, trait_, modifier } => PredicateData::TraitBound {
            ty: substitute_type_expression(ty, arguments),
            trait_: Box::leak(Box::new(substitute_trait_expression(trait_, arguments))),
            modifier: *modifier,
        },
        PredicateData::LifetimeOutlives { lifetime, outlives } => PredicateData::LifetimeOutlives {
            lifetime: *lifetime,
            outlives: *outlives,
        },
        PredicateData::TypeOutlives { ty, lifetime } => PredicateData::TypeOutlives {
            ty: substitute_type_expression(ty, arguments),
            lifetime: *lifetime,
        },
        PredicateData::TypeEquality { left, right } => PredicateData::TypeEquality {
            left: substitute_type_expression(left, arguments),
            right: substitute_type_expression(right, arguments),
        },
        PredicateData::ConstEvaluatable(expression) => {
            PredicateData::ConstEvaluatable(substitute_const_expression(*expression, arguments))
        }
    }
}

/// Creates a typed associated constant binding without a dynamic value adapter.
pub const fn associated_const_binding(
    definition: &'static AssociatedConst,
    expression: crate::runtime::expression::ConstExpression,
    source: crate::runtime::descriptor::impl_descriptor::AssociatedItemSource,
) -> AssociatedConstBindingMetadata {
    AssociatedConstBindingMetadata::new(definition, expression, source)
}

/// Builds a registration value from a marker-bound impl definition.
#[doc(hidden)]
pub fn register_impl_definition<D: 'static>(
    definition: &'static ImplDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::metadata_only()
        .with_impl_definition(definition.metadata())
}

/// Builds an impl definition registration from a cached static metadata
/// provider.
#[doc(hidden)]
pub fn register_impl_definition_from<D: 'static>(
    definition: fn() -> &'static ImplDefinitionSpec<D>,
) -> crate::__private::codegen_v4::registration::Registration {
    register_impl_definition(definition())
}

/// Builds a concrete registration value from an impl application.
#[doc(hidden)]
pub fn register_impl_application<T: crate::runtime::descriptor::Reflect + 'static>(
    application: &'static ImplApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    crate::__private::codegen_v4::registration::Registration::metadata_only()
        .with_impl_application(application.metadata())
}

/// Builds an impl application registration from a cached static metadata
/// provider.
#[doc(hidden)]
pub fn register_impl_application_from<T: crate::runtime::descriptor::Reflect + 'static>(
    application: fn() -> &'static ImplApplicationSpec<T>,
) -> crate::__private::codegen_v4::registration::Registration {
    register_impl_application(application())
}

const fn no_dependencies() -> &'static [std::any::TypeId] {
    &[]
}

/// Creates a global definition fragment without exposing inventory payloads.
#[doc(hidden)]
pub const fn impl_definition_fragment(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_global_registration(label, registration)
}

/// Creates an impl application fragment bound to `T`.
#[doc(hidden)]
pub const fn impl_application_fragment<T: crate::runtime::descriptor::Reflect + 'static>(
    label: &'static str,
    registration: fn() -> crate::__private::codegen_v4::registration::Registration,
) -> crate::__private::codegen_v4::registration::RegistrationFragment {
    crate::__private::codegen_v4::registration::RegistrationFragment::for_target::<T>(
        ("generated", None, "generated", Some(label), None, None, label),
        no_dependencies,
        registration,
    )
}

/// Submits one marker-bound impl definition.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_impl_definition {
    ($marker:ty, $definition:path) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::impls::register_impl_definition_from::<$marker>($definition)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::impls::impl_definition_fragment(
                    stringify!($marker),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_impl_definition as submit_impl_definition;

/// Submits an impl application linked to the reflected target `T`.
#[doc(hidden)]
#[macro_export]
macro_rules! __qubit_reflect_submit_impl_application {
    ($target:ty, $application:path) => {
        const _: () = {
            fn registration() -> $crate::__private::codegen_v4::registration::Registration {
                $crate::__private::codegen_v4::impls::register_impl_application_from::<$target>($application)
            }
            $crate::__private::__inventory::submit! {
                $crate::__private::codegen_v4::impls::impl_application_fragment::<$target>(
                    concat!(stringify!($target), "::impl-application"),
                    registration,
                )
            }
        };
    };
}

#[doc(hidden)]
pub use crate::__qubit_reflect_submit_impl_application as submit_impl_application;

#[cfg(test)]
mod tests {
    use super::associated_const_bindings_with_defaults;
    use super::associated_type_bindings_with_defaults;
    use super::substitute_const_expression;
    use super::substitute_predicate;
    use super::substitute_trait_expression;
    use super::substitute_type_expression;
    use crate::runtime::descriptor::generic::GenericArgumentMetadata;
    use crate::runtime::descriptor::generic::GenericDefinitionMetadata;
    use crate::runtime::descriptor::generic::GenericParameterMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitApplicationMetadata;
    use crate::runtime::descriptor::trait_descriptor::TraitCompleteness;
    use crate::runtime::descriptor::trait_descriptor::Visibility;
    use crate::runtime::expression::ConstExpression;
    use crate::runtime::expression::ExpressionData;
    use crate::runtime::expression::GenericExpressionArgument;
    use crate::runtime::expression::LifetimeExpression;
    use crate::runtime::expression::PredicateData;
    use crate::runtime::expression::TraitBoundModifier;
    use crate::runtime::expression::TraitExpressionData;

    static U8: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    static U16: ExpressionData = ExpressionData::Named {
        path: "u16",
        definition: None,
        arguments: &[],
    };
    static U_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::type_parameter("U", 1, &[], None);
    static U_PARAMETER_EXPRESSION: ExpressionData = ExpressionData::Parameter(&U_PARAMETER);
    static USIZE: ExpressionData = ExpressionData::Named {
        path: "usize",
        definition: None,
        arguments: &[],
    };
    static TYPE_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::type_parameter("T", 0, &[], None);
    static CONST_PARAMETER: GenericParameterMetadata = GenericParameterMetadata::const_parameter("N", 2, &USIZE, None);
    static TYPE_PARAMETER_EXPRESSION: ExpressionData = ExpressionData::Parameter(&TYPE_PARAMETER);
    static TYPE_ARGUMENTS: [GenericArgumentMetadata; 3] = [
        GenericArgumentMetadata::type_argument::<u8>(&TYPE_PARAMETER, &U8),
        GenericArgumentMetadata::type_argument::<u16>(&U_PARAMETER, &U16),
        GenericArgumentMetadata::const_argument(&CONST_PARAMETER, ConstExpression::UnsignedInteger(7)),
    ];

    #[test]
    fn type_substitution_rewrites_parameters_recursively_and_preserves_non_parameters() {
        let reference = Box::leak(Box::new(ExpressionData::Reference {
            lifetime: Some(LifetimeExpression::Named("a")),
            mutable: true,
            target: &TYPE_PARAMETER_EXPRESSION,
        }));
        let array = Box::leak(Box::new(ExpressionData::Array {
            element: reference,
            length: ConstExpression::Parameter("N"),
        }));
        let tuple = Box::leak(Box::new(ExpressionData::Tuple(Box::leak(
            vec![array, &ExpressionData::Never].into_boxed_slice(),
        ))));

        let substituted = substitute_type_expression(tuple, &TYPE_ARGUMENTS);
        let ExpressionData::Tuple(items) = substituted else {
            panic!("tuple expression is retained");
        };
        let ExpressionData::Array { element, length } = items[0] else {
            panic!("array element is retained");
        };
        assert_eq!(*length, ConstExpression::UnsignedInteger(7));
        let ExpressionData::Reference {
            lifetime,
            mutable,
            target,
        } = element
        else {
            panic!("reference modifiers are retained");
        };
        assert_eq!(*lifetime, Some(LifetimeExpression::Named("a")));
        assert!(mutable);
        assert!(matches!(target, ExpressionData::Named { path: "u8", .. }));
        assert!(matches!(items[1], ExpressionData::Never));

        assert!(matches!(
            substitute_type_expression(&ExpressionData::ParameterName("Missing"), &TYPE_ARGUMENTS),
            ExpressionData::ParameterName("Missing")
        ));
        assert!(matches!(
            substitute_type_expression(&ExpressionData::ParameterName("T"), &TYPE_ARGUMENTS),
            ExpressionData::Named { path: "u8", .. }
        ));
        assert!(matches!(
            substitute_type_expression(&ExpressionData::SelfType, &TYPE_ARGUMENTS),
            ExpressionData::SelfType
        ));
        assert_eq!(
            substitute_const_expression(ConstExpression::SignedInteger(3), &TYPE_ARGUMENTS),
            ConstExpression::SignedInteger(3)
        );
    }

    #[test]
    fn trait_and_predicate_substitution_rewrites_nested_arguments_and_bounds() {
        static TRAIT_ARGUMENTS: [GenericExpressionArgument; 5] = [
            GenericExpressionArgument::Type(&TYPE_PARAMETER_EXPRESSION),
            GenericExpressionArgument::Lifetime(LifetimeExpression::Placeholder),
            GenericExpressionArgument::Const(ConstExpression::Parameter("N")),
            GenericExpressionArgument::AssociatedType {
                name: "Item",
                value: &TYPE_PARAMETER_EXPRESSION,
            },
            GenericExpressionArgument::AssociatedTypeBound {
                name: "Bounded",
                bounds: &[PredicateData::TypeEquality {
                    left: &TYPE_PARAMETER_EXPRESSION,
                    right: &U16,
                }],
            },
        ];
        static TRAIT: TraitExpressionData = TraitExpressionData::new("Trait", &TRAIT_ARGUMENTS, &[]);

        static NAMED: ExpressionData = ExpressionData::Named {
            path: "Wrapper",
            definition: None,
            arguments: &TRAIT_ARGUMENTS,
        };

        let result = substitute_trait_expression(&TRAIT, &TYPE_ARGUMENTS);
        assert_eq!(result.path, "Trait");
        assert!(matches!(
            result.arguments[0],
            GenericExpressionArgument::Type(ExpressionData::Named { path: "u8", .. })
        ));
        assert!(matches!(
            result.arguments[1],
            GenericExpressionArgument::Lifetime(LifetimeExpression::Placeholder)
        ));
        assert!(matches!(
            result.arguments[2],
            GenericExpressionArgument::Const(ConstExpression::UnsignedInteger(7))
        ));
        assert!(matches!(
            result.arguments[3],
            GenericExpressionArgument::AssociatedType {
                name: "Item",
                value: ExpressionData::Named { path: "u8", .. }
            }
        ));
        let GenericExpressionArgument::AssociatedTypeBound { bounds, .. } = result.arguments[4] else {
            panic!("associated type bounds are retained");
        };
        assert!(matches!(
            bounds[0],
            PredicateData::TypeEquality {
                left: ExpressionData::Named { path: "u8", .. },
                right: ExpressionData::Named { path: "u16", .. }
            }
        ));

        let ExpressionData::Named { path, arguments, .. } = substitute_type_expression(&NAMED, &TYPE_ARGUMENTS) else {
            panic!("named path expression is retained");
        };
        assert_eq!(*path, "Wrapper");
        assert!(matches!(
            arguments[2],
            GenericExpressionArgument::Const(ConstExpression::UnsignedInteger(7))
        ));

        let trait_bound = PredicateData::TraitBound {
            ty: &TYPE_PARAMETER_EXPRESSION,
            trait_: &TRAIT,
            modifier: TraitBoundModifier::Maybe,
        };
        let bound = substitute_predicate(&trait_bound, &TYPE_ARGUMENTS);
        assert!(matches!(
            bound,
            PredicateData::TraitBound {
                ty: ExpressionData::Named { path: "u8", .. },
                modifier: TraitBoundModifier::Maybe,
                ..
            }
        ));
        assert!(matches!(
            substitute_predicate(
                &PredicateData::ConstEvaluatable(ConstExpression::Parameter("N")),
                &TYPE_ARGUMENTS
            ),
            PredicateData::ConstEvaluatable(ConstExpression::UnsignedInteger(7))
        ));
        assert!(matches!(
            substitute_predicate(
                &PredicateData::LifetimeOutlives {
                    lifetime: LifetimeExpression::Named("a"),
                    outlives: LifetimeExpression::Static,
                },
                &TYPE_ARGUMENTS
            ),
            PredicateData::LifetimeOutlives {
                lifetime: LifetimeExpression::Named("a"),
                outlives: LifetimeExpression::Static,
            }
        ));
    }

    #[test]
    fn substitution_covers_pointer_projection_slice_function_trait_object_and_opaque_nodes() {
        static PROJECTION_ARGUMENTS: [GenericExpressionArgument; 1] =
            [GenericExpressionArgument::Type(&U_PARAMETER_EXPRESSION)];
        static PROJECTION_TRAIT_ARGUMENTS: [GenericExpressionArgument; 1] =
            [GenericExpressionArgument::Type(&TYPE_PARAMETER_EXPRESSION)];
        static PROJECTION_TRAIT: TraitExpressionData =
            TraitExpressionData::new("Trait", &PROJECTION_TRAIT_ARGUMENTS, &[]);
        static PROJECTION: ExpressionData = ExpressionData::AssociatedType {
            self_type: &TYPE_PARAMETER_EXPRESSION,
            trait_: Some(&PROJECTION_TRAIT),
            name: "Item",
            arguments: &PROJECTION_ARGUMENTS,
        };
        static RAW: ExpressionData = ExpressionData::RawPointer {
            mutable: false,
            target: &PROJECTION,
        };
        static SLICE: ExpressionData = ExpressionData::Slice(&RAW);
        static FUNCTION_ARGUMENTS: [&ExpressionData; 1] = [&SLICE];
        static FUNCTION: ExpressionData =
            ExpressionData::FunctionPointer(&crate::runtime::expression::FunctionPointerData::new(
                &FUNCTION_ARGUMENTS,
                Some(&TYPE_PARAMETER_EXPRESSION),
                true,
                Some("C"),
                true,
                &[],
            ));
        static TRAIT_ARGUMENTS: [GenericExpressionArgument; 1] =
            [GenericExpressionArgument::Type(&TYPE_PARAMETER_EXPRESSION)];
        static OBJECT_TRAITS: [TraitExpressionData; 1] = [TraitExpressionData::new("Trait", &TRAIT_ARGUMENTS, &[])];
        static OBJECT: ExpressionData = ExpressionData::TraitObject(&crate::runtime::expression::TraitObjectData::new(
            &OBJECT_TRAITS,
            Some(LifetimeExpression::Static),
        ));
        static OPAQUE_PREDICATES: [PredicateData; 1] = [PredicateData::TypeOutlives {
            ty: &TYPE_PARAMETER_EXPRESSION,
            lifetime: LifetimeExpression::Static,
        }];
        static OPAQUE: ExpressionData =
            ExpressionData::Opaque(&crate::runtime::expression::OpaqueTypeData::new(&OPAQUE_PREDICATES));

        let substituted = [&RAW, &SLICE, &FUNCTION, &OBJECT, &OPAQUE]
            .map(|expression| substitute_type_expression(expression, &TYPE_ARGUMENTS));
        assert!(matches!(
            substituted[0],
            ExpressionData::RawPointer {
                mutable: false,
                target: ExpressionData::AssociatedType {
                    self_type: ExpressionData::Named { path: "u8", .. },
                    name: "Item",
                    trait_: Some(TraitExpressionData {
                        arguments: [GenericExpressionArgument::Type(ExpressionData::Named {
                            path: "u8",
                            ..
                        })],
                        ..
                    }),
                    arguments: [GenericExpressionArgument::Type(ExpressionData::Named {
                        path: "u16",
                        ..
                    })],
                    ..
                }
            }
        ));
        assert!(matches!(substituted[1], ExpressionData::Slice(_)));
        assert!(matches!(substituted[2], ExpressionData::FunctionPointer(data)
            if data.unsafe_ && data.abi == Some("C") && data.variadic));
        assert!(matches!(substituted[3], ExpressionData::TraitObject(data)
            if data.lifetime == Some(LifetimeExpression::Static)));
        assert!(matches!(substituted[4], ExpressionData::Opaque(_)));
    }

    #[test]
    fn associated_default_merges_preserve_declared_override_and_default_sources() {
        static GENERICS: GenericDefinitionMetadata = GenericDefinitionMetadata::new(&[], &[]);
        static TYPE_DEFAULT: ExpressionData = ExpressionData::Parameter(&TYPE_PARAMETER);
        static TYPE_DECLARATIONS: [crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata; 2] = [
            crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata::new(
                "Item",
                Visibility::Public,
                &[],
                Some(&TYPE_DEFAULT),
            ),
            crate::runtime::descriptor::trait_descriptor::AssociatedTypeDefinitionMetadata::new(
                "Required",
                Visibility::Public,
                &[],
                None,
            ),
        ];
        static CONST_DECLARATIONS: [crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata;
            1] = [
            crate::runtime::descriptor::trait_descriptor::AssociatedConstDefinitionMetadata::new(
                "COUNT",
                Visibility::Public,
                &USIZE,
                Some(ConstExpression::Parameter("N")),
            ),
        ];
        struct TraitMarker;
        static TRAIT: crate::__private::codegen_v4::traits::TraitDefinitionSpec<TraitMarker> =
            crate::__private::codegen_v4::traits::TraitDefinitionSpec::new(
                "Trait",
                "crate::Trait",
                Visibility::Public,
                TraitCompleteness::Complete,
                &GENERICS,
                &[],
                &[],
                &TYPE_DECLARATIONS,
                &CONST_DECLARATIONS,
            );
        static APPLICATION: TraitApplicationMetadata = TraitApplicationMetadata::new(
            TRAIT.metadata(),
            &TYPE_ARGUMENTS,
            &[],
            &[],
            &[],
            crate::runtime::descriptor::trait_descriptor::SupertraitCompleteness::Complete,
        );
        static EXPLICIT_TYPE: crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata =
            crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata::new(
                &TYPE_DECLARATIONS[0],
                &U16,
                None,
                crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Declared,
            );
        static EXPLICIT_TYPES: [crate::runtime::descriptor::impl_descriptor::AssociatedTypeBindingMetadata; 1] =
            [EXPLICIT_TYPE];
        static EXPLICIT_CONST: crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata =
            crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata::new(
                &CONST_DECLARATIONS[0],
                ConstExpression::SignedInteger(12),
                crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Declared,
            );
        static EXPLICIT_CONSTS: [crate::runtime::descriptor::impl_descriptor::AssociatedConstBindingMetadata; 1] =
            [EXPLICIT_CONST];

        let type_bindings = associated_type_bindings_with_defaults(&TRAIT, &APPLICATION, &[], &EXPLICIT_TYPES);
        assert_eq!(type_bindings.len(), 2);
        assert_eq!(
            type_bindings[0].source,
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Overridden
        );
        assert!(matches!(
            type_bindings[1].source,
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Defaulted
        ));
        assert!(matches!(
            type_bindings[1].expression,
            ExpressionData::Named { path: "u8", .. }
        ));
        assert!(std::ptr::eq(
            type_bindings[1].concrete.expect("default type resolves")(),
            crate::runtime::descriptor::metadata::TypeMetadata::of::<u8>()
        ));

        let const_bindings = associated_const_bindings_with_defaults(&TRAIT, &APPLICATION, &[], &EXPLICIT_CONSTS);
        assert_eq!(const_bindings.len(), 2);
        assert_eq!(
            const_bindings[0].source,
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Overridden
        );
        assert!(matches!(
            const_bindings[1].source,
            crate::runtime::descriptor::impl_descriptor::AssociatedItemSource::Defaulted
        ));
        assert_eq!(const_bindings[1].expression, ConstExpression::UnsignedInteger(7));

        let required = associated_type_bindings_with_defaults(&TRAIT, &APPLICATION, &["Item"], &[]);
        assert!(required.is_empty(), "declared names suppress trait defaults");
    }
}
