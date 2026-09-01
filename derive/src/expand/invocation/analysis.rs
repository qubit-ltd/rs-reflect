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
    pub(crate) target: &'a TokenStream,
    pub(crate) extension_receiver: Option<TokenStream>,
    pub(crate) default_method: bool,
    pub(crate) has_unproven_associated_type: bool,
}

impl<'a> MethodContext<'a> {
    /// Creates ordinary impl-method analysis context.
    pub(crate) fn implementation(target: &'a TokenStream) -> Self {
        Self {
            target,
            extension_receiver: None,
            default_method: false,
            has_unproven_associated_type: false,
        }
    }

    /// Creates default-trait-method analysis context.
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
    let default_blocked = context.default_method
        && (!method.has_default
            || !method.generics.where_predicates.is_empty()
            || context.has_unproven_associated_type);
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

fn unavailable_reasons(
    method: &MethodIr,
    receiver: Option<&ReceiverPlan>,
    parameters: &[ParameterPlan],
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
    if parameters.iter().any(|plan| plan.unsupported_unsized) {
        reasons.push(UnavailableReasonPlan::UnsupportedUnsizedValue);
    }
    if invocation_disabled_by_policy(method) {
        reasons.push(UnavailableReasonPlan::DisabledByPolicy);
    }
    if reasons.is_empty() {
        reasons.push(UnavailableReasonPlan::DisabledByPolicy);
    }
    reasons
}

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

fn has_helper(method: &MethodIr, helper: HelperName) -> bool {
    method.attributes.iter().any(|attribute| attribute.name == helper)
}

/// Returns whether a parameter can cross the safe dynamic boundary.
pub(crate) fn supports_invocation_parameter(ty: &TypeIr) -> bool {
    match &ty.kind {
        TypeKindIr::Reference { element, .. } => supports_owned_dynamic_type(element),
        _ => supports_owned_dynamic_type(ty),
    }
}

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
pub(crate) fn supports_invocation_return(return_type: &ReturnTypeIr) -> bool {
    match return_type {
        ReturnTypeIr::Unit => true,
        ReturnTypeIr::Type(ty) => {
            matches!(ty.kind, TypeKindIr::Reference { .. } | TypeKindIr::Never) || supports_owned_dynamic_type(ty)
        }
    }
}

fn invocation_disabled_by_policy(method: &MethodIr) -> bool {
    method
        .attributes
        .iter()
        .any(|attribute| matches!(attribute.name, HelperName::NoInvoke | HelperName::Skip))
}

fn has_unsupported_unsized_parameter(ty: &TypeIr) -> bool {
    matches!(
        &ty.kind,
        TypeKindIr::Reference { element, .. }
            if matches!(element.kind, TypeKindIr::Slice(_) | TypeKindIr::TraitObject { .. })
    )
}

/// Returns the owned standard container for an explicit receiver.
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

fn is_self_type(ty: &TypeIr) -> bool {
    matches!(&ty.kind, TypeKindIr::Path(path) if path.segments.len() == 1 && path.segments[0].name == "Self")
}

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
pub(crate) fn return_contains_non_static_lifetime(return_type: &ReturnTypeIr) -> bool {
    super::lifetime::return_contains_non_static_lifetime(return_type)
}

/// Returns whether a shared borrow can retain the invocation call lifetime.
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

    fn impl_method(input: TokenStream) -> MethodIr {
        let parsed = parse_and_validate_declaration(MacroKind::Impl, TokenStream::new(), input)
            .expect("the reflected impl should parse");
        let DeclarationIr::Impl(declaration) = parsed.declaration else {
            panic!("expected an impl declaration");
        };
        declaration.methods.into_iter().next().expect("one method")
    }

    fn plan(method: &MethodIr) -> InvocationPlan {
        analyze_method(method, MethodContext::implementation(&quote!(Service)))
            .expect("method analysis should be infallible")
    }

    fn reasons(plan: &InvocationPlan) -> &[UnavailableReasonPlan] {
        match &plan.availability {
            AvailabilityPlan::Executable => &[],
            AvailabilityPlan::DescribedOnly(reasons) => reasons,
        }
    }

    #[test]
    fn safe_method_is_executable() {
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

    #[test]
    fn unsafe_abi_and_generic_reasons_are_canonical() {
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
    fn policy_and_async_borrow_disable_execution() {
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
    fn adapter_modes_and_pinned_receiver_are_retained() {
        let thread_safe = impl_method(quote! {
            impl Service {
                #[reflect(thread_safe)]
                fn execute(self: ::std::pin::Pin<&mut Self>) -> u32 { 0 }
            }
        });
        let pinned_plan = plan(&thread_safe);
        assert!(pinned_plan.modes.thread_safe);
        assert_eq!(pinned_plan.pinned_receiver_mutability(), Some(true));
        assert_eq!(reasons(&pinned_plan), &[UnavailableReasonPlan::DisabledByPolicy],);

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
    fn trait_default_requires_proven_owner_facts() {
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
        assert_eq!(reasons(&plan), &[UnavailableReasonPlan::DisabledByPolicy],);
    }
}
