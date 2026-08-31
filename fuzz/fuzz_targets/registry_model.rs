// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#![no_main]

use libfuzzer_sys::fuzz_target;
use qubit_reflect::TypeDescriptor;
use qubit_reflect::identity::CapabilityId;
use qubit_reflect::identity::ExternalTraitId;
use qubit_reflect::registry::ReflectRegistry;

const MAX_INPUT_BYTES: usize = 4_096;
const MAX_OPERATIONS: usize = 128;
const MAX_PAYLOAD_BYTES: usize = 64;

fn selected_descriptor(index: u8) -> &'static TypeDescriptor {
    match index % 16 {
        0 => TypeDescriptor::of::<bool>(),
        1 => TypeDescriptor::of::<u8>(),
        2 => TypeDescriptor::of::<u16>(),
        3 => TypeDescriptor::of::<u32>(),
        4 => TypeDescriptor::of::<u64>(),
        5 => TypeDescriptor::of::<i64>(),
        6 => TypeDescriptor::of::<String>(),
        7 => TypeDescriptor::of::<Option<String>>(),
        8 => TypeDescriptor::of::<Vec<u8>>(),
        9 => TypeDescriptor::of::<Vec<String>>(),
        10 => TypeDescriptor::of::<Option<Vec<u8>>>(),
        11 => TypeDescriptor::of::<Box<u64>>(),
        12 => TypeDescriptor::of::<std::sync::Arc<u64>>(),
        13 => TypeDescriptor::of::<(u8, String)>(),
        14 => TypeDescriptor::of::<[u8; 4]>(),
        _ => TypeDescriptor::of::<std::collections::BTreeMap<String, u64>>(),
    }
}

// Executes a bounded, input-driven sequence over public safe identity,
// descriptor, and registry APIs. Frozen indexes are compared with direct
// registry scans as an independent reference model after every query action.
fuzz_target!(|unbounded: &[u8]| {
    let data = &unbounded[..unbounded.len().min(MAX_INPUT_BYTES)];
    let registry =
        ReflectRegistry::initialize().expect("linked fuzz-target registry fragments must form a valid snapshot");
    let mut cursor = 0;
    let mut operations = 0;
    let mut selected = selected_descriptor(0);
    let mut last_identifier_was_valid = false;

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
                let first = CapabilityId::new(query.as_ref());
                let second = CapabilityId::new(query.as_ref());
                last_identifier_was_valid = first.is_ok();
                assert_eq!(first, second);
            }
            1 => {
                let first = ExternalTraitId::new(query.as_ref());
                let second = ExternalTraitId::new(query.as_ref());
                last_identifier_was_valid = first.is_ok();
                assert_eq!(first, second);
            }
            2 => {
                let actual = registry
                    .find_by_query_name(query.as_ref())
                    .into_iter()
                    .collect::<Vec<_>>();
                let expected = registry
                    .types()
                    .iter()
                    .copied()
                    .filter(|descriptor| descriptor.query_name() == query.as_ref())
                    .collect::<Vec<_>>();
                assert_eq!(actual.len(), expected.len());
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(left, right)| std::ptr::eq(*left, right))
                );
            }
            3 => {
                let actual = registry
                    .find_by_type_name(query.as_ref())
                    .into_iter()
                    .collect::<Vec<_>>();
                let expected = registry
                    .types()
                    .iter()
                    .copied()
                    .filter(|descriptor| descriptor.type_name() == query.as_ref())
                    .collect::<Vec<_>>();
                assert_eq!(actual.len(), expected.len());
                assert!(
                    actual
                        .iter()
                        .zip(expected)
                        .all(|(left, right)| std::ptr::eq(*left, right))
                );
            }
            4 => {
                selected = selected_descriptor(command.wrapping_add(requested as u8));
                let repeated = selected_descriptor(command.wrapping_add(requested as u8));
                assert!(std::ptr::eq(selected, repeated));
                assert_eq!(selected.type_id(), repeated.type_id());
            }
            5 => {
                let actual = registry.get(selected.type_id());
                let expected = registry
                    .types()
                    .iter()
                    .copied()
                    .find(|descriptor| descriptor.type_id() == selected.type_id());
                assert_eq!(actual.is_some(), expected.is_some());
                if let (Some(actual), Some(expected)) = (actual, expected) {
                    assert!(std::ptr::eq(actual, expected));
                }
            }
            6 => {
                if !registry.types().is_empty() {
                    let index = usize::from(command)
                        .wrapping_add(requested)
                        .wrapping_add(usize::from(last_identifier_was_valid))
                        % registry.types().len();
                    selected = registry.types()[index];
                    assert!(std::ptr::eq(
                        registry
                            .get(selected.type_id())
                            .expect("enumerated descriptor must have a TypeId index"),
                        selected,
                    ));
                }
            }
            _ => {
                let by_query = registry.find_by_query_name(selected.query_name());
                let direct_count = registry
                    .types()
                    .iter()
                    .filter(|descriptor| descriptor.query_name() == selected.query_name())
                    .count();
                assert_eq!(by_query.len(), direct_count);
            }
        }
    }
});
