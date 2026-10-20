// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use super::schema::ConstructionShape;
use crate::runtime::value::DynamicOwned;
use crate::runtime::value::Local;
use crate::runtime::value::ValueFor;
use crate::runtime::value::ValueMode;

pub(crate) trait IntoRuntimeOwned<M: ValueMode> {
    fn into_runtime_owned(self) -> DynamicOwned<M>;
}

impl<M: ValueMode> IntoRuntimeOwned<M> for DynamicOwned<M> {
    fn into_runtime_owned(self) -> DynamicOwned<M> {
        self
    }
}

impl IntoRuntimeOwned<Local> for crate::__private::codegen_v4::value::DynamicOwned<Local> {
    fn into_runtime_owned(self) -> DynamicOwned<Local> {
        self.into_runtime()
    }
}

impl IntoRuntimeOwned<crate::runtime::value::ThreadSafe>
    for crate::__private::codegen_v4::value::DynamicOwned<crate::__private::codegen_v4::value::ThreadSafe>
{
    fn into_runtime_owned(self) -> DynamicOwned<crate::runtime::value::ThreadSafe> {
        self.into_runtime()
    }
}

/// Ordered named field values supplied to construction or update.
pub(crate) struct NamedFields<M: ValueMode = Local> {
    pub(crate) entries: Vec<(Box<str>, DynamicOwned<M>)>,
}

impl<M: ValueMode> NamedFields<M> {
    pub(crate) fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub(crate) fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.entries.push((name.into(), DynamicOwned::new(value)));
        self
    }

    pub(crate) fn field_dynamic<V: IntoRuntimeOwned<M>>(mut self, name: impl Into<Box<str>>, value: V) -> Self {
        self.entries.push((name.into(), value.into_runtime_owned()));
        self
    }

    pub(crate) fn entries(&self) -> Vec<InputEntryRef<'_, M>> {
        self.entries
            .iter()
            .map(|(name, value)| InputEntryRef::Named { name, value })
            .collect()
    }

    pub(crate) fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Named(self.entries)
    }
}

impl<M: ValueMode> Default for NamedFields<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Ordered positional field values supplied to tuple construction.
pub(crate) struct PositionalFields<M: ValueMode = Local> {
    pub(super) values: Vec<DynamicOwned<M>>,
}

impl<M: ValueMode> PositionalFields<M> {
    pub(crate) fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub(crate) fn value<T: ValueFor<M>>(mut self, value: T) -> Self {
        self.values.push(DynamicOwned::new(value));
        self
    }

    pub(crate) fn value_dynamic<V: IntoRuntimeOwned<M>>(mut self, value: V) -> Self {
        self.values.push(value.into_runtime_owned());
        self
    }

    pub(crate) fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Positional(self.values)
    }
}

impl<M: ValueMode> Default for PositionalFields<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// The input shape for unit structs and unit enum variants.
pub(crate) struct UnitFields;

impl UnitFields {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) fn into_input<M: ValueMode>(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Unit
    }
}

/// An owned base value and ordered field replacements for struct update.
pub(crate) struct UpdateInput<M: ValueMode = Local> {
    pub(crate) base: DynamicOwned<M>,
    pub(crate) replacements: NamedFields<M>,
}

impl<M: ValueMode> UpdateInput<M> {
    pub(crate) fn new<T: ValueFor<M>>(base: T) -> Self {
        Self::from_dynamic(DynamicOwned::new(base))
    }

    pub(crate) fn from_dynamic<V: IntoRuntimeOwned<M>>(base: V) -> Self {
        Self {
            base: base.into_runtime_owned(),
            replacements: NamedFields::new(),
        }
    }

    pub(crate) fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.replacements = self.replacements.field(name, value);
        self
    }

    pub(crate) fn field_dynamic<V: IntoRuntimeOwned<M>>(mut self, name: impl Into<Box<str>>, value: V) -> Self {
        self.replacements = self.replacements.field_dynamic(name, value);
        self
    }
}

pub(crate) trait ConstructionInput<M: ValueMode>: sealed::Sealed<M> {}

pub(crate) enum ConstructionInputValue<M: ValueMode> {
    Named(Vec<(Box<str>, DynamicOwned<M>)>),
    Positional(Vec<DynamicOwned<M>>),
    Unit,
}

impl<M: ValueMode> ConstructionInputValue<M> {
    pub(crate) fn shape(&self) -> ConstructionShape {
        match self {
            Self::Named(_) => ConstructionShape::Named,
            Self::Positional(_) => ConstructionShape::Positional,
            Self::Unit => ConstructionShape::Unit,
        }
    }

    pub(crate) fn entries(&self) -> Vec<InputEntryRef<'_, M>> {
        match self {
            Self::Named(entries) => entries
                .iter()
                .map(|(name, value)| InputEntryRef::Named { name, value })
                .collect(),
            Self::Positional(values) => values.iter().map(InputEntryRef::Positional).collect(),
            Self::Unit => Vec::new(),
        }
    }

    pub(crate) fn into_entries(self) -> Vec<InputEntry<M>> {
        match self {
            Self::Named(entries) => entries
                .into_iter()
                .map(|(_, value)| InputEntry::Named(Some(value)))
                .collect(),
            Self::Positional(values) => values
                .into_iter()
                .map(|value| InputEntry::Positional(Some(value)))
                .collect(),
            Self::Unit => Vec::new(),
        }
    }
}

impl<M: ValueMode> ConstructionInput<M> for NamedFields<M> {}
impl<M: ValueMode> sealed::Sealed<M> for NamedFields<M> {
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Named(self.entries)
    }
}

impl<M: ValueMode> ConstructionInput<M> for PositionalFields<M> {}
impl<M: ValueMode> sealed::Sealed<M> for PositionalFields<M> {
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Positional(self.values)
    }
}

impl<M: ValueMode> ConstructionInput<M> for UnitFields {}
impl<M: ValueMode> sealed::Sealed<M> for UnitFields {
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Unit
    }
}

pub(crate) mod sealed {
    use super::ConstructionInputValue;
    use crate::runtime::value::ValueMode;

    pub(crate) trait Sealed<M: ValueMode> {
        fn into_input(self) -> ConstructionInputValue<M>;
    }
}

pub(crate) enum InputEntryRef<'a, M: ValueMode> {
    Named { name: &'a str, value: &'a DynamicOwned<M> },
    Positional(&'a DynamicOwned<M>),
}

pub(crate) enum InputEntry<M: ValueMode> {
    Named(Option<DynamicOwned<M>>),
    Positional(Option<DynamicOwned<M>>),
}

impl<M: ValueMode> InputEntry<M> {
    pub(crate) fn take_value(&mut self) -> DynamicOwned<M> {
        match self {
            Self::Named(value) | Self::Positional(value) => value.take().expect("bound input is consumed once"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConstructionInputValue;
    use super::InputEntry;
    use super::InputEntryRef;
    use super::NamedFields;
    use super::PositionalFields;
    use super::UnitFields;
    use super::UpdateInput;
    use crate::__private::codegen_v4::value::DynamicOwned as CodegenOwned;
    use crate::runtime::value::DynamicOwned;
    use crate::runtime::value::Local;
    use crate::runtime::value::ThreadSafe;

    #[test]
    fn named_fields_keep_order_and_accept_runtime_and_codegen_values() {
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
        assert_eq!(input.shape(), super::ConstructionShape::Named);
        assert_eq!(input.entries().len(), 2);
    }

    #[test]
    fn positional_fields_keep_order_and_convert_codegen_values() {
        let input = PositionalFields::<Local>::new()
            .value(3_u8)
            .value_dynamic(CodegenOwned::<Local>::new(5_u16))
            .into_input();

        assert_eq!(input.shape(), super::ConstructionShape::Positional);
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
    fn unit_input_has_no_entries() {
        let input = UnitFields.into_input::<Local>();

        assert_eq!(input.shape(), super::ConstructionShape::Unit);
        assert!(input.entries().is_empty());
        assert!(input.into_entries().is_empty());
    }

    #[test]
    fn consuming_named_and_positional_inputs_preserves_values_and_binding_mode() {
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
    fn update_input_preserves_base_and_replacement_order() {
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
    fn thread_safe_codegen_values_convert_to_runtime_owned_values() {
        let fields = NamedFields::<ThreadSafe>::new().field_dynamic("value", CodegenOwned::<ThreadSafe>::new(13_u32));

        match &fields.entries()[0] {
            InputEntryRef::Named { value, .. } => {
                assert_eq!(value.downcast_ref::<u32>(), Some(&13))
            }
            InputEntryRef::Positional(_) => panic!("named fields must stay named"),
        }
    }

    #[test]
    fn taking_a_bound_value_consumes_its_slot() {
        let mut entry = InputEntry::Positional(Some(DynamicOwned::<Local>::new(21_u32)));

        assert_eq!(entry.take_value().downcast_ref::<u32>(), Some(&21));
        assert!(matches!(entry, InputEntry::Positional(None)));
    }

    #[test]
    fn empty_named_and_positional_inputs_keep_their_distinct_shapes() {
        let named = NamedFields::<Local>::default().into_input();
        let positional = PositionalFields::<Local>::default().into_input();
        let unit = ConstructionInputValue::<Local>::Unit;

        assert_eq!(named.shape(), super::ConstructionShape::Named);
        assert_eq!(positional.shape(), super::ConstructionShape::Positional);
        assert_eq!(unit.shape(), super::ConstructionShape::Unit);
        assert!(named.entries().is_empty());
        assert!(positional.entries().is_empty());
    }
}
