use crate::runtime::value::{DynamicOwned, Local, ValueFor, ValueMode};

use super::schema::ConstructionShape;

/// Ordered named field values supplied to construction or update.
pub(crate) struct NamedFields<M: ValueMode = Local> {
    pub(crate) entries: Vec<(Box<str>, DynamicOwned<M>)>,
}

impl<M: ValueMode> NamedFields<M> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub(crate) fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.entries.push((name.into(), DynamicOwned::new(value)));
        self
    }

    pub(crate) fn field_dynamic(
        mut self,
        name: impl Into<Box<str>>,
        value: DynamicOwned<M>,
    ) -> Self {
        self.entries.push((name.into(), value));
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

    pub(crate) fn value_dynamic(mut self, value: DynamicOwned<M>) -> Self {
        self.values.push(value);
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

    pub(crate) fn from_dynamic(base: DynamicOwned<M>) -> Self {
        Self {
            base,
            replacements: NamedFields::new(),
        }
    }

    pub(crate) fn field<T: ValueFor<M>>(mut self, name: impl Into<Box<str>>, value: T) -> Self {
        self.replacements = self.replacements.field(name, value);
        self
    }

    pub(crate) fn field_dynamic(
        mut self,
        name: impl Into<Box<str>>,
        value: DynamicOwned<M>,
    ) -> Self {
        self.replacements = self.replacements.field_dynamic(name, value);
        self
    }
}

pub(crate) trait ConstructionInput<M: ValueMode>: sealed::Sealed {
    fn into_input(self) -> ConstructionInputValue<M>;
}

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
                .map(|(name, value)| InputEntry::Named {
                    name,
                    value: Some(value),
                })
                .collect(),
            Self::Positional(values) => values
                .into_iter()
                .map(|value| InputEntry::Positional(Some(value)))
                .collect(),
            Self::Unit => Vec::new(),
        }
    }
}

impl<M: ValueMode> ConstructionInput<M> for NamedFields<M> {
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Named(self.entries)
    }
}

impl<M: ValueMode> ConstructionInput<M> for PositionalFields<M> {
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Positional(self.values)
    }
}

impl<M: ValueMode> ConstructionInput<M> for UnitFields {
    fn into_input(self) -> ConstructionInputValue<M> {
        ConstructionInputValue::Unit
    }
}

pub(crate) mod sealed {
    pub trait Sealed {}
}

impl<M: ValueMode> sealed::Sealed for NamedFields<M> {}
impl<M: ValueMode> sealed::Sealed for PositionalFields<M> {}
impl sealed::Sealed for UnitFields {}

pub(crate) enum InputEntryRef<'a, M: ValueMode> {
    Named {
        name: &'a str,
        value: &'a DynamicOwned<M>,
    },
    Positional(&'a DynamicOwned<M>),
}

pub(crate) enum InputEntry<M: ValueMode> {
    Named {
        name: Box<str>,
        value: Option<DynamicOwned<M>>,
    },
    Positional(Option<DynamicOwned<M>>),
}

impl<M: ValueMode> InputEntry<M> {
    pub(crate) fn take_value(&mut self) -> DynamicOwned<M> {
        match self {
            Self::Named { value, .. } | Self::Positional(value) => {
                value.take().expect("bound input is consumed once")
            }
        }
    }
}
