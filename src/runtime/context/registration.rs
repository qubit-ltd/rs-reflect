use std::any::TypeId;

use crate::runtime::context::ReflectionContext;
use crate::runtime::descriptor::{Reflect, TypeDescriptor, TypeRegistration};
use crate::runtime::identity::RegistrationOrigin;

type RegistrationSortKey = (
    &'static str,
    Option<&'static str>,
    &'static str,
    Option<&'static str>,
    Option<u32>,
    Option<u32>,
    &'static str,
);

#[derive(Clone, Copy)]
pub(crate) struct TypeRecord {
    type_id: fn() -> TypeId,
    rust_name: fn() -> &'static str,
    descriptor: for<'ctx> fn(&'ctx ReflectionContext) -> TypeDescriptor<'ctx>,
}

impl TypeRecord {
    fn of<T: Reflect>() -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
            descriptor: descriptor_for::<T>,
        }
    }

    pub(crate) fn type_id(self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn rust_name(self) -> &'static str {
        (self.rust_name)()
    }

    pub(crate) fn descriptor(self, context: &ReflectionContext) -> TypeDescriptor<'_> {
        (self.descriptor)(context)
    }
}

fn descriptor_for<'ctx, T: Reflect>(context: &'ctx ReflectionContext) -> TypeDescriptor<'ctx> {
    TypeDescriptor::new(T::__reflect_registration().metadata(), context)
}

#[derive(Clone)]
pub(crate) struct Registration {
    type_record: TypeRecord,
    origin: RegistrationOrigin,
}

impl Registration {
    pub(crate) fn of<T: Reflect>() -> Self {
        Self::new::<T>()
    }

    fn new<T: Reflect>() -> Self {
        let type_record = TypeRecord::of::<T>();
        Self {
            type_record,
            origin: RegistrationOrigin {
                crate_name: "explicit",
                crate_version: None,
                module_path: type_record.rust_name(),
                file: None,
                line: None,
                column: None,
            },
        }
    }

    pub(crate) const fn type_record(&self) -> TypeRecord {
        self.type_record
    }

    pub(crate) fn origin(&self) -> &RegistrationOrigin {
        &self.origin
    }

    pub(crate) fn sort_key(&self) -> RegistrationSortKey {
        (
            self.origin.crate_name,
            self.origin.crate_version,
            self.origin.module_path,
            self.origin.file,
            self.origin.line,
            self.origin.column,
            self.type_record.rust_name(),
        )
    }
}

impl<T: Reflect> From<&'static TypeRegistration<T>> for Registration {
    fn from(_registration: &'static TypeRegistration<T>) -> Self {
        Self::new::<T>()
    }
}

/// A statically linked type registration consumed while building a context.
#[derive(Clone, Copy)]
pub(crate) struct RegistrationFragment {
    sort_key: RegistrationSortKey,
    registration: fn() -> Registration,
}

impl RegistrationFragment {
    pub(crate) const fn new(
        sort_key: RegistrationSortKey,
        registration: fn() -> Registration,
    ) -> Self {
        Self {
            sort_key,
            registration,
        }
    }

    pub(crate) fn materialize(self) -> Registration {
        (self.registration)()
    }

    pub(crate) const fn sort_key(&self) -> RegistrationSortKey {
        self.sort_key
    }
}

inventory::collect!(RegistrationFragment);
