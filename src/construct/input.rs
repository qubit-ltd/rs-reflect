// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Caller-owned construction input and per-field policy declarations.

use std::fmt;

use crate::construct::ConstructionRecovery;
use crate::construct::RecoveredConstructionValue;
use crate::descriptor::FieldDescriptor;
use crate::value::DynamicOwned;
use crate::value::Mode;

/// A mode-specific explicit provider for one omitted field value.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of values produced by the provider.
pub type ConstructionDefaultProvider<M> = fn() -> DynamicOwned<M>;

/// Runtime construction policy generated for one declared field.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode accepted by the policy providers.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::ConstructionFieldPolicy;
/// use qubit_reflect::value::Local;
///
/// let policy = ConstructionFieldPolicy::<Local>::Required;
/// assert!(matches!(policy, ConstructionFieldPolicy::Required));
/// ```
#[must_use]
pub enum ConstructionFieldPolicy<M: Mode> {
    /// The caller must supply this field during from-zero construction.
    Required,
    /// A missing value is produced only by this explicit provider.
    Default(
        /// Creates the value when the caller omits this field.
        ConstructionDefaultProvider<M>,
    ),
    /// The field is always produced by this provider and rejects caller input.
    ProviderOnly(
        /// Creates the value that callers are forbidden to provide.
        ConstructionDefaultProvider<M>,
    ),
    /// The generated construction path cannot safely supply this field.
    Unavailable(
        /// Stable reason the generated constructor cannot supply the field.
        crate::construct::ConstructionUnavailableReason,
    ),
}

impl<M: Mode> Clone for ConstructionFieldPolicy<M> {
    /// Copies the immutable generated policy.
    ///
    /// # Returns
    ///
    /// Returns the same field policy and provider function.
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Mode> Copy for ConstructionFieldPolicy<M> {}

impl<M: Mode> fmt::Debug for ConstructionFieldPolicy<M> {
    /// Formats policy facts without exposing provider addresses.
    ///
    /// # Parameters
    ///
    /// - `formatter`: Destination receiving the policy description.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the policy.
    ///
    /// # Errors
    ///
    /// Returns the formatter's error if it cannot accept the output.
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
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode accepted by the generated constructor.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::construct::ConstructionField;
/// use qubit_reflect::value::Local;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct User {
///     name: String,
/// }
///
/// let descriptor = TypeDescriptor::of::<User>().field("name").expect("field exists");
/// let policy = ConstructionField::<Local>::required(descriptor);
/// assert_eq!(policy.descriptor().query_name(), Some("name"));
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[must_use]
pub struct ConstructionField<M: Mode> {
    /// The immutable reflected field whose construction policy is declared.
    descriptor: &'static FieldDescriptor,
    /// The generated rule for supplying this field during construction.
    policy: ConstructionFieldPolicy<M>,
}

impl<M: Mode> Clone for ConstructionField<M> {
    /// Copies the immutable descriptor reference and policy.
    ///
    /// # Returns
    ///
    /// Returns a copy of this field and its generated policy.
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: Mode> Copy for ConstructionField<M> {}

impl<M: Mode> ConstructionField<M> {
    /// Declares a field that must be present in from-zero construction input.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The field descriptor owned by the target root.
    ///
    /// # Returns
    ///
    /// Returns a required-field construction policy.
    #[inline]
    pub const fn required(descriptor: &'static FieldDescriptor) -> Self {
        Self {
            descriptor,
            policy: ConstructionFieldPolicy::Required,
        }
    }

    /// Declares a field with an explicit generated default provider.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The field descriptor owned by the target root.
    /// - `provider`: The function that creates the omitted field value.
    ///
    /// # Returns
    ///
    /// Returns a policy that accepts caller input or invokes `provider`.
    #[inline]
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
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The field descriptor owned by the target root.
    /// - `provider`: The function that supplies this field during construction.
    ///
    /// # Returns
    ///
    /// Returns a policy that rejects caller input and invokes `provider`.
    #[inline]
    pub const fn provider_only(descriptor: &'static FieldDescriptor, provider: ConstructionDefaultProvider<M>) -> Self {
        Self {
            descriptor,
            policy: ConstructionFieldPolicy::ProviderOnly(provider),
        }
    }

    /// Marks a field as blocking this generated construction path.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The field descriptor owned by the target root.
    /// - `reason`: The stable reason the generated path is unavailable.
    ///
    /// # Returns
    ///
    /// Returns an unavailable-field policy.
    #[inline]
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
    ///
    /// # Returns
    ///
    /// Returns the exact descriptor used to declare this policy.
    #[must_use]
    #[inline]
    pub const fn descriptor(&self) -> &'static FieldDescriptor {
        self.descriptor
    }

    /// Returns the explicit generated construction policy.
    ///
    /// # Returns
    ///
    /// Returns the copyable policy and its provider, when present.
    #[must_use]
    #[inline]
    pub const fn policy(&self) -> ConstructionFieldPolicy<M> {
        self.policy
    }
}

impl<M: Mode> fmt::Debug for ConstructionField<M> {
    /// Formats the descriptor-local field index and construction policy.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the field metadata.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
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
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::UpdateFieldPolicy;
///
/// let policy = UpdateFieldPolicy::Allowed;
/// assert_eq!(policy, UpdateFieldPolicy::Allowed);
/// ```
#[must_use]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UpdateFieldPolicy {
    /// The generated updater accepts an exact whole-field replacement.
    Allowed,
    /// The generated updater must reject replacement of this field.
    Unavailable(
        /// Stable reason the generated updater cannot replace the field.
        crate::construct::ConstructionUnavailableReason,
    ),
}

/// A declared field paired with its independent owned-update policy.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// #[cfg(feature = "derive")]
/// fn main() {
/// use qubit_reflect::construct::UpdateField;
/// use qubit_reflect::Reflect;
/// use qubit_reflect::TypeDescriptor;
///
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct User {
///     name: String,
/// }
///
/// let field = TypeDescriptor::of::<User>().field("name").expect("field exists");
/// let update = UpdateField::allowed(field);
/// assert_eq!(update.descriptor().query_name(), Some("name"));
/// }
/// #[cfg(not(feature = "derive"))]
/// fn main() {}
/// ```
#[derive(Clone, Copy)]
#[must_use]
pub struct UpdateField {
    /// The immutable reflected field whose update policy is declared.
    descriptor: &'static FieldDescriptor,
    /// Whether exact whole-field replacement is available.
    policy: UpdateFieldPolicy,
}

impl UpdateField {
    /// Declares a field whose exact whole value may be replaced by update.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The field descriptor owned by the target root.
    ///
    /// # Returns
    ///
    /// Returns an update policy that permits whole-field replacement.
    #[inline]
    pub const fn allowed(descriptor: &'static FieldDescriptor) -> Self {
        Self {
            descriptor,
            policy: UpdateFieldPolicy::Allowed,
        }
    }

    /// Declares a field that the generated update path cannot replace.
    ///
    /// # Parameters
    ///
    /// - `descriptor`: The field descriptor owned by the target root.
    /// - `reason`: The stable reason replacement is unavailable.
    ///
    /// # Returns
    ///
    /// Returns an update policy that rejects replacement.
    #[inline]
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
    ///
    /// # Returns
    ///
    /// Returns the exact descriptor used to declare this policy.
    #[must_use]
    #[inline]
    pub const fn descriptor(&self) -> &'static FieldDescriptor {
        self.descriptor
    }

    /// Returns the independent generated update policy.
    ///
    /// # Returns
    ///
    /// Returns the copyable update policy.
    #[must_use]
    #[inline]
    pub const fn policy(&self) -> UpdateFieldPolicy {
        self.policy
    }
}

impl fmt::Debug for UpdateField {
    /// Formats the field index and update policy.
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the field metadata.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
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
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the supplied values.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// let input = NamedConstructionInput::<Local>::new([
///     ("name", DynamicOwned::<Local>::new(String::from("Ada"))),
/// ]);
/// assert_eq!(input.fields()[0].0.as_ref(), "name");
/// ```
pub struct NamedConstructionInput<M: Mode> {
    /// Caller-supplied names and values, preserved in insertion order.
    fields: Vec<(Box<str>, DynamicOwned<M>)>,
}

impl<M: Mode> NamedConstructionInput<M> {
    /// Collects named owned values in caller order without validating them.
    ///
    /// # Type Parameters
    ///
    /// - `I`: The input iterator type.
    /// - `N`: The field-name type converted into `Box<str>`.
    ///
    /// # Parameters
    ///
    /// - `fields`: The caller-ordered query names and owned values.
    ///
    /// # Returns
    ///
    /// Returns input ready for later shape, name, and type validation.
    #[must_use]
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
    ///
    /// # Returns
    ///
    /// Returns each original query name and its untouched dynamic value.
    #[must_use]
    #[inline]
    pub fn fields(&self) -> &[(Box<str>, DynamicOwned<M>)] {
        &self.fields
    }

    /// Extracts the raw ordered bindings for validation or recovery.
    ///
    /// # Returns
    ///
    /// Returns the owned bindings in their original caller order.
    pub(crate) fn into_fields(self) -> Vec<(Box<str>, DynamicOwned<M>)> {
        self.fields
    }

    /// Converts untouched bindings into a recovery payload.
    ///
    /// # Parameters
    ///
    /// - `error`: The validation error associated with these bindings.
    ///
    /// # Returns
    ///
    /// Returns recovery owning the error and every named input value.
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the binding names.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NamedConstructionInput")
            .field("names", &self.fields.iter().map(|(name, _)| name).collect::<Vec<_>>())
            .finish()
    }
}

/// Source-ordered owned values collected for tuple or newtype construction.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode of the supplied values.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::TupleConstructionInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// let input = TupleConstructionInput::<Local>::new([
///     DynamicOwned::<Local>::new(7_u32),
///     DynamicOwned::<Local>::new(String::from("seven")),
/// ]);
/// assert_eq!(input.values().len(), 2);
/// ```
pub struct TupleConstructionInput<M: Mode> {
    /// Caller-supplied values, preserved in positional order.
    values: Vec<DynamicOwned<M>>,
}

impl<M: Mode> TupleConstructionInput<M> {
    /// Collects positional owned values without validating or extracting them.
    ///
    /// # Type Parameters
    ///
    /// - `I`: The iterator yielding positional values.
    ///
    /// # Parameters
    ///
    /// - `values`: The source-ordered dynamic values.
    ///
    /// # Returns
    ///
    /// Returns input ready for later shape and type validation.
    #[must_use]
    pub fn new<I>(values: I) -> Self
    where
        I: IntoIterator<Item = DynamicOwned<M>>,
    {
        Self {
            values: values.into_iter().collect(),
        }
    }

    /// Returns values in their original positional order.
    ///
    /// # Returns
    ///
    /// Returns the untouched dynamic values in source order.
    #[must_use]
    #[inline]
    pub fn values(&self) -> &[DynamicOwned<M>] {
        &self.values
    }

    /// Extracts raw positional values after successful validation.
    ///
    /// # Returns
    ///
    /// Returns the owned positional values in their original order.
    pub(crate) fn into_values(self) -> Vec<DynamicOwned<M>> {
        self.values
    }

    /// Converts untouched values into a recovery payload.
    ///
    /// # Parameters
    ///
    /// - `error`: The validation error associated with these values.
    ///
    /// # Returns
    ///
    /// Returns recovery owning the error and each positional value.
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the value count.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TupleConstructionInput")
            .field("value_count", &self.values.len())
            .finish()
    }
}

/// An exact owned base value and named whole-field update overrides.
///
/// # Type Parameters
///
/// - `M`: The dynamic ownership mode shared by the base and override values.
///
/// # Examples
///
/// ```
/// use qubit_reflect::construct::NamedConstructionInput;
/// use qubit_reflect::construct::StructUpdateInput;
/// use qubit_reflect::value::DynamicOwned;
/// use qubit_reflect::value::Local;
///
/// let base = DynamicOwned::<Local>::new((1_u32, String::from("before")));
/// let overrides = NamedConstructionInput::new([
///     ("name", DynamicOwned::<Local>::new(String::from("after"))),
/// ]);
/// let input = StructUpdateInput::new(base, overrides);
/// assert_eq!(input.overrides().fields().len(), 1);
/// ```
pub struct StructUpdateInput<M: Mode> {
    /// Exact owned root value whose fields may be replaced.
    base: DynamicOwned<M>,
    /// Caller-supplied whole-field replacements in their original order.
    overrides: NamedConstructionInput<M>,
}

impl<M: Mode> StructUpdateInput<M> {
    /// Collects an owned base and overrides without mutating or extracting
    /// them.
    ///
    /// # Parameters
    ///
    /// - `base`: The exact owned struct value to update.
    /// - `overrides`: Named whole-field replacements in caller order.
    ///
    /// # Returns
    ///
    /// Returns an unvalidated update request that retains both inputs.
    #[must_use]
    #[inline]
    pub const fn new(base: DynamicOwned<M>, overrides: NamedConstructionInput<M>) -> Self {
        Self { base, overrides }
    }

    /// Returns the untouched owned base value.
    ///
    /// # Returns
    ///
    /// Returns a borrow of the owned update base.
    #[must_use]
    #[inline]
    pub const fn base(&self) -> &DynamicOwned<M> {
        &self.base
    }

    /// Returns overrides in their original caller order.
    ///
    /// # Returns
    ///
    /// Returns the named overrides without validating or consuming them.
    #[must_use]
    #[inline]
    pub const fn overrides(&self) -> &NamedConstructionInput<M> {
        &self.overrides
    }

    /// Extracts input parts after all validation succeeds.
    ///
    /// # Returns
    ///
    /// Returns the base and overrides, transferring their ownership.
    pub(crate) fn into_parts(self) -> (DynamicOwned<M>, NamedConstructionInput<M>) {
        (self.base, self.overrides)
    }

    /// Converts the untouched base and overrides into ordered recovery values.
    ///
    /// # Parameters
    ///
    /// - `error`: The validation error associated with this update request.
    ///
    /// # Returns
    ///
    /// Returns recovery with the base first and overrides afterward.
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
    ///
    /// # Parameters
    ///
    /// - `formatter`: The destination formatter.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after formatting the number of overrides.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to `formatter` fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StructUpdateInput")
            .field("override_count", &self.overrides.fields.len())
            .finish()
    }
}
