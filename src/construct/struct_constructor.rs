//! Struct construction adapter contract and descriptor-bound dispatch.

use std::any::TypeId;
use std::fmt;

use crate::construct::{
    ConstructionError, ConstructionField, ConstructionRecovery, ConstructionShape,
    NamedConstructionInput, TupleConstructionInput, ValidatedConstructionInput,
};
use crate::descriptor::{StructKind, TypeDescriptor};
use crate::value::{DynamicOwned, Local, Mode, ThreadSafe};

/// A mode-specific safe adapter generated inside the declaring struct module.
pub type StructConstructionAdapter<M> = fn(ValidatedConstructionInput<M>) -> DynamicOwned<M>;

/// A descriptor-bound two-phase constructor for one concrete struct root.
pub struct StructConstructor<M: Mode + 'static> {
    descriptor: &'static TypeDescriptor,
    fields: &'static [ConstructionField<M>],
    adapter: StructConstructionAdapter<M>,
}

impl<M: Mode + 'static> StructConstructor<M> {
    /// Creates a constructor from generated immutable descriptor data.
    ///
    /// `fields` must correspond to the descriptor's direct fields in source
    /// order, and `adapter` must return the descriptor's exact root type.
    #[doc(hidden)]
    pub const fn new(
        descriptor: &'static TypeDescriptor,
        fields: &'static [ConstructionField<M>],
        adapter: StructConstructionAdapter<M>,
    ) -> Self {
        Self {
            descriptor,
            fields,
            adapter,
        }
    }

    /// Returns the concrete struct root descriptor.
    pub const fn descriptor(&self) -> &'static TypeDescriptor {
        self.descriptor
    }

    /// Returns field construction policies in source declaration order.
    pub const fn fields(&self) -> &'static [ConstructionField<M>] {
        self.fields
    }

    /// Returns the shape required by this struct constructor.
    pub fn shape(&self) -> ConstructionShape {
        match self
            .descriptor
            .as_struct()
            .unwrap_or_else(|| panic!("a StructConstructor requires a struct descriptor"))
            .kind()
        {
            StructKind::Named => ConstructionShape::Named,
            StructKind::Tuple | StructKind::Newtype => ConstructionShape::Tuple,
            StructKind::Unit => ConstructionShape::Unit,
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

    /// Invokes generated code and enforces its exact-root output invariant.
    fn execute(
        &self,
        validated: ValidatedConstructionInput<M>,
        value_type_id: fn(&DynamicOwned<M>) -> TypeId,
    ) -> DynamicOwned<M> {
        let output = (self.adapter)(validated);
        assert_eq!(
            value_type_id(&output),
            self.descriptor.type_id(),
            "a struct construction adapter must return its exact declared root type"
        );
        output
    }

    /// Enforces generated descriptor/field alignment before accepting input.
    fn assert_descriptor_contract(&self) {
        assert!(
            self.descriptor.as_struct().is_some(),
            "a StructConstructor requires a struct descriptor"
        );
        assert_eq!(
            self.descriptor.fields().len(),
            self.fields.len(),
            "construction policy must cover every direct struct field"
        );
        for (descriptor_field, construction_field) in
            self.descriptor.fields().iter().zip(self.fields)
        {
            assert!(
                std::ptr::eq(descriptor_field, construction_field.descriptor()),
                "construction policy fields must be the descriptor's own fields"
            );
        }
    }
}

impl StructConstructor<Local> {
    /// Validates and constructs a local named struct.
    pub fn construct_named(
        &self,
        input: NamedConstructionInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_named_with(input, local_type_id)
    }

    /// Validates and constructs a local tuple or newtype struct.
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<Local>,
    ) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_tuple_with(input, local_type_id)
    }

    /// Validates and constructs a local unit struct.
    pub fn construct_unit(&self) -> Result<DynamicOwned<Local>, ConstructionRecovery<Local>> {
        self.construct_unit_with(local_type_id)
    }
}

impl StructConstructor<ThreadSafe> {
    /// Validates and constructs a thread-safe named struct.
    pub fn construct_named(
        &self,
        input: NamedConstructionInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_named_with(input, thread_safe_type_id)
    }

    /// Validates and constructs a thread-safe tuple or newtype struct.
    pub fn construct_tuple(
        &self,
        input: TupleConstructionInput<ThreadSafe>,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_tuple_with(input, thread_safe_type_id)
    }

    /// Validates and constructs a thread-safe unit struct.
    pub fn construct_unit(
        &self,
    ) -> Result<DynamicOwned<ThreadSafe>, ConstructionRecovery<ThreadSafe>> {
        self.construct_unit_with(thread_safe_type_id)
    }
}

impl<M: Mode + 'static> fmt::Debug for StructConstructor<M> {
    /// Formats descriptor and policy facts without exposing adapter addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StructConstructor")
            .field("descriptor", &self.descriptor.type_name())
            .field("shape", &self.shape())
            .field("fields", &self.fields)
            .finish()
    }
}

/// Returns the exact local erased value type identity.
pub(crate) fn local_type_id(value: &DynamicOwned<Local>) -> TypeId {
    value
        .as_any()
        .map(std::any::Any::type_id)
        .unwrap_or_else(|| unreachable!("owned local values are Any-compatible"))
}

/// Returns the exact thread-safe erased value type identity.
pub(crate) fn thread_safe_type_id(value: &DynamicOwned<ThreadSafe>) -> TypeId {
    value
        .as_any()
        .map(std::any::Any::type_id)
        .unwrap_or_else(|| unreachable!("owned thread-safe values are Any-compatible"))
}
