// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

use super::super::data::ConstExpression;
use super::super::data::ExpressionData;
use super::super::data::FunctionPointerData;
use super::super::data::GenericExpressionArgument;
use super::super::data::LifetimeExpression;
use super::super::data::OpaqueTypeData;
use super::super::data::PredicateData;
use super::super::data::TraitBoundModifier;
use super::super::data::TraitExpressionData;
use super::super::data::TraitObjectData;
use super::super::type_expression::TypeExpression;
use crate::runtime::context::ReflectionContext;

/// Formatting adapter for displaying retained type-expression source facts.
pub(in crate::runtime::expression) struct ExpressionDisplay<'ctx> {
    /// Context-bound expression rendered by the formatter.
    pub(in crate::runtime::expression) expression: TypeExpression<'ctx>,
}

impl fmt::Display for ExpressionDisplay<'_> {
    /// Formats the retained structure as Rust-like diagnostic text.
    ///
    /// # Parameters
    ///
    /// - `self`: Formatting adapter containing the expression to render.
    /// - `formatter`: Destination formatter.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to the destination fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expression.data {
            ExpressionData::Named {
                path, arguments, ..
            } => {
                formatter.write_str(path)?;
                write_generic_arguments(formatter, arguments, self.expression.context)
            }
            ExpressionData::Parameter(parameter) => formatter.write_str(parameter.name),
            ExpressionData::ParameterName(name) => formatter.write_str(name),
            ExpressionData::SelfType => formatter.write_str("Self"),
            ExpressionData::AssociatedType {
                self_type,
                trait_,
                name,
                arguments,
            } => {
                if let Some(trait_) = trait_ {
                    write!(
                        formatter,
                        "<{} as ",
                        TypeExpression::new(self_type, self.expression.context).display()
                    )?;
                    write_trait_expression(formatter, trait_, self.expression.context)?;
                    formatter.write_str(">::")?;
                    formatter.write_str(name)?;
                } else {
                    write!(
                        formatter,
                        "{}::{name}",
                        TypeExpression::new(self_type, self.expression.context).display()
                    )?;
                }
                write_generic_arguments(formatter, arguments, self.expression.context)
            }
            ExpressionData::Reference {
                lifetime,
                mutable,
                target,
            } => {
                formatter.write_str("&")?;
                if let Some(lifetime) =
                    lifetime.filter(|value| *value != LifetimeExpression::Elided)
                {
                    write_lifetime(formatter, lifetime)?;
                    formatter.write_str(" ")?;
                }
                if *mutable {
                    formatter.write_str("mut ")?;
                }
                write!(
                    formatter,
                    "{}",
                    TypeExpression::new(target, self.expression.context).display()
                )
            }
            ExpressionData::RawPointer { mutable, target } => write!(
                formatter,
                "*{} {}",
                if *mutable { "mut" } else { "const" },
                TypeExpression::new(target, self.expression.context).display()
            ),
            ExpressionData::Slice(element) => write!(
                formatter,
                "[{}]",
                TypeExpression::new(element, self.expression.context).display()
            ),
            ExpressionData::Array { element, length } => {
                write!(
                    formatter,
                    "[{}; ",
                    TypeExpression::new(element, self.expression.context).display()
                )?;
                write_const(formatter, *length)?;
                formatter.write_str("]")
            }
            ExpressionData::Tuple(elements) => {
                formatter.write_str("(")?;
                for (index, element) in elements.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(", ")?;
                    }
                    write!(
                        formatter,
                        "{}",
                        TypeExpression::new(element, self.expression.context).display()
                    )?;
                }
                if elements.len() == 1 {
                    formatter.write_str(",")?;
                }
                formatter.write_str(")")
            }
            ExpressionData::FunctionPointer(function) => {
                write_function_pointer(formatter, function, self.expression.context)
            }
            ExpressionData::TraitObject(object) => {
                write_trait_object(formatter, object, self.expression.context)
            }
            ExpressionData::Opaque(opaque) => {
                write_opaque(formatter, opaque, self.expression.context)
            }
            ExpressionData::Never => formatter.write_str("!"),
        }
    }
}

/// Writes generic and associated-type arguments in source order.
///
/// # Parameters
///
/// - `formatter`: Destination formatter.
/// - `arguments`: Retained generic arguments to write.
/// - `context`: Context used when recursively formatting nested type values.
///
/// # Errors
///
/// Returns a formatting error if writing any argument fails.
fn write_generic_arguments(
    formatter: &mut fmt::Formatter<'_>,
    arguments: &[GenericExpressionArgument],
    context: &ReflectionContext,
) -> fmt::Result {
    if arguments.is_empty() {
        return Ok(());
    }
    formatter.write_str("<")?;
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        match argument {
            GenericExpressionArgument::Type(ty) => {
                write!(formatter, "{}", TypeExpression::new(ty, context).display())?
            }
            GenericExpressionArgument::Lifetime(lifetime) => write_lifetime(formatter, *lifetime)?,
            GenericExpressionArgument::Const(value) => write_const(formatter, *value)?,
            GenericExpressionArgument::AssociatedType { name, value } => write!(
                formatter,
                "{name} = {}",
                TypeExpression::new(value, context).display()
            )?,
            GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                write!(formatter, "{name}: ")?;
                write_predicates(formatter, bounds, context)?;
            }
        }
    }
    formatter.write_str(">")
}

/// Writes a lifetime using its retained source-level spelling.
///
/// # Parameters
///
/// - `formatter`: Destination formatter.
/// - `lifetime`: Lifetime value to render.
///
/// # Errors
///
/// Returns a formatting error if writing to the destination fails.
fn write_lifetime(formatter: &mut fmt::Formatter<'_>, lifetime: LifetimeExpression) -> fmt::Result {
    match lifetime {
        LifetimeExpression::Static => formatter.write_str("'static"),
        LifetimeExpression::Named(name) => write!(formatter, "'{name}"),
        LifetimeExpression::Elided => Ok(()),
        LifetimeExpression::Placeholder => formatter.write_str("'_"),
    }
}

/// Writes the supported structural const value without evaluating it.
///
/// # Parameters
///
/// - `formatter`: Destination formatter.
/// - `expression`: Retained const value or source name to render.
///
/// # Errors
///
/// Returns a formatting error if writing to the destination fails.
fn write_const(formatter: &mut fmt::Formatter<'_>, expression: ConstExpression) -> fmt::Result {
    match expression {
        ConstExpression::SignedInteger(value) => write!(formatter, "{value}"),
        ConstExpression::UnsignedInteger(value) => write!(formatter, "{value}"),
        ConstExpression::Boolean(value) => write!(formatter, "{value}"),
        ConstExpression::Character(value) => write!(formatter, "{value:?}"),
        ConstExpression::Parameter(name) | ConstExpression::Path(name) => formatter.write_str(name),
    }
}

/// Writes function-pointer qualifiers, arguments, and optional return type.
///
/// # Parameters
///
/// - `formatter`: Destination formatter.
/// - `function`: Retained signature and qualifier facts.
/// - `context`: Context used to render nested type expressions.
///
/// # Errors
///
/// Returns a formatting error if writing any signature component fails.
fn write_function_pointer(
    formatter: &mut fmt::Formatter<'_>,
    function: &FunctionPointerData,
    context: &ReflectionContext,
) -> fmt::Result {
    write_lifetime_binder(formatter, function.lifetimes)?;
    if function.unsafe_ {
        formatter.write_str("unsafe ")?;
    }
    if let Some(abi) = function.abi {
        write!(formatter, "extern {abi:?} ")?;
    }
    formatter.write_str("fn(")?;
    for (index, argument) in function.arguments.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        write!(
            formatter,
            "{}",
            TypeExpression::new(argument, context).display()
        )?;
    }
    if function.variadic {
        if !function.arguments.is_empty() {
            formatter.write_str(", ")?;
        }
        formatter.write_str("...")?;
    }
    formatter.write_str(")")?;
    if let Some(result) = function.result {
        write!(
            formatter,
            " -> {}",
            TypeExpression::new(result, context).display()
        )?;
    }
    Ok(())
}

/// Writes trait-object bounds and the optional object lifetime.
///
/// # Parameters
///
/// - `formatter`: Destination formatter.
/// - `object`: Retained trait bounds and object lifetime.
///
/// # Errors
///
/// Returns a formatting error if writing a bound fails.
fn write_trait_object(
    formatter: &mut fmt::Formatter<'_>,
    object: &TraitObjectData,
    context: &ReflectionContext,
) -> fmt::Result {
    formatter.write_str("dyn ")?;
    for (index, trait_) in object.traits.iter().enumerate() {
        if index > 0 {
            formatter.write_str(" + ")?;
        }
        write_trait_expression(formatter, trait_, context)?;
    }
    if let Some(lifetime) = object.lifetime {
        if !object.traits.is_empty() {
            formatter.write_str(" + ")?;
        }
        write_lifetime(formatter, lifetime)?;
    }
    Ok(())
}

/// Writes the retained trait and lifetime bounds of an opaque type.
///
/// # Parameters
///
/// - `formatter`: Destination formatter.
/// - `opaque`: Opaque type bounds retained from source syntax.
///
/// # Errors
///
/// Returns a formatting error if writing any bound fails.
fn write_opaque(
    formatter: &mut fmt::Formatter<'_>,
    opaque: &OpaqueTypeData,
    context: &ReflectionContext,
) -> fmt::Result {
    formatter.write_str("impl ")?;
    for (index, bound) in opaque.bounds.iter().enumerate() {
        if index > 0 {
            formatter.write_str(" + ")?;
        }
        write_predicate(formatter, bound, context)?;
    }
    Ok(())
}

fn write_lifetime_binder(
    formatter: &mut fmt::Formatter<'_>,
    lifetimes: &[LifetimeExpression],
) -> fmt::Result {
    if lifetimes.is_empty() {
        return Ok(());
    }
    formatter.write_str("for<")?;
    for (index, lifetime) in lifetimes.iter().enumerate() {
        if index > 0 {
            formatter.write_str(", ")?;
        }
        if *lifetime == LifetimeExpression::Elided {
            // An elided lifetime cannot name a binder; keep malformed public
            // metadata visible instead of producing the empty `for<>` form.
            formatter.write_str("'_")?;
        } else {
            write_lifetime(formatter, *lifetime)?;
        }
    }
    formatter.write_str("> ")
}

fn write_trait_expression(
    formatter: &mut fmt::Formatter<'_>,
    trait_: &TraitExpressionData,
    context: &ReflectionContext,
) -> fmt::Result {
    write_lifetime_binder(formatter, trait_.lifetimes)?;
    write_trait_path_and_arguments(formatter, trait_, context)
}

fn write_trait_path_and_arguments(
    formatter: &mut fmt::Formatter<'_>,
    trait_: &TraitExpressionData,
    context: &ReflectionContext,
) -> fmt::Result {
    formatter.write_str(trait_.path)?;
    write_generic_arguments(formatter, trait_.arguments, context)
}

fn write_predicates(
    formatter: &mut fmt::Formatter<'_>,
    predicates: &[PredicateData],
    context: &ReflectionContext,
) -> fmt::Result {
    for (index, predicate) in predicates.iter().enumerate() {
        if index > 0 {
            formatter.write_str(" + ")?;
        }
        write_predicate(formatter, predicate, context)?;
    }
    Ok(())
}

fn write_predicate(
    formatter: &mut fmt::Formatter<'_>,
    predicate: &PredicateData,
    context: &ReflectionContext,
) -> fmt::Result {
    match predicate {
        PredicateData::TraitBound {
            trait_, modifier, ..
        } => {
            write_lifetime_binder(formatter, trait_.lifetimes)?;
            if *modifier == TraitBoundModifier::Maybe {
                formatter.write_str("?")?;
            }
            write_trait_path_and_arguments(formatter, trait_, context)
        }
        PredicateData::LifetimeOutlives { lifetime, outlives } => {
            write_lifetime(formatter, *lifetime)?;
            if *outlives != LifetimeExpression::Elided {
                formatter.write_str(": ")?;
                write_lifetime(formatter, *outlives)?;
            }
            Ok(())
        }
        PredicateData::TypeOutlives { lifetime, .. } => write_lifetime(formatter, *lifetime),
        PredicateData::TypeEquality { left, right } => write!(
            formatter,
            "{} = {}",
            TypeExpression::new(left, context).display(),
            TypeExpression::new(right, context).display()
        ),
        PredicateData::ConstEvaluatable(expression) => {
            formatter.write_str("const ")?;
            write_const(formatter, *expression)
        }
    }
}
