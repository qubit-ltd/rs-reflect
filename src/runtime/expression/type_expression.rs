// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

use super::data::ConstExpression;
use super::data::ExpressionData;
use super::data::FunctionPointerData;
use super::data::GenericExpressionArgument;
use super::data::LifetimeExpression;
use super::data::OpaqueTypeData;
use super::data::TraitExpressionData;
use super::data::TraitObjectData;
use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::GenericParameterDescriptor;
use crate::runtime::descriptor::generic::TypeDefinitionDescriptor;
use crate::runtime::descriptor::trait_descriptor::TraitDefinitionDescriptor;

/// A read-only, context-bound view of source-level Rust type syntax.
#[derive(Clone, Copy)]
pub struct TypeExpression<'ctx> {
    data: &'static ExpressionData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TypeExpression<'ctx> {
    /// Binds static expression metadata to the supplied context.
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

    /// Formats source facts for logs and diagnostics only.
    pub fn display(self) -> impl fmt::Display + 'ctx {
        ExpressionDisplay { expression: self }
    }
}

/// The complete structural shape of a source-level Rust type expression.
pub enum TypeExpressionKind<'ctx> {
    /// A named path with source-level generic arguments.
    Named(NamedTypeExpression<'ctx>),
    /// A generic parameter linked to its declaration metadata.
    Parameter(GenericParameterDescriptor<'ctx>),
    /// A generic parameter name without a definition link.
    ParameterName(&'static str),
    /// The special `Self` type expression.
    SelfType,
    /// An associated-type projection.
    AssociatedType(AssociatedTypeExpression<'ctx>),
    /// A borrowed type and its optional explicit lifetime.
    Reference {
        /// Lifetime written on the reference, when present.
        lifetime: Option<LifetimeExpression>,
        /// Whether the reference is mutable.
        mutable: bool,
        /// Type referenced by the borrow.
        target: TypeExpression<'ctx>,
    },
    /// A raw pointer and its mutability.
    RawPointer {
        /// Whether the pointed-to value is mutable.
        mutable: bool,
        /// Type pointed to by the raw pointer.
        target: TypeExpression<'ctx>,
    },
    /// A slice and its element type.
    Slice(TypeExpression<'ctx>),
    /// An array and its element type and structural length.
    Array {
        /// Type of each array element.
        element: TypeExpression<'ctx>,
        /// Recorded array length expression; it is not evaluated here.
        length: ConstExpression,
    },
    /// Tuple elements in source order.
    Tuple(Box<[TypeExpression<'ctx>]>),
    /// A function pointer signature.
    FunctionPointer(FunctionPointerExpression<'ctx>),
    /// A trait-object bound list.
    TraitObject(TraitObjectExpression<'ctx>),
    /// Bounds of an opaque `impl Trait` type.
    Opaque(OpaqueTypeExpression<'ctx>),
    /// The never type `!`.
    Never,
}

/// A named path that may resolve to a known generic definition.
#[derive(Clone, Copy)]
pub struct NamedTypeExpression<'ctx> {
    path: &'static str,
    definition: Option<&'static super::super::descriptor::generic::TypeDefinitionMetadata>,
    arguments: &'static [GenericExpressionArgument],
    context: &'ctx ReflectionContext,
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

/// A projection such as `<T as Iterator>::Item`.
#[derive(Clone, Copy)]
pub struct AssociatedTypeExpression<'ctx> {
    self_type: TypeExpression<'ctx>,
    trait_: Option<TraitExpression<'ctx>>,
    name: &'static str,
    arguments: &'static [GenericExpressionArgument],
}

impl<'ctx> AssociatedTypeExpression<'ctx> {
    /// Returns the context used by the projected type and trait.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.self_type.context()
    }

    /// Returns the type from which the associated type is projected.
    pub fn self_type(self) -> TypeExpression<'ctx> {
        self.self_type
    }
    /// Returns the trait qualifier, or `None` for an unqualified projection.
    pub fn trait_(self) -> Option<TraitExpression<'ctx>> {
        self.trait_
    }
    /// Returns the projected associated-type name.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns generic arguments written on the projected associated type.
    pub fn arguments(self) -> GenericExpressionArguments<'ctx> {
        GenericExpressionArguments::new(self.arguments, self.context())
    }
}

/// A trait path attached to a projection or trait object.
#[derive(Clone, Copy)]
pub struct TraitExpression<'ctx> {
    data: &'static TraitExpressionData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpression<'ctx> {
    pub(crate) const fn new(data: &'static TraitExpressionData, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested trait argument views.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the trait path as written in the source expression.
    pub const fn path(self) -> &'static str {
        self.data.path
    }

    /// Returns the definition associated with this expression, if available.
    ///
    /// This source expression carries only a path, not a trait identity, so a
    /// matching path in the context is not sufficient to establish a link.
    pub fn find_definition(self) -> Option<TraitDefinitionDescriptor<'ctx>> {
        None
    }

    /// Returns explicit lifetime arguments in source order.
    pub const fn lifetimes(self) -> &'static [LifetimeExpression] {
        self.data.lifetimes
    }
    /// Returns generic and associated-type arguments in source order.
    pub fn arguments(self) -> GenericExpressionArguments<'ctx> {
        GenericExpressionArguments::new(self.data.arguments, self.context)
    }

    /// Returns associated-type equality and bound facts from the source path.
    pub fn associated_type_constraints(self) -> AssociatedTypeConstraints<'ctx> {
        AssociatedTypeConstraints {
            arguments: self.data.arguments,
            context: self.context,
        }
    }
}

/// A context-bound view of one named, lifetime, const, or associated argument.
#[derive(Clone, Copy)]
pub struct GenericExpressionArgumentView<'ctx> {
    data: GenericExpressionArgument,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericExpressionArgumentView<'ctx> {
    const fn new(data: GenericExpressionArgument, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested expressions and predicates.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the structural form of this argument, binding nested views to
    /// its context.
    pub fn kind(self) -> GenericExpressionArgumentKind<'ctx> {
        match self.data {
            GenericExpressionArgument::Type(ty) => {
                GenericExpressionArgumentKind::Type(TypeExpression::new(ty, self.context))
            }
            GenericExpressionArgument::Lifetime(value) => GenericExpressionArgumentKind::Lifetime(value),
            GenericExpressionArgument::Const(value) => GenericExpressionArgumentKind::Const(value),
            GenericExpressionArgument::AssociatedType { name, value } => {
                GenericExpressionArgumentKind::AssociatedType {
                    name,
                    value: TypeExpression::new(value, self.context),
                }
            }
            GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                GenericExpressionArgumentKind::AssociatedTypeBound {
                    name,
                    bounds: super::predicate::Predicates::new(bounds, self.context),
                }
            }
        }
    }
}

/// Structural kind of a source-level generic argument.
pub enum GenericExpressionArgumentKind<'ctx> {
    /// A nested type argument.
    Type(TypeExpression<'ctx>),
    /// A lifetime argument.
    Lifetime(LifetimeExpression),
    /// A structural const argument.
    Const(ConstExpression),
    /// An associated-type equality such as `Item = u8`.
    AssociatedType {
        /// Name of the associated type.
        name: &'static str,
        /// Type assigned to the associated type.
        value: TypeExpression<'ctx>,
    },
    /// An associated-type bound such as `Item: Copy`.
    AssociatedTypeBound {
        /// Name of the associated type.
        name: &'static str,
        /// Predicates recorded for the associated type.
        bounds: super::predicate::Predicates<'ctx>,
    },
}

/// An ordered context-bound view over type-expression arguments.
#[derive(Clone, Copy)]
pub struct GenericExpressionArguments<'ctx> {
    arguments: &'static [GenericExpressionArgument],
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericExpressionArguments<'ctx> {
    const fn new(arguments: &'static [GenericExpressionArgument], context: &'ctx ReflectionContext) -> Self {
        Self { arguments, context }
    }

    /// Returns the context used by nested argument expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the number of source arguments.
    pub fn len(self) -> usize {
        self.arguments.len()
    }

    /// Returns whether the source argument list is empty.
    pub fn is_empty(self) -> bool {
        self.arguments.is_empty()
    }

    /// Returns one argument by its zero-based source position.
    pub fn get(self, index: usize) -> Option<GenericExpressionArgumentView<'ctx>> {
        self.arguments
            .get(index)
            .copied()
            .map(|argument| GenericExpressionArgumentView::new(argument, self.context))
    }

    /// Iterates over arguments in source order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = GenericExpressionArgumentView<'ctx>> + 'ctx {
        self.arguments
            .iter()
            .copied()
            .map(move |argument| GenericExpressionArgumentView::new(argument, self.context))
    }
}

/// Source-level associated-type constraints attached to a trait expression.
#[derive(Clone, Copy)]
pub struct AssociatedTypeConstraints<'ctx> {
    arguments: &'static [GenericExpressionArgument],
    context: &'ctx ReflectionContext,
}

impl<'ctx> AssociatedTypeConstraints<'ctx> {
    /// Iterates through associated-type bounds in source order.
    pub fn iter(self) -> impl Iterator<Item = AssociatedTypeConstraintDescriptor<'ctx>> + 'ctx {
        self.arguments.iter().filter_map(move |argument| match argument {
            GenericExpressionArgument::AssociatedType { name, value } => Some(AssociatedTypeConstraintDescriptor {
                name,
                kind: AssociatedTypeConstraintKind::Equality(TypeExpression::new(value, self.context)),
            }),
            GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                Some(AssociatedTypeConstraintDescriptor {
                    name,
                    kind: AssociatedTypeConstraintKind::Bounds(super::predicate::Predicates::new(bounds, self.context)),
                })
            }
            _ => None,
        })
    }

    /// Returns the number of associated-type bounds.
    pub fn len(self) -> usize {
        self.arguments
            .iter()
            .filter(|argument| {
                matches!(
                    argument,
                    GenericExpressionArgument::AssociatedType { .. }
                        | GenericExpressionArgument::AssociatedTypeBound { .. }
                )
            })
            .count()
    }

    /// Returns whether this trait expression has no associated-type bounds.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}

/// One associated-type bound retained from a trait expression.
#[derive(Clone, Copy)]
pub struct AssociatedTypeConstraintDescriptor<'ctx> {
    name: &'static str,
    kind: AssociatedTypeConstraintKind<'ctx>,
}

/// The source form of one associated-type constraint.
#[derive(Clone, Copy)]
pub enum AssociatedTypeConstraintKind<'ctx> {
    /// Assigns one type expression to the associated type, as in `Item = u8`.
    Equality(TypeExpression<'ctx>),
    /// Records one or more predicates on the associated type, as in `Item:
    /// Copy`.
    Bounds(super::predicate::Predicates<'ctx>),
}

impl<'ctx> AssociatedTypeConstraintDescriptor<'ctx> {
    /// Returns the associated-type name.
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns the source form and value or predicates for this constraint.
    pub fn kind(self) -> AssociatedTypeConstraintKind<'ctx> {
        self.kind
    }
}

/// Facts retained for a function-pointer expression.
#[derive(Clone, Copy)]
pub struct FunctionPointerExpression<'ctx> {
    data: &'static FunctionPointerData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> FunctionPointerExpression<'ctx> {
    /// Returns the context used by argument and result expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the function pointer's explicit higher-ranked lifetimes.
    pub const fn lifetimes(self) -> &'static [LifetimeExpression] {
        self.data.lifetimes
    }

    /// Returns parameter types in source order.
    pub fn arguments(self) -> Box<[TypeExpression<'ctx>]> {
        self.data
            .arguments
            .iter()
            .map(|argument| TypeExpression::new(argument, self.context))
            .collect()
    }
    /// Returns the explicit return type, or `None` when the source omits it.
    pub fn result(self) -> Option<TypeExpression<'ctx>> {
        self.data.result.map(|result| TypeExpression::new(result, self.context))
    }
    /// Returns whether the function pointer is declared `unsafe`.
    pub const fn is_unsafe(self) -> bool {
        self.data.unsafe_
    }
    /// Returns the ABI string, or `None` for Rust's default ABI.
    pub const fn abi(self) -> Option<&'static str> {
        self.data.abi
    }
    /// Returns whether the function pointer accepts variadic arguments.
    pub const fn is_variadic(self) -> bool {
        self.data.variadic
    }
}

/// Facts retained for a trait-object expression.
#[derive(Clone, Copy)]
pub struct TraitObjectExpression<'ctx> {
    data: &'static TraitObjectData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitObjectExpression<'ctx> {
    /// Returns the context used by the trait descriptors.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the explicit object lifetime, if one was written.
    pub const fn lifetime(self) -> Option<LifetimeExpression> {
        self.data.lifetime
    }
    /// Returns trait bounds in source order.
    pub fn traits(self) -> Box<[TraitExpression<'ctx>]> {
        self.data
            .traits
            .iter()
            .map(|data| TraitExpression {
                data,
                context: self.context,
            })
            .collect()
    }
}

/// Bounds retained when an opaque source type cannot be navigated.
#[derive(Clone, Copy)]
pub struct OpaqueTypeExpression<'ctx> {
    data: &'static OpaqueTypeData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> OpaqueTypeExpression<'ctx> {
    /// Returns the predicates retained from the opaque type's bounds.
    pub fn bounds(self) -> super::predicate::Predicates<'ctx> {
        super::predicate::Predicates::new(self.data.bounds, self.context)
    }
    /// Returns the context used by nested predicate expressions.
    pub fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
}

struct ExpressionDisplay<'ctx> {
    expression: TypeExpression<'ctx>,
}

impl fmt::Display for ExpressionDisplay<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expression.data {
            ExpressionData::Named { path, arguments, .. } => {
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
                    formatter.write_str(trait_.path)?;
                    write_generic_arguments(formatter, trait_.arguments, self.expression.context)?;
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
                if let Some(lifetime) = lifetime.filter(|value| *value != LifetimeExpression::Elided) {
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
            ExpressionData::TraitObject(object) => write_trait_object(formatter, object),
            ExpressionData::Opaque(opaque) => write_opaque(formatter, opaque),
            ExpressionData::Never => formatter.write_str("!"),
        }
    }
}

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
            GenericExpressionArgument::Type(ty) => write!(formatter, "{}", TypeExpression::new(ty, context).display())?,
            GenericExpressionArgument::Lifetime(lifetime) => write_lifetime(formatter, *lifetime)?,
            GenericExpressionArgument::Const(value) => write_const(formatter, *value)?,
            GenericExpressionArgument::AssociatedType { name, value } => {
                write!(formatter, "{name} = {}", TypeExpression::new(value, context).display())?
            }
            GenericExpressionArgument::AssociatedTypeBound { name, .. } => write!(formatter, "{name}: …")?,
        }
    }
    formatter.write_str(">")
}

fn write_lifetime(formatter: &mut fmt::Formatter<'_>, lifetime: LifetimeExpression) -> fmt::Result {
    match lifetime {
        LifetimeExpression::Static => formatter.write_str("'static"),
        LifetimeExpression::Named(name) => write!(formatter, "'{name}"),
        LifetimeExpression::Elided => Ok(()),
        LifetimeExpression::Placeholder => formatter.write_str("'_"),
    }
}

fn write_const(formatter: &mut fmt::Formatter<'_>, expression: ConstExpression) -> fmt::Result {
    match expression {
        ConstExpression::SignedInteger(value) => write!(formatter, "{value}"),
        ConstExpression::UnsignedInteger(value) => write!(formatter, "{value}"),
        ConstExpression::Boolean(value) => write!(formatter, "{value}"),
        ConstExpression::Character(value) => write!(formatter, "{value:?}"),
        ConstExpression::Parameter(name) | ConstExpression::Path(name) => formatter.write_str(name),
    }
}

fn write_function_pointer(
    formatter: &mut fmt::Formatter<'_>,
    function: &FunctionPointerData,
    context: &ReflectionContext,
) -> fmt::Result {
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
        write!(formatter, "{}", TypeExpression::new(argument, context).display())?;
    }
    if function.variadic {
        if !function.arguments.is_empty() {
            formatter.write_str(", ")?;
        }
        formatter.write_str("...")?;
    }
    formatter.write_str(")")?;
    if let Some(result) = function.result {
        write!(formatter, " -> {}", TypeExpression::new(result, context).display())?;
    }
    Ok(())
}

fn write_trait_object(formatter: &mut fmt::Formatter<'_>, object: &TraitObjectData) -> fmt::Result {
    formatter.write_str("dyn ")?;
    for (index, trait_) in object.traits.iter().enumerate() {
        if index > 0 {
            formatter.write_str(" + ")?;
        }
        formatter.write_str(trait_.path)?;
    }
    if let Some(lifetime) = object.lifetime {
        if !object.traits.is_empty() {
            formatter.write_str(" + ")?;
        }
        write_lifetime(formatter, lifetime)?;
    }
    Ok(())
}

fn write_opaque(formatter: &mut fmt::Formatter<'_>, opaque: &OpaqueTypeData) -> fmt::Result {
    formatter.write_str("impl ")?;
    for (index, bound) in opaque.bounds.iter().enumerate() {
        if index > 0 {
            formatter.write_str(" + ")?;
        }
        match bound {
            super::data::PredicateData::TraitBound { trait_, .. } => formatter.write_str(trait_.path)?,
            super::data::PredicateData::LifetimeOutlives { lifetime, .. } => write_lifetime(formatter, *lifetime)?,
            _ => formatter.write_str("_")?,
        }
    }
    Ok(())
}
