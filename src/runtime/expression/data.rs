use crate::runtime::descriptor::generic::GenericParameterMetadata;

/// A lifetime retained in the source type expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifetimeExpression {
    Static,
    Named(&'static str),
    Elided,
    Placeholder,
}

/// A structural const expression; it never evaluates Rust const code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstExpression {
    SignedInteger(i128),
    UnsignedInteger(u128),
    Boolean(bool),
    Character(char),
    Parameter(&'static str),
    Path(&'static str),
}

/// A generic argument embedded in a source type path.
#[derive(Clone, Copy)]
pub enum GenericExpressionArgument {
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
pub enum PredicateData {
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
pub enum TraitBoundModifier {
    None,
    Maybe,
}

/// A trait path and its source-level arguments.
#[derive(Clone, Copy)]
pub struct TraitExpressionData {
    pub path: &'static str,
    pub arguments: &'static [GenericExpressionArgument],
    pub lifetimes: &'static [LifetimeExpression],
}

impl TraitExpressionData {
    /// Creates static source facts for a trait path and its generic arguments.
    pub const fn new(
        path: &'static str,
        arguments: &'static [GenericExpressionArgument],
        lifetimes: &'static [LifetimeExpression],
    ) -> Self {
        Self {
            path,
            arguments,
            lifetimes,
        }
    }
}

/// A source-level type expression independent of any runtime context.
#[derive(Clone, Copy)]
pub enum ExpressionData {
    Named {
        path: &'static str,
        definition: Option<&'static super::super::descriptor::generic::TypeDefinitionMetadata>,
        arguments: &'static [GenericExpressionArgument],
    },
    Parameter(&'static GenericParameterMetadata),
    /// A source generic parameter referenced by name when no definition link is available.
    ParameterName(&'static str),
    SelfType,
    AssociatedType {
        self_type: &'static ExpressionData,
        trait_: Option<&'static TraitExpressionData>,
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
pub struct FunctionPointerData {
    pub lifetimes: &'static [LifetimeExpression],
    pub arguments: &'static [&'static ExpressionData],
    pub result: Option<&'static ExpressionData>,
    pub unsafe_: bool,
    pub abi: Option<&'static str>,
    pub variadic: bool,
}

impl FunctionPointerData {
    /// Creates static source facts for a function pointer type.
    pub const fn new(
        arguments: &'static [&'static ExpressionData],
        result: Option<&'static ExpressionData>,
        unsafe_: bool,
        abi: Option<&'static str>,
        variadic: bool,
        lifetimes: &'static [LifetimeExpression],
    ) -> Self {
        Self {
            lifetimes,
            arguments,
            result,
            unsafe_,
            abi,
            variadic,
        }
    }
}

/// Trait bounds retained for a trait-object expression.
#[derive(Clone, Copy)]
pub struct TraitObjectData {
    pub traits: &'static [TraitExpressionData],
    pub lifetime: Option<LifetimeExpression>,
}

impl TraitObjectData {
    /// Creates static source facts for a trait-object type.
    pub const fn new(
        traits: &'static [TraitExpressionData],
        lifetime: Option<LifetimeExpression>,
    ) -> Self {
        Self { traits, lifetime }
    }
}

/// Opaque source type facts; this does not claim the type can be navigated.
#[derive(Clone, Copy)]
pub struct OpaqueTypeData {
    pub bounds: &'static [PredicateData],
}

impl OpaqueTypeData {
    /// Creates static source facts for an opaque type's bounds.
    pub const fn new(bounds: &'static [PredicateData]) -> Self {
        Self { bounds }
    }
}
