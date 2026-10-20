// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Helper-attribute IR and the shared helper target matrix.

use proc_macro2::Span;
use proc_macro2::TokenStream;

use crate::ir::TypeIr;

/// Identifies a supported reflection helper key.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum HelperName {
    /// Exclude a field, variant, or method from reflection.
    Skip,
}

impl HelperName {
    /// Returns the source spelling of this helper key.
    ///
    /// # Returns
    ///
    /// Returns the canonical attribute key spelling.
    #[inline]
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Skip => "skip",
        }
    }

    /// Resolves a source helper key, returning `None` for unknown keys.
    ///
    /// # Parameters
    ///
    /// - `value`: Source key spelling to resolve.
    ///
    /// # Returns
    ///
    /// Returns the matching helper name, or `None` when unrecognized.
    #[must_use]
    pub(crate) fn from_str(value: &str) -> Option<Self> {
        (value == "skip").then_some(Self::Skip)
    }

    /// Returns whether this helper is legal on `target`.
    ///
    /// # Parameters
    ///
    /// - `target`: Declaration element carrying the helper.
    ///
    /// # Returns
    ///
    /// Returns whether the shared legality matrix permits the placement.
    #[must_use]
    pub(crate) fn supports(self, target: HelperTarget) -> bool {
        HELPER_RULES
            .iter()
            .find(|rule| rule.name == self)
            .is_some_and(|rule| rule.targets.contains(target))
    }
}

/// Identifies the declaration element carrying a helper attribute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HelperTarget {
    /// Type or trait object declaration.
    Type,
    /// Struct or union field.
    Field,
    /// Enum variant.
    Variant,
    /// Method declaration.
    Method,
    /// Implementation block.
    Impl,
    /// Trait declaration.
    Trait,
    /// Associated type or constant.
    AssociatedItem,
}

impl HelperTarget {
    /// Returns a diagnostic label for this helper target.
    ///
    /// # Returns
    ///
    /// Returns the human-readable target label.
    #[inline]
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Field => "field",
            Self::Variant => "variant",
            Self::Method => "method",
            Self::Impl => "impl block",
            Self::Trait => "trait",
            Self::AssociatedItem => "associated item",
        }
    }

    /// Returns the bit assigned to this target in the static target matrix.
    ///
    /// # Returns
    ///
    /// Returns the target's unique bit mask.
    #[inline]
    #[must_use]
    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}

/// A compact set of helper targets used by the shared matrix.
#[derive(Clone, Copy)]
struct TargetSet(
    /// Bit set of supported `HelperTarget` values.
    u16,
);

impl TargetSet {
    /// Creates a target set from its bit representation.
    ///
    /// # Parameters
    ///
    /// - `bits`: Combined target bit masks.
    ///
    /// # Returns
    ///
    /// Returns the compact target set.
    #[inline]
    #[must_use]
    const fn new(bits: u16) -> Self {
        Self(bits)
    }

    /// Returns whether `target` belongs to this set.
    ///
    /// # Parameters
    ///
    /// - `target`: Target to look up.
    ///
    /// # Returns
    ///
    /// Returns whether its bit is set.
    #[inline]
    #[must_use]
    const fn contains(self, target: HelperTarget) -> bool {
        let Self(bits) = self;
        bits & target.bit() != 0
    }
}

/// Describes one supported helper and all targets that accept it.
struct HelperRule {
    /// Typed helper key.
    name: HelperName,
    /// Canonical source spelling accepted by the parser.
    source_name: &'static str,
    /// Legal declaration targets.
    targets: TargetSet,
}

/// The single legality table used by both parsing diagnostics and validation.
const HELPER_RULES: &[HelperRule] = &[HelperRule {
    name: HelperName::Skip,
    source_name: "skip",
    targets: TargetSet::new(HelperTarget::Field.bit() | HelperTarget::Variant.bit() | HelperTarget::Method.bit()),
}];

/// One parsed helper occurrence, retained separately until duplicate
/// validation.
#[derive(Clone, Debug)]
pub(crate) struct HelperAttributeIr {
    /// The normalized helper key.
    pub(crate) name: HelperName,
    /// The parsed value carried by this occurrence.
    pub(crate) value: HelperValueIr,
    /// The declaration element on which the helper appeared.
    pub(crate) target: HelperTarget,
    /// The source span used for helper diagnostics.
    pub(crate) span: Span,
    /// The helper value span, or the key span for a flag.
    pub(crate) value_span: Span,
}

/// Carries the syntax-specific value of a helper occurrence.
#[derive(Clone, Debug)]
pub(crate) enum HelperValueIr {
    /// A flag helper with no explicit value.
    Flag,
}

/// A named concrete specialization before type/const kind resolution.
#[derive(Clone, Debug)]
pub(crate) struct SpecializationIr {
    /// Named concrete arguments in source order.
    pub(crate) bindings: Vec<SpecializationBindingIr>,
    /// The span of the complete specialization list.
    pub(crate) span: Span,
}

/// One named specialization argument.
#[derive(Clone, Debug)]
pub(crate) struct SpecializationBindingIr {
    /// The generic parameter name supplied by the user.
    pub(crate) name: String,
    /// The concrete type or const expression tokens.
    pub(crate) value: SpecializationValueIr,
    /// The parameter-name span used for diagnostics.
    pub(crate) span: Span,
    /// The concrete value span used for kind diagnostics.
    pub(crate) value_span: Span,
}

/// A specialization RHS parsed exactly once at the parser boundary.
#[derive(Clone, Debug)]
pub(crate) enum SpecializationValueIr {
    /// Concrete Rust type argument.
    Type(TypeIr),
    /// Const expression tokens.
    Const(TokenStream),
    /// Path syntax retained until generic parameter kind is known.
    AmbiguousPath(TokenStream),
}

impl HelperAttributeIr {}
