//! Immutable declarations and concrete instances of reflected methods.

use std::fmt;

use crate::descriptor::{
    ImplDefinitionDescriptor, TraitDefinitionDescriptor, TypeDescriptor, TypeDescriptorResolver,
};
use crate::expression::{FunctionAbi, GenericDefinitionDescriptor, TypeExpression};
use crate::identity::{MemberId, Visibility};

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
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the identifier used for named binding.
    ///
    /// `None` denotes a wildcard or destructuring pattern.
    pub const fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Returns the parser-independent source pattern category.
    pub const fn pattern(&self) -> &ParameterPatternDescriptor {
        &self.pattern
    }

    /// Returns how the argument crosses the method boundary.
    pub const fn passing_mode(&self) -> ParameterPassingMode {
        self.passing_mode
    }

    /// Returns the declared, possibly symbolic parameter type.
    pub const fn signature_type(&self) -> &TypeExpression {
        &self.signature_type
    }

    /// Returns the exact reflected parameter type when it is known.
    ///
    /// `None` denotes a symbolic, opaque, or otherwise unresolved type.
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
    pub const fn unit() -> Self {
        Self::new(ReturnKind::Unit, None, None)
    }

    /// Returns the structural return category.
    pub const fn kind(&self) -> ReturnKind {
        self.kind
    }

    /// Returns the declared return type expression.
    ///
    /// `None` means the unit or never category carries the complete fact.
    pub const fn signature_type(&self) -> Option<&TypeExpression> {
        self.signature_type.as_ref()
    }

    /// Returns the exact reflected return type when it is known.
    ///
    /// `None` denotes unit, never, a reference, opaque output, or an unresolved
    /// symbolic type.
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
    pub is_async: bool,
    /// Whether the declaration is `unsafe`.
    pub is_unsafe: bool,
    /// Whether the declaration is `const`.
    pub is_const: bool,
    /// The explicitly declared ABI, or `None` for the ordinary Rust ABI.
    pub abi: Option<FunctionAbi>,
    /// Whether the declaration has a variadic tail.
    pub is_variadic: bool,
}

impl Default for MethodQualifiers {
    /// Returns the qualifiers of an ordinary safe Rust method.
    fn default() -> Self {
        Self {
            is_async: false,
            is_unsafe: false,
            is_const: false,
            abi: None,
            is_variadic: false,
        }
    }
}

/// A stable reason why a concrete method instance cannot be invoked dynamically.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InvocationUnavailableReason {
    /// The receiver form has no safe adapter.
    UnsupportedReceiver,
    /// A parameter pattern has no safe adapter.
    UnsupportedParameterPattern,
    /// The declaration is generic and has no registered specialization.
    UnspecializedGeneric,
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
    /// Invocation was disabled by reflection policy.
    DisabledByPolicy,
}

/// An opaque invocation entry point supplied by a later invocation layer.
///
/// This descriptor layer records adapter identity and availability. The
/// invocation layer owns argument validation and the complete call contract.
#[derive(Clone, Copy, Debug)]
pub struct InvocationAdapter {
    entry_point: fn(),
}

impl InvocationAdapter {
    /// Creates an opaque adapter token for generated descriptor data.
    #[doc(hidden)]
    pub const fn new(entry_point: fn()) -> Self {
        Self { entry_point }
    }

    /// Returns the opaque entry-point identity.
    #[doc(hidden)]
    pub const fn entry_point(&self) -> fn() {
        self.entry_point
    }
}

/// An immutable reflected method declaration.
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
    pub fn builder(
        identity: MemberId,
        rust_name: &'static str,
        query_name: &'static str,
        declaration_owner: MethodDeclarationOwner,
    ) -> MethodDescriptorBuilder {
        MethodDescriptorBuilder::new(identity, rust_name, query_name, declaration_owner)
    }

    /// Returns the stable composite member identity.
    pub const fn identity(&self) -> &MemberId {
        &self.identity
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns normalized source visibility facts.
    pub const fn visibility(&self) -> &MethodVisibility {
        &self.visibility
    }

    /// Returns the receiver, or `None` for an associated function.
    pub const fn receiver(&self) -> Option<&ReceiverDescriptor> {
        self.receiver.as_ref()
    }

    /// Returns non-receiver parameters in source declaration order.
    pub const fn parameters(&self) -> &[ParameterDescriptor] {
        &self.parameters
    }

    /// Finds a uniquely named identifier parameter.
    ///
    /// `None` means no parameter has the requested identifier.
    pub fn parameter(&self, name: &str) -> Option<&ParameterDescriptor> {
        self.parameters
            .iter()
            .find(|parameter| parameter.name() == Some(name))
    }

    /// Returns a non-receiver parameter by declaration index.
    ///
    /// `None` means `index` is outside the parameter range.
    pub fn parameter_at(&self, index: usize) -> Option<&ParameterDescriptor> {
        self.parameters.get(index)
    }

    /// Returns the declared return facts.
    pub const fn return_value(&self) -> &ReturnDescriptor {
        &self.return_value
    }

    /// Returns callability-related source qualifiers.
    pub const fn qualifiers(&self) -> &MethodQualifiers {
        &self.qualifiers
    }

    /// Returns generic parameters and predicates in source order.
    pub const fn generic_definition(&self) -> &GenericDefinitionDescriptor {
        &self.generic_definition
    }

    /// Returns the owning trait definition for a trait method.
    ///
    /// `None` means this method is declared by an impl definition.
    pub const fn declaring_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(descriptor) => Some(descriptor),
            MethodDeclarationOwner::Impl(_) => None,
        }
    }

    /// Returns the owning impl definition for an implementation method.
    ///
    /// `None` means this method is declared by a trait definition.
    pub const fn declaring_impl(&self) -> Option<&'static ImplDefinitionDescriptor> {
        match self.declaration_owner {
            MethodDeclarationOwner::Trait(_) => None,
            MethodDeclarationOwner::Impl(descriptor) => Some(descriptor),
        }
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
            declaration_owner,
        }
    }

    /// Sets normalized source visibility.
    pub fn visibility(mut self, visibility: MethodVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Sets the receiver; `None` describes an associated function.
    pub fn receiver(mut self, receiver: Option<ReceiverDescriptor>) -> Self {
        self.receiver = receiver;
        self
    }

    /// Sets non-receiver parameters in source order.
    pub fn parameters(mut self, parameters: Vec<ParameterDescriptor>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Sets the return declaration.
    pub fn return_value(mut self, return_value: ReturnDescriptor) -> Self {
        self.return_value = return_value;
        self
    }

    /// Sets source qualifiers that affect invocation availability.
    pub fn qualifiers(mut self, qualifiers: MethodQualifiers) -> Self {
        self.qualifiers = qualifiers;
        self
    }

    /// Copies the method's generic declaration and preserves source order.
    pub fn generic_definition(mut self, generic_definition: &GenericDefinitionDescriptor) -> Self {
        self.generic_definition = generic_definition.clone();
        self
    }

    /// Builds the immutable declaration.
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
            Self::TraitMethodNotOwnedByTrait => {
                formatter.write_str("a trait method instance must be owned by a trait")
            }
            Self::RequiredMethodHasAdapter => {
                formatter.write_str("a required method cannot have an invocation adapter")
            }
            Self::OverriddenMethodMissingImplementation => {
                formatter.write_str("an overridden method must name its impl method")
            }
            Self::UnexpectedImplementationMethod => {
                formatter.write_str("only an overridden method can name an impl method")
            }
            Self::AdapterHasUnavailableReasons => formatter
                .write_str("an available invocation adapter cannot have unavailable reasons"),
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
            unavailable_reasons,
        })
    }

    /// Returns the declaration shared by this concrete specialization.
    pub const fn declaration(&self) -> &'static MethodDescriptor {
        self.declaration
    }

    /// Returns the explicit impl method used by an overridden instance.
    ///
    /// `None` means the instance is required or uses its trait default.
    pub const fn implementation_method(&self) -> Option<&'static MethodDescriptor> {
        self.implementation_method
    }

    /// Returns the effective declaration or explicit implementation method.
    pub const fn effective_method(&self) -> &'static MethodDescriptor {
        match self.implementation_method {
            Some(method) => method,
            None => self.declaration,
        }
    }

    /// Returns whether the implementation is required, defaulted, or overridden.
    pub const fn implementation_source(&self) -> MethodImplementationSource {
        self.implementation_source
    }

    /// Returns the safe invocation adapter when one is available.
    ///
    /// `None` means callers must inspect [`Self::unavailable_reasons`].
    pub const fn adapter(&self) -> Option<&'static InvocationAdapter> {
        self.adapter
    }

    /// Returns stable reasons that prevent dynamic invocation.
    pub const fn unavailable_reasons(&self) -> &[InvocationUnavailableReason] {
        &self.unavailable_reasons
    }
}
