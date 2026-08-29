//! Enum-variant construction adapter contract and descriptor-bound dispatch.

use std::any::TypeId;
use std::fmt;

use crate::construct::struct_constructor::{local_type_id, thread_safe_type_id};
use crate::construct::{
    ConstructionError, ConstructionField, ConstructionRecovery, ConstructionShape,
    NamedConstructionInput, TupleConstructionInput, ValidatedConstructionInput,
};
use crate::descriptor::{VariantDescriptor, VariantKind};
use crate::value::{DynamicOwned, Local, Mode, ThreadSafe};

/// A mode-specific adapter generated inside the declaring enum module.
pub type VariantConstructionAdapter<M> = fn(ValidatedConstructionInput<M>) -> DynamicOwned<M>;

/// A descriptor-bound two-phase constructor for one concrete enum variant.
pub struct VariantConstructor<M: Mode + 'static> {
    variant: &'static VariantDescriptor,
    fields: &'static [ConstructionField<M>],
    adapter: VariantConstructionAdapter<M>,
}

impl<M: Mode + 'static> VariantConstructor<M> {
    /// Creates a variant constructor from generated immutable descriptor data.
    ///
    /// `fields` must correspond to the variant's fields in source order, and
    /// `adapter` must return the declaring enum's exact root type.
    #[doc(hidden)]
    pub const fn new(
        variant: &'static VariantDescriptor,
        fields: &'static [ConstructionField<M>],
        adapter: VariantConstructionAdapter<M>,
    ) -> Self {
        Self {
            variant,
            fields,
            adapter,
        }
    }

    /// Returns the immutable enum variant descriptor.
    pub const fn variant(&self) -> &'static VariantDescriptor {
        self.variant
    }

    /// Returns field construction policies in source declaration order.
    pub const fn fields(&self) -> &'static [ConstructionField<M>] {
        self.fields
    }

    /// Returns the input shape required by this variant.
    pub const fn shape(&self) -> ConstructionShape {
        match self.variant.kind() {
            VariantKind::Struct => ConstructionShape::Named,
            VariantKind::Tuple => ConstructionShape::Tuple,
            VariantKind::Unit => ConstructionShape::Unit,
        }
    }

    /// Executes an adapter only after named input validation succeeds.
    fn construct_named_with(
        &self,
        input: NamedConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        if self.shape() != ConstructionShape::Named {
            return Err(input.into_recovery(ConstructionError::WrongShape {
                expected: self.shape(),
                actual: ConstructionShape::Named,
            }));
        }
        let validated =
            crate::construct::validated::validate_named(input, self.fields, value_type_id)?;
        Ok(self.execute(validated, value_type_id))
    }

    /// Executes an adapter only after positional input validation succeeds.
    fn construct_tuple_with(
        &self,
        input: TupleConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        if self.shape() != ConstructionShape::Tuple {
            return Err(input.into_recovery(ConstructionError::WrongShape {
                expected: self.shape(),
                actual: ConstructionShape::Tuple,
            }));
        }
        let validated =
            crate::construct::validated::validate_tuple(input, self.fields, value_type_id)?;
        Ok(self.execute(validated, value_type_id))
    }

    /// Executes an adapter only after unit-shape validation succeeds.
    fn construct_unit_with(
        &self,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> Result<DynamicOwned<M>, ConstructionRecovery<M>> {
        self.assert_descriptor_contract();
        if self.shape() != ConstructionShape::Unit {
            return Err(ConstructionRecovery::new(
                ConstructionError::WrongShape {
                    expected: self.shape(),
                    actual: ConstructionShape::Unit,
                },
                Vec::new(),
            ));
        }
        let validated = crate::construct::validated::validate_unit(self.fields)
            .map_err(|error| ConstructionRecovery::new(error, Vec::new()))?;
        Ok(self.execute(validated, value_type_id))
    }

    /// Invokes generated code and enforces its exact enum-root output invariant.
    fn execute(
        &self,
        validated: ValidatedConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> DynamicOwned<M> {
        let output = (self.adapter)(validated);
        assert_eq!(
            value_type_id(&output),
            self.variant.declaring_type().type_id(),
            "a variant construction adapter must return its exact declaring enum type"
        );
        output
    }

    /// Enforces generated variant/field alignment before accepting input.
    fn assert_descriptor_contract(&self) {
        assert_eq!(
            self.variant.fields().len(),
            self.fields.len(),
            "construction policy must cover every variant field"
        );
        for (descriptor_field, construction_field) in self.variant.fields().iter().zip(self.fields)
        {
            assert!(
                std::ptr::eq(descriptor_field, construction_field.descriptor()),
                "construction policy fields must be the variant's own fields"
            );
        }
    }
}

impl VariantConstructor<Local> {
    /// Validates and constructs a local named enum variant.
    pub fn construct_named(
        &self,
        input: NamedConstructionInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_named_with(input, local_type_id)
    }

    /// Validates and constructs a local tuple enum variant.
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_tuple_with(input, local_type_id)
    }

    /// Validates and constructs a local unit enum variant.
    pub fn construct_unit(&self) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_unit_with(local_type_id)
    }
}

impl VariantConstructor<ThreadSafe> {
    /// Validates and constructs a thread-safe named enum variant.
    pub fn construct_named(
        &self,
        input: NamedConstructionInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_named_with(input, thread_safe_type_id)
    }

    /// Validates and constructs a thread-safe tuple enum variant.
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_tuple_with(input, thread_safe_type_id)
    }

    /// Validates and constructs a thread-safe unit enum variant.
    pub fn construct_unit(
        &self,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_unit_with(thread_safe_type_id)
    }
}

impl<M: Mode + 'static> fmt::Debug for VariantConstructor<M> {
    /// Formats variant and policy facts without exposing adapter addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VariantConstructor")
            .field("variant", &self.variant.rust_name())
            .field("shape", &self.shape())
            .field("fields", &self.fields)
            .finish()
    }
}
