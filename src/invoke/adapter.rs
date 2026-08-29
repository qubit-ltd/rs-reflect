//! Function-pointer contracts used by generated invocation adapters.

use crate::invoke::{Invocation, InvocationFailure, InvocationOutput};

/// A mode-specific adapter that validates and invokes one concrete method.
///
/// The higher-ranked call lifetime prevents generated adapters from extending
/// receiver, argument, output, or future borrows beyond a single invocation.
pub type InvocationAdapter<M> =
    for<'call> fn(
        Invocation<'call, M>,
    ) -> Result<InvocationOutput<'call, M>, InvocationFailure<'call, M>>;
