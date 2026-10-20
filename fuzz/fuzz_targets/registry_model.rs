// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use std::any::TypeId;
use std::collections::BTreeMap;
use std::sync::Arc;

use libfuzzer_sys::fuzz_target;
use qubit_reflect::ReflectionContext;
use qubit_reflect::TypeDescriptor;

const MAX_INPUT_BYTES: usize = 4_096;
const MAX_OPERATIONS: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 64;

fn selected_descriptor<'ctx>(index: u8, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    match index % 16 {
        0 => TypeDescriptor::of_in::<bool>(context),
        1 => TypeDescriptor::of_in::<u8>(context),
        2 => TypeDescriptor::of_in::<u16>(context),
        3 => TypeDescriptor::of_in::<u32>(context),
        4 => TypeDescriptor::of_in::<u64>(context),
        5 => TypeDescriptor::of_in::<i64>(context),
        6 => TypeDescriptor::of_in::<String>(context),
        7 => TypeDescriptor::of_in::<Option<String>>(context),
        8 => TypeDescriptor::of_in::<Vec<u8>>(context),
        9 => TypeDescriptor::of_in::<Vec<String>>(context),
        10 => TypeDescriptor::of_in::<Option<Vec<u8>>>(context),
        11 => TypeDescriptor::of_in::<Box<u64>>(context),
        12 => TypeDescriptor::of_in::<Arc<u64>>(context),
        13 => TypeDescriptor::of_in::<(u8, String)>(context),
        14 => TypeDescriptor::of_in::<[u8; 4]>(context),
        _ => TypeDescriptor::of_in::<BTreeMap<String, u64>>(context),
    }
}

fn matching_type_ids(context: &ReflectionContext, short_name: &str) -> Vec<TypeId> {
    let mut ids = context
        .types()
        .expect("a frozen context must enumerate registered types")
        .iter()
        .filter(|descriptor| descriptor.short_name() == short_name)
        .map(|descriptor| descriptor.type_id())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn lookup_type_ids(context: &ReflectionContext, short_name: &str) -> Vec<TypeId> {
    let mut ids = context
        .lookup_type_by_short_name(short_name)
        .expect("a frozen context must answer short-name lookups")
        .candidates()
        .iter()
        .map(|descriptor| descriptor.type_id())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

// Executes a bounded, input-driven sequence over public safe descriptor and
// context APIs. Frozen indexes are compared with direct
// context scans as an independent reference model after every query action.
fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let context = ReflectionContext::global();
    context
        .initialize()
        .expect("linked fuzz-target fragments must form a valid context");
    let mut cursor = 0;
    let mut operations = 0;
    let mut selected = selected_descriptor(0, context);
    let mut last_lookup_found = false;

    while cursor < data.len() && operations < MAX_OPERATIONS {
        let command = data[cursor];
        cursor += 1;
        let requested = data.get(cursor).copied().unwrap_or_default() as usize;
        cursor = (cursor + 1).min(data.len());
        let payload_len = requested.min(MAX_PAYLOAD_BYTES).min(data.len() - cursor);
        let payload = &data[cursor..cursor + payload_len];
        cursor += payload_len;
        operations += 1;

        let query = String::from_utf8_lossy(payload);
        match command % 8 {
            0 => {
                let first = lookup_type_ids(context, query.as_ref());
                let second = lookup_type_ids(context, query.as_ref());
                last_lookup_found = !first.is_empty();
                assert_eq!(first, second, "repeated context lookups must be deterministic");
            }
            1 => {
                let lookup = context
                    .lookup_type_by_short_name(query.as_ref())
                    .expect("a frozen context must answer short-name lookups");
                last_lookup_found = !lookup.is_empty();
                assert_eq!(lookup.is_unique(), lookup.len() == 1);
            }
            2 => {
                assert_eq!(
                    lookup_type_ids(context, query.as_ref()),
                    matching_type_ids(context, query.as_ref()),
                    "short-name index results must match a direct context scan"
                );
            }
            3 => {
                let lookup = context
                    .lookup_type_by_short_name(query.as_ref())
                    .expect("a frozen context must answer short-name lookups");
                assert_eq!(lookup.len(), matching_type_ids(context, query.as_ref()).len());
                assert_eq!(lookup.is_unique(), lookup.len() == 1);
            }
            4 => {
                selected = selected_descriptor(command.wrapping_add(requested as u8), context);
                let repeated = selected_descriptor(command.wrapping_add(requested as u8), context);
                assert_eq!(selected.type_id(), repeated.type_id());
                assert_eq!(selected.type_id(), repeated.type_id());
            }
            5 => {
                let actual = context
                    .find_type_by_id(selected.type_id())
                    .expect("a frozen context must answer TypeId lookups");
                let expected = context
                    .types()
                    .expect("a frozen context must enumerate registered types")
                    .iter()
                    .find(|descriptor| descriptor.type_id() == selected.type_id());
                assert_eq!(actual.is_some(), expected.is_some());
                if let (Some(actual), Some(expected)) = (actual, expected) {
                    assert_eq!(actual.type_id(), expected.type_id());
                }
            }
            6 => {
                let types = context
                    .types()
                    .expect("a frozen context must enumerate registered types");
                if !types.is_empty() {
                    let index = usize::from(command)
                        .wrapping_add(requested)
                        .wrapping_add(usize::from(last_lookup_found))
                        % types.len();
                    selected = types.get(index).expect("selected type index must exist");
                    assert_eq!(
                        context
                            .find_type_by_id(selected.type_id())
                            .expect("a frozen context must answer TypeId lookups")
                            .map(|descriptor| descriptor.type_id()),
                        Some(selected.type_id()),
                    );
                }
            }
            _ => {
                assert_eq!(
                    lookup_type_ids(context, selected.short_name()),
                    matching_type_ids(context, selected.short_name())
                );
            }
        }
    }
});
