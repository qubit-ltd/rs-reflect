use crate::runtime::descriptor::{Reflect, field::Visibility};
use crate::runtime::identity::TypeIdentity;
use crate::runtime::value::{DynamicOwned, Local, ThreadSafe, ValueMode};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionShape {
    Named,
    Positional,
    Unit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConstructVisibility {
    Public,
    Crate,
    Restricted(&'static str),
    Private,
}

impl ConstructVisibility {
    pub(crate) fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionFieldPolicy {
    /// The caller must supply this field.
    Required,
    /// The provider initializes this field when the caller omits it.
    Defaulted,
    /// Only the provider may initialize this field.
    ProviderOnly,
    /// The field prevents safe construction of this target.
    Unavailable(ConstructionUnavailableReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionUnavailableReason {
    NoProvider,
    FieldNotReflectable,
    UnsupportedShape,
    PrivateRequiredField,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateFieldPolicy {
    /// The caller may replace this field.
    Replaceable,
    /// The existing value is preserved and cannot be replaced.
    Preserved,
    /// The provider cannot safely update this field.
    Unavailable(UpdateUnavailableReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum UpdateUnavailableReason {
    NoProvider,
    FieldNotReplaceable,
}

pub struct ConstructField {
    pub(crate) name: &'static str,
    pub(crate) value: fn() -> TypeIdentity,
    pub(crate) visibility: ConstructVisibility,
    pub(crate) policy: ConstructionFieldPolicy,
    pub(crate) update_policy: UpdateFieldPolicy,
}

impl ConstructField {
    /// Creates field policy metadata bound to the field's Rust type.
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

    pub(crate) fn value_identity(&self) -> TypeIdentity {
        (self.value)()
    }
}

/// Type checked providers invoked only after all input validation succeeds.
///
/// Implementations receive fields in declaration order. They must return a dynamic
/// value with the exact target identity declared by the associated construction plan.
pub(crate) trait ConstructProvider<M: ValueMode>: Sync {
    fn construct(&self, fields: Vec<Option<DynamicOwned<M>>>) -> DynamicOwned<M>;
    fn update(
        &self,
        base: DynamicOwned<M>,
        replacements: Vec<Option<DynamicOwned<M>>>,
    ) -> DynamicOwned<M>;
}

pub struct TypedConstructProvider<T: 'static, M: ValueMode> {
    construct: fn(Vec<Option<DynamicOwned<M>>>) -> T,
    update: fn(T, Vec<Option<DynamicOwned<M>>>) -> T,
    _type: std::marker::PhantomData<fn() -> T>,
}

impl<T: 'static, M: ValueMode> TypedConstructProvider<T, M> {
    /// Creates typed construction and update callbacks for one concrete target type.
    pub const fn new(
        construct: fn(Vec<Option<DynamicOwned<M>>>) -> T,
        update: fn(T, Vec<Option<DynamicOwned<M>>>) -> T,
    ) -> Self {
        Self {
            construct,
            update,
            _type: std::marker::PhantomData,
        }
    }
}

impl<T, M> ConstructProvider<M> for TypedConstructProvider<T, M>
where
    T: crate::runtime::value::ValueFor<M>,
    M: ValueMode,
{
    fn construct(&self, fields: Vec<Option<DynamicOwned<M>>>) -> DynamicOwned<M> {
        DynamicOwned::new((self.construct)(fields))
    }

    fn update(
        &self,
        base: DynamicOwned<M>,
        replacements: Vec<Option<DynamicOwned<M>>>,
    ) -> DynamicOwned<M> {
        let base = match base.downcast::<T>() {
            Ok(value) => value,
            Err(_) => panic!("validated update base must match its typed provider"),
        };
        DynamicOwned::new((self.update)(base, replacements))
    }
}

#[derive(Clone, Copy)]
pub struct ConstructionProviders {
    local: Option<&'static dyn ConstructProvider<Local>>,
    thread_safe: Option<&'static dyn ConstructProvider<ThreadSafe>>,
}

impl ConstructionProviders {
    pub(crate) const fn from_runtime_providers(
        local: Option<&'static dyn ConstructProvider<Local>>,
        thread_safe: Option<&'static dyn ConstructProvider<ThreadSafe>>,
    ) -> Self {
        Self { local, thread_safe }
    }

    /// Creates providers with the local mode, which works for all reflected values.
    pub const fn new<T: crate::runtime::value::ValueFor<Local>>(
        local: Option<&'static TypedConstructProvider<T, Local>>,
    ) -> Self {
        Self {
            local: match local {
                Some(provider) => Some(provider),
                None => None,
            },
            thread_safe: None,
        }
    }

    /// Adds a thread-safe provider when the target and its fields are `Send + Sync`.
    pub const fn with_thread_safe<T: crate::runtime::value::ValueFor<ThreadSafe>>(
        mut self,
        provider: &'static TypedConstructProvider<T, ThreadSafe>,
    ) -> Self {
        self.thread_safe = Some(provider);
        self
    }

    pub(crate) fn for_mode<M: ConstructionMode>(&self) -> Option<&dyn ConstructProvider<M>> {
        M::provider(self)
    }
}

pub(crate) trait ConstructionMode: ValueMode {
    fn provider(providers: &ConstructionProviders) -> Option<&dyn ConstructProvider<Self>>;
}

impl ConstructionMode for Local {
    fn provider(providers: &ConstructionProviders) -> Option<&dyn ConstructProvider<Self>> {
        providers
            .local
            .map(|provider| provider as &dyn ConstructProvider<Self>)
    }
}

impl ConstructionMode for ThreadSafe {
    fn provider(providers: &ConstructionProviders) -> Option<&dyn ConstructProvider<Self>> {
        providers
            .thread_safe
            .map(|provider| provider as &dyn ConstructProvider<Self>)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstructionAvailability {
    Available,
    Unavailable {
        reasons: Box<[ConstructionUnavailableReason]>,
    },
}

pub(crate) struct ConstructionTarget {
    identity: fn() -> TypeIdentity,
    pub(crate) kind: ConstructionTargetKind,
    pub(crate) variant_index: Option<usize>,
}

impl ConstructionTarget {
    pub(crate) fn identity(&self) -> TypeIdentity {
        (self.identity)()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConstructionTargetKind {
    Struct,
    Variant,
    Unsupported,
}

pub struct ConstructionPlan {
    pub(crate) target: ConstructionTarget,
    pub(crate) shape: ConstructionShape,
    pub(crate) fields: &'static [ConstructField],
    pub(crate) provider: ConstructionProviders,
    pub(crate) availability: ConstructionAvailability,
}

impl ConstructionPlan {
    /// Creates a plan for a concrete struct target.
    pub const fn for_type<T: Reflect>(
        shape: ConstructionShape,
        fields: &'static [ConstructField],
        provider: ConstructionProviders,
        availability: ConstructionAvailability,
    ) -> Self {
        Self::new(
            ConstructionTarget {
                identity: type_identity::<T>,
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
    pub const fn for_variant<T: Reflect>(
        variant_index: usize,
        shape: ConstructionShape,
        fields: &'static [ConstructField],
        provider: ConstructionProviders,
        availability: ConstructionAvailability,
    ) -> Self {
        Self::new(
            ConstructionTarget {
                identity: type_identity::<T>,
                kind: ConstructionTargetKind::Variant,
                variant_index: Some(variant_index),
            },
            shape,
            fields,
            provider,
            availability,
        )
    }

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

    pub(crate) fn availability(&self, unchecked_visibility: bool) -> ConstructionAvailability {
        let mut reasons = match &self.availability {
            ConstructionAvailability::Available => Vec::new(),
            ConstructionAvailability::Unavailable { reasons } => reasons.to_vec(),
        };
        for field in self.fields {
            if let ConstructionFieldPolicy::Unavailable(reason) = field.policy {
                if !reasons.contains(&reason) {
                    reasons.push(reason);
                }
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

    pub(crate) fn update_unavailable_reason(&self) -> Option<UpdateUnavailableReason> {
        self.fields
            .iter()
            .find_map(|field| match field.update_policy {
                UpdateFieldPolicy::Unavailable(reason) => Some(reason),
                UpdateFieldPolicy::Replaceable | UpdateFieldPolicy::Preserved => None,
            })
    }
}

fn type_identity<T: 'static>() -> TypeIdentity {
    TypeIdentity {
        type_id: std::any::TypeId::of::<T>(),
        rust_name: std::any::type_name::<T>(),
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ConstructionDescriptor<'ctx> {
    pub(crate) plan: &'ctx ConstructionPlan,
}

pub(crate) fn resolve_plan<'a>(
    target: TypeIdentity,
    plans: &'a [ConstructionPlan],
) -> Result<&'a ConstructionPlan, ConstructionError> {
    let mut matching = plans.iter().filter(|plan| {
        plan.target.kind == ConstructionTargetKind::Struct
            && plan.target.identity().type_id == target.type_id
    });
    let Some(plan) = matching.next() else {
        return Err(ConstructionError::MissingProvider);
    };
    if matching.next().is_some() {
        let count = plans
            .iter()
            .filter(|plan| {
                plan.target.kind == ConstructionTargetKind::Struct
                    && plan.target.identity().type_id == target.type_id
            })
            .count();
        return Err(ConstructionError::AmbiguousProvider { target, count });
    }
    Ok(plan)
}

pub(crate) fn resolve_variant_plan<'a>(
    owner: TypeIdentity,
    variant_index: usize,
    plans: &'a [ConstructionPlan],
) -> Result<&'a ConstructionPlan, ConstructionError> {
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
        return Err(ConstructionError::AmbiguousProvider {
            target: owner,
            count,
        });
    }
    Ok(plan)
}

impl<'ctx> ConstructionDescriptor<'ctx> {
    pub(crate) const fn new(plan: &'ctx ConstructionPlan) -> Self {
        Self { plan }
    }

    pub(crate) fn target(&self) -> &ConstructionTarget {
        &self.plan.target
    }

    pub(crate) fn shape(&self) -> ConstructionShape {
        self.plan.shape
    }

    pub(crate) fn availability(&self) -> ConstructionAvailability {
        self.plan.availability(false)
    }

    pub(crate) fn fields(&self) -> &[ConstructField] {
        self.plan.fields
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum ConstructionError {
    MissingProvider,
    AmbiguousProvider {
        target: TypeIdentity,
        count: usize,
    },
    WrongTargetKind,
    Unavailable(ConstructionAvailability),
    TargetTypeMismatch(crate::runtime::value::TypeMismatch),
    ShapeMismatch {
        expected: ConstructionShape,
        actual: ConstructionShape,
    },
    MissingField {
        field: &'static str,
    },
    DuplicateField {
        field: &'static str,
    },
    UnknownField {
        field: Box<str>,
    },
    ProviderOnlyField {
        field: &'static str,
    },
    FieldUnavailable {
        field: &'static str,
        reason: ConstructionUnavailableReason,
    },
    FieldNotPublic {
        field: &'static str,
    },
    ValueTypeMismatch {
        field: &'static str,
        mismatch: crate::runtime::value::TypeMismatch,
    },
    TooManyValues {
        expected_max: usize,
        actual: usize,
    },
}

impl fmt::Display for ConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ConstructionError {}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum UpdateError {
    BaseTypeMismatch(crate::runtime::value::TypeMismatch),
    TargetTypeMismatch(crate::runtime::value::TypeMismatch),
    WrongTargetKind,
    MissingProvider,
    AmbiguousProvider {
        target: TypeIdentity,
        count: usize,
    },
    Unavailable(ConstructionAvailability),
    ShapeMismatch {
        expected: ConstructionShape,
        actual: ConstructionShape,
    },
    MissingField {
        field: &'static str,
    },
    DuplicateField {
        field: &'static str,
    },
    UnknownField {
        field: Box<str>,
    },
    ProviderOnlyField {
        field: &'static str,
    },
    FieldUnavailable {
        field: &'static str,
        reason: ConstructionUnavailableReason,
    },
    FieldNotPublic {
        field: &'static str,
    },
    ValueTypeMismatch {
        field: &'static str,
        mismatch: crate::runtime::value::TypeMismatch,
    },
    TooManyValues {
        expected_max: usize,
        actual: usize,
    },
    FieldPreserved {
        field: &'static str,
    },
    FieldUpdateUnavailable {
        field: &'static str,
        reason: UpdateUnavailableReason,
    },
    UpdateUnavailable(UpdateUnavailableReason),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for UpdateError {}

impl From<ConstructionError> for UpdateError {
    fn from(error: ConstructionError) -> Self {
        match error {
            ConstructionError::MissingProvider => Self::MissingProvider,
            ConstructionError::AmbiguousProvider { target, count } => {
                Self::AmbiguousProvider { target, count }
            }
            ConstructionError::WrongTargetKind => Self::WrongTargetKind,
            ConstructionError::Unavailable(reason) => Self::Unavailable(reason),
            ConstructionError::TargetTypeMismatch(mismatch) => Self::TargetTypeMismatch(mismatch),
            ConstructionError::ShapeMismatch { expected, actual } => {
                Self::ShapeMismatch { expected, actual }
            }
            ConstructionError::MissingField { field } => Self::MissingField { field },
            ConstructionError::DuplicateField { field } => Self::DuplicateField { field },
            ConstructionError::UnknownField { field } => Self::UnknownField { field },
            ConstructionError::ProviderOnlyField { field } => Self::ProviderOnlyField { field },
            ConstructionError::FieldUnavailable { field, reason } => {
                Self::FieldUnavailable { field, reason }
            }
            ConstructionError::FieldNotPublic { field } => Self::FieldNotPublic { field },
            ConstructionError::ValueTypeMismatch { field, mismatch } => {
                Self::ValueTypeMismatch { field, mismatch }
            }
            ConstructionError::TooManyValues {
                expected_max,
                actual,
            } => Self::TooManyValues {
                expected_max,
                actual,
            },
        }
    }
}
