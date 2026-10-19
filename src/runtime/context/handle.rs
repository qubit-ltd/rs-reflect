use std::sync::OnceLock;

pub(crate) struct ReflectionContext {
    _private: (),
}

impl ReflectionContext {
    pub(crate) fn global() -> &'static Self {
        static GLOBAL: OnceLock<ReflectionContext> = OnceLock::new();
        GLOBAL.get_or_init(|| Self { _private: () })
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self { _private: () }
    }

    pub(crate) fn handle(&self) -> ContextHandle<'_> {
        ContextHandle(self)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ContextHandle<'ctx>(&'ctx ReflectionContext);

impl<'ctx> ContextHandle<'ctx> {
    pub(crate) fn context(self) -> &'ctx ReflectionContext {
        self.0
    }
}
