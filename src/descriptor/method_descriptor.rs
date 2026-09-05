// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow public-type-layout
//! Immutable declarations and concrete instances of reflected methods.

use std::any::Any;
use std::fmt;

use crate::descriptor::ImplDefinitionDescriptor;
use crate::descriptor::TraitDefinitionDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::descriptor::trait_descriptor::TraitApplicationSubstitutions;
use crate::expression::FunctionAbi;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::TypeExpression;
use crate::identity::MemberId;
use crate::identity::Visibility;

/// How a non-receiver parameter is passed to a reflected method.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ParameterPassingMode {
    /// The method consumes an owned argument.
    Owned,
    /// The method borrows an argument immutably.
    SharedBorrow,
    /// The method borrows an argument mutably.
    MutableBorrow,
}

/// The source pattern category of a non-receiver parameter.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ParameterPatternDescriptor {
    /// A simple identifier that can participate in named binding.
    Identifier,
    /// A wildcard pattern without a bindable name.
    Wildcard,
    /// A destructuring pattern retained for positional binding and diagnostics.
    Destructure(Box<str>),
}

/// One non-receiver method parameter in declaration order.
#[derive(Clone, Debug)]
pub struct ParameterDescriptor {
    index: usize,
    name: Option<&'static str>,
    pattern: ParameterPatternDescriptor,
    passing_mode: ParameterPassingMode,
    signature_type: TypeExpression,
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ParameterDescriptor {
    /// Creates immutable parameter facts.
    ///
    /// `index` excludes the receiver. `name` must be `None` for wildcard and
    /// destructuring patterns. `concrete_type` is present only when the
    /// declaration can navigate to an exact reflected root.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        index: usize,
        name: Option<&'static str>,
        pattern: ParameterPatternDescriptor,
        passing_mode: ParameterPassingMode,
        signature_type: TypeExpression,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            index,
            name,
            pattern,
            passing_mode,
            signature_type,
            concrete_type,
        }
    }

    /// Returns the zero-based non-receiver parameter index.
    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the identifier used for named binding.
    ///
    /// `None` denotes a wildcard or destructuring pattern.
    #[must_use]
    #[inline(always)]
    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Returns the parser-independent source pattern category.
    #[must_use]
    #[inline(always)]
    pub const fn pattern(&self) -> &ParameterPatternDescriptor {
        &self.pattern
    }

    /// Returns how the argument crosses the method boundary.
    #[must_use]
    #[inline(always)]
    pub const fn passing_mode(&self) -> ParameterPassingMode {
        self.passing_mode
    }

    /// Returns the declared, possibly symbolic parameter type.
    #[must_use]
    #[inline(always)]
    pub const fn signature_type(&self) -> &TypeExpression {
        &self.signature_type
    }

    /// Returns the exact reflected parameter type when it is known.
    ///
    /// `None` denotes a symbolic, opaque, or otherwise unresolved type.
    #[must_use]
    #[inline(always)]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// The receiver form written by a reflected method declaration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReceiverDescriptor {
    /// A by-value `self` receiver.
    Owned,
    /// A shared `&self` receiver.
    Shared,
    /// An exclusive `&mut self` receiver.
    Mutable,
    /// A supported explicit receiver whose source form is retained.
    Explicit(&'static str),
}

/// The structural category of a method return value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReturnKind {
    /// The unit return type `()`.
    Unit,
    /// The never return type `!`.
    Never,
    /// A concrete owned value.
    Concrete,
    /// A shared or mutable reference.
    Reference,
    /// An opaque `impl Trait` return value.
    Opaque,
}

/// The return declaration of a reflected method.
#[derive(Clone, Debug)]
pub struct ReturnDescriptor {
    kind: ReturnKind,
    signature_type: Option<TypeExpression>,
    concrete_type: Option<TypeDescriptorResolver>,
}

impl ReturnDescriptor {
    /// Creates immutable return facts.
    ///
    /// `signature_type` is absent for unit and never returns when their
    /// [`ReturnKind`] is sufficient. `concrete_type` is present only for an
    /// exact reflected root.
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        kind: ReturnKind,
        signature_type: Option<TypeExpression>,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            kind,
            signature_type,
            concrete_type,
        }
    }

    /// Creates a unit return descriptor.
    #[must_use]
    pub const fn unit() -> Self {
        Self::new(ReturnKind::Unit, None, None)
    }

    /// Returns the structural return category.
    #[must_use]
    #[inline(always)]
    pub const fn kind(&self) -> ReturnKind {
        self.kind
    }

    /// Returns the declared return type expression.
    ///
    /// `None` means the unit or never category carries the complete fact.
    #[must_use]
    #[inline(always)]
    pub const fn signature_type(&self) -> Option<&TypeExpression> {
        self.signature_type.as_ref()
    }

    /// Returns the exact reflected return type when it is known.
    ///
    /// `None` denotes unit, never, a reference, opaque output, or an unresolved
    /// symbolic type.
    #[must_use]
    #[inline(always)]
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// Where a method declaration obtains its source visibility.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MethodVisibility {
    /// Visibility declared by an inherent or implementation method.
    Declared(Visibility),
    /// Trait-item reachability inherited from the declaring trait.
    InheritedFromTrait,
}

/// Qualifiers that affect whether a declaration can have an invocation adapter.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MethodQualifiers {
    /// Whether the declaration is `async`.
    pub(crate) is_async: bool,
    /// Whether the declaration is `unsafe`.
    pub(crate) is_unsafe: bool,
    /// Whether the declaration is `const`.
    pub(crate) is_const: bool,
    /// The explicitly declared ABI, or `None` for the ordinary Rust ABI.
    pub(crate) abi: Option<FunctionAbi>,
    /// Whether the declaration has a variadic tail.
    pub(crate) is_variadic: bool,
}

impl MethodQualifiers {
    /// Creates the complete set of method qualifiers.
    #[must_use]
    pub const fn new(
        is_async: bool,
        is_unsafe: bool,
        is_const: bool,
        abi: Option<FunctionAbi>,
        is_variadic: bool,
    ) -> Self {
        Self {
            is_async,
            is_unsafe,
            is_const,
            abi,
            is_variadic,
        }
    }

    /// Returns whether the declaration is asynchronous.
    #[must_use]
    pub const fn is_async(&self) -> bool {
        self.is_async
    }
    /// Returns whether the declaration is unsafe.
    #[must_use]
    pub const fn is_unsafe(&self) -> bool {
        self.is_unsafe
    }
    /// Returns whether the declaration is const.
    #[must_use]
    pub const fn is_const(&self) -> bool {
        self.is_const
    }
    /// Returns the explicitly declared ABI.
    #[must_use]
    pub const fn abi(&self) -> Option<&FunctionAbi> {
        self.abi.as_ref()
    }
    /// Returns whether the declaration has a variadic tail.
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.is_variadic
    }
}

impl Default for MethodQualifiers {
    /// Returns the qualifiers of an ordinary safe Rust method.
    fn default() -> Self {
        Self::new(false, false, false, None, false)
    }
}

/// A stable reason why a concrete method instance cannot be invoked
/// dynamically.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationUnavailableReason {
    /// The receiver form has no safe adapter.
    UnsupportedReceiver,
    /// A parameter pattern has no safe adapter.
    UnsupportedParameterPattern,
    /// The declaration is generic and has no registered specialization.
    UnspecializedGeneric,
    /// A concrete specialization was registered but has no safe generated
    /// invocation adapter.
    UnsupportedSpecialization,
    /// The declaration is unsafe.
    UnsafeMethod,
    /// The declared ABI has no safe adapter.
    UnsupportedAbi,
    /// Variadic invocation is not supported.
    Variadic,
    /// The return borrow cannot be related safely to an input borrow.
    UnsupportedBorrowedReturn,
    /// An opaque return value cannot cross the dynamic boundary.
    OpaqueReturn,
    /// An unsized value has no dedicated safe adapter.
    UnsupportedUnsizedValue,
    /// A default method has call-site bounds that reflection cannot prove.
    UnprovenDefaultConstraint,
    /// A default method depends on an associated type that is not proven at
    /// the declaration hook.
    UnprovenAssociatedType,
    /// A pinned receiver conflicts with the requested async, thread-safe,
    /// catching, or borrowed-output mode.
    PinnedModeConflict,
    /// Invocation was disabled by reflection policy.
    DisabledByPolicy,
}

/// Availability of an explicitly requested panic-catching invocation entry
/// point.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CatchingAvailability {
    /// No catching adapter was requested for this method.
    NotRequested,
    /// The explicitly requested catching adapter is callable.
    Available,
    /// Catching was requested but the binary uses abort-on-panic semantics.
    UnavailablePanicAbort,
}

/// An opaque invocation entry point supplied by a later invocation layer.
///
/// This descriptor layer records adapter identity and availability. The
/// invocation layer owns argument validation and the complete call contract.
#[derive(Clone, Copy, Debug)]
pub struct InvocationAdapter {
    entry_point: fn(),
    local: Option<crate::invoke::InvocationAdapter<crate::value::Local>>,
    thread_safe: Option<crate::invoke::InvocationAdapter<crate::value::ThreadSafe>>,
    catching_local: Option<crate::invoke::CatchingInvocationAdapter<crate::value::Local>>,
    catching_thread_safe: Option<crate::invoke::CatchingInvocationAdapter<crate::value::ThreadSafe>>,
    catching_availability: CatchingAvailability,
    pinned_ref_local: Option<&'static (dyn Any + Send + Sync)>,
    pinned_mut_local: Option<&'static (dyn Any + Send + Sync)>,
}

impl InvocationAdapter {
    /// Creates an opaque adapter token for generated descriptor data.
    #[doc(hidden)]
    pub const fn new(entry_point: fn()) -> Self {
        Self {
            entry_point,
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for one callable local invocation entry point.
    ///
    /// The generated function is higher-ranked over the invocation lifetime,
    /// so it cannot extend erased input borrows beyond one invocation.
    #[doc(hidden)]
    pub const fn local(entry_point: crate::invoke::InvocationAdapter<crate::value::Local>) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: Some(entry_point),
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for one callable thread-safe invocation entry
    /// point.
    ///
    /// The entry point's type preserves both the call lifetime and the runtime
    /// `Send` boundary required by [`ThreadSafe`](crate::value::ThreadSafe).
    #[doc(hidden)]
    pub const fn thread_safe(entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: Some(entry_point),
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a local adapter paired with an explicitly generated catching
    /// entry point.
    #[doc(hidden)]
    pub const fn local_with_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::Local>,
        catching_entry_point: crate::invoke::CatchingInvocationAdapter<crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: Some(entry_point),
            thread_safe: None,
            catching_local: Some(catching_entry_point),
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::Available,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a thread-safe adapter paired with an explicitly generated
    /// catching entry point.
    #[doc(hidden)]
    pub const fn thread_safe_with_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>,
        catching_entry_point: crate::invoke::CatchingInvocationAdapter<crate::value::ThreadSafe>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: Some(entry_point),
            catching_local: None,
            catching_thread_safe: Some(catching_entry_point),
            catching_availability: CatchingAvailability::Available,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a local adapter whose requested catching entry point is
    /// unavailable because this binary aborts on panic.
    #[doc(hidden)]
    pub const fn local_with_unavailable_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: Some(entry_point),
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::UnavailablePanicAbort,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a thread-safe adapter whose requested catching entry point is
    /// unavailable because this binary aborts on panic.
    #[doc(hidden)]
    pub const fn thread_safe_with_unavailable_catching(
        entry_point: crate::invoke::InvocationAdapter<crate::value::ThreadSafe>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: Some(entry_point),
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::UnavailablePanicAbort,
            pinned_ref_local: None,
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for a typed local `Pin<&T>` entry point.
    ///
    /// The concrete adapter remains behind `Any` only for descriptor storage;
    /// [`Self::invoke_pinned_ref_local`] downcasts it by the caller's `T`
    /// without erasing or reconstructing the pin proof.
    #[doc(hidden)]
    pub const fn pinned_ref_local<T: 'static>(
        entry_point: &'static crate::invoke::PinnedRefAdapter<T, crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: Some(entry_point),
            pinned_mut_local: None,
        }
    }

    /// Creates a descriptor for a typed local `Pin<&mut T>` entry point.
    #[doc(hidden)]
    pub const fn pinned_mut_local<T: 'static>(
        entry_point: &'static crate::invoke::PinnedMutAdapter<T, crate::value::Local>,
    ) -> Self {
        Self {
            entry_point: unavailable_entry_point,
            local: None,
            thread_safe: None,
            catching_local: None,
            catching_thread_safe: None,
            catching_availability: CatchingAvailability::NotRequested,
            pinned_ref_local: None,
            pinned_mut_local: Some(entry_point),
        }
    }

    /// Returns the opaque entry-point identity.
    #[doc(hidden)]
    #[must_use]
    #[inline(always)]
    pub const fn entry_point(&self) -> fn() {
        self.entry_point
    }

    /// Reports whether an explicitly requested panic-catching entry point is
    /// callable in this binary.
    #[must_use]
    #[inline(always)]
    pub const fn catching_availability(&self) -> CatchingAvailability {
        self.catching_availability
    }

    /// Invokes the local generated entry point when this descriptor has one.
    ///
    /// This raw adapter entry point accepts positional inputs only. Named
    /// bindings return `NamedBindingRequiresDescriptor`; use
    /// [`MethodInstanceDescriptor::invoke_local`] for descriptor-aware binding.
    ///
    /// Returns `None` for legacy descriptor-only entries and for adapters that
    /// are available exclusively in another invocation mode.
    #[must_use]
    pub fn invoke_local<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::InvocationFailure<'call, crate::value::Local>,
        >,
    > {
        self.local.map(|entry_point| entry_point(invocation))
    }

    /// Invokes the thread-safe generated entry point when this descriptor has
    /// one.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_thread_safe`] for named bindings.
    ///
    /// Returns `None` when this method was not explicitly generated with a
    /// thread-safe adapter.
    #[must_use]
    pub fn invoke_thread_safe<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::ThreadSafe>,
            crate::invoke::InvocationFailure<'call, crate::value::ThreadSafe>,
        >,
    > {
        self.thread_safe.map(|entry_point| entry_point(invocation))
    }

    /// Invokes the explicit local catching entry point when one was generated.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_catching_local`] for named bindings.
    #[must_use]
    pub fn invoke_catching_local<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<crate::invoke::CatchingInvocationResult<'call, crate::value::Local>> {
        self.catching_local.map(|entry_point| entry_point(invocation))
    }

    /// Invokes the explicit thread-safe catching entry point when one was
    /// generated.
    #[must_use]
    pub fn invoke_catching_thread_safe<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<crate::invoke::CatchingInvocationResult<'call, crate::value::ThreadSafe>> {
        self.catching_thread_safe.map(|entry_point| entry_point(invocation))
    }

    /// Invokes a typed local `Pin<&T>` entry point when its exact receiver
    /// type matches this method's generated adapter.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_pinned_ref_local`] for named
    /// bindings.
    ///
    /// `None` means this method has no such adapter or `T` is not its exact
    /// receiver type. The `Err` case preserves the original pin and arguments.
    #[must_use]
    pub fn invoke_pinned_ref_local<'call, T: 'static>(
        &self,
        invocation: crate::invoke::PinnedRefInvocation<'call, T, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedRefInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        self.pinned_ref_local
            .and_then(|entry_point| {
                entry_point.downcast_ref::<crate::invoke::PinnedRefAdapter<T, crate::value::Local>>()
            })
            .map(|entry_point| entry_point(invocation))
    }

    /// Invokes a typed local `Pin<&mut T>` entry point when its exact receiver
    /// type matches this method's generated adapter.
    ///
    /// This raw adapter entry point accepts positional inputs only. Use
    /// [`MethodInstanceDescriptor::invoke_pinned_mut_local`] for named
    /// bindings.
    ///
    /// `None` means this method has no such adapter or `T` is not its exact
    /// receiver type. The `Err` case preserves the original pin and arguments.
    #[must_use]
    pub fn invoke_pinned_mut_local<'call, T: 'static>(
        &self,
        invocation: crate::invoke::PinnedMutInvocation<'call, T, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedMutInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        self.pinned_mut_local
            .and_then(|entry_point| {
                entry_point.downcast_ref::<crate::invoke::PinnedMutAdapter<T, crate::value::Local>>()
            })
            .map(|entry_point| entry_point(invocation))
    }
}

/// Serves as a stable opaque token for adapters whose real entry point is
/// typed.
fn unavailable_entry_point() {}

/// An immutable reflected method declaration.
///
/// # Examples
///
/// ```
/// # #![allow(proc_macro_derive_resolution_fallback)]
/// use qubit_reflect::{Reflect, TypeDescriptor};
/// #[cfg(feature = "derive")]
/// use qubit_reflect::reflect_impl;
///
/// #[cfg(feature = "derive")]
/// #[derive(Reflect)]
/// #[reflect(crate = qubit_reflect)]
/// struct Service;
///
/// #[cfg(feature = "derive")]
/// #[reflect_impl(crate = qubit_reflect)]
/// impl Service {
///     fn ping(&self) {}
/// }
///
/// # #[cfg(feature = "derive")]
/// # fn main() -> Result<(), qubit_reflect::error::RegistryError> {
/// let method = TypeDescriptor::of::<Service>()
///     .impls()?
///     .first()
///     .and_then(|implementation| implementation.method("ping"))
///     .expect("reflected method");
/// assert_eq!(method.rust_name(), "ping");
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "derive"))]
/// # fn main() {}
/// ```
#[derive(Clone, Debug)]
pub struct MethodDescriptor {
    identity: MemberId,
    rust_name: &'static str,
    query_name: &'static str,
    visibility: MethodVisibility,
    receiver: Option<ReceiverDescriptor>,
    parameters: Box<[ParameterDescriptor]>,
    return_value: ReturnDescriptor,
    qualifiers: MethodQualifiers,
    generic_definition: GenericDefinitionDescriptor,
    has_default: bool,
    declaration_owner: MethodDeclarationOwner,
}

/// The declaration that owns a method descriptor.
#[derive(Clone, Copy, Debug)]
pub enum MethodDeclarationOwner {
    /// A method declared by a trait definition.
    Trait(&'static TraitDefinitionDescriptor),
    /// A method explicitly declared by an impl definition.
    Impl(&'static ImplDefinitionDescriptor),
}

impl MethodDescriptor {
    /// Starts a builder for one method declaration.
    ///
    /// The member identity remains independent of `query_name`, so renaming a
    /// method does not change its Rust identity.
    #[must_use]
    pub fn builder(
        identity: MemberId,
        rust_name: &'static str,
        query_name: &'static str,
        declaration_owner: MethodDeclarationOwner,
    ) -> MethodDescriptorBuilder {
        MethodDescriptorBuilder::new(identity, rust_name, query_name, declaration_owner)
    }

    /// Returns the stable composite member identity.
    #[must_use]
    #[inline(always)]
    pub const fn identity(&self) -> &MemberId {
        &self.identity
    }

    /// Returns the Rust declaration name.
    #[must_use]
    #[inline(always)]
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    #[must_use]
    #[inline(always)]
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns normalized source visibility facts.
    #[must_use]
    #[inline(always)]
    pub const fn visibility(&self) -> &MethodVisibility {
        &self.visibility
    }

    /// Returns the receiver, or `None` for an associated function.
    #[must_use]
    #[inline(always)]
    pub const fn receiver(&self) -> Option<&ReceiverDescriptor> {
        self.receiver.as_ref()
    }

    /// Returns non-receiver parameters in source declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn parameters(&self) -> &[ParameterDescriptor] {
        &self.parameters
    }

    /// Finds a uniquely named identifier parameter.
    ///
    /// `None` means no parameter has the requested identifier.
    #[must_use]
    pub fn parameter(&self, name: &str) -> Option<&ParameterDescriptor> {
        self.parameters.iter().find(|parameter| parameter.name() == Some(name))
    }

    /// Returns a non-receiver parameter by declaration index.
    ///
    /// `None` means `index` is outside the parameter range.
    #[must_use]
    pub fn parameter_at(&self, index: usize) -> Option<&ParameterDescriptor> {
        self.parameters.get(index)
    }

    /// Returns the declared return facts.
    #[must_use]
    #[inline(always)]
    pub const fn return_value(&self) -> &ReturnDescriptor {
        &self.return_value
    }

    /// Returns callability-related source qualifiers.
    #[must_use]
    #[inline(always)]
    pub const fn qualifiers(&self) -> &MethodQualifiers {
        &self.qualifiers
    }

    /// Returns generic parameters and predicates in source order.
    #[must_use]
    #[inline(always)]
    pub const fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        &self.generic_definition
    }

    /// Returns whether the trait declaration supplies a default body.
    #[must_use]
    #[inline(always)]
    pub const fn has_default(&self) -> bool {
        self.has_default
    }

    /// Returns the owning trait definition for a trait method.
    ///
    /// `None` means this method is declared by an impl definition.
    #[must_use]
    #[inline(always)]
    pub const fn declaring_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(descriptor) => Some(descriptor),
            MethodDeclarationOwner::Impl(_) => None,
        }
    }

    /// Returns the owning impl definition for an implementation method.
    ///
    /// `None` means this method is declared by a trait definition.
    #[must_use]
    #[inline(always)]
    pub const fn declaring_impl(&self) -> Option<&'static ImplDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(_) => None,
            MethodDeclarationOwner::Impl(descriptor) => Some(descriptor),
        }
    }

    /// Applies concrete trait arguments to every signature relationship while
    /// preserving the declaration identity and source metadata.
    pub(crate) fn substituted_for_trait_application(&self, substitutions: &TraitApplicationSubstitutions) -> Self {
        let mut result = self.clone();
        for parameter in &mut result.parameters {
            parameter.signature_type = substitutions.type_expression(&parameter.signature_type);
        }
        result.return_value.signature_type = result
            .return_value
            .signature_type
            .as_ref()
            .map(|expression| substitutions.type_expression(expression));
        result.generic_definition.predicates = result
            .generic_definition
            .predicates
            .iter()
            .map(|predicate| substitutions.predicate(predicate))
            .collect();
        result
    }

    /// Returns whether applying the substitutions changes any method-level
    /// signature or predicate fact.
    pub(crate) fn needs_trait_application_substitution(&self, substitutions: &TraitApplicationSubstitutions) -> bool {
        self.parameters
            .iter()
            .any(|parameter| substitutions.type_expression(&parameter.signature_type) != parameter.signature_type)
            || self
                .return_value
                .signature_type
                .as_ref()
                .is_some_and(|expression| substitutions.type_expression(expression) != *expression)
            || self
                .generic_definition
                .predicates
                .iter()
                .any(|predicate| substitutions.predicate(predicate) != *predicate)
    }
}

/// Builds a declaration while preserving source order for all collections.
#[derive(Debug)]
pub struct MethodDescriptorBuilder {
    identity: MemberId,
    rust_name: &'static str,
    query_name: &'static str,
    visibility: MethodVisibility,
    receiver: Option<ReceiverDescriptor>,
    parameters: Vec<ParameterDescriptor>,
    return_value: ReturnDescriptor,
    qualifiers: MethodQualifiers,
    generic_definition: GenericDefinitionDescriptor,
    has_default: bool,
    declaration_owner: MethodDeclarationOwner,
}

impl MethodDescriptorBuilder {
    /// Creates a builder with ordinary private, non-generic method defaults.
    fn new(
        identity: MemberId,
        rust_name: &'static str,
        query_name: &'static str,
        declaration_owner: MethodDeclarationOwner,
    ) -> Self {
        Self {
            identity,
            rust_name,
            query_name,
            visibility: MethodVisibility::Declared(Visibility::Private),
            receiver: None,
            parameters: Vec::new(),
            return_value: ReturnDescriptor::unit(),
            qualifiers: MethodQualifiers::default(),
            generic_definition: GenericDefinitionDescriptor {
                parameters: Box::new([]),
                predicates: Box::new([]),
                diagnostic: Default::default(),
            },
            has_default: false,
            declaration_owner,
        }
    }

    /// Sets normalized source visibility.
    #[must_use]
    pub fn visibility(mut self, visibility: MethodVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets the receiver; `None` describes an associated function.
    #[must_use]
    pub fn receiver(mut self, receiver: Option<ReceiverDescriptor>) -> Self {
        self.receiver = receiver;
        self
    }

    /// Sets non-receiver parameters in source order.
    #[must_use]
    pub fn parameters(mut self, parameters: Vec<ParameterDescriptor>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Sets the return declaration.
    #[must_use]
    pub fn return_value(mut self, return_value: ReturnDescriptor) -> Self {
        self.return_value = return_value;
        self
    }

    /// Sets source qualifiers that affect invocation availability.
    #[must_use]
    pub fn qualifiers(mut self, qualifiers: MethodQualifiers) -> Self {
        self.qualifiers = qualifiers;
        self
    }

    /// Copies the method's generic declaration and preserves source order.
    #[must_use]
    pub fn generic_definition(mut self, generic_definition: &GenericDefinitionDescriptor) -> Self {
        self.generic_definition = generic_definition.clone();
        self
    }

    /// Records whether the declared trait method has a default body.
    #[must_use]
    pub fn has_default(mut self, has_default: bool) -> Self {
        self.has_default = has_default;
        self
    }

    /// Builds the immutable declaration.
    #[must_use]
    pub fn build(self) -> MethodDescriptor {
        MethodDescriptor {
            identity: self.identity,
            rust_name: self.rust_name,
            query_name: self.query_name,
            visibility: self.visibility,
            receiver: self.receiver,
            parameters: self.parameters.into_boxed_slice(),
            return_value: self.return_value,
            qualifiers: self.qualifiers,
            generic_definition: self.generic_definition,
            has_default: self.has_default,
            declaration_owner: self.declaration_owner,
        }
    }
}

/// The effective source of a concrete method instance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodImplementationSource {
    /// A method declared directly by an inherent impl.
    Declared,
    /// A required trait declaration has no implementation adapter.
    Required,
    /// The instance uses a trait default for the concrete target type.
    Defaulted,
    /// The implementation explicitly overrides the trait declaration.
    Overridden,
}

/// A concrete specialization of one method declaration.
#[derive(Clone, Debug)]
pub struct MethodInstanceDescriptor {
    declaration: &'static MethodDescriptor,
    implementation_method: Option<&'static MethodDescriptor>,
    implementation_source: MethodImplementationSource,
    adapter: Option<&'static InvocationAdapter>,
    arguments: Box<[GenericArgument]>,
    unavailable_reasons: Box<[InvocationUnavailableReason]>,
}

/// An inconsistent method implementation source or invocation capability.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MethodInstanceBuildError {
    /// An inherent instance does not reference an impl-owned declaration.
    DeclaredMethodNotOwnedByImpl,
    /// A trait instance does not reference a trait-owned declaration.
    TraitMethodNotOwnedByTrait,
    /// A required method incorrectly advertises an invocation adapter.
    RequiredMethodHasAdapter,
    /// An overridden instance does not name its concrete impl method.
    OverriddenMethodMissingImplementation,
    /// A non-overridden instance names a concrete impl method.
    UnexpectedImplementationMethod,
    /// An available adapter and unavailable reasons were supplied together.
    AdapterHasUnavailableReasons,
    /// No adapter and no structured unavailable reason were supplied.
    UnavailableMethodMissingReasons,
}

impl fmt::Display for MethodInstanceBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeclaredMethodNotOwnedByImpl => {
                formatter.write_str("a declared inherent method must be owned by an impl")
            }
            Self::TraitMethodNotOwnedByTrait => formatter.write_str("a trait method instance must be owned by a trait"),
            Self::RequiredMethodHasAdapter => {
                formatter.write_str("a required method cannot have an invocation adapter")
            }
            Self::OverriddenMethodMissingImplementation => {
                formatter.write_str("an overridden method must name its impl method")
            }
            Self::UnexpectedImplementationMethod => {
                formatter.write_str("only an overridden method can name an impl method")
            }
            Self::AdapterHasUnavailableReasons => {
                formatter.write_str("an available invocation adapter cannot have unavailable reasons")
            }
            Self::UnavailableMethodMissingReasons => {
                formatter.write_str("an unavailable method must provide a structured reason")
            }
        }
    }
}

impl std::error::Error for MethodInstanceBuildError {}

impl MethodInstanceDescriptor {
    /// Creates a concrete method instance.
    ///
    /// `adapter` is present only when `unavailable_reasons` is empty and the
    /// later invocation layer supplied a safe entry point.
    #[doc(hidden)]
    pub fn new(
        declaration: &'static MethodDescriptor,
        implementation_method: Option<&'static MethodDescriptor>,
        implementation_source: MethodImplementationSource,
        adapter: Option<&'static InvocationAdapter>,
        unavailable_reasons: Box<[InvocationUnavailableReason]>,
    ) -> Result<Self, MethodInstanceBuildError> {
        Self::with_arguments(
            declaration,
            implementation_method,
            implementation_source,
            adapter,
            Box::new([]),
            unavailable_reasons,
        )
    }

    /// Binds and invokes this concrete method through its thread-safe
    /// panic-catching adapter.
    ///
    /// Returns `None` when this instance has no explicitly generated
    /// thread-safe catching adapter.
    #[must_use]
    pub fn invoke_catching_thread_safe<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<crate::invoke::CatchingInvocationResult<'call, crate::value::ThreadSafe>> {
        let entry_point = self.adapter?.catching_thread_safe?;
        Some(
            match invocation.bind_arguments(self.effective_method().identity(), self.effective_method().parameters()) {
                Ok(invocation) => entry_point(invocation),
                Err(failure) => Err(failure),
            },
        )
    }

    /// Creates a concrete method specialization with its generic arguments in
    /// declaration order.
    #[doc(hidden)]
    pub fn with_arguments(
        declaration: &'static MethodDescriptor,
        implementation_method: Option<&'static MethodDescriptor>,
        implementation_source: MethodImplementationSource,
        adapter: Option<&'static InvocationAdapter>,
        arguments: Box<[GenericArgument]>,
        unavailable_reasons: Box<[InvocationUnavailableReason]>,
    ) -> Result<Self, MethodInstanceBuildError> {
        if implementation_source == MethodImplementationSource::Required && adapter.is_some() {
            return Err(MethodInstanceBuildError::RequiredMethodHasAdapter);
        }
        match implementation_source {
            MethodImplementationSource::Declared if declaration.declaring_impl().is_none() => {
                return Err(MethodInstanceBuildError::DeclaredMethodNotOwnedByImpl);
            }
            MethodImplementationSource::Required
            | MethodImplementationSource::Defaulted
            | MethodImplementationSource::Overridden
                if declaration.declaring_trait().is_none() =>
            {
                return Err(MethodInstanceBuildError::TraitMethodNotOwnedByTrait);
            }
            _ => {}
        }
        match (implementation_source, implementation_method) {
            (MethodImplementationSource::Overridden, None) => {
                return Err(MethodInstanceBuildError::OverriddenMethodMissingImplementation);
            }
            (
                MethodImplementationSource::Declared
                | MethodImplementationSource::Required
                | MethodImplementationSource::Defaulted,
                Some(_),
            ) => {
                return Err(MethodInstanceBuildError::UnexpectedImplementationMethod);
            }
            _ => {}
        }
        if adapter.is_some() && !unavailable_reasons.is_empty() {
            return Err(MethodInstanceBuildError::AdapterHasUnavailableReasons);
        }
        if adapter.is_none() && unavailable_reasons.is_empty() {
            return Err(MethodInstanceBuildError::UnavailableMethodMissingReasons);
        }
        Ok(Self {
            declaration,
            implementation_method,
            implementation_source,
            adapter,
            arguments,
            unavailable_reasons,
        })
    }

    /// Returns the declaration shared by this concrete specialization.
    #[must_use]
    #[inline(always)]
    pub const fn declaration(&self) -> &'static MethodDescriptor {
        self.declaration
    }

    /// Returns the explicit impl method used by an overridden instance.
    ///
    /// `None` means the instance is required or uses its trait default.
    #[must_use]
    #[inline(always)]
    pub const fn implementation_method(&self) -> Option<&'static MethodDescriptor> {
        self.implementation_method
    }

    /// Returns the effective declaration or explicit implementation method.
    #[must_use]
    #[inline(always)]
    pub const fn effective_method(&self) -> &'static MethodDescriptor {
        match self.implementation_method {
            Some(method) => method,
            None => self.declaration,
        }
    }

    /// Returns whether the implementation is required, defaulted, or
    /// overridden.
    #[must_use]
    #[inline(always)]
    pub const fn implementation_source(&self) -> MethodImplementationSource {
        self.implementation_source
    }

    /// Returns the safe invocation adapter when one is available.
    ///
    /// `None` means callers must inspect [`Self::unavailable_reasons`].
    #[must_use]
    #[inline(always)]
    pub const fn adapter(&self) -> Option<&'static InvocationAdapter> {
        self.adapter
    }

    /// Returns the concrete type and const arguments of this method
    /// specialization in declaration order.
    #[must_use]
    #[inline(always)]
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns stable reasons that prevent dynamic invocation.
    #[must_use]
    #[inline(always)]
    pub const fn unavailable_reasons(&self) -> &[InvocationUnavailableReason] {
        &self.unavailable_reasons
    }

    /// Binds and invokes this concrete method through its local adapter.
    ///
    /// Positional inputs bind the next unoccupied declaration-order parameter;
    /// named inputs may be interleaved and bind only a unique simple
    /// identifier parameter. Binding, receiver, mode, and exact-type checks
    /// all occur before the generated adapter extracts any owned value. Their
    /// failures therefore retain the complete original invocation recovery.
    ///
    /// Returns `None` when this instance has no local adapter. Otherwise the
    /// result contains either the invocation output or a structured
    /// pre-execution failure.
    #[must_use]
    pub fn invoke_local<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::InvocationFailure<'call, crate::value::Local>,
        >,
    > {
        let entry_point = self.adapter?.local?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(entry_point),
        )
    }

    /// Binds and invokes this concrete method through its thread-safe adapter.
    ///
    /// Binding uses the same interleaved named/positional rules and complete
    /// pre-execution recovery contract as [`Self::invoke_local`].
    ///
    /// Returns `None` when this instance has no explicitly generated
    /// thread-safe adapter. Otherwise the result contains either the invocation
    /// output or a structured pre-execution failure.
    #[must_use]
    pub fn invoke_thread_safe<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::ThreadSafe>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::ThreadSafe>,
            crate::invoke::InvocationFailure<'call, crate::value::ThreadSafe>,
        >,
    > {
        let entry_point = self.adapter?.thread_safe?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(entry_point),
        )
    }

    /// Binds and invokes this concrete method through its local panic-catching
    /// adapter.
    ///
    /// Named and positional inputs follow the same descriptor-aware binding
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// A panic after successful validation is returned as
    /// [`InvocationPanic`](crate::invoke::InvocationPanic), independently of
    /// binding or type-validation failures.
    ///
    /// Returns `None` when this instance has no explicitly generated local
    /// catching adapter.
    #[must_use]
    pub fn invoke_catching_local<'call>(
        &self,
        invocation: crate::invoke::Invocation<'call, crate::value::Local>,
    ) -> Option<crate::invoke::CatchingInvocationResult<'call, crate::value::Local>> {
        let entry_point = self.adapter?.catching_local?;
        Some(
            match invocation.bind_arguments(self.effective_method().identity(), self.effective_method().parameters()) {
                Ok(invocation) => entry_point(invocation),
                Err(failure) => Err(failure),
            },
        )
    }

    /// Binds and invokes this concrete method through a typed local
    /// `Pin<&T>` adapter.
    ///
    /// Named and positional arguments follow the same descriptor-aware rules
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// The receiver remains typed and pinned throughout binding and adapter
    /// validation.
    ///
    /// Returns `None` when this instance has no pinned shared adapter for the
    /// exact receiver type `T`.
    #[must_use]
    pub fn invoke_pinned_ref_local<'call, T: 'static>(
        &self,
        invocation: crate::invoke::PinnedRefInvocation<'call, T, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedRefInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        let entry_point = self
            .adapter?
            .pinned_ref_local?
            .downcast_ref::<crate::invoke::PinnedRefAdapter<T, crate::value::Local>>()?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(entry_point),
        )
    }

    /// Binds and invokes this concrete method through a typed local
    /// `Pin<&mut T>` adapter.
    ///
    /// Named and positional arguments follow the same descriptor-aware rules
    /// and complete pre-execution recovery contract as [`Self::invoke_local`].
    /// The receiver remains typed and pinned throughout binding and adapter
    /// validation.
    ///
    /// Returns `None` when this instance has no pinned mutable adapter for the
    /// exact receiver type `T`.
    #[must_use]
    pub fn invoke_pinned_mut_local<'call, T: 'static>(
        &self,
        invocation: crate::invoke::PinnedMutInvocation<'call, T, crate::value::Local>,
    ) -> Option<
        Result<
            crate::invoke::InvocationOutput<'call, crate::value::Local>,
            crate::invoke::PinnedMutInvocationFailure<'call, T, crate::value::Local>,
        >,
    > {
        let entry_point = self
            .adapter?
            .pinned_mut_local?
            .downcast_ref::<crate::invoke::PinnedMutAdapter<T, crate::value::Local>>()?;
        Some(
            invocation
                .bind_arguments(self.effective_method().identity(), self.effective_method().parameters())
                .and_then(entry_point),
        )
    }
}
