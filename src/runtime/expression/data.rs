// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::runtime::descriptor::generic::GenericParameterMetadata;

/// A lifetime retained in the source type expression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifetimeExpression {
    /// The `'static` lifetime.
    Static,
    /// A named lifetime without its leading apostrophe.
    Named(&'static str),
    /// A lifetime omitted in the source and elided by Rust.
    Elided,
    /// The placeholder lifetime `'_`.
    Placeholder,
}

/// A structural const expression; it never evaluates Rust const code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstExpression {
    /// A signed integer value recorded without evaluating source code.
    SignedInteger(i128),
    /// An unsigned integer value recorded without evaluating source code.
    UnsignedInteger(u128),
    /// A boolean const value.
    Boolean(bool),
    /// A character const value.
    Character(char),
    /// A const generic parameter name.
    Parameter(&'static str),
    /// A path retained as a const expression.
    Path(&'static str),
}

impl ConstExpression {
    /// Converts a supported Rust const-generic value to its structural
    /// representation.
    #[doc(hidden)]
    pub fn from_const<T: IntoConstExpression>(value: T) -> Self {
        value.into_const_expression()
    }
}

/// Converts a primitive const-generic value to reflection metadata.
#[doc(hidden)]
pub trait IntoConstExpression {
    /// Converts this supported primitive value to structural const metadata.
    fn into_const_expression(self) -> ConstExpression;
}

macro_rules! signed_const_expression {
    ($($ty:ty),+ $(,)?) => {$(
        impl IntoConstExpression for $ty {
            fn into_const_expression(self) -> ConstExpression {
                ConstExpression::SignedInteger(self as i128)
            }
        }
    )+};
}

macro_rules! unsigned_const_expression {
    ($($ty:ty),+ $(,)?) => {$(
        impl IntoConstExpression for $ty {
            fn into_const_expression(self) -> ConstExpression {
                ConstExpression::UnsignedInteger(self as u128)
            }
        }
    )+};
}

signed_const_expression!(i8, i16, i32, i64, i128, isize);
unsigned_const_expression!(u8, u16, u32, u64, u128, usize);

impl IntoConstExpression for bool {
    fn into_const_expression(self) -> ConstExpression {
        ConstExpression::Boolean(self)
    }
}

impl IntoConstExpression for char {
    fn into_const_expression(self) -> ConstExpression {
        ConstExpression::Character(self)
    }
}

/// A generic argument embedded in a source type path.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum GenericExpressionArgument {
    /// A nested type expression.
    Type(&'static ExpressionData),
    /// A lifetime argument.
    Lifetime(LifetimeExpression),
    /// A const argument.
    Const(ConstExpression),
    /// An associated-type equality.
    AssociatedType {
        /// Associated-type name.
        name: &'static str,
        /// Assigned type expression.
        value: &'static ExpressionData,
    },
    /// An associated-type bound list.
    AssociatedTypeBound {
        /// Associated-type name.
        name: &'static str,
        /// Predicates attached to the associated type.
        bounds: &'static [PredicateData],
    },
}

/// A structural predicate copied from a declaration or where clause.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum PredicateData {
    /// A type's trait bound and optional `?Trait` modifier.
    TraitBound {
        /// Type subject to the bound.
        ty: &'static ExpressionData,
        /// Trait path and arguments required by the bound.
        trait_: &'static TraitExpressionData,
        /// Modifier attached to the bound.
        modifier: TraitBoundModifier,
    },
    /// A lifetime outlives another lifetime.
    LifetimeOutlives {
        /// Lifetime required to outlive `outlives`.
        lifetime: LifetimeExpression,
        /// Lifetime that `lifetime` must outlive.
        outlives: LifetimeExpression,
    },
    /// A type outlives a lifetime.
    TypeOutlives {
        /// Type subject to the requirement.
        ty: &'static ExpressionData,
        /// Lifetime that the type must outlive.
        lifetime: LifetimeExpression,
    },
    /// Two type expressions are constrained to be equal.
    TypeEquality {
        /// Left side of the equality.
        left: &'static ExpressionData,
        /// Right side of the equality.
        right: &'static ExpressionData,
    },
    /// A const expression appears in a const-evaluatable predicate.
    ConstEvaluatable(ConstExpression),
}

/// The modifier syntax attached to a Rust trait bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraitBoundModifier {
    /// No modifier was written.
    None,
    /// The bound uses `?Trait` syntax.
    Maybe,
}

/// A trait path and its source-level arguments.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct TraitExpressionData {
    /// Trait path as written in the source.
    pub path: &'static str,
    /// Type, lifetime, const, and associated-type arguments in source order.
    pub arguments: &'static [GenericExpressionArgument],
    /// Explicit lifetime arguments in source order.
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
#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum ExpressionData {
    /// A named path and its generic arguments.
    Named {
        /// Path preserved from the source.
        path: &'static str,
        /// Linked definition metadata, when known.
        definition: Option<&'static super::super::descriptor::generic::TypeDefinitionMetadata>,
        /// Generic arguments in source order.
        arguments: &'static [GenericExpressionArgument],
    },
    /// A generic parameter linked to its declaration metadata.
    Parameter(&'static GenericParameterMetadata),
    /// A source generic parameter referenced by name when no definition link is
    /// available.
    /// A generic parameter referenced by name without a metadata link.
    ParameterName(&'static str),
    /// The special `Self` type.
    SelfType,
    /// An associated-type projection.
    AssociatedType {
        /// Type from which the associated item is projected.
        self_type: &'static ExpressionData,
        /// Optional trait qualifier.
        trait_: Option<&'static TraitExpressionData>,
        /// Associated-type name.
        name: &'static str,
    },
    /// A reference type.
    Reference {
        /// Explicit lifetime, when present.
        lifetime: Option<LifetimeExpression>,
        /// Whether the reference is mutable.
        mutable: bool,
        /// Referenced type expression.
        target: &'static ExpressionData,
    },
    /// A raw pointer type.
    RawPointer {
        /// Whether the pointed-to type is mutable.
        mutable: bool,
        /// Pointed-to type expression.
        target: &'static ExpressionData,
    },
    /// A slice type and its element expression.
    Slice(&'static ExpressionData),
    /// An array type and its structural length expression.
    Array {
        /// Element type expression.
        element: &'static ExpressionData,
        /// Const length; it is not evaluated by reflection.
        length: ConstExpression,
    },
    /// Tuple element expressions in source order.
    Tuple(&'static [&'static ExpressionData]),
    /// Function pointer signature facts.
    FunctionPointer(&'static FunctionPointerData),
    /// Trait-object bounds and optional object lifetime.
    TraitObject(&'static TraitObjectData),
    /// Bounds of an opaque `impl Trait` type.
    Opaque(&'static OpaqueTypeData),
    /// The never type `!`.
    Never,
}

/// Function pointer facts retained from source syntax.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct FunctionPointerData {
    /// Explicit higher-ranked lifetimes.
    pub lifetimes: &'static [LifetimeExpression],
    /// Parameter type expressions in source order.
    pub arguments: &'static [&'static ExpressionData],
    /// Explicit return type, or `None` when omitted.
    pub result: Option<&'static ExpressionData>,
    /// Whether the function pointer is unsafe.
    pub unsafe_: bool,
    /// ABI name, or `None` for Rust's default ABI.
    pub abi: Option<&'static str>,
    /// Whether the function pointer is variadic.
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
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct TraitObjectData {
    /// Trait bounds in source order.
    pub traits: &'static [TraitExpressionData],
    /// Explicit object lifetime, if present.
    pub lifetime: Option<LifetimeExpression>,
}

impl TraitObjectData {
    /// Creates static source facts for a trait-object type.
    pub const fn new(traits: &'static [TraitExpressionData], lifetime: Option<LifetimeExpression>) -> Self {
        Self { traits, lifetime }
    }
}

/// Opaque source type facts; this does not claim the type can be navigated.
#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct OpaqueTypeData {
    /// Predicates retained from the opaque type bounds.
    pub bounds: &'static [PredicateData],
}

impl OpaqueTypeData {
    /// Creates static source facts for an opaque type's bounds.
    pub const fn new(bounds: &'static [PredicateData]) -> Self {
        Self { bounds }
    }
}

#[cfg(test)]
mod tests {
    use super::ConstExpression;
    use super::ExpressionData;
    use super::FunctionPointerData;
    use super::GenericExpressionArgument;
    use super::GenericParameterMetadata;
    use super::LifetimeExpression;
    use super::OpaqueTypeData;
    use super::PredicateData;
    use super::TraitBoundModifier;
    use super::TraitExpressionData;
    use super::TraitObjectData;
    use crate::runtime::context::ReflectionContext;
    use crate::runtime::descriptor::generic::GenericParameterMetadataKind;
    use crate::runtime::expression::predicate::PredicateDescriptor;
    use crate::runtime::expression::predicate::PredicateKind;
    use crate::runtime::expression::type_expression::TraitExpression;
    use crate::runtime::expression::type_expression::TypeExpression;
    use crate::runtime::expression::type_expression::TypeExpressionKind;

    static TYPE_PARAMETER: GenericParameterMetadata = GenericParameterMetadata {
        name: "T",
        index: 0,
        kind: GenericParameterMetadataKind::Type {
            bounds: &[],
            default: None,
        },
    };
    static TYPE: ExpressionData = ExpressionData::Parameter(&TYPE_PARAMETER);
    static U8: ExpressionData = ExpressionData::Named {
        path: "u8",
        definition: None,
        arguments: &[],
    };
    static DISPLAY: TraitExpressionData = TraitExpressionData::new("core::fmt::Display", &[], &[]);
    static PREDICATES: [PredicateData; 2] = [
        PredicateData::TraitBound {
            ty: &TYPE,
            trait_: &DISPLAY,
            modifier: TraitBoundModifier::Maybe,
        },
        PredicateData::ConstEvaluatable(ConstExpression::Boolean(true)),
    ];

    #[test]
    fn primitive_const_conversions_preserve_signedness_and_values() {
        assert_eq!(
            ConstExpression::from_const(i8::MIN),
            ConstExpression::SignedInteger(i8::MIN as i128)
        );
        assert_eq!(
            ConstExpression::from_const(i16::MIN),
            ConstExpression::SignedInteger(i16::MIN as i128)
        );
        assert_eq!(
            ConstExpression::from_const(i32::MIN),
            ConstExpression::SignedInteger(i32::MIN as i128)
        );
        assert_eq!(
            ConstExpression::from_const(i64::MIN),
            ConstExpression::SignedInteger(i64::MIN as i128)
        );
        assert_eq!(
            ConstExpression::from_const(i128::MIN),
            ConstExpression::SignedInteger(i128::MIN)
        );
        assert_eq!(
            ConstExpression::from_const(isize::MIN),
            ConstExpression::SignedInteger(isize::MIN as i128)
        );
        assert_eq!(ConstExpression::from_const(0_u8), ConstExpression::UnsignedInteger(0));
        assert_eq!(
            ConstExpression::from_const(u16::MAX),
            ConstExpression::UnsignedInteger(u16::MAX as u128)
        );
        assert_eq!(
            ConstExpression::from_const(u32::MAX),
            ConstExpression::UnsignedInteger(u32::MAX as u128)
        );
        assert_eq!(
            ConstExpression::from_const(u64::MAX),
            ConstExpression::UnsignedInteger(u64::MAX as u128)
        );
        assert_eq!(
            ConstExpression::from_const(u128::MAX),
            ConstExpression::UnsignedInteger(u128::MAX)
        );
        assert_eq!(
            ConstExpression::from_const(usize::MAX),
            ConstExpression::UnsignedInteger(usize::MAX as u128)
        );
        assert_eq!(ConstExpression::from_const(false), ConstExpression::Boolean(false));
        assert_eq!(ConstExpression::from_const('x'), ConstExpression::Character('x'));
    }

    #[test]
    fn constructors_and_argument_views_preserve_empty_and_ordered_facts() {
        static ARGUMENTS: [GenericExpressionArgument; 5] = [
            GenericExpressionArgument::Type(&TYPE),
            GenericExpressionArgument::Lifetime(LifetimeExpression::Static),
            GenericExpressionArgument::Const(ConstExpression::SignedInteger(-3)),
            GenericExpressionArgument::AssociatedType {
                name: "Item",
                value: &U8,
            },
            GenericExpressionArgument::AssociatedTypeBound {
                name: "Error",
                bounds: &PREDICATES,
            },
        ];
        static TRAIT: TraitExpressionData =
            TraitExpressionData::new("fixture::Trait", &ARGUMENTS, &[LifetimeExpression::Named("a")]);

        let context = ReflectionContext::empty();
        let expression = TraitExpression::new(&TRAIT, &context);
        assert_eq!(expression.path(), "fixture::Trait");
        assert_eq!(expression.lifetimes(), &[LifetimeExpression::Named("a")]);
        assert!(expression.find_definition().is_none());
        assert!(std::ptr::eq(expression.context(), &context));

        let arguments = expression.arguments();
        assert_eq!(arguments.len(), ARGUMENTS.len());
        assert!(!arguments.is_empty());
        assert!(arguments.get(usize::MAX).is_none());
        assert!(std::ptr::eq(arguments.context(), &context));
        assert!(matches!(
            arguments.get(0).expect("type argument exists").kind(),
            crate::runtime::expression::type_expression::GenericExpressionArgumentKind::Type(ty)
                if matches!(ty.kind(), TypeExpressionKind::Parameter(_))
                    && std::ptr::eq(ty.context(), &context)
        ));
        assert!(matches!(
            arguments.get(1).expect("lifetime argument exists").kind(),
            crate::runtime::expression::type_expression::GenericExpressionArgumentKind::Lifetime(
                LifetimeExpression::Static
            )
        ));
        assert!(matches!(
            arguments.get(2).expect("const argument exists").kind(),
            crate::runtime::expression::type_expression::GenericExpressionArgumentKind::Const(
                ConstExpression::SignedInteger(-3)
            )
        ));
        assert!(matches!(
            arguments.get(3).expect("associated equality exists").kind(),
            crate::runtime::expression::type_expression::GenericExpressionArgumentKind::AssociatedType {
                name: "Item",
                value,
            } if value.display().to_string() == "u8"
        ));
        assert!(matches!(
            arguments.get(4).expect("associated bound exists").kind(),
            crate::runtime::expression::type_expression::GenericExpressionArgumentKind::AssociatedTypeBound {
                name: "Error",
                bounds,
            } if bounds.len() == 2
        ));
        assert!(arguments.iter().last().is_some());

        let constraints = expression.associated_type_constraints();
        assert_eq!(constraints.len(), 2);
        let mut constraints = constraints.iter();
        assert_eq!(constraints.next().expect("equality exists").name(), "Item");
        assert_eq!(constraints.next().expect("bound exists").name(), "Error");
        assert!(constraints.next().is_none());

        static NO_ARGUMENTS: TraitExpressionData = TraitExpressionData::new("fixture::NoArguments", &[], &[]);
        let empty = TraitExpression::new(&NO_ARGUMENTS, &context).arguments();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.get(0).is_none());
        assert_eq!(empty.iter().len(), 0);
        assert!(
            TraitExpression::new(&NO_ARGUMENTS, &context)
                .associated_type_constraints()
                .is_empty()
        );
    }

    #[test]
    fn predicate_variants_retain_nested_context_and_values() {
        static PREDICATES: [PredicateData; 5] = [
            PredicateData::TraitBound {
                ty: &TYPE,
                trait_: &DISPLAY,
                modifier: TraitBoundModifier::Maybe,
            },
            PredicateData::LifetimeOutlives {
                lifetime: LifetimeExpression::Named("a"),
                outlives: LifetimeExpression::Static,
            },
            PredicateData::TypeOutlives {
                ty: &U8,
                lifetime: LifetimeExpression::Placeholder,
            },
            PredicateData::TypeEquality {
                left: &TYPE,
                right: &U8,
            },
            PredicateData::ConstEvaluatable(ConstExpression::Path("N + 1")),
        ];
        let context = ReflectionContext::empty();

        let trait_bound = PredicateDescriptor::new(&PREDICATES[0], &context);
        assert!(std::ptr::eq(trait_bound.context(), &context));
        assert!(matches!(
            trait_bound.kind(),
            PredicateKind::TraitBound {
                ty,
                trait_,
                modifier: TraitBoundModifier::Maybe,
            } if ty.display().to_string() == "T"
                && trait_.path() == "core::fmt::Display"
                && std::ptr::eq(trait_.context(), &context)
        ));
        assert!(matches!(
            PredicateDescriptor::new(&PREDICATES[1], &context).kind(),
            PredicateKind::LifetimeOutlives {
                lifetime: LifetimeExpression::Named("a"),
                outlives: LifetimeExpression::Static,
            }
        ));
        assert!(matches!(
            PredicateDescriptor::new(&PREDICATES[2], &context).kind(),
            PredicateKind::TypeOutlives {
                ty,
                lifetime: LifetimeExpression::Placeholder,
            } if ty.display().to_string() == "u8"
        ));
        assert!(matches!(
            PredicateDescriptor::new(&PREDICATES[3], &context).kind(),
            PredicateKind::TypeEquality { left, right }
                if left.display().to_string() == "T" && right.display().to_string() == "u8"
        ));
        assert!(matches!(
            PredicateDescriptor::new(&PREDICATES[4], &context).kind(),
            PredicateKind::ConstEvaluatable(ConstExpression::Path("N + 1"))
        ));
    }

    #[test]
    fn function_trait_object_and_opaque_constructors_format_edge_cases() {
        static FUNCTION_ARGS: [&ExpressionData; 1] = [&U8];
        static FUNCTION: FunctionPointerData = FunctionPointerData::new(
            &FUNCTION_ARGS,
            None,
            false,
            None,
            true,
            &[LifetimeExpression::Named("a")],
        );
        static TRAITS: [TraitExpressionData; 2] = [
            TraitExpressionData::new("core::fmt::Display", &[], &[]),
            TraitExpressionData::new("fixture::Send", &[], &[]),
        ];
        static OBJECT: TraitObjectData = TraitObjectData::new(&TRAITS, None);
        static OBJECT_LIFETIME_ONLY: TraitObjectData = TraitObjectData::new(&[], Some(LifetimeExpression::Placeholder));
        static OPAQUE: OpaqueTypeData = OpaqueTypeData::new(&[
            PredicateData::LifetimeOutlives {
                lifetime: LifetimeExpression::Named("a"),
                outlives: LifetimeExpression::Static,
            },
            PredicateData::TypeOutlives {
                ty: &TYPE,
                lifetime: LifetimeExpression::Named("a"),
            },
        ]);

        let context = ReflectionContext::empty();
        static FUNCTION_EXPRESSION: ExpressionData = ExpressionData::FunctionPointer(&FUNCTION);
        let TypeExpressionKind::FunctionPointer(function) = TypeExpression::new(&FUNCTION_EXPRESSION, &context).kind()
        else {
            panic!("function pointer expression expected");
        };
        assert_eq!(function.lifetimes(), &[LifetimeExpression::Named("a")]);
        assert_eq!(function.arguments().len(), 1);
        assert!(function.result().is_none());
        assert!(!function.is_unsafe());
        assert_eq!(function.abi(), None);
        assert!(function.is_variadic());
        assert_eq!(
            TypeExpression::new(&FUNCTION_EXPRESSION, &context)
                .display()
                .to_string(),
            "fn(u8, ...)"
        );

        static OBJECT_EXPRESSION: ExpressionData = ExpressionData::TraitObject(&OBJECT);
        let TypeExpressionKind::TraitObject(object) = TypeExpression::new(&OBJECT_EXPRESSION, &context).kind() else {
            panic!("trait object expression expected");
        };
        assert_eq!(object.traits().len(), 2);
        assert_eq!(object.traits()[1].path(), "fixture::Send");
        assert_eq!(object.lifetime(), None);
        assert_eq!(
            TypeExpression::new(&OBJECT_EXPRESSION, &context).display().to_string(),
            "dyn core::fmt::Display + fixture::Send"
        );
        static LIFETIME_ONLY: ExpressionData = ExpressionData::TraitObject(&OBJECT_LIFETIME_ONLY);
        assert_eq!(
            TypeExpression::new(&LIFETIME_ONLY, &context).display().to_string(),
            "dyn '_"
        );

        static OPAQUE_EXPRESSION: ExpressionData = ExpressionData::Opaque(&OPAQUE);
        let TypeExpressionKind::Opaque(opaque) = TypeExpression::new(&OPAQUE_EXPRESSION, &context).kind() else {
            panic!("opaque expression expected");
        };
        assert_eq!(opaque.bounds().len(), 2);
        assert!(std::ptr::eq(opaque.context(), &context));
        assert_eq!(
            TypeExpression::new(&OPAQUE_EXPRESSION, &context).display().to_string(),
            "impl 'a + _"
        );
    }
}
