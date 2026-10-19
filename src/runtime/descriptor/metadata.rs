use std::any::TypeId;

pub(crate) struct TypeMetadata {
    type_id: fn() -> TypeId,
    rust_name: fn() -> &'static str,
}

impl TypeMetadata {
    pub(crate) const fn new<T: 'static>() -> Self {
        Self { type_id: TypeId::of::<T>, rust_name: std::any::type_name::<T> }
    }

    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn rust_name(&self) -> &'static str {
        (self.rust_name)()
    }
}
