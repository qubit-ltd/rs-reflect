// qubit-style: allow public-type-layout
//! Caller-owned construction input and per-field policy declarations.

use std::fmt;

use crate::construct::ConstructionRecovery;
use crate::construct::RecoveredConstructionValue;
use crate::descriptor::FieldDescriptor;
use crate::value::DynamicOwned;
use crate::value::Mode;

/// A mode-specific explicit provider for one omitted field value.
pub type ConstructionDefaultProvider<M> = fn() -> DynamicOwned<M>;

/// Runtime construction policy generated for one declared field.
pub enum ConstructionFieldPolicy<M: Mode> {
    /// The caller must supply this field during from-zero construction.
    Required,
    /// A missing value is produced only by this explicit provider.
    Default(ConstructionDefaultProvider<M>),
    /// The field is always produced by this provider and rejects caller input.
    ProviderOnly(ConstructionDefaultProvider<M>),
    /// The generated construction path cannot safely supply this field.
    Unavailable(crate::construct::ConstructionUnavailableReason),
}

impl<M: Mode> Clone for ConstructionFieldPolicy<M> {
    /// Copies the immutable generated policy.
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Mode> Copy for ConstructionFieldPolicy<M> {}

impl<M: Mode> fmt::Debug for ConstructionFieldPolicy<M> {
    /// Formats policy facts without exposing provider addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required => formatter.write_str("Required"),
            Self::Default(_) => formatter.write_str("Default(<provider>)"),
            Self::ProviderOnly(_) => formatter.write_str("ProviderOnly(<provider>)"),
            Self::Unavailable(reason) => formatter.debug_tuple("Unavailable").field(reason).finish(),
        }
    }
}

/// A declared field paired with its generated construction policy.
pub struct ConstructionField<M: Mode> {
    descriptor: &'static FieldDescriptor,
    policy: ConstructionFieldPolicy<M>,
}

impl<M: Mode> Clone for ConstructionField<M> {
    /// Copies the immutable descriptor reference and policy.
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Mode> Copy for ConstructionField<M> {}

impl<M: Mode> ConstructionField<M> {
    /// Declares a field that must be present in from-zero construction input.
    pub const fn required(descriptor: &'static FieldDescriptor) -> Self {
        Self {
            descriptor,
            policy: ConstructionFieldPolicy::Required,
        }
    }

    /// Declares a field with an explicit generated default provider.
    pub const fn defaulted(descriptor: &'static FieldDescriptor, provider: ConstructionDefaultProvider<M>) -> Self {
        Self {
            descriptor,
            policy: ConstructionFieldPolicy::Default(provider),
        }
    }

    /// Declares a skipped or no-construct field supplied only by its provider.
    ///
    /// Omitting the field invokes `provider`; directly binding the field is a
    /// validation error and returns every caller-owned input.
    pub const fn provider_only(descriptor: &'static FieldDescriptor, provider: ConstructionDefaultProvider<M>) -> Self {
        Self {
            descriptor,
            policy: ConstructionFieldPolicy::ProviderOnly(provider),
        }
    }

    /// Marks a field as blocking this generated construction path.
    pub const fn unavailable(
        descriptor: &'static FieldDescriptor,
        reason: crate::construct::ConstructionUnavailableReason,
    ) -> Self {
        Self {
            descriptor,
            policy: ConstructionFieldPolicy::Unavailable(reason),
        }
    }

    /// Returns the immutable structural field descriptor.
    pub const fn descriptor(&self) -> &'static FieldDescriptor {
        self.descriptor
    }

    /// Returns the explicit generated construction policy.
    pub const fn policy(&self) -> ConstructionFieldPolicy<M> {
        self.policy
    }
}

impl<M: Mode> fmt::Debug for ConstructionField<M> {
    /// Formats the descriptor-local field index and construction policy.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConstructionField")
            .field("index", &self.descriptor.index())
            .field("query_name", &self.descriptor.query_name())
            .field("policy", &self.policy)
            .finish()
    }
}

/// Runtime policy controlling whether an owned update may replace one field.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UpdateFieldPolicy {
    /// The generated updater accepts an exact whole-field replacement.
    Allowed,
    /// The generated updater must reject replacement of this field.
    Unavailable(crate::construct::ConstructionUnavailableReason),
}

/// A declared field paired with its independent owned-update policy.
#[derive(Clone, Copy)]
pub struct UpdateField {
    descriptor: &'static FieldDescriptor,
    policy: UpdateFieldPolicy,
}

impl UpdateField {
    /// Declares a field whose exact whole value may be replaced by update.
    pub const fn allowed(descriptor: &'static FieldDescriptor) -> Self {
        Self {
            descriptor,
            policy: UpdateFieldPolicy::Allowed,
        }
    }

    /// Declares a field that the generated update path cannot replace.
    pub const fn unavailable(
        descriptor: &'static FieldDescriptor,
        reason: crate::construct::ConstructionUnavailableReason,
    ) -> Self {
        Self {
            descriptor,
            policy: UpdateFieldPolicy::Unavailable(reason),
        }
    }

    /// Returns the immutable structural field descriptor.
    pub const fn descriptor(&self) -> &'static FieldDescriptor {
        self.descriptor
    }

    /// Returns the independent generated update policy.
    pub const fn policy(&self) -> UpdateFieldPolicy {
        self.policy
    }
}

impl fmt::Debug for UpdateField {
    /// Formats the field index and update policy.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UpdateField")
            .field("index", &self.descriptor.index())
            .field("query_name", &self.descriptor.query_name())
            .field("policy", &self.policy)
            .finish()
    }
}

/// Query-name bindings collected for named struct or variant construction.
pub struct NamedConstructionInput<M: Mode> {
    fields: Vec<(Box<str>, DynamicOwned<M>)>,
}

impl<M: Mode> NamedConstructionInput<M> {
    /// Collects named owned values in caller order without validating them.
    pub fn new<I, N>(fields: I) -> Self
    where
        I: IntoIterator<Item = (N, DynamicOwned<M>)>,
        N: Into<Box<str>>,
    {
        Self {
            fields: fields.into_iter().map(|(name, value)| (name.into(), value)).collect(),
        }
    }

    /// Returns named values in their original caller order.
    pub fn fields(&self) -> &[(Box<str>, DynamicOwned<M>)] {
        &self.fields
    }

    /// Extracts the raw ordered bindings for validation or recovery.
    pub(crate) fn into_fields(self) -> Vec<(Box<str>, DynamicOwned<M>)> {
        self.fields
    }

    /// Converts untouched bindings into a recovery payload.
    pub(crate) fn into_recovery(self, error: crate::construct::ConstructionError) -> ConstructionRecovery<M> {
        let values = self
            .fields
            .into_iter()
            .map(|(name, value)| RecoveredConstructionValue::Named { name, value })
            .collect();
        ConstructionRecovery::new(error, values)
    }
}

impl<M: Mode> fmt::Debug for NamedConstructionInput<M> {
    /// Formats binding names without requiring erased values to be `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NamedConstructionInput")
            .field("names", &self.fields.iter().map(|(name, _)| name).collect::<Vec<_>>())
            .finish()
    }
}

/// Source-ordered owned values collected for tuple or newtype construction.
pub struct TupleConstructionInput<M: Mode> {
    values: Vec<DynamicOwned<M>>,
}

impl<M: Mode> TupleConstructionInput<M> {
    /// Collects positional owned values without validating or extracting them.
    pub fn new<I>(values: I) -> Self
    where
        I: IntoIterator<Item = DynamicOwned<M>>,
    {
        Self {
            values: values.into_iter().collect(),
        }
    }

    /// Returns values in their original positional order.
    pub fn values(&self) -> &[DynamicOwned<M>] {
        &self.values
    }

    /// Extracts raw positional values after successful validation.
    pub(crate) fn into_values(self) -> Vec<DynamicOwned<M>> {
        self.values
    }

    /// Converts untouched values into a recovery payload.
    pub(crate) fn into_recovery(self, error: crate::construct::ConstructionError) -> ConstructionRecovery<M> {
        let values = self
            .values
            .into_iter()
            .enumerate()
            .map(|(index, value)| RecoveredConstructionValue::Positional { index, value })
            .collect();
        ConstructionRecovery::new(error, values)
    }
}

impl<M: Mode> fmt::Debug for TupleConstructionInput<M> {
    /// Formats the value count without requiring erased values to be `Debug`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TupleConstructionInput")
            .field("value_count", &self.values.len())
            .finish()
    }
}

/// An exact owned base value and named whole-field update overrides.
pub struct StructUpdateInput<M: Mode> {
    base: DynamicOwned<M>,
    overrides: NamedConstructionInput<M>,
}

impl<M: Mode> StructUpdateInput<M> {
    /// Collects an owned base and overrides without mutating or extracting
    /// them.
    pub const fn new(base: DynamicOwned<M>, overrides: NamedConstructionInput<M>) -> Self {
        Self { base, overrides }
    }

    /// Returns the untouched owned base value.
    pub const fn base(&self) -> &DynamicOwned<M> {
        &self.base
    }

    /// Returns overrides in their original caller order.
    pub const fn overrides(&self) -> &NamedConstructionInput<M> {
        &self.overrides
    }

    /// Extracts input parts after all validation succeeds.
    pub(crate) fn into_parts(self) -> (DynamicOwned<M>, NamedConstructionInput<M>) {
        (self.base, self.overrides)
    }

    /// Converts the untouched base and overrides into ordered recovery values.
    pub(crate) fn into_recovery(self, error: crate::construct::ConstructionError) -> ConstructionRecovery<M> {
        let mut values = Vec::with_capacity(1 + self.overrides.fields.len());
        values.push(RecoveredConstructionValue::Base(self.base));
        values.extend(
            self.overrides
                .fields
                .into_iter()
                .map(|(name, value)| RecoveredConstructionValue::Named { name, value }),
        );
        ConstructionRecovery::new(error, values)
    }
}

impl<M: Mode> fmt::Debug for StructUpdateInput<M> {
    /// Formats override names without formatting the erased base or values.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StructUpdateInput")
            .field("override_count", &self.overrides.fields.len())
            .finish()
    }
}
