use std::any::TypeId;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::TypeDescriptor;

use super::borrowed::{DynamicMut, DynamicRef};
use super::mode::ValueMode;
use super::owned::DynamicOwned;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DynamicValueKind {
    Owned,
    Ref,
    Mut,
}

pub(crate) enum DynamicValue<'a, M: ValueMode = super::mode::Local> {
    Owned(DynamicOwned<M>),
    Ref(DynamicRef<'a, M>),
    Mut(DynamicMut<'a, M>),
}

impl<'a, M: ValueMode> DynamicValue<'a, M> {
    pub(crate) fn kind(&self) -> DynamicValueKind {
        match self {
            Self::Owned(_) => DynamicValueKind::Owned,
            Self::Ref(_) => DynamicValueKind::Ref,
            Self::Mut(_) => DynamicValueKind::Mut,
        }
    }

    pub(crate) fn type_id(&self) -> TypeId {
        match self {
            Self::Owned(value) => value.type_id(),
            Self::Ref(value) => value.clone().type_id(),
            Self::Mut(value) => value.type_id(),
        }
    }

    pub(crate) fn ty(&self) -> TypeDescriptor<'static> {
        self.ty_in(ReflectionContext::global())
    }

    pub(crate) fn ty_in<'ctx>(&self, context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
        match self {
            Self::Owned(value) => value.ty_in(context),
            Self::Ref(value) => value.clone().ty_in(context),
            Self::Mut(value) => value.ty_in(context),
        }
    }

    pub(crate) fn as_ref(&self) -> DynamicRef<'_, M> {
        match self {
            Self::Owned(value) => value.as_ref(),
            Self::Ref(value) => value.reborrow(),
            Self::Mut(value) => value.as_ref(),
        }
    }
}
