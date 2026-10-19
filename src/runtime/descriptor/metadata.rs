use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub(crate) struct TypeMetadata {
    type_id: fn() -> TypeId,
    rust_name: fn() -> &'static str,
}

impl TypeMetadata {
    pub(crate) const fn new<T: ?Sized + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>,
            rust_name: std::any::type_name::<T>,
        }
    }

    pub(crate) fn type_id(&self) -> TypeId {
        (self.type_id)()
    }

    pub(crate) fn rust_name(&self) -> &'static str {
        (self.rust_name)()
    }

    pub(crate) fn of<T: ?Sized + 'static>() -> &'static Self {
        let type_id = TypeId::of::<T>();
        let metadata = metadata_cache();
        let mut metadata = metadata
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *metadata
            .entry(type_id)
            .or_insert_with(|| Box::leak(Box::new(Self::new::<T>())))
    }
}

fn metadata_cache() -> &'static Mutex<HashMap<TypeId, &'static TypeMetadata>> {
    static METADATA: OnceLock<Mutex<HashMap<TypeId, &'static TypeMetadata>>> = OnceLock::new();
    METADATA.get_or_init(|| Mutex::new(HashMap::new()))
}
