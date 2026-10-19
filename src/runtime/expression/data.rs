use crate::runtime::descriptor::generic::GenericParameterMetadata;

/// A lifetime retained in the source type expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LifetimeExpression {
    Static,
    Named(&'static str),
    Elided,
    Placeholder,
}

/// A structural const expression; it never evaluates Rust const code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConstExpression {
    SignedInteger(i128),
    UnsignedInteger(u128),
    Boolean(bool),
    Character(char),
    Parameter(&'static str),
    Path(&'static str),
}

/// A generic argument embedded in a source type path.
#[derive(Clone, Copy)]
pub(crate) enum GenericExpressionArgument {
    Type(&'static ExpressionData),
    Lifetime(LifetimeExpression),
    Const(ConstExpression),
    AssociatedType {
        name: &'static str,
        value: &'static ExpressionData,
    },
    AssociatedTypeBound {
        name: &'static str,
        bounds: &'static [PredicateData],
    },
}

/// A structural predicate copied from a declaration or where clause.
#[derive(Clone, Copy)]
pub(crate) enum PredicateData {
    TraitBound {
        ty: &'static ExpressionData,
        trait_: &'static TraitExpressionData,
        modifier: TraitBoundModifier,
    },
    LifetimeOutlives {
        lifetime: LifetimeExpression,
        outlives: LifetimeExpression,
    },
    TypeOutlives {
        ty: &'static ExpressionData,
        lifetime: LifetimeExpression,
    },
    TypeEquality {
        left: &'static ExpressionData,
        right: &'static ExpressionData,
    },
    ConstEvaluatable(ConstExpression),
}

/// The modifier syntax attached to a Rust trait bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TraitBoundModifier {
    None,
    Maybe,
}

/// A trait path and its source-level arguments.
#[derive(Clone, Copy)]
pub(crate) struct TraitExpressionData {
    pub(crate) path: &'static str,
    pub(crate) arguments: &'static [GenericExpressionArgument],
}

/// A source-level type expression independent of any runtime context.
#[derive(Clone, Copy)]
pub(crate) enum ExpressionData {
    Named {
        path: &'static str,
        definition: Option<&'static super::super::descriptor::generic::TypeDefinitionMetadata>,
        arguments: &'static [GenericExpressionArgument],
    },
    Parameter(&'static GenericParameterMetadata),
    SelfType,
    AssociatedType {
        self_type: &'static ExpressionData,
        trait_: &'static TraitExpressionData,
        name: &'static str,
    },
    Reference {
        lifetime: Option<LifetimeExpression>,
        mutable: bool,
        target: &'static ExpressionData,
    },
    RawPointer {
        mutable: bool,
        target: &'static ExpressionData,
    },
    Slice(&'static ExpressionData),
    Array {
        element: &'static ExpressionData,
        length: ConstExpression,
    },
    Tuple(&'static [&'static ExpressionData]),
    FunctionPointer(&'static FunctionPointerData),
    TraitObject(&'static TraitObjectData),
    Opaque(&'static OpaqueTypeData),
    Never,
}

/// Function pointer facts retained from source syntax.
#[derive(Clone, Copy)]
pub(crate) struct FunctionPointerData {
    pub(crate) arguments: &'static [&'static ExpressionData],
    pub(crate) result: Option<&'static ExpressionData>,
    pub(crate) unsafe_: bool,
    pub(crate) abi: Option<&'static str>,
    pub(crate) variadic: bool,
}

/// Trait bounds retained for a trait-object expression.
#[derive(Clone, Copy)]
pub(crate) struct TraitObjectData {
    pub(crate) traits: &'static [TraitExpressionData],
    pub(crate) lifetime: Option<LifetimeExpression>,
}

/// Opaque source type facts; this does not claim the type can be navigated.
#[derive(Clone, Copy)]
pub(crate) struct OpaqueTypeData {
    pub(crate) bounds: &'static [PredicateData],
}
