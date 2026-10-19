use std::any::TypeId;
use std::collections::HashSet;

use crate::runtime::error::ContextError;

use super::indexes::ContextIndexes;
use super::registration::{Registration, TypeRecord};

pub(crate) struct ContextData {
    pub(crate) types: Box<[TypeRecord]>,
    pub(crate) indexes: ContextIndexes,
}

impl ContextData {
    pub(crate) fn empty() -> Self {
        Self {
            types: Box::new([]),
            indexes: ContextIndexes::default(),
        }
    }

    pub(crate) fn build(mut registrations: Vec<Registration>) -> Result<Self, ContextError> {
        registrations.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));

        let mut seen = HashSet::with_capacity(registrations.len());
        let mut types = Vec::with_capacity(registrations.len());
        for registration in registrations {
            let type_record = registration.type_record();
            let type_id = type_record.type_id();
            if !seen.insert(type_id) {
                return Err(ContextError::new(format!(
                    "multiple registrations claim Rust type {}",
                    type_record.rust_name(),
                )));
            }
            types.push(type_record);
        }

        let types = types.into_boxed_slice();
        let indexes = ContextIndexes::build(&types);
        Ok(Self { types, indexes })
    }

    pub(crate) fn type_by_id(&self, type_id: TypeId) -> Option<TypeRecord> {
        self.indexes
            .types_by_id
            .get(&type_id)
            .map(|index| self.types[*index])
    }
}
