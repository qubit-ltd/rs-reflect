// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Pure invocation analysis shared by impl and trait expansion.

// qubit-style: allow type-file-name

use proc_macro2::TokenStream;
use quote::quote;

use super::plan::AdapterModes;
use super::plan::AvailabilityPlan;
use super::plan::InvocationPlan;
use super::plan::OutputPlan;
use super::plan::ParameterPlan;
use super::plan::ReceiverPlan;
use super::plan::UnavailableReasonPlan;
use crate::ir::HelperName;
use crate::ir::MethodIr;
use crate::ir::PathArgumentIr;
use crate::ir::PathArgumentsIr;
use crate::ir::ReceiverKindIr;
use crate::ir::ReturnTypeIr;
use crate::ir::TypeIr;
use crate::ir::TypeKindIr;

/// Owner-specific facts needed by otherwise pure method analysis.
pub(crate) struct MethodContext<'a> {
    /// Concrete target type used to analyze explicit receiver forms.
    pub(crate) target: &'a TokenStream,
    /// Substituted extension receiver type, when provided by the owner.
    pub(crate) extension_receiver: Option<TokenStream>,
    /// Whether the method is a trait default body.
    pub(crate) default_method: bool,
    /// Whether associated types needed by a default body are not proven.
    pub(crate) has_unproven_associated_type: bool,
}

impl<'a> MethodContext<'a> {
    /// Creates ordinary impl-method analysis context.
    ///
    /// # Parameters
    ///
    /// - `target`: Concrete impl target type tokens.
    ///
    /// # Returns
    ///
    /// Returns an implementation context without trait-default constraints.
    pub(crate) fn implementation(target: &'a TokenStream) -> Self {
        Self {
            target,
            extension_receiver: None,
            default_method: false,
            has_unproven_associated_type: false,
        }
    }

    /// Creates default-trait-method analysis context.
    ///
    /// # Parameters
    ///
    /// - `target`: Trait self type tokens.
    /// - `has_unproven_associated_type`: Whether required owner facts are
    ///   unavailable.
    ///
    /// # Returns
    ///
    /// Returns a context for checking a trait default method.
    pub(crate) fn trait_default(target: &'a TokenStream, has_unproven_associated_type: bool) -> Self {
        Self {
            target,
            extension_receiver: None,
            default_method: true,
            has_unproven_associated_type,
        }
    }
}

/// Produces one complete invocation decision without emitting tokens.
///
/// # Parameters
///
/// - `method`: Validated method declaration and its invocation attributes.
/// - `context`: Owner-specific type and proof facts.
///
/// # Returns
///
/// Returns the invocation plan.
///
/// # Errors
///
/// This function currently returns no errors; the result type is retained for
/// compatibility with callers that compose invocation analysis with parsing.
pub(crate) fn analyze_method(method: &MethodIr, context: MethodContext<'_>) -> syn::Result<InvocationPlan> {
    let receiver = method.receiver.as_ref().map(|receiver| match receiver.kind {
        ReceiverKindIr::Value => ReceiverPlan::Value,
        ReceiverKindIr::SharedReference => ReceiverPlan::SharedReference,
        ReceiverKindIr::MutableReference => ReceiverPlan::MutableReference,
        ReceiverKindIr::Typed => {
            if let Some(receiver) = typed_owned_receiver_type(receiver, context.target) {
                ReceiverPlan::OwnedContainer(receiver)
            } else if let Some(mutable) = typed_pinned_receiver_mutable(receiver) {
                ReceiverPlan::Pinned { mutable }
            } else if let Some(receiver) = context.extension_receiver {
                ReceiverPlan::Extension(receiver)
            } else {
                ReceiverPlan::Unsupported
            }
        }
    });
    let parameters = method
        .parameters
        .iter()
        .map(|parameter| ParameterPlan {
            index: parameter.index,
            supported: supports_invocation_parameter(&parameter.ty),
            unsupported_unsized: has_unsupported_unsized_parameter(&parameter.ty),
        })
        .collect::<Vec<_>>();
    debug_assert!(
        parameters
            .iter()
            .zip(&method.parameters)
            .all(|(plan, parameter)| plan.index == parameter.index)
    );
    let output = output_plan(&method.return_type);
    let modes = AdapterModes {
        thread_safe: has_helper(method, HelperName::ThreadSafe),
        catching: has_helper(method, HelperName::CatchUnwind) && !method.qualifiers.is_async,
        asynchronous: method.qualifiers.is_async,
    };
    let pinned = matches!(receiver, Some(ReceiverPlan::Pinned { .. }));
    let supported_receiver = !matches!(receiver, Some(ReceiverPlan::Unsupported));
    let supported_parameters = parameters.iter().all(|plan| plan.supported);
    let supported_borrow = !return_contains_non_static_lifetime(&method.return_type)
        || is_supported_shared_borrow_return(&method.return_type)
        || is_supported_mutable_borrow_return(method);
    let policy_disabled = invocation_disabled_by_policy(method);
    let unproven_default_constraint = context.default_method && !method.generics.where_predicates.is_empty();
    let unproven_associated_type = context.default_method && context.has_unproven_associated_type;
    let default_blocked =
        context.default_method && (!method.has_default || unproven_default_constraint || unproven_associated_type);
    let mode_blocked = pinned
        && (method.qualifiers.is_async || is_borrow_return(&method.return_type) || modes.thread_safe || modes.catching);
    let executable = supported_receiver
        && supported_parameters
        && method.generics.params.is_empty()
        && !method.qualifiers.is_unsafe
        && method.qualifiers.abi.is_none()
        && !method.qualifiers.is_variadic
        && supported_borrow
        && (!method.qualifiers.is_async || !is_borrow_return(&method.return_type))
        && !policy_disabled
        && !default_blocked
        && !mode_blocked
        && supports_invocation_return(&method.return_type);
    let availability = if executable {
        AvailabilityPlan::Executable
    } else {
        AvailabilityPlan::DescribedOnly(unavailable_reasons(
            method,
            receiver.as_ref(),
            &parameters,
            unproven_default_constraint,
            unproven_associated_type,
            mode_blocked,
        ))
    };
    debug_assert!(!matches!(receiver, Some(ReceiverPlan::Unsupported)) || !executable);
    debug_assert!(!matches!(output, OutputPlan::Unsupported | OutputPlan::Opaque) || !executable);
    Ok(InvocationPlan {
        receiver,
        parameters,
        output,
        modes,
        availability,
    })
}

/// Collects every invocation blocker in the project's canonical order.
///
/// # Parameters
///
/// - `method`: Method declaration being analyzed.
/// - `receiver`: Classified receiver plan, if present.
/// - `parameters`: Classified parameter plans.
/// - `unproven_default_constraint`: Whether a default method has unproven
///   predicates.
/// - `unproven_associated_type`: Whether a default method depends on unproven
///   associated types.
/// - `pinned_mode_conflict`: Whether pinned invocation conflicts with requested
///   modes.
///
/// # Returns
///
/// Returns ordered reasons explaining why the adapter is unavailable.
fn unavailable_reasons(
    method: &MethodIr,
    receiver: Option<&ReceiverPlan>,
    parameters: &[ParameterPlan],
    unproven_default_constraint: bool,
    unproven_associated_type: bool,
    pinned_mode_conflict: bool,
) -> Vec<UnavailableReasonPlan> {
    let mut reasons = Vec::new();
    if matches!(receiver, Some(ReceiverPlan::Unsupported)) {
        reasons.push(UnavailableReasonPlan::UnsupportedReceiver);
    }
    if !method.generics.params.is_empty() {
        reasons.push(UnavailableReasonPlan::UnspecializedGeneric);
    }
    if method.qualifiers.is_unsafe {
        reasons.push(UnavailableReasonPlan::UnsafeMethod);
    }
    if method.qualifiers.abi.is_some() {
        reasons.push(UnavailableReasonPlan::UnsupportedAbi);
    }
    if method.qualifiers.is_variadic {
        reasons.push(UnavailableReasonPlan::Variadic);
    }
    if (return_contains_non_static_lifetime(&method.return_type)
        && !is_supported_shared_borrow_return(&method.return_type)
        && !is_supported_mutable_borrow_return(method))
        || (method.qualifiers.is_async && is_borrow_return(&method.return_type))
    {
        reasons.push(UnavailableReasonPlan::UnsupportedBorrowedReturn);
    }
    if matches!(output_plan(&method.return_type), OutputPlan::Opaque) {
        reasons.push(UnavailableReasonPlan::OpaqueReturn);
    }
    if parameters.iter().any(|plan| plan.unsupported_unsized)
        || matches!(&method.return_type, ReturnTypeIr::Type(ty) if has_unsupported_unsized_parameter(ty))
    {
        reasons.push(UnavailableReasonPlan::UnsupportedUnsizedValue);
    }
    if unproven_default_constraint {
        reasons.push(UnavailableReasonPlan::UnprovenDefaultConstraint);
    }
    if unproven_associated_type {
        reasons.push(UnavailableReasonPlan::UnprovenAssociatedType);
    }
    if pinned_mode_conflict {
        reasons.push(UnavailableReasonPlan::PinnedModeConflict);
    }
    if invocation_disabled_by_policy(method) {
        reasons.push(UnavailableReasonPlan::DisabledByPolicy);
    }
    if reasons.is_empty() {
        reasons.push(UnavailableReasonPlan::DisabledByPolicy);
    }
    reasons
}

/// Classifies the invocation output form before checking adapter support.
///
/// # Parameters
///
/// - `return_type`: Parsed method return declaration.
///
/// # Returns
///
/// Returns the corresponding output plan.
fn output_plan(return_type: &ReturnTypeIr) -> OutputPlan {
    match return_type {
        ReturnTypeIr::Unit => OutputPlan::Unit,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Never,
            ..
        }) => OutputPlan::Never,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Reference { mutable: false, .. },
            ..
        }) => OutputPlan::SharedBorrow,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Reference { mutable: true, .. },
            ..
        }) => OutputPlan::MutableBorrow,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::ImplTrait { .. },
            ..
        }) => OutputPlan::Opaque,
        ReturnTypeIr::Type(ty) if supports_owned_dynamic_type(ty) => OutputPlan::Owned,
        ReturnTypeIr::Type(_) => OutputPlan::Unsupported,
    }
}

/// Returns whether a method has a particular reflection helper attribute.
///
/// # Parameters
///
/// - `method`: Method whose attributes are checked.
/// - `helper`: Helper attribute name to find.
///
/// # Returns
///
/// Returns `true` when the method declares that helper.
fn has_helper(method: &MethodIr, helper: HelperName) -> bool {
    method.attributes.iter().any(|attribute| attribute.name == helper)
}

/// Returns whether a parameter can cross the safe dynamic boundary.
///
/// # Parameters
///
/// - `ty`: Parsed parameter type.
///
/// # Returns
///
/// Returns `true` when the type has a supported owned dynamic representation.
pub(crate) fn supports_invocation_parameter(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Reference { element, .. } => supports_owned_dynamic_type(element),
        _ => supports_owned_dynamic_type(ty),
    }
}

/// Returns whether a type has a sized owned dynamic representation.
///
/// # Parameters
///
/// - `ty`: Parsed type to classify.
///
/// # Returns
///
/// Returns `true` for supported structural type forms.
fn supports_owned_dynamic_type(ty: &TypeIr) -> bool {
    matches!(
        ty.kind,
        TypeKindIr::Path(_)
            | TypeKindIr::Tuple(_)
            | TypeKindIr::Array { .. }
            | TypeKindIr::Pointer { .. }
            | TypeKindIr::BareFunction { .. }
    )
}

/// Returns whether an invocation adapter can represent this output.
///
/// # Parameters
///
/// - `return_type`: Parsed method return declaration.
///
/// # Returns
///
/// Returns whether the output can be represented by the invocation API.
pub(crate) fn supports_invocation_return(return_type: &ReturnTypeIr) -> bool {
    match return_type {
        ReturnTypeIr::Unit => true,
        ReturnTypeIr::Type(ty) if has_unsupported_unsized_parameter(ty) => false,
        ReturnTypeIr::Type(ty) => {
            matches!(ty.kind, TypeKindIr::Reference { .. } | TypeKindIr::Never) || supports_owned_dynamic_type(ty)
        }
    }
}

/// Returns whether reflection attributes disable invocation for a method.
///
/// # Parameters
///
/// - `method`: Method whose policy attributes are checked.
///
/// # Returns
///
/// Returns `true` for `no_invoke` or `skip`.
fn invocation_disabled_by_policy(method: &MethodIr) -> bool {
    method
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.name, HelperName::NoInvoke | HelperName::Skip))
}

/// Returns whether a reference type contains an unsupported unsized value.
///
/// # Parameters
///
/// - `ty`: Parsed type to inspect.
///
/// # Returns
///
/// Returns `true` for slice and trait-object reference targets.
fn has_unsupported_unsized_parameter(ty: &TypeIr) -> bool {
    matches!(
        &ty.kind,
        TypeKindIr::Reference { element, .. }
            if matches!(element.kind, TypeKindIr::Slice(_) | TypeKindIr::TraitObject { .. })
    )
}

/// Returns the owned standard container for an explicit receiver.
///
/// # Parameters
///
/// - `receiver`: Explicit receiver declaration to inspect.
/// - `target`: Concrete impl target type tokens.
///
/// # Returns
///
/// Returns a recognized owned container type, or `None` for other receivers.
pub(crate) fn typed_owned_receiver_type(receiver: &crate::ir::ReceiverIr, target: &TokenStream) -> Option<TokenStream> {
    if receiver.kind != ReceiverKindIr::Typed {
        return None;
    }
    let TypeKindIr::Path(path) = &receiver.ty.kind else {
        return None;
    };
    let segment = path.segments.last()?;
    let PathArgumentsIr::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match (segment.name.as_str(), arguments.as_slice()) {
        ("Box", [PathArgumentIr::Type(argument)]) if is_self_type(argument) => Some(quote!(::std::boxed::Box<#target>)),
        ("Rc", [PathArgumentIr::Type(argument)]) if is_self_type(argument) => Some(quote!(::std::rc::Rc<#target>)),
        ("Arc", [PathArgumentIr::Type(argument)]) if is_self_type(argument) => Some(quote!(::std::sync::Arc<#target>)),
        ("Pin", [PathArgumentIr::Type(argument)]) if is_box_self_type(argument) => {
            Some(quote!(::std::pin::Pin<::std::boxed::Box<#target>>))
        }
        _ => None,
    }
}

/// Returns pinned shared/mutable receiver mode for `Pin<&Self>` shapes.
///
/// # Parameters
///
/// - `receiver`: Explicit receiver declaration to inspect.
///
/// # Returns
///
/// Returns `Some(false)` for shared pin, `Some(true)` for mutable pin, or
/// `None`.
pub(crate) fn typed_pinned_receiver_mutable(receiver: &crate::ir::ReceiverIr) -> Option<bool> {
    if receiver.kind != ReceiverKindIr::Typed {
        return None;
    }
    let TypeKindIr::Path(path) = &receiver.ty.kind else {
        return None;
    };
    let segment = path.segments.last()?;
    let PathArgumentsIr::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let [PathArgumentIr::Type(argument)] = arguments.as_slice() else {
        return None;
    };
    let TypeKindIr::Reference { mutable, element, .. } = &argument.kind else {
        return None;
    };
    (segment.name == "Pin" && is_self_type(element)).then_some(*mutable)
}

/// Returns whether a type is the unqualified `Self` type.
///
/// # Parameters
///
/// - `ty`: Parsed type to inspect.
///
/// # Returns
///
/// Returns `true` only for the direct `Self` path.
fn is_self_type(ty: &TypeIr) -> bool {
    matches!(&ty.kind, TypeKindIr::Path(path) if path.segments.len() == 1 && path.segments[0].name == "Self")
}

/// Returns whether a type is the standard `Box<Self>` receiver shape.
///
/// # Parameters
///
/// - `ty`: Parsed type to inspect.
///
/// # Returns
///
/// Returns `true` for a `Box` path with exactly the `Self` type argument.
fn is_box_self_type(ty: &TypeIr) -> bool {
    let TypeKindIr::Path(path) = &ty.kind else {
        return false;
    };
    let Some(segment) = path.segments.last() else {
        return false;
    };
    matches!(
        (&*segment.name, &segment.arguments),
        ("Box", PathArgumentsIr::AngleBracketed(arguments))
            if matches!(arguments.as_slice(), [PathArgumentIr::Type(argument)] if is_self_type(argument))
    )
}

/// Returns whether a return declaration contains a non-static lifetime.
///
/// # Parameters
///
/// - `return_type`: Parsed method return declaration.
///
/// # Returns
///
/// Returns whether it contains a non-static borrow lifetime.
pub(crate) fn return_contains_non_static_lifetime(return_type: &ReturnTypeIr) -> bool {
    super::lifetime::return_contains_non_static_lifetime(return_type)
}

/// Returns whether a shared borrow can retain the invocation call lifetime.
///
/// # Parameters
///
/// - `return_type`: Parsed method return declaration.
///
/// # Returns
///
/// Returns `true` for a shared reference return.
pub(crate) fn is_supported_shared_borrow_return(return_type: &ReturnTypeIr) -> bool {
    matches!(
        return_type,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Reference { mutable: false, .. },
            ..
        })
    )
}

/// Returns whether a unique mutable-borrow origin can be identified.
///
/// # Parameters
///
/// - `method`: Method whose receiver, parameters and return are inspected.
///
/// # Returns
///
/// Returns `true` when the mutable return originates uniquely from `&mut self`.
pub(crate) fn is_supported_mutable_borrow_return(method: &MethodIr) -> bool {
    matches!(
        method.receiver.as_ref().map(|receiver| receiver.kind),
        Some(ReceiverKindIr::MutableReference)
    ) && !method
        .parameters
        .iter()
        .any(|parameter| matches!(parameter.ty.kind, TypeKindIr::Reference { mutable: true, .. }))
        && matches!(
            method.return_type,
            ReturnTypeIr::Type(TypeIr {
                kind: TypeKindIr::Reference { mutable: true, .. },
                ..
            })
        )
}

/// Returns whether the declaration returns a borrow.
///
/// # Parameters
///
/// - `return_type`: Parsed method return declaration.
///
/// # Returns
///
/// Returns `true` for shared or mutable reference returns.
pub(crate) fn is_borrow_return(return_type: &ReturnTypeIr) -> bool {
    matches!(
        return_type,
        ReturnTypeIr::Type(TypeIr {
            kind: TypeKindIr::Reference { .. },
            ..
        })
    )
}

/// Returns whether `ty` names Rust's built-in unsized `str` type.
///
/// # Parameters
///
/// - `ty`: Parsed type to inspect.
///
/// # Returns
///
/// Returns `true` for an unparameterized path ending in `str`.
pub(crate) fn is_str_type(ty: &TypeIr) -> bool {
    matches!(
        &ty.kind,
        TypeKindIr::Path(path)
            if path.segments.last().is_some_and(|segment| {
                segment.name == "str" && matches!(segment.arguments, PathArgumentsIr::None)
            })
    )
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    use super::super::plan::AvailabilityPlan;
    use super::super::plan::InvocationPlan;
    use super::super::plan::OutputPlan;
    use super::super::plan::ReceiverPlan;
    use super::super::plan::UnavailableReasonPlan;
    use super::MethodContext;
    use super::analyze_method;
    use crate::ir::DeclarationIr;
    use crate::ir::MacroKind;
    use crate::ir::MethodIr;
    use crate::parse::parse_and_validate_declaration;

    /// Parses one test impl and returns its only method.
    ///
    /// # Parameters
    ///
    /// - `input`: Impl tokens used as the test fixture.
    ///
    /// # Returns
    ///
    /// Returns the parsed method declaration.
    fn impl_method(input: TokenStream) -> MethodIr {
        let parsed = parse_and_validate_declaration(MacroKind::Impl, TokenStream::new(), input)
            .expect("the reflected impl should parse");
        let DeclarationIr::Impl(declaration) = parsed.declaration else {
            panic!("expected an impl declaration");
        };
        declaration.methods.into_iter().next().expect("one method")
    }

    /// Analyzes one test method as an ordinary implementation method.
    ///
    /// # Parameters
    ///
    /// - `method`: Method declaration to analyze.
    ///
    /// # Returns
    ///
    /// Returns its invocation plan.
    fn plan(method: &MethodIr) -> InvocationPlan {
        analyze_method(method, MethodContext::implementation(&quote!(Service)))
            .expect("method analysis should be infallible")
    }

    /// Returns the ordered blocker list for a described-only plan.
    ///
    /// # Parameters
    ///
    /// - `plan`: Invocation plan to inspect.
    ///
    /// # Returns
    ///
    /// Returns an empty slice for executable plans, or their blocker list.
    fn reasons(plan: &InvocationPlan) -> &[UnavailableReasonPlan] {
        match &plan.availability {
            AvailabilityPlan::Executable => &[],
            AvailabilityPlan::DescribedOnly(reasons) => reasons,
        }
    }

    #[test]
    fn test_safe_method_is_executable() {
        let method = impl_method(quote! {
            impl Service {
                fn execute(&self, value: u32) -> String { unreachable!() }
            }
        });
        let plan = plan(&method);
        assert!(plan.is_executable());
        assert_eq!(plan.output, OutputPlan::Owned);
        assert!(matches!(plan.receiver, Some(ReceiverPlan::SharedReference)));
    }

    /// Unsized slices remain described without an invalid Sized adapter.
    #[test]
    fn test_slice_return_is_described_only() {
        let method = impl_method(quote! {
            impl Service {
                fn values(&self) -> &[String] { unreachable!() }
            }
        });
        assert_eq!(
            reasons(&plan(&method)),
            &[UnavailableReasonPlan::UnsupportedUnsizedValue]
        );
    }

    #[test]
    fn test_unsafe_abi_and_generic_reasons_are_canonical() {
        let unsafe_method = impl_method(quote! {
            impl Service {
                unsafe extern "C" fn execute(&self, value: u32) -> u32 { value }
            }
        });
        assert_eq!(
            reasons(&plan(&unsafe_method)),
            &[
                UnavailableReasonPlan::UnsafeMethod,
                UnavailableReasonPlan::UnsupportedAbi,
            ],
        );

        let generic_method = impl_method(quote! {
            impl Service {
                fn execute<T>(&self, value: T) -> T { value }
            }
        });
        assert_eq!(
            reasons(&plan(&generic_method)),
            &[UnavailableReasonPlan::UnspecializedGeneric],
        );
    }

    #[test]
    fn test_policy_and_async_borrow_disable_execution() {
        let disabled = impl_method(quote! {
            impl Service {
                #[reflect(no_invoke)]
                fn execute(&self) {}
            }
        });
        assert_eq!(reasons(&plan(&disabled)), &[UnavailableReasonPlan::DisabledByPolicy],);

        let asynchronous = impl_method(quote! {
            impl Service {
                async fn execute(&self) -> &str { "value" }
            }
        });
        assert_eq!(
            reasons(&plan(&asynchronous)),
            &[UnavailableReasonPlan::UnsupportedBorrowedReturn],
        );
    }

    #[test]
    fn test_adapter_modes_and_pinned_receiver_are_retained() {
        let thread_safe = impl_method(quote! {
            impl Service {
                #[reflect(thread_safe)]
                fn execute(self: ::std::pin::Pin<&mut Self>) -> u32 { 0 }
            }
        });
        let pinned_plan = plan(&thread_safe);
        assert!(pinned_plan.modes.thread_safe);
        assert_eq!(pinned_plan.pinned_receiver_mutability(), Some(true));
        assert_eq!(reasons(&pinned_plan), &[UnavailableReasonPlan::PinnedModeConflict],);

        let catching = impl_method(quote! {
            impl Service {
                #[reflect(catch_unwind)]
                fn execute(&self) -> u32 { 0 }
            }
        });
        let plan = plan(&catching);
        assert!(plan.is_executable());
        assert!(plan.modes.catching);
    }

    #[test]
    fn test_trait_default_requires_proven_owner_facts() {
        let parsed = parse_and_validate_declaration(
            MacroKind::Trait,
            TokenStream::new(),
            quote! {
                trait Service {
                    fn execute(&self) -> Self::Output { unreachable!() }
                    type Output;
                }
            },
        )
        .expect("the reflected trait should parse");
        let DeclarationIr::Trait(declaration) = parsed.declaration else {
            panic!("expected a trait declaration");
        };
        let method = &declaration.methods[0];
        let plan = analyze_method(method, MethodContext::trait_default(&quote!(Self), true))
            .expect("method analysis should be infallible");
        assert_eq!(reasons(&plan), &[UnavailableReasonPlan::UnprovenAssociatedType],);
    }
}
