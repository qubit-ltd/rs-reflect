use std::fmt;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::generic::{GenericParameterDescriptor, TypeDefinitionDescriptor};

use super::data::{
    ConstExpression, ExpressionData, FunctionPointerData, GenericExpressionArgument,
    LifetimeExpression, OpaqueTypeData, TraitExpressionData, TraitObjectData,
};

/// A read-only, context-bound view of source-level Rust type syntax.
#[derive(Clone, Copy)]
pub(crate) struct TypeExpression<'ctx> {
    data: &'static ExpressionData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TypeExpression<'ctx> {
    /// Binds static expression metadata to the supplied context.
    pub(crate) const fn new(
        data: &'static ExpressionData,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { data, context }
    }

    /// Returns the context used by nested descriptors.
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    /// Returns the structural kind without resolving generic parameters.
    pub(crate) fn kind(self) -> TypeExpressionKind<'ctx> {
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
            ExpressionData::Parameter(parameter) => TypeExpressionKind::Parameter(
                GenericParameterDescriptor::new(parameter, self.context),
            ),
            ExpressionData::SelfType => TypeExpressionKind::SelfType,
            ExpressionData::AssociatedType {
                self_type,
                trait_,
                name,
            } => TypeExpressionKind::AssociatedType(AssociatedTypeExpression {
                self_type: Self::new(self_type, self.context),
                trait_: TraitExpression {
                    data: trait_,
                    context: self.context,
                },
                name,
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
            ExpressionData::Slice(element) => {
                TypeExpressionKind::Slice(Self::new(element, self.context))
            }
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
            ExpressionData::FunctionPointer(data) => {
                TypeExpressionKind::FunctionPointer(FunctionPointerExpression {
                    data,
                    context: self.context,
                })
            }
            ExpressionData::TraitObject(data) => {
                TypeExpressionKind::TraitObject(TraitObjectExpression {
                    data,
                    context: self.context,
                })
            }
            ExpressionData::Opaque(data) => TypeExpressionKind::Opaque(OpaqueTypeExpression {
                data,
                context: self.context,
            }),
            ExpressionData::Never => TypeExpressionKind::Never,
        }
    }

    /// Formats source facts for logs and diagnostics only.
    pub(crate) fn display(self) -> impl fmt::Display + 'ctx {
        ExpressionDisplay { expression: self }
    }
}

/// The complete structural shape of a source-level Rust type expression.
pub(crate) enum TypeExpressionKind<'ctx> {
    Named(NamedTypeExpression<'ctx>),
    Parameter(GenericParameterDescriptor<'ctx>),
    SelfType,
    AssociatedType(AssociatedTypeExpression<'ctx>),
    Reference {
        lifetime: Option<LifetimeExpression>,
        mutable: bool,
        target: TypeExpression<'ctx>,
    },
    RawPointer {
        mutable: bool,
        target: TypeExpression<'ctx>,
    },
    Slice(TypeExpression<'ctx>),
    Array {
        element: TypeExpression<'ctx>,
        length: ConstExpression,
    },
    Tuple(Box<[TypeExpression<'ctx>]>),
    FunctionPointer(FunctionPointerExpression<'ctx>),
    TraitObject(TraitObjectExpression<'ctx>),
    Opaque(OpaqueTypeExpression<'ctx>),
    Never,
}

/// A named path that may resolve to a known generic definition.
#[derive(Clone, Copy)]
pub(crate) struct NamedTypeExpression<'ctx> {
    path: &'static str,
    definition: Option<&'static super::super::descriptor::generic::TypeDefinitionMetadata>,
    arguments: &'static [GenericExpressionArgument],
    context: &'ctx ReflectionContext,
}

impl<'ctx> NamedTypeExpression<'ctx> {
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
    /// Returns a definition only when metadata explicitly links one.
    pub(crate) fn find_definition(self) -> Option<TypeDefinitionDescriptor<'ctx>> {
        self.definition
            .map(|metadata| TypeDefinitionDescriptor::new(metadata, self.context))
    }

    /// Returns the complete source path, whether reflection knows it or not.
    pub(crate) const fn path(self) -> &'static str {
        self.path
    }

    /// Returns generic arguments in the original source order.
    pub(crate) fn arguments(self) -> Box<[GenericExpressionArgumentView<'ctx>]> {
        self.arguments
            .iter()
            .copied()
            .map(|argument| GenericExpressionArgumentView::new(argument, self.context))
            .collect()
    }
}

/// A projection such as `<T as Iterator>::Item`.
#[derive(Clone, Copy)]
pub(crate) struct AssociatedTypeExpression<'ctx> {
    self_type: TypeExpression<'ctx>,
    trait_: TraitExpression<'ctx>,
    name: &'static str,
}

impl<'ctx> AssociatedTypeExpression<'ctx> {
    pub(crate) const fn self_type(self) -> TypeExpression<'ctx> {
        self.self_type
    }
    pub(crate) const fn trait_(self) -> TraitExpression<'ctx> {
        self.trait_
    }
    pub(crate) const fn name(self) -> &'static str {
        self.name
    }
}

/// A trait path attached to a projection or trait object.
#[derive(Clone, Copy)]
pub(crate) struct TraitExpression<'ctx> {
    data: &'static TraitExpressionData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitExpression<'ctx> {
    pub(crate) const fn new(
        data: &'static TraitExpressionData,
        context: &'ctx ReflectionContext,
    ) -> Self {
        Self { data, context }
    }

    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }

    pub(crate) const fn path(self) -> &'static str {
        self.data.path
    }
    pub(crate) fn arguments(self) -> Box<[GenericExpressionArgumentView<'ctx>]> {
        self.data
            .arguments
            .iter()
            .copied()
            .map(|argument| GenericExpressionArgumentView::new(argument, self.context))
            .collect()
    }
}

/// A context-bound view of one named, lifetime, const, or associated argument.
#[derive(Clone, Copy)]
pub(crate) struct GenericExpressionArgumentView<'ctx> {
    data: GenericExpressionArgument,
    context: &'ctx ReflectionContext,
}

impl<'ctx> GenericExpressionArgumentView<'ctx> {
    const fn new(data: GenericExpressionArgument, context: &'ctx ReflectionContext) -> Self {
        Self { data, context }
    }
    pub(crate) fn kind(self) -> GenericExpressionArgumentKind<'ctx> {
        match self.data {
            GenericExpressionArgument::Type(ty) => {
                GenericExpressionArgumentKind::Type(TypeExpression::new(ty, self.context))
            }
            GenericExpressionArgument::Lifetime(value) => {
                GenericExpressionArgumentKind::Lifetime(value)
            }
            GenericExpressionArgument::Const(value) => GenericExpressionArgumentKind::Const(value),
            GenericExpressionArgument::AssociatedType { name, value } => {
                GenericExpressionArgumentKind::AssociatedType {
                    name,
                    value: TypeExpression::new(value, self.context),
                }
            }
            GenericExpressionArgument::AssociatedTypeBound { name, bounds } => {
                GenericExpressionArgumentKind::AssociatedTypeBound { name, bounds }
            }
        }
    }
}

pub(crate) enum GenericExpressionArgumentKind<'ctx> {
    Type(TypeExpression<'ctx>),
    Lifetime(LifetimeExpression),
    Const(ConstExpression),
    AssociatedType {
        name: &'static str,
        value: TypeExpression<'ctx>,
    },
    AssociatedTypeBound {
        name: &'static str,
        bounds: &'static [super::data::PredicateData],
    },
}

/// Facts retained for a function-pointer expression.
#[derive(Clone, Copy)]
pub(crate) struct FunctionPointerExpression<'ctx> {
    data: &'static FunctionPointerData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> FunctionPointerExpression<'ctx> {
    pub(crate) fn arguments(self) -> Box<[TypeExpression<'ctx>]> {
        self.data
            .arguments
            .iter()
            .map(|argument| TypeExpression::new(argument, self.context))
            .collect()
    }
    pub(crate) fn result(self) -> Option<TypeExpression<'ctx>> {
        self.data
            .result
            .map(|result| TypeExpression::new(result, self.context))
    }
    pub(crate) const fn is_unsafe(self) -> bool {
        self.data.unsafe_
    }
    pub(crate) const fn abi(self) -> Option<&'static str> {
        self.data.abi
    }
    pub(crate) const fn is_variadic(self) -> bool {
        self.data.variadic
    }
}

/// Facts retained for a trait-object expression.
#[derive(Clone, Copy)]
pub(crate) struct TraitObjectExpression<'ctx> {
    data: &'static TraitObjectData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> TraitObjectExpression<'ctx> {
    pub(crate) const fn lifetime(self) -> Option<LifetimeExpression> {
        self.data.lifetime
    }
    pub(crate) fn traits(self) -> Box<[TraitExpression<'ctx>]> {
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
pub(crate) struct OpaqueTypeExpression<'ctx> {
    data: &'static OpaqueTypeData,
    context: &'ctx ReflectionContext,
}

impl<'ctx> OpaqueTypeExpression<'ctx> {
    pub(crate) const fn bounds(self) -> &'static [super::data::PredicateData] {
        self.data.bounds
    }
    pub(crate) const fn context(self) -> &'ctx ReflectionContext {
        self.context
    }
}

struct ExpressionDisplay<'ctx> {
    expression: TypeExpression<'ctx>,
}

impl fmt::Display for ExpressionDisplay<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expression.data {
            ExpressionData::Named {
                path, arguments, ..
            } => {
                formatter.write_str(path)?;
                if !arguments.is_empty() {
                    formatter.write_str("<")?;
                    for (index, argument) in arguments.iter().enumerate() {
                        if index > 0 {
                            formatter.write_str(", ")?;
                        }
                        match argument {
                            GenericExpressionArgument::Type(ty) => write!(
                                formatter,
                                "{}",
                                TypeExpression::new(ty, self.expression.context).display()
                            )?,
                            GenericExpressionArgument::Lifetime(lifetime) => {
                                write_lifetime(formatter, *lifetime)?
                            }
                            GenericExpressionArgument::Const(value) => {
                                write_const(formatter, *value)?
                            }
                            GenericExpressionArgument::AssociatedType { name, value } => write!(
                                formatter,
                                "{name} = {}",
                                TypeExpression::new(value, self.expression.context).display()
                            )?,
                            GenericExpressionArgument::AssociatedTypeBound { name, .. } => {
                                write!(formatter, "{name}: …")?
                            }
                        }
                    }
                    formatter.write_str(">")?;
                }
                Ok(())
            }
            ExpressionData::Parameter(parameter) => formatter.write_str(parameter.name),
            ExpressionData::SelfType => formatter.write_str("Self"),
            ExpressionData::AssociatedType {
                self_type,
                trait_,
                name,
            } => write!(
                formatter,
                "<{} as {}>::{name}",
                TypeExpression::new(self_type, self.expression.context).display(),
                trait_.path
            ),
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
            ExpressionData::TraitObject(object) => write_trait_object(formatter, object),
            ExpressionData::Opaque(opaque) => write_opaque(formatter, opaque),
            ExpressionData::Never => formatter.write_str("!"),
        }
    }
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
            super::data::PredicateData::TraitBound { trait_, .. } => {
                formatter.write_str(trait_.path)?
            }
            super::data::PredicateData::LifetimeOutlives { lifetime, .. } => {
                write_lifetime(formatter, *lifetime)?
            }
            _ => formatter.write_str("_")?,
        }
    }
    Ok(())
}
