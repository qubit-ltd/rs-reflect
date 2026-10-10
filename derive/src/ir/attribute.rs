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

use crate::ir::PathIr;
use crate::ir::TypeIr;

/// Identifies a supported reflection helper key.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum HelperName {
    /// Rename the reflected query name.
    Rename,
    /// Hide type details behind an opaque descriptor.
    Opaque,
    /// Declare type capabilities.
    Capabilities,
    /// Exclude a field, variant, or method from reflection.
    Skip,
    /// Disable mutation through a field descriptor.
    ReadOnly,
    /// Disable construction through a field or variant.
    NoConstruct,
    /// Supply a default value or path.
    Default,
    /// Disable dynamic method invocation.
    NoInvoke,
    /// Enable panic-catching invocation.
    CatchUnwind,
    /// Require thread-safe reflection behavior.
    ThreadSafe,
    /// Select a concrete generic specialization.
    Specialize,
    /// Declare an external trait identity on an impl.
    ExternalTraitId,
    /// Map a trait supertrait to an external identity.
    ExternalTrait,
    /// Declare a reflected supertrait relationship.
    Supertrait,
    /// Assert dyn compatibility and inherited bindings.
    DynCompatible,
    /// Select the runtime crate facade.
    RuntimeCrate,
    /// Select a v2 type definition provider.
    DefinitionProviderV2,
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
            Self::Rename => "rename",
            Self::Opaque => "opaque",
            Self::Capabilities => "capabilities",
            Self::Skip => "skip",
            Self::ReadOnly => "read_only",
            Self::NoConstruct => "no_construct",
            Self::Default => "default",
            Self::NoInvoke => "no_invoke",
            Self::CatchUnwind => "catch_unwind",
            Self::ThreadSafe => "thread_safe",
            Self::Specialize => "specialize",
            Self::ExternalTraitId => "external_trait_id",
            Self::ExternalTrait => "external_trait",
            Self::Supertrait => "supertrait",
            Self::DynCompatible => "dyn_compatible",
            Self::RuntimeCrate => "crate",
            Self::DefinitionProviderV2 => "definition_provider_v2",
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
        HELPER_RULES
            .iter()
            .find(|rule| rule.source_name == value)
            .map(|rule| rule.name)
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

const TYPE: u16 = HelperTarget::Type.bit();
const FIELD: u16 = HelperTarget::Field.bit();
const VARIANT: u16 = HelperTarget::Variant.bit();
const METHOD: u16 = HelperTarget::Method.bit();
const IMPL: u16 = HelperTarget::Impl.bit();
const TRAIT: u16 = HelperTarget::Trait.bit();

/// The single legality table used by both parsing diagnostics and validation.
const HELPER_RULES: &[HelperRule] = &[
    HelperRule {
        name: HelperName::DefinitionProviderV2,
        source_name: "definition_provider_v2",
        targets: TargetSet::new(TYPE | IMPL),
    },
    HelperRule {
        name: HelperName::Rename,
        source_name: "rename",
        targets: TargetSet::new(TYPE | FIELD | VARIANT | METHOD | TRAIT),
    },
    HelperRule {
        name: HelperName::Opaque,
        source_name: "opaque",
        targets: TargetSet::new(TYPE | FIELD),
    },
    HelperRule {
        name: HelperName::Capabilities,
        source_name: "capabilities",
        targets: TargetSet::new(TYPE),
    },
    HelperRule {
        name: HelperName::Skip,
        source_name: "skip",
        targets: TargetSet::new(FIELD | VARIANT | METHOD),
    },
    HelperRule {
        name: HelperName::ReadOnly,
        source_name: "read_only",
        targets: TargetSet::new(FIELD),
    },
    HelperRule {
        name: HelperName::NoConstruct,
        source_name: "no_construct",
        targets: TargetSet::new(FIELD | VARIANT),
    },
    HelperRule {
        name: HelperName::Default,
        source_name: "default",
        targets: TargetSet::new(FIELD),
    },
    HelperRule {
        name: HelperName::NoInvoke,
        source_name: "no_invoke",
        targets: TargetSet::new(METHOD),
    },
    HelperRule {
        name: HelperName::CatchUnwind,
        source_name: "catch_unwind",
        targets: TargetSet::new(METHOD),
    },
    HelperRule {
        name: HelperName::ThreadSafe,
        source_name: "thread_safe",
        targets: TargetSet::new(TYPE | METHOD),
    },
    HelperRule {
        name: HelperName::Specialize,
        source_name: "specialize",
        targets: TargetSet::new(METHOD | IMPL),
    },
    HelperRule {
        name: HelperName::ExternalTraitId,
        source_name: "external_trait_id",
        targets: TargetSet::new(IMPL),
    },
    HelperRule {
        name: HelperName::ExternalTrait,
        source_name: "external_trait",
        targets: TargetSet::new(TRAIT),
    },
    HelperRule {
        name: HelperName::Supertrait,
        source_name: "supertrait",
        targets: TargetSet::new(TRAIT),
    },
    HelperRule {
        name: HelperName::DynCompatible,
        source_name: "dyn_compatible",
        targets: TargetSet::new(TRAIT),
    },
    HelperRule {
        name: HelperName::RuntimeCrate,
        source_name: "crate",
        targets: TargetSet::new(TYPE | TRAIT | IMPL),
    },
];

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
    /// A query-name override.
    Rename(String),
    /// One or more Rust paths.
    Paths(Vec<PathIr>),
    /// Optional path-based default value.
    DefaultPath(Option<PathIr>),
    /// Named concrete generic specialization.
    Specialization(SpecializationIr),
    /// Stable external trait identity.
    ExternalTraitId(String),
    /// External trait path and stable identity mapping.
    ExternalTrait(ExternalTraitIr),
    /// Explicit inherited associated type bindings for dyn compatibility.
    DynCompatible(Vec<PathIr>),
    /// Runtime crate facade path.
    RuntimeCrate(PathIr),
    /// Explicit v2 definition-provider identifier.
    DefinitionProviderV2(syn::Ident),
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

/// An external trait path mapped to a stable identity literal.
#[derive(Clone, Debug)]
pub(crate) struct ExternalTraitIr {
    /// The diagnostic Rust path of the external trait.
    pub(crate) path: PathIr,
    /// The stable external identity literal.
    pub(crate) id: String,
    /// The identity literal span used for format diagnostics.
    pub(crate) id_span: Span,
    /// The complete mapping span used for conflict diagnostics.
    pub(crate) span: Span,
}

impl HelperAttributeIr {
    /// Returns a borrowed rename value when this is a `rename` helper.
    ///
    /// # Returns
    ///
    /// Returns the rename string, or `None` for other helper values.
    #[inline]
    #[must_use]
    pub(crate) fn rename(&self) -> Option<&str> {
        match &self.value {
            HelperValueIr::Rename(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the specialization carried by this helper, if any.
    ///
    /// # Returns
    ///
    /// Returns the specialization data, or `None` for other helper values.
    #[inline]
    #[must_use]
    pub(crate) fn specialization(&self) -> Option<&SpecializationIr> {
        match &self.value {
            HelperValueIr::Specialization(value) => Some(value),
            _ => None,
        }
    }
}
