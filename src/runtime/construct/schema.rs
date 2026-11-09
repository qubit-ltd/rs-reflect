// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::fmt;

use crate::runtime::descriptor::Reflect;
use crate::runtime::descriptor::field::Visibility;
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ThreadSafe;
use crate::runtime::value::ValueMode;

/// Input layout used to construct or update a target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionShape {
    /// Fields are supplied by their reflected names.
    Named,
    /// Fields are supplied in declaration order.
    Positional,
    /// The target has no input fields.
    Unit,
}

/// Visibility of a field to the construction caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConstructVisibility {
    /// The field is publicly accessible.
    Public,
    /// The field is accessible within the current crate.
    Crate,
    /// The field is accessible only within the named path.
    Restricted(
        /// Accessible path that may use the field.
        &'static str,
    ),
    /// The field is private to its declaring module.
    Private,
}

impl ConstructVisibility {
    /// Returns whether the field is public to an external caller.
    pub(crate) fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

/// Describes how a construction provider handles one field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionFieldPolicy {
    /// The caller must supply this field.
    Required,
    /// The provider initializes this field when the caller omits it.
    Defaulted,
    /// Only the provider may initialize this field.
    ProviderOnly,
    /// The field prevents safe construction of this target.
    Unavailable(
        /// Reason this field cannot be constructed safely.
        ConstructionUnavailableReason,
    ),
}

/// Reason a target or field cannot be safely constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionUnavailableReason {
    /// No generated provider is available for the target.
    NoProvider,
    /// The field's type or metadata cannot be reflected for construction.
    FieldNotReflectable,
    /// The target has a shape that the construction provider does not support.
    UnsupportedShape,
    /// A required field is not public to the caller.
    PrivateRequiredField,
}

/// Describes whether an existing field may be changed by an update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateFieldPolicy {
    /// The caller may replace this field.
    Replaceable,
    /// The existing value is preserved and cannot be replaced.
    Preserved,
    /// The provider cannot safely update this field.
    Unavailable(
        /// Reason this field cannot be updated safely.
        UpdateUnavailableReason,
    ),
}

/// Reason an update operation or field update is unavailable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UpdateUnavailableReason {
    /// No generated update provider is available for the target.
    NoProvider,
    /// The field is not supported by the update provider.
    FieldNotReplaceable,
}

/// Type and access policy metadata for one field in a construction plan.
pub struct ConstructField {
    /// Reflected field name.
    pub(crate) name: &'static str,
    /// Callback that returns the field's Rust type identity.
    pub(crate) value: fn() -> TypeIdentity,
    /// Visibility available to the construction caller.
    pub(crate) visibility: ConstructVisibility,
    /// Policy applied when constructing this field.
    pub(crate) policy: ConstructionFieldPolicy,
    /// Policy applied when updating this field.
    pub(crate) update_policy: UpdateFieldPolicy,
}

impl ConstructField {
    /// Creates field policy metadata bound to the field's Rust type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — the Rust type of the field.
    ///
    /// # Parameters
    ///
    /// * `name` — the field's reflected name.
    /// * `visibility` — the field's access level.
    /// * `policy` — construction behavior for the field.
    /// * `update_policy` — update behavior for the field.
    pub const fn typed<T: 'static>(
        name: &'static str,
        visibility: Visibility,
        policy: ConstructionFieldPolicy,
        update_policy: UpdateFieldPolicy,
    ) -> Self {
        Self {
            name,
            value: type_identity::<T>,
            visibility: match visibility {
                Visibility::Public => ConstructVisibility::Public,
                Visibility::Crate => ConstructVisibility::Crate,
                Visibility::Restricted(path) => ConstructVisibility::Restricted(path),
                Visibility::Private => ConstructVisibility::Private,
            },
            policy,
            update_policy,
        }
    }

    /// Returns the Rust type identity associated with this field.
    pub(crate) fn value_identity(&self) -> TypeIdentity {
        (self.value)()
    }

    /// Returns the field's reflected name.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns how construction handles omitted values for this field.
    pub fn policy(&self) -> ConstructionFieldPolicy {
        self.policy
    }

    /// Returns whether updates can replace or preserve this field.
    pub fn update_policy(&self) -> UpdateFieldPolicy {
        self.update_policy
    }
}

/// Type checked providers invoked only after all input validation succeeds.
///
/// Implementations receive fields in declaration order. They must return a
/// dynamic value with the exact target identity declared by the associated
/// construction plan.
///
/// # Type Parameters
///
/// * `M` — local or thread-safe value mode.
pub(crate) trait ConstructProvider<M: ValueMode>: Sync {
    /// Constructs the target from validated field values in declaration order.
    ///
    /// # Type Parameters
    ///
    /// * `M` — local or thread-safe value mode.
    ///
    /// # Parameters
    ///
    /// * `self` — generated provider for the target type.
    /// * `fields` — validated field values in declaration order.
    fn construct(&self, fields: Vec<Option<DynamicOwned<M>>>) -> DynamicOwned<M>;
    /// Applies validated replacements to an existing target value.
    ///
    /// # Type Parameters
    ///
    /// * `M` — local or thread-safe value mode.
    ///
    /// # Parameters
    ///
    /// * `self` — generated provider for the target type.
    /// * `base` — existing target value to update.
    /// * `replacements` — validated replacement values by field position.
    fn update(&self, base: DynamicOwned<M>, replacements: Vec<Option<DynamicOwned<M>>>) -> DynamicOwned<M>;
}

/// Runtime providers generated for local and thread-safe value modes.
#[derive(Clone, Copy)]
pub struct ConstructionProviders {
    /// Provider used for local values.
    local: Option<&'static dyn ConstructProvider<Local>>,
    /// Provider used for thread-safe values.
    thread_safe: Option<&'static dyn ConstructProvider<ThreadSafe>>,
}

impl ConstructionProviders {
    /// Creates the provider pair generated for the two value modes.
    ///
    /// # Parameters
    ///
    /// * `local` — provider for local values, if supported.
    /// * `thread_safe` — provider for thread-safe values, if supported.
    pub(crate) const fn from_runtime_providers(
        local: Option<&'static dyn ConstructProvider<Local>>,
        thread_safe: Option<&'static dyn ConstructProvider<ThreadSafe>>,
    ) -> Self {
        Self { local, thread_safe }
    }

    /// Adds the thread-safe provider to an existing provider pair.
    ///
    /// # Parameters
    ///
    /// * `provider` — generated provider for thread-safe values.
    pub(crate) const fn with_runtime_thread_safe(
        mut self,
        provider: &'static dyn ConstructProvider<ThreadSafe>,
    ) -> Self {
        self.thread_safe = Some(provider);
        self
    }

    /// Selects the provider compatible with the requested value mode.
    ///
    /// # Type Parameters
    ///
    /// * `M` — local or thread-safe value mode.
    ///
    /// # Parameters
    ///
    /// * `self` — the provider pair to inspect.
    pub(crate) fn for_mode<M: ConstructionMode>(&self) -> Option<&dyn ConstructProvider<M>> {
        M::provider(self)
    }
}

/// Connects a value mode to its compatible construction provider.
pub(crate) trait ConstructionMode: ValueMode {
    /// Returns the provider associated with this value mode.
    ///
    /// # Parameters
    ///
    /// * `providers` — providers generated for both value modes.
    fn provider(providers: &ConstructionProviders) -> Option<&dyn ConstructProvider<Self>>;
}

impl ConstructionMode for Local {
    /// Returns the local-value construction provider.
    ///
    /// # Parameters
    ///
    /// * `providers` — providers generated for both value modes.
    fn provider(providers: &ConstructionProviders) -> Option<&dyn ConstructProvider<Self>> {
        providers.local.map(|provider| provider as &dyn ConstructProvider<Self>)
    }
}

impl ConstructionMode for ThreadSafe {
    /// Returns the thread-safe construction provider.
    ///
    /// # Parameters
    ///
    /// * `providers` — providers generated for both value modes.
    fn provider(providers: &ConstructionProviders) -> Option<&dyn ConstructProvider<Self>> {
        providers
            .thread_safe
            .map(|provider| provider as &dyn ConstructProvider<Self>)
    }
}

/// Whether a construction plan can safely construct its target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstructionAvailability {
    /// All fields required for construction are available.
    Available,
    /// Construction is unavailable for the listed reasons.
    Unavailable {
        /// Reasons that prevent construction.
        reasons: Box<[ConstructionUnavailableReason]>,
    },
}

/// Identity and kind of the target represented by a construction plan.
#[derive(Clone, Copy)]
pub(crate) struct ConstructionTarget {
    /// Static or runtime identity of the target type.
    identity: ConstructionIdentity,
    /// Struct, enum variant, or unsupported target shape.
    pub(crate) kind: ConstructionTargetKind,
    /// Variant index for enum targets.
    pub(crate) variant_index: Option<usize>,
}

impl ConstructionTarget {
    /// Resolves the target's concrete type identity.
    pub(crate) fn identity(&self) -> TypeIdentity {
        match self.identity {
            ConstructionIdentity::Static(resolve) => resolve(),
            ConstructionIdentity::Runtime(identity) => identity,
        }
    }

    /// Creates target metadata from a runtime-computed identity.
    ///
    /// # Parameters
    ///
    /// * `identity` — the concrete target type identity.
    /// * `kind` — the construction target kind.
    /// * `variant_index` — the enum variant index, when applicable.
    pub(crate) const fn for_runtime(
        identity: TypeIdentity,
        kind: ConstructionTargetKind,
        variant_index: Option<usize>,
    ) -> Self {
        Self {
            identity: ConstructionIdentity::Runtime(identity),
            kind,
            variant_index,
        }
    }
}

/// Static or runtime-resolved type identity for a construction target.
/// Static or runtime-resolved type identity for a construction target.
#[derive(Clone, Copy)]
enum ConstructionIdentity {
    /// Identity obtained from the target's Rust type.
    Static(
        /// Resolves the Rust type identity when needed.
        fn() -> TypeIdentity,
    ),
    /// Identity supplied by a runtime descriptor.
    Runtime(
        /// Concrete type identity supplied by the descriptor.
        TypeIdentity,
    ),
}

/// Shape category supported by a construction provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConstructionTargetKind {
    /// A struct target.
    Struct,
    /// An enum variant target.
    Variant,
    /// A target shape unsupported by the provider.
    Unsupported,
}

/// Static metadata and generated providers for constructing one reflected
/// target.
pub struct ConstructionPlan {
    /// Identity and kind of the target.
    pub(crate) target: ConstructionTarget,
    /// Input layout accepted by the provider.
    pub(crate) shape: ConstructionShape,
    /// Field metadata in declaration order.
    pub(crate) fields: &'static [ConstructField],
    /// Generated providers for supported value modes.
    pub(crate) provider: ConstructionProviders,
    /// Static construction availability from generated metadata.
    pub(crate) availability: ConstructionAvailability,
}

impl ConstructionPlan {
    /// Creates a plan for a concrete struct target.
    ///
    /// # Type Parameters
    ///
    /// * `T` — the reflected struct type.
    ///
    /// # Parameters
    ///
    /// * `shape` — the input layout accepted by the provider.
    /// * `fields` — field metadata in declaration order.
    /// * `provider` — generated construction providers.
    /// * `availability` — static availability of construction.
    pub const fn for_type<T: Reflect>(
        shape: ConstructionShape,
        fields: &'static [ConstructField],
        provider: ConstructionProviders,
        availability: ConstructionAvailability,
    ) -> Self {
        Self::new(
            ConstructionTarget {
                identity: ConstructionIdentity::Static(type_identity::<T>),
                kind: ConstructionTargetKind::Struct,
                variant_index: None,
            },
            shape,
            fields,
            provider,
            availability,
        )
    }

    /// Creates a plan for one enum variant of a concrete owner type.
    ///
    /// # Type Parameters
    ///
    /// * `T` — the reflected enum type.
    ///
    /// # Parameters
    ///
    /// * `variant_index` — declaration index of the variant.
    /// * `shape` — the input layout accepted by the provider.
    /// * `fields` — variant field metadata in declaration order.
    /// * `provider` — generated construction providers.
    /// * `availability` — static availability of construction.
    pub const fn for_variant<T: Reflect>(
        variant_index: usize,
        shape: ConstructionShape,
        fields: &'static [ConstructField],
        provider: ConstructionProviders,
        availability: ConstructionAvailability,
    ) -> Self {
        Self::new(
            ConstructionTarget {
                identity: ConstructionIdentity::Static(type_identity::<T>),
                kind: ConstructionTargetKind::Variant,
                variant_index: Some(variant_index),
            },
            shape,
            fields,
            provider,
            availability,
        )
    }

    /// Creates a plan from precomputed target and provider metadata.
    /// Creates a plan from precomputed target and provider metadata.
    ///
    /// # Parameters
    ///
    /// * `target` — identity and kind of the construction target.
    /// * `shape` — input layout accepted by the provider.
    /// * `fields` — target fields in declaration order.
    /// * `provider` — generated provider pair.
    /// * `availability` — static construction availability.
    pub(crate) const fn new(
        target: ConstructionTarget,
        shape: ConstructionShape,
        fields: &'static [ConstructField],
        provider: ConstructionProviders,
        availability: ConstructionAvailability,
    ) -> Self {
        Self {
            target,
            shape,
            fields,
            provider,
            availability,
        }
    }

    /// Computes availability after applying the requested visibility policy.
    ///
    /// # Parameters
    ///
    /// * `unchecked_visibility` — whether required-field visibility is ignored.
    pub(crate) fn availability(&self, unchecked_visibility: bool) -> ConstructionAvailability {
        let mut reasons = match &self.availability {
            ConstructionAvailability::Available => Vec::new(),
            ConstructionAvailability::Unavailable { reasons } => reasons.to_vec(),
        };
        for field in self.fields {
            if let ConstructionFieldPolicy::Unavailable(reason) = field.policy
                && !reasons.contains(&reason)
            {
                reasons.push(reason);
            }
            if !unchecked_visibility
                && !field.visibility.is_public()
                && matches!(field.policy, ConstructionFieldPolicy::Required)
                && !reasons.contains(&ConstructionUnavailableReason::PrivateRequiredField)
            {
                reasons.push(ConstructionUnavailableReason::PrivateRequiredField);
            }
        }
        if reasons.is_empty() {
            ConstructionAvailability::Available
        } else {
            ConstructionAvailability::Unavailable {
                reasons: reasons.into_boxed_slice(),
            }
        }
    }

    /// Returns the first field-level reason that prevents updates.
    pub(crate) fn update_unavailable_reason(&self) -> Option<UpdateUnavailableReason> {
        self.fields.iter().find_map(|field| match field.update_policy {
            UpdateFieldPolicy::Unavailable(reason) => Some(reason),
            UpdateFieldPolicy::Replaceable | UpdateFieldPolicy::Preserved => None,
        })
    }
}

/// Returns runtime identity metadata for a Rust type.
///
/// # Type Parameters
///
/// * `T` — the type whose identity is requested.
fn type_identity<T: 'static>() -> TypeIdentity {
    TypeIdentity {
        type_id: std::any::TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

/// Construction metadata bound to one reflection context.
#[derive(Clone, Copy)]
pub(crate) struct ConstructionDescriptor<'ctx> {
    /// Registered plan, if one is available.
    pub(crate) plan: Option<&'ctx ConstructionPlan>,
    /// Target identity and kind.
    target: ConstructionTarget,
    /// Input layout of the target.
    shape: ConstructionShape,
    /// Target fields in declaration order.
    fields: &'static [ConstructField],
    /// Static reason a provider is unavailable.
    unavailable_reason: Option<ConstructionUnavailableReason>,
}

/// Finds the unique struct construction plan for a target type.
///
/// # Parameters
///
/// * `target` — identity of the requested struct type.
/// * `plans` — candidate plans available to the context.
#[cfg(test)]
pub(crate) fn resolve_plan(
    target: TypeIdentity,
    plans: &[ConstructionPlan],
) -> Result<&ConstructionPlan, ConstructionError> {
    let mut matching = plans.iter().filter(|plan| {
        plan.target.kind == ConstructionTargetKind::Struct && plan.target.identity().type_id == target.type_id
    });
    let Some(plan) = matching.next() else {
        return Err(ConstructionError::MissingProvider);
    };
    if matching.next().is_some() {
        let count = plans
            .iter()
            .filter(|plan| {
                plan.target.kind == ConstructionTargetKind::Struct && plan.target.identity().type_id == target.type_id
            })
            .count();
        return Err(ConstructionError::AmbiguousProvider { target, count });
    }
    Ok(plan)
}

/// Finds the unique construction plan for an enum variant.
///
/// # Parameters
///
/// * `owner` — identity of the enum type.
/// * `variant_index` — declaration index of the requested variant.
/// * `plans` — candidate plans available to the context.
#[cfg(test)]
pub(crate) fn resolve_variant_plan(
    owner: TypeIdentity,
    variant_index: usize,
    plans: &[ConstructionPlan],
) -> Result<&ConstructionPlan, ConstructionError> {
    let mut matching = plans.iter().filter(|plan| {
        plan.target.kind == ConstructionTargetKind::Variant
            && plan.target.identity().type_id == owner.type_id
            && plan.target.variant_index == Some(variant_index)
    });
    let Some(plan) = matching.next() else {
        return Err(ConstructionError::MissingProvider);
    };
    if matching.next().is_some() {
        let count = plans
            .iter()
            .filter(|plan| {
                plan.target.kind == ConstructionTargetKind::Variant
                    && plan.target.identity().type_id == owner.type_id
                    && plan.target.variant_index == Some(variant_index)
            })
            .count();
        return Err(ConstructionError::AmbiguousProvider { target: owner, count });
    }
    Ok(plan)
}

impl<'ctx> ConstructionDescriptor<'ctx> {
    /// Creates a descriptor for a registered construction plan.
    ///
    /// # Parameters
    ///
    /// * `plan` — registered construction plan to describe.
    pub(crate) const fn new(plan: &'ctx ConstructionPlan) -> Self {
        Self {
            plan: Some(plan),
            target: plan.target,
            shape: plan.shape,
            fields: plan.fields,
            unavailable_reason: None,
        }
    }

    /// Creates a descriptor for a target without a usable plan.
    ///
    /// # Parameters
    ///
    /// * `target` — identity and kind of the target.
    /// * `shape` — input layout exposed by the target.
    /// * `fields` — field metadata available for the target.
    /// * `reason` — reason construction is unavailable.
    pub(crate) const fn unavailable(
        target: ConstructionTarget,
        shape: ConstructionShape,
        fields: &'static [ConstructField],
        reason: ConstructionUnavailableReason,
    ) -> Self {
        Self {
            plan: None,
            target,
            shape,
            fields,
            unavailable_reason: Some(reason),
        }
    }

    /// Returns the target identity and kind.
    pub(crate) fn target(&self) -> &ConstructionTarget {
        &self.target
    }

    /// Returns the target's input layout.
    pub(crate) fn shape(&self) -> ConstructionShape {
        self.shape
    }

    /// Returns the reasons construction is unavailable, if any.
    pub(crate) fn availability(&self) -> ConstructionAvailability {
        if let Some(plan) = self.plan {
            return plan.availability(false);
        }
        ConstructionAvailability::Unavailable {
            reasons: Box::new([self
                .unavailable_reason
                .unwrap_or(ConstructionUnavailableReason::NoProvider)]),
        }
    }

    /// Returns the target's field metadata.
    pub(crate) fn fields(&self) -> &[ConstructField] {
        self.fields
    }
}

/// Error returned when a reflected target cannot be constructed.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionError {
    /// No construction provider is registered for the target.
    MissingProvider,
    /// More than one provider matches the target.
    AmbiguousProvider {
        /// Identity of the requested target.
        target: TypeIdentity,
        /// Number of matching providers.
        count: usize,
    },
    /// The selected plan does not construct the requested target kind.
    WrongTargetKind,
    /// Construction is unavailable for one or more metadata reasons.
    Unavailable(
        /// Reasons that prevent construction.
        ConstructionAvailability,
    ),
    /// The provider returned a value with the wrong target type.
    TargetTypeMismatch(
        /// Expected and actual target type identities.
        crate::runtime::value::TypeMismatch,
    ),
    /// The supplied input layout differs from the construction plan.
    ShapeMismatch {
        /// Input layout required by the plan.
        expected: ConstructionShape,
        /// Input layout supplied by the caller.
        actual: ConstructionShape,
    },
    /// A required field value was not supplied.
    MissingField {
        /// Reflected name of the required field.
        field: &'static str,
    },
    /// A field was supplied more than once.
    DuplicateField {
        /// Reflected name of the duplicated field.
        field: &'static str,
    },
    /// A named input does not match a field in the target.
    UnknownField {
        /// Name supplied by the caller.
        field: Box<str>,
    },
    /// The caller supplied a value reserved for the provider.
    ProviderOnlyField {
        /// Reflected name of the provider-only field.
        field: &'static str,
    },
    /// A required field is unavailable to the construction provider.
    FieldUnavailable {
        /// Reflected name of the unavailable field.
        field: &'static str,
        /// Reason the field cannot be constructed.
        reason: ConstructionUnavailableReason,
    },
    /// A required field is not public to the caller.
    FieldNotPublic {
        /// Reflected name of the inaccessible field.
        field: &'static str,
    },
    /// A supplied value has the wrong Rust type for its field.
    ValueTypeMismatch {
        /// Reflected name of the field.
        field: &'static str,
        /// Expected and actual value type identities.
        mismatch: crate::runtime::value::TypeMismatch,
    },
    /// More positional values were supplied than the plan accepts.
    TooManyValues {
        /// Maximum number of values accepted by the plan.
        expected_max: usize,
        /// Number of values supplied by the caller.
        actual: usize,
    },
}

impl fmt::Display for ConstructionError {
    /// Writes the construction diagnostic using its debug representation.
    ///
    /// # Parameters
    ///
    /// * `formatter` — output formatter supplied by the caller.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ConstructionError {}

/// Error returned when an existing reflected target cannot be updated.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UpdateError {
    /// The supplied base value has the wrong Rust type.
    BaseTypeMismatch(
        /// Expected and actual base type identities.
        crate::runtime::value::TypeMismatch,
    ),
    /// The provider returned a value with the wrong target type.
    TargetTypeMismatch(
        /// Expected and actual target type identities.
        crate::runtime::value::TypeMismatch,
    ),
    /// The selected plan does not update the requested target kind.
    WrongTargetKind,
    /// No update provider is registered for the target.
    MissingProvider,
    /// More than one update provider matches the target.
    AmbiguousProvider {
        /// Identity of the requested target.
        target: TypeIdentity,
        /// Number of matching providers.
        count: usize,
    },
    /// Updating is unavailable for one or more metadata reasons.
    Unavailable(
        /// Reasons that prevent updates.
        ConstructionAvailability,
    ),
    /// The supplied input layout differs from the update plan.
    ShapeMismatch {
        /// Input layout required by the plan.
        expected: ConstructionShape,
        /// Input layout supplied by the caller.
        actual: ConstructionShape,
    },
    /// A required replacement value was not supplied.
    MissingField {
        /// Reflected name of the required field.
        field: &'static str,
    },
    /// A field replacement was supplied more than once.
    DuplicateField {
        /// Reflected name of the duplicated field.
        field: &'static str,
    },
    /// A named replacement does not match a field in the target.
    UnknownField {
        /// Name supplied by the caller.
        field: Box<str>,
    },
    /// The caller attempted to replace a provider-only field.
    ProviderOnlyField {
        /// Reflected name of the provider-only field.
        field: &'static str,
    },
    /// A field cannot be updated by the provider.
    FieldUnavailable {
        /// Reflected name of the unavailable field.
        field: &'static str,
        /// Reason the field cannot be updated.
        reason: ConstructionUnavailableReason,
    },
    /// A field is not public to the caller.
    FieldNotPublic {
        /// Reflected name of the inaccessible field.
        field: &'static str,
    },
    /// A replacement has the wrong Rust type for its field.
    ValueTypeMismatch {
        /// Reflected name of the field.
        field: &'static str,
        /// Expected and actual value type identities.
        mismatch: crate::runtime::value::TypeMismatch,
    },
    /// More positional replacements were supplied than the plan accepts.
    TooManyValues {
        /// Maximum number of values accepted by the plan.
        expected_max: usize,
        /// Number of values supplied by the caller.
        actual: usize,
    },
    /// The field's update policy preserves its current value.
    FieldPreserved {
        /// Reflected name of the preserved field.
        field: &'static str,
    },
    /// The provider cannot safely update the selected field.
    FieldUpdateUnavailable {
        /// Reflected name of the unavailable field.
        field: &'static str,
        /// Reason the field cannot be updated.
        reason: UpdateUnavailableReason,
    },
    /// The target has no usable update provider.
    UpdateUnavailable(
        /// Reason the update provider is unavailable.
        UpdateUnavailableReason,
    ),
}

impl fmt::Display for UpdateError {
    /// Writes the update diagnostic using its debug representation.
    ///
    /// # Parameters
    ///
    /// * `formatter` — output formatter supplied by the caller.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for UpdateError {}

impl From<ConstructionError> for UpdateError {
    /// Converts a construction failure into the corresponding update failure.
    ///
    /// # Parameters
    ///
    /// * `error` — construction error to translate.
    fn from(error: ConstructionError) -> Self {
        match error {
            ConstructionError::MissingProvider => Self::MissingProvider,
            ConstructionError::AmbiguousProvider { target, count } => Self::AmbiguousProvider { target, count },
            ConstructionError::WrongTargetKind => Self::WrongTargetKind,
            ConstructionError::Unavailable(reason) => Self::Unavailable(reason),
            ConstructionError::TargetTypeMismatch(mismatch) => Self::TargetTypeMismatch(mismatch),
            ConstructionError::ShapeMismatch { expected, actual } => Self::ShapeMismatch { expected, actual },
            ConstructionError::MissingField { field } => Self::MissingField { field },
            ConstructionError::DuplicateField { field } => Self::DuplicateField { field },
            ConstructionError::UnknownField { field } => Self::UnknownField { field },
            ConstructionError::ProviderOnlyField { field } => Self::ProviderOnlyField { field },
            ConstructionError::FieldUnavailable { field, reason } => Self::FieldUnavailable { field, reason },
            ConstructionError::FieldNotPublic { field } => Self::FieldNotPublic { field },
            ConstructionError::ValueTypeMismatch { field, mismatch } => Self::ValueTypeMismatch { field, mismatch },
            ConstructionError::TooManyValues { expected_max, actual } => Self::TooManyValues { expected_max, actual },
        }
    }
}
