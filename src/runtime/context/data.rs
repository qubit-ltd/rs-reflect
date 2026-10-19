use std::any::TypeId;
use std::collections::HashMap;

use crate::runtime::error::{ContextError, ContextIssue};

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

        let mut registration_group_by_type = HashMap::with_capacity(registrations.len());
        let mut origins_by_type = Vec::<(TypeId, Vec<_>)>::with_capacity(registrations.len());
        let mut types = Vec::with_capacity(registrations.len());
        for registration in registrations {
            let type_record = registration.type_record();
            let type_id = type_record.type_id();
            let index = *registration_group_by_type.entry(type_id).or_insert_with(|| {
                let index = origins_by_type.len();
                origins_by_type.push((type_id, Vec::new()));
                index
            });
            let origins = &mut origins_by_type[index].1;
            origins.push(registration.origin().clone());
            if origins.len() == 1 {
                types.push(type_record);
            }
        }

        let issues: Box<[_]> = origins_by_type
            .into_iter()
            .filter_map(|(type_id, origins)| {
                (origins.len() > 1).then(|| ContextIssue::DuplicateTypeIdentity {
                    type_id,
                    registrations: origins.into_boxed_slice(),
                })
            })
            .collect();
        if let Some(error) = ContextError::from_issues(issues) {
            return Err(error);
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
