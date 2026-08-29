//! Immutable structural facts about reflected fields.

use std::fmt;

use crate::access::FieldAccessError;
use crate::access::FieldAccessOperation;
use crate::access::FieldAccessPolicy;
use crate::access::FieldGetAdapter;
use crate::access::FieldGetMutAdapter;
use crate::access::FieldIdentity;
use crate::access::FieldSetAdapter;
use crate::access::FieldVisibility;
use crate::access::field_adapter::dynamic_mut_type_id;
use crate::access::field_adapter::dynamic_owned_type_id;
use crate::access::field_adapter::dynamic_ref_type_id;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeRef;
use crate::error::TypeMismatch;
use crate::identity::Visibility;
use crate::value::ReflectedMut;
use crate::value::ReflectedOwned;
use crate::value::ReflectedRef;

/// A function that resolves a field or variant's declaring root descriptor.
///
/// Resolver indirection permits immutable static descriptor graphs to contain
/// cycles.
pub type TypeDescriptorResolver = fn() -> &'static TypeDescriptor;

/// The immutable structural description of one reflected field.
pub struct FieldDescriptor {
    declaring_type: TypeDescriptorResolver,
    index: usize,
    rust_name: Option<&'static str>,
    query_name: Option<&'static str>,
    field_type: &'static TypeRef,
    visibility: Visibility,
    variant_index: Option<usize>,
    variant_rust_name: Option<&'static str>,
    access_policy: FieldAccessPolicy,
    get: Option<FieldGetAdapter>,
    get_mut: Option<FieldGetMutAdapter>,
    set: Option<FieldSetAdapter>,
}

impl FieldDescriptor {
    /// Creates a frozen field descriptor for generated or hand-written
    /// descriptor data.
    ///
    /// The resolver must return the root containing this field. Positional
    /// fields use `None` for both names. This constructor performs no
    /// allocation and never calls the resolver.
    #[doc(hidden)]
    pub(crate) const fn new(
        declaring_type: TypeDescriptorResolver,
        index: usize,
        rust_name: Option<&'static str>,
        query_name: Option<&'static str>,
        field_type: &'static TypeRef,
        visibility: Visibility,
    ) -> Self {
        Self {
            declaring_type,
            index,
            rust_name,
            query_name,
            field_type,
            visibility,
            variant_index: None,
            variant_rust_name: None,
            access_policy: FieldAccessPolicy::ReadWrite,
            get: None,
            get_mut: None,
            set: None,
        }
    }

    /// Attaches safe generated adapters and their source access policy.
    ///
    /// Descriptors without adapters remain useful for structural navigation;
    /// their dynamic operations return [`FieldAccessError::Unavailable`]. The
    /// policy is enforced before any adapter is invoked.
    #[doc(hidden)]
    pub const fn with_access(
        mut self,
        access_policy: FieldAccessPolicy,
        get: Option<FieldGetAdapter>,
        get_mut: Option<FieldGetMutAdapter>,
        set: Option<FieldSetAdapter>,
    ) -> Self {
        self.access_policy = access_policy;
        self.get = get;
        self.get_mut = get_mut;
        self.set = set;
        self
    }

    /// Marks this field as belonging to one enum variant.
    ///
    /// The variant source index and Rust name become part of the field's
    /// runtime identity, so fields at equal positions in different variants
    /// remain distinct.
    #[doc(hidden)]
    pub const fn with_variant(mut self, variant_index: usize, variant_rust_name: &'static str) -> Self {
        self.variant_index = Some(variant_index);
        self.variant_rust_name = Some(variant_rust_name);
        self
    }

    /// Returns the root type that contains this field.
    pub fn declaring_type(&self) -> &'static TypeDescriptor {
        (self.declaring_type)()
    }

    /// Returns the zero-based source declaration index.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust field name, or `None` for tuple and newtype fields.
    pub const fn rust_name(&self) -> Option<&'static str> {
        self.rust_name
    }

    /// Returns the lookup name, or `None` for tuple and newtype fields.
    pub const fn query_name(&self) -> Option<&'static str> {
        self.query_name
    }

    /// Returns the resolved, explicitly opaque, or symbolic field type.
    pub const fn field_type(&self) -> &'static TypeRef {
        self.field_type
    }

    /// Returns declared struct-field visibility or the explicit fact that an
    /// enum-variant field inherits its enclosing access boundary.
    pub const fn visibility(&self) -> FieldVisibility<'_> {
        match self.variant_index {
            Some(_) => FieldVisibility::VariantInherited,
            None => FieldVisibility::Declared(&self.visibility),
        }
    }

    /// Returns the source policy controlling this field's dynamic adapters.
    pub const fn access_policy(&self) -> FieldAccessPolicy {
        self.access_policy
    }

    /// Returns the containing variant's source index for an enum field.
    ///
    /// `None` identifies a direct struct field.
    pub const fn variant_index(&self) -> Option<usize> {
        self.variant_index
    }

    /// Returns the containing variant's Rust name for an enum field.
    ///
    /// `None` identifies a direct struct field.
    pub const fn variant_rust_name(&self) -> Option<&'static str> {
        self.variant_rust_name
    }

    /// Reads this field from an exact declaring-type target.
    ///
    /// Returns a target mismatch, policy, or unavailable-adapter error before
    /// invoking generated code. An enum-field adapter may additionally report
    /// [`FieldAccessError::InactiveVariant`]. The returned borrow cannot
    /// outlive `target`.
    pub fn get<'a>(&self, target: ReflectedRef<'a>) -> Result<ReflectedRef<'a>, FieldAccessError> {
        self.validate_shared_target(&target)?;
        self.validate_policy(FieldAccessOperation::Get)?;
        let adapter = self.get.ok_or_else(|| self.unavailable(FieldAccessOperation::Get))?;
        adapter(target)
    }

    /// Mutably borrows this field from an exact declaring-type target.
    ///
    /// Returns a target mismatch, read-only/skipped policy, or unavailable-
    /// adapter error before invoking generated code. The returned exclusive
    /// borrow cannot outlive `target`.
    pub fn get_mut<'a>(&self, target: ReflectedMut<'a>) -> Result<ReflectedMut<'a>, FieldAccessError> {
        self.validate_mutable_target(&target)?;
        self.validate_policy(FieldAccessOperation::GetMut)?;
        let adapter = self
            .get_mut
            .ok_or_else(|| self.unavailable(FieldAccessOperation::GetMut))?;
        adapter(target)
    }

    /// Replaces this field using an exact declaring-type target and exact
    /// declared field value.
    ///
    /// All target, policy, value, and adapter-availability checks complete
    /// before generated code is invoked, so these failures do not modify the
    /// target. A symbolic definition-level field has no exact runtime identity
    /// and therefore returns [`FieldAccessError::Unavailable`].
    pub fn set(&self, target: ReflectedMut<'_>, value: ReflectedOwned) -> Result<(), FieldAccessError> {
        self.validate_mutable_target(&target)?;
        self.validate_policy(FieldAccessOperation::Set)?;
        let (expected, _) = self
            .concrete_field_identity()
            .ok_or_else(|| self.unavailable(FieldAccessOperation::Set))?;
        let actual = dynamic_owned_type_id(&value);
        if actual != expected {
            return Err(FieldAccessError::ValueTypeMismatch {
                field: self.identity(),
                mismatch: Box::new(TypeMismatch::new(expected, actual)),
            });
        }
        let adapter = self.set.ok_or_else(|| self.unavailable(FieldAccessOperation::Set))?;
        adapter(target, value)
    }

    /// Validates a shared target without consuming or changing it.
    fn validate_shared_target(&self, target: &ReflectedRef<'_>) -> Result<(), FieldAccessError> {
        let expected = self.declaring_type().type_id();
        let actual = dynamic_ref_type_id(target);
        self.validate_target_identity(expected, actual)
    }

    /// Validates a mutable target without consuming or changing it.
    fn validate_mutable_target(&self, target: &ReflectedMut<'_>) -> Result<(), FieldAccessError> {
        let expected = self.declaring_type().type_id();
        let actual = dynamic_mut_type_id(target);
        self.validate_target_identity(expected, actual)
    }

    /// Compares exact target identities and attaches this field's identity to
    /// a mismatch.
    fn validate_target_identity(
        &self,
        expected: std::any::TypeId,
        actual: std::any::TypeId,
    ) -> Result<(), FieldAccessError> {
        if actual == expected {
            Ok(())
        } else {
            Err(FieldAccessError::TargetTypeMismatch {
                field: self.identity(),
                mismatch: Box::new(TypeMismatch::new(expected, actual)),
            })
        }
    }

    /// Enforces skip and read-only policy before dispatching an adapter.
    fn validate_policy(&self, operation: FieldAccessOperation) -> Result<(), FieldAccessError> {
        match (self.access_policy, operation) {
            (FieldAccessPolicy::Skipped, _) => Err(FieldAccessError::Skipped {
                field: self.identity(),
                operation,
            }),
            (FieldAccessPolicy::ReadOnly, FieldAccessOperation::GetMut | FieldAccessOperation::Set) => {
                Err(FieldAccessError::ReadOnly {
                    field: self.identity(),
                    operation,
                })
            }
            (FieldAccessPolicy::ReadWrite | FieldAccessPolicy::ReadOnly, _) => Ok(()),
        }
    }

    /// Returns the exact runtime identity and diagnostic name for a concrete
    /// resolved or explicitly opaque field.
    fn concrete_field_identity(&self) -> Option<(std::any::TypeId, &'static str)> {
        match self.field_type {
            TypeRef::Resolved(descriptor) => Some((descriptor.type_id(), descriptor.type_name())),
            TypeRef::Opaque(descriptor) => Some((descriptor.type_id(), descriptor.type_name())),
            TypeRef::Symbolic(_) => None,
        }
    }

    /// Builds the source identity included in every field access error.
    fn identity(&self) -> FieldIdentity {
        let declaring_type = self.declaring_type();
        match (self.variant_index, self.variant_rust_name) {
            (Some(variant_index), Some(variant_rust_name)) => FieldIdentity::new_variant(
                declaring_type.type_id(),
                declaring_type.type_name(),
                self.index,
                self.rust_name,
                variant_index,
                variant_rust_name,
            ),
            (None, None) => FieldIdentity::new(
                declaring_type.type_id(),
                declaring_type.type_name(),
                self.index,
                self.rust_name,
            ),
            _ => unreachable!("variant field identity is assigned atomically"),
        }
    }

    /// Builds an unavailable-adapter error for `operation`.
    fn unavailable(&self, operation: FieldAccessOperation) -> FieldAccessError {
        FieldAccessError::Unavailable {
            field: self.identity(),
            operation,
        }
    }
}

impl fmt::Debug for FieldDescriptor {
    /// Formats local facts without following declaring or field-type
    /// relationships recursively.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FieldDescriptor")
            .field("declaring_type", &"<resolver>")
            .field("index", &self.index)
            .field("rust_name", &self.rust_name)
            .field("query_name", &self.query_name)
            .field("field_type", &self.field_type)
            .field("visibility", &self.visibility)
            .field("variant_index", &self.variant_index)
            .field("variant_rust_name", &self.variant_rust_name)
            .field("access_policy", &self.access_policy)
            .field("has_get", &self.get.is_some())
            .field("has_get_mut", &self.get_mut.is_some())
            .field("has_set", &self.set.is_some())
            .finish()
    }
}
