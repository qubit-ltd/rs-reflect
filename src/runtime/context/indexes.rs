use std::any::TypeId;
use std::collections::HashMap;

use super::registration::TypeRecord;

#[derive(Default)]
pub(crate) struct ContextIndexes {
    pub(crate) types_by_id: HashMap<TypeId, usize>,
    pub(crate) types_by_name: HashMap<Box<str>, Box<[usize]>>,
    pub(crate) types_by_short_name: HashMap<Box<str>, Box<[usize]>>,
}

impl ContextIndexes {
    pub(crate) fn build(types: &[TypeRecord]) -> Self {
        let mut types_by_id = HashMap::with_capacity(types.len());
        let mut types_by_name: HashMap<Box<str>, Vec<usize>> = HashMap::new();
        let mut types_by_short_name: HashMap<Box<str>, Vec<usize>> = HashMap::new();

        for (index, metadata) in types.iter().enumerate() {
            types_by_id.insert(metadata.type_id(), index);
            let name = metadata.rust_name();
            types_by_name.entry(name.into()).or_default().push(index);
            types_by_short_name
                .entry(short_name(name).into())
                .or_default()
                .push(index);
        }

        Self {
            types_by_id,
            types_by_name: freeze_candidates(types_by_name),
            types_by_short_name: freeze_candidates(types_by_short_name),
        }
    }
}

fn freeze_candidates(index: HashMap<Box<str>, Vec<usize>>) -> HashMap<Box<str>, Box<[usize]>> {
    index
        .into_iter()
        .map(|(name, candidates)| (name, candidates.into_boxed_slice()))
        .collect()
}

fn short_name(name: &str) -> &str {
    let base = name.split('<').next().unwrap_or(name);
    base.rsplit("::").next().unwrap_or(base)
}
