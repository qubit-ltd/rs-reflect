// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Shape-preserving values used to construct or update reflected values.

mod construction_input;
mod construction_input_value;
mod input_entry;
mod input_entry_ref;
mod into_runtime_owned;
mod named_fields;
mod positional_fields;
pub(crate) mod sealed;
mod unit_fields;
mod update_input;

pub(crate) use construction_input::ConstructionInput;
pub(crate) use construction_input_value::ConstructionInputValue;
pub(crate) use input_entry_ref::InputEntryRef;
pub(crate) use named_fields::NamedFields;
pub(crate) use positional_fields::PositionalFields;
pub(crate) use unit_fields::UnitFields;
pub(crate) use update_input::UpdateInput;

#[cfg(test)]
mod tests {
    use super::super::schema::ConstructionShape;
    use super::ConstructionInputValue;
    use super::InputEntryRef;
    use super::NamedFields;
    use super::PositionalFields;
    use super::UnitFields;
    use super::UpdateInput;
    use super::input_entry::InputEntry;
    use crate::__private::codegen_v4::value::DynamicOwned as CodegenOwned;
    use crate::runtime::value::DynamicOwned;
    use crate::runtime::value::Local;
    use crate::runtime::value::ThreadSafe;

    #[test]
    fn test_named_fields_keep_order_and_accept_runtime_and_codegen_values() {
        let fields = NamedFields::<Local>::new()
            .field("first", 11_u32)
            .field_dynamic("second", CodegenOwned::<Local>::new("value".to_owned()));

        let entries = fields.entries();
        assert_eq!(entries.len(), 2);
        match &entries[0] {
            InputEntryRef::Named { name, value } => {
                assert_eq!(*name, "first");
                assert_eq!(value.downcast_ref::<u32>(), Some(&11));
            }
            InputEntryRef::Positional(_) => panic!("named fields must stay named"),
        }
        match &entries[1] {
            InputEntryRef::Named { name, value } => {
                assert_eq!(*name, "second");
                assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("value"));
            }
            InputEntryRef::Positional(_) => panic!("named fields must stay named"),
        }

        let input = <NamedFields<Local> as super::sealed::Sealed<Local>>::into_input(fields);
        assert_eq!(input.shape(), ConstructionShape::Named);
        assert_eq!(input.entries().len(), 2);
    }

    #[test]
    fn test_positional_fields_keep_order_and_convert_codegen_values() {
        let input = PositionalFields::<Local>::new()
            .value(3_u8)
            .value_dynamic(CodegenOwned::<Local>::new(5_u16))
            .into_input();

        assert_eq!(input.shape(), ConstructionShape::Positional);
        let entries = input.entries();
        assert_eq!(entries.len(), 2);
        match &entries[0] {
            InputEntryRef::Positional(value) => assert_eq!(value.downcast_ref::<u8>(), Some(&3)),
            InputEntryRef::Named { .. } => panic!("positional fields must stay positional"),
        }
        match &entries[1] {
            InputEntryRef::Positional(value) => assert_eq!(value.downcast_ref::<u16>(), Some(&5)),
            InputEntryRef::Named { .. } => panic!("positional fields must stay positional"),
        }
    }

    #[test]
    fn test_unit_input_has_no_entries() {
        let input = UnitFields.into_input::<Local>();

        assert_eq!(input.shape(), ConstructionShape::Unit);
        assert!(input.entries().is_empty());
        assert!(input.into_entries().is_empty());
    }

    #[test]
    fn test_consuming_named_and_positional_inputs_preserves_values_and_binding_mode() {
        let named = NamedFields::<Local>::new()
            .field("answer", 42_u32)
            .into_input()
            .into_entries();
        assert_eq!(named.len(), 1);
        match named.into_iter().next().expect("one named entry") {
            InputEntry::Named(Some(value)) => assert_eq!(value.downcast_ref::<u32>(), Some(&42)),
            InputEntry::Named(None) => panic!("input value must be present"),
            InputEntry::Positional(_) => panic!("named binding mode must be preserved"),
        }

        let positional = PositionalFields::<Local>::new().value(7_u8).into_input().into_entries();
        assert_eq!(positional.len(), 1);
        match positional.into_iter().next().expect("one positional entry") {
            InputEntry::Positional(Some(value)) => assert_eq!(value.downcast_ref::<u8>(), Some(&7)),
            InputEntry::Positional(None) => panic!("input value must be present"),
            InputEntry::Named(_) => panic!("positional binding mode must be preserved"),
        }
    }

    #[test]
    fn test_update_input_preserves_base_and_replacement_order() {
        let update = UpdateInput::<Local>::new("base".to_owned())
            .field("left", 1_u16)
            .field_dynamic("right", CodegenOwned::<Local>::new(2_u32));

        assert_eq!(update.base.downcast_ref::<String>().map(String::as_str), Some("base"));
        assert_eq!(update.replacements.entries.len(), 2);
        assert_eq!(update.replacements.entries[0].0.as_ref(), "left");
        assert_eq!(update.replacements.entries[0].1.downcast_ref::<u16>(), Some(&1));
        assert_eq!(update.replacements.entries[1].0.as_ref(), "right");
        assert_eq!(update.replacements.entries[1].1.downcast_ref::<u32>(), Some(&2));

        let dynamic = UpdateInput::<Local>::from_dynamic(DynamicOwned::new(9_u8));
        assert_eq!(dynamic.base.downcast_ref::<u8>(), Some(&9));
        assert!(dynamic.replacements.entries.is_empty());
    }

    #[test]
    fn test_thread_safe_codegen_values_convert_to_runtime_owned_values() {
        let fields = NamedFields::<ThreadSafe>::new().field_dynamic("value", CodegenOwned::<ThreadSafe>::new(13_u32));

        match &fields.entries()[0] {
            InputEntryRef::Named { value, .. } => {
                assert_eq!(value.downcast_ref::<u32>(), Some(&13))
            }
            InputEntryRef::Positional(_) => panic!("named fields must stay named"),
        }
    }

    #[test]
    fn test_taking_a_bound_value_consumes_its_slot() {
        let mut entry = InputEntry::Positional(Some(DynamicOwned::<Local>::new(21_u32)));

        assert_eq!(entry.take_value().downcast_ref::<u32>(), Some(&21));
        assert!(matches!(entry, InputEntry::Positional(None)));
    }

    #[test]
    fn test_empty_named_and_positional_inputs_keep_their_distinct_shapes() {
        let named = NamedFields::<Local>::default().into_input();
        let positional = PositionalFields::<Local>::default().into_input();
        let unit = ConstructionInputValue::<Local>::Unit;

        assert_eq!(named.shape(), ConstructionShape::Named);
        assert_eq!(positional.shape(), ConstructionShape::Positional);
        assert_eq!(unit.shape(), ConstructionShape::Unit);
        assert!(named.entries().is_empty());
        assert!(positional.entries().is_empty());
    }
}
