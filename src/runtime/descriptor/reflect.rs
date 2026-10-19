use std::marker::PhantomData;

use super::metadata::TypeMetadata;

pub(crate) trait Reflect: 'static {
    fn __reflect_registration() -> &'static TypeRegistration<Self>
    where
        Self: Sized;
}

pub(crate) struct TypeRegistration<T: 'static> {
    metadata: TypeMetadata,
    _type: PhantomData<fn() -> T>,
}

impl<T: 'static> TypeRegistration<T> {
    pub(crate) const fn new() -> Self {
        Self { metadata: TypeMetadata::new::<T>(), _type: PhantomData }
    }

    pub(crate) fn metadata(&'static self) -> &'static TypeMetadata {
        &self.metadata
    }
}
