use std::any::TypeId;

use crate::runtime::identity::TypeIdentity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeMismatch {
    expected: TypeIdentity,
    actual: TypeIdentity,
}

impl TypeMismatch {
    pub(crate) fn new(expected: TypeIdentity, actual: TypeIdentity) -> Self {
        Self { expected, actual }
    }

    pub(crate) fn expected(&self) -> &TypeIdentity {
        &self.expected
    }
    pub(crate) fn actual(&self) -> &TypeIdentity {
        &self.actual
    }
}

impl std::fmt::Display for TypeMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "type mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for TypeMismatch {}

#[must_use]
pub(crate) struct DowncastFailure<V> {
    mismatch: TypeMismatch,
    value: V,
}

impl<V> DowncastFailure<V> {
    pub(crate) fn new(expected: TypeIdentity, actual: TypeIdentity, value: V) -> Self {
        Self {
            mismatch: TypeMismatch::new(expected, actual),
            value,
        }
    }

    pub(crate) fn expected(&self) -> TypeId {
        self.mismatch.expected().type_id
    }
    pub(crate) fn actual(&self) -> TypeId {
        self.mismatch.actual().type_id
    }
    pub(crate) fn value(&self) -> &V {
        &self.value
    }
    pub(crate) fn into_value(self) -> V {
        self.value
    }
    pub(crate) fn into_parts(self) -> (TypeMismatch, V) {
        (self.mismatch, self.value)
    }
}

impl<V> std::fmt::Debug for DowncastFailure<V> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DowncastFailure")
            .field("mismatch", &self.mismatch)
            .field("value", &std::any::type_name::<V>())
            .finish()
    }
}
