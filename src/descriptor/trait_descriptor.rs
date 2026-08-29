// qubit-style: allow public-type-layout
//! Trait definitions, concrete applications, supertraits, and associated items.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::descriptor::MethodDescriptor;
use crate::expression::ConstExpression;
use crate::expression::GenericArgument;
use crate::expression::GenericDefinitionDescriptor;
use crate::expression::GenericParameterDescriptor;
use crate::expression::PredicateDescriptor;
use crate::expression::TypeExpression;
use crate::identity::ExternalTraitId;
use crate::identity::Visibility;

type AppliedTraitCache = HashMap<(TypeId, AppliedTraitId), Arc<OnceLock<&'static TraitDescriptor>>>;
type ExternalSupertraitCache = HashMap<(TypeId, ExternalTraitId, Box<[GenericArgument]>), &'static TraitDescriptor>;

/// The process-local identity source of a reflected or external trait.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TraitId {
    /// A marker type generated for a reflected trait declaration.
    Reflected(TypeId),
    /// A stable, caller-supplied identity for an unreflected trait.
    External(ExternalTraitId),
}

/// The complete identity of one concrete trait application.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppliedTraitId {
    definition: TraitId,
    arguments: Box<[GenericArgument]>,
}

impl AppliedTraitId {
    /// Returns the reflected marker or external definition identity.
    pub const fn definition(&self) -> &TraitId {
        &self.definition
    }

    /// Returns concrete type and const arguments in declaration order.
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }
}

/// How much of a trait declaration is known to reflection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraitCompleteness {
    /// The trait declaration, supertraits, and associated items are known.
    Complete,
    /// Only facts proven by an observed external trait impl are known.
    ExternalIncomplete,
}

/// Declaration-level facts shared by every concrete application of a trait.
#[derive(Debug)]
pub struct TraitDefinitionDescriptor {
    trait_id: TraitId,
    rust_name: &'static str,
    rust_path: &'static str,
    query_name: &'static str,
    completeness: TraitCompleteness,
    generic_definition: &'static GenericDefinitionDescriptor,
    visibility: Visibility,
}

/// Concrete trait facts supplied by a reflected trait's hidden implementation
/// hook.
///
/// The hook carries declaration identity without imposing object-safety
/// requirements on the reflected trait. Implementation expansion enriches this
/// payload with concrete application details before it becomes part of an
/// implementation descriptor.
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct TraitImplPayload {
    definition: &'static TraitDefinitionDescriptor,
    applied: &'static TraitDescriptor,
}

impl TraitImplPayload {
    /// Creates a payload for one reflected trait declaration.
    #[doc(hidden)]
    pub const fn new(definition: &'static TraitDefinitionDescriptor, applied: &'static TraitDescriptor) -> Self {
        Self { definition, applied }
    }

    /// Returns the complete trait declaration shared by every implementation.
    #[doc(hidden)]
    pub const fn definition(self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the concrete applied trait descriptor for the hook receiver.
    #[doc(hidden)]
    pub const fn applied(self) -> &'static TraitDescriptor {
        self.applied
    }

    /// Reuses an applied descriptor without constructing a discarded candidate
    /// on cache hits.
    #[doc(hidden)]
    pub fn cached_with_arguments<T: ?Sized + 'static>(
        definition: &'static TraitDefinitionDescriptor,
        arguments: Vec<GenericArgument>,
        build: impl FnOnce(Vec<GenericArgument>) -> Result<TraitDescriptor, TraitDescriptorBuildError>,
    ) -> Self {
        static CACHE: LazyLock<Mutex<AppliedTraitCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));
        let identity = AppliedTraitId {
            definition: definition.trait_id().clone(),
            arguments: arguments.clone().into_boxed_slice(),
        };
        let key = (TypeId::of::<T>(), identity);
        let mut cache = CACHE.lock().expect("trait payload cache mutex must not be poisoned");
        let cell = cache.entry(key).or_insert_with(|| Arc::new(OnceLock::new())).clone();
        drop(cache);
        let applied = *cell.get_or_init(|| {
            Box::leak(Box::new(
                build(arguments).expect("a reflected trait must build a valid applied descriptor"),
            ))
        });
        Self::new(definition, applied)
    }
}

/// Returns a cached incomplete descriptor for an explicitly mapped external
/// supertrait.
#[doc(hidden)]
pub fn external_supertrait<T: ?Sized + 'static>(
    id: &'static str,
    rust_path: &'static str,
    arguments: Vec<GenericArgument>,
) -> &'static TraitDescriptor {
    static CACHE: LazyLock<Mutex<ExternalSupertraitCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));
    let external_id =
        ExternalTraitId::new(id).expect("the macro validator must only emit valid external trait identifiers");
    let key = (
        TypeId::of::<T>(),
        external_id.clone(),
        arguments.clone().into_boxed_slice(),
    );
    let mut cache = CACHE.lock().expect("external trait cache mutex must not be poisoned");
    cache.entry(key).or_insert_with(|| {
        let definition = Box::leak(Box::new(TraitDefinitionDescriptor::new(
            TraitId::External(external_id),
            rust_path.rsplit("::").next().unwrap_or(rust_path),
            rust_path,
            rust_path,
            TraitCompleteness::ExternalIncomplete,
            Box::leak(Box::new(GenericDefinitionDescriptor {
                parameters: Box::new([]),
                predicates: Box::new([]),
                diagnostic: crate::expression::DiagnosticText::default(),
            })),
        )));
        Box::leak(Box::new(
            TraitDescriptor::builder(definition)
                .arguments(arguments)
                .build()
                .expect("an external supertrait descriptor must be valid"),
        ))
    })
}

impl TraitDefinitionDescriptor {
    /// Creates immutable trait definition facts.
    #[doc(hidden)]
    pub const fn new(
        trait_id: TraitId,
        rust_name: &'static str,
        rust_path: &'static str,
        query_name: &'static str,
        completeness: TraitCompleteness,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Self {
        Self::new_with_visibility(
            trait_id,
            rust_name,
            rust_path,
            query_name,
            completeness,
            generic_definition,
            Visibility::Private,
        )
    }

    /// Creates immutable trait definition facts with normalized source
    /// visibility.
    #[doc(hidden)]
    pub const fn new_with_visibility(
        trait_id: TraitId,
        rust_name: &'static str,
        rust_path: &'static str,
        query_name: &'static str,
        completeness: TraitCompleteness,
        generic_definition: &'static GenericDefinitionDescriptor,
        visibility: Visibility,
    ) -> Self {
        Self {
            trait_id,
            rust_name,
            rust_path,
            query_name,
            completeness,
            generic_definition,
            visibility,
        }
    }

    /// Returns the trait declaration's normalized source visibility.
    pub const fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Returns the reflected marker or external trait identity.
    pub const fn trait_id(&self) -> &TraitId {
        &self.trait_id
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the diagnostic fully qualified Rust path.
    pub const fn rust_path(&self) -> &'static str {
        self.rust_path
    }

    /// Returns the lookup name, which may differ from the Rust name.
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns whether the complete declaration is known.
    pub const fn completeness(&self) -> TraitCompleteness {
        self.completeness
    }

    /// Returns generic parameters and predicates in source order.
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }
}

/// One associated type declaration.
#[derive(Clone, Debug)]
pub struct AssociatedTypeDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    bounds: Box<[PredicateDescriptor]>,
    default: Option<TypeExpression>,
}

impl AssociatedTypeDescriptor {
    /// Creates associated type facts in declaration order.
    #[doc(hidden)]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        bounds: Box<[PredicateDescriptor]>,
        default: Option<TypeExpression>,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            bounds,
            default,
        }
    }

    /// Returns the source declaration index.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns declared bounds in source order.
    pub const fn bounds(&self) -> &[PredicateDescriptor] {
        &self.bounds
    }

    /// Returns the symbolic default type.
    ///
    /// `None` means the trait requires implementations to provide the binding.
    pub const fn default(&self) -> Option<&TypeExpression> {
        self.default.as_ref()
    }
}

/// One associated constant declaration.
#[derive(Clone, Debug)]
pub struct AssociatedConstDescriptor {
    index: usize,
    rust_name: &'static str,
    query_name: &'static str,
    declared_type: TypeExpression,
    has_default: bool,
}

impl AssociatedConstDescriptor {
    /// Creates associated constant facts in declaration order.
    #[doc(hidden)]
    pub const fn new(
        index: usize,
        rust_name: &'static str,
        query_name: &'static str,
        declared_type: TypeExpression,
        has_default: bool,
    ) -> Self {
        Self {
            index,
            rust_name,
            query_name,
            declared_type,
            has_default,
        }
    }

    /// Returns the source declaration index.
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.rust_name
    }

    /// Returns the lookup name.
    pub const fn query_name(&self) -> &'static str {
        self.query_name
    }

    /// Returns the declared constant type.
    pub const fn declared_type(&self) -> &TypeExpression {
        &self.declared_type
    }

    /// Returns whether the trait declaration provides a default value.
    pub const fn has_default(&self) -> bool {
        self.has_default
    }
}

/// A static reference used by direct and transitive supertrait views.
#[derive(Clone, Copy, Debug)]
pub struct TraitDescriptorRef(&'static TraitDescriptor);

impl TraitDescriptorRef {
    /// Creates a supertrait reference.
    #[doc(hidden)]
    pub const fn new(descriptor: &'static TraitDescriptor) -> Self {
        Self(descriptor)
    }

    /// Returns the referenced applied trait descriptor.
    pub const fn descriptor(self) -> &'static TraitDescriptor {
        self.0
    }
}

impl Deref for TraitDescriptorRef {
    type Target = TraitDescriptor;

    /// Dereferences to the applied trait descriptor.
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A deterministic, duplicate-free transitive supertrait view.
#[derive(Clone, Copy, Debug)]
pub struct SupertraitClosure<'a> {
    descriptors: &'a [TraitDescriptorRef],
}

impl<'a> SupertraitClosure<'a> {
    /// Returns applied supertraits in deterministic path order.
    pub fn iter(self) -> impl ExactSizeIterator<Item = &'a TraitDescriptor> {
        self.descriptors.iter().map(|descriptor| descriptor.descriptor())
    }

    /// Returns the number of distinct transitive supertraits.
    pub const fn len(self) -> usize {
        self.descriptors.len()
    }

    /// Returns whether the closure is empty.
    pub const fn is_empty(self) -> bool {
        self.descriptors.is_empty()
    }
}

/// An applied trait descriptor with concrete generic arguments.
pub struct TraitDescriptor {
    definition: &'static TraitDefinitionDescriptor,
    trait_id: AppliedTraitId,
    arguments: Box<[GenericArgument]>,
    direct_supertraits: Box<[TraitDescriptorRef]>,
    all_supertraits: Box<[TraitDescriptorRef]>,
    methods: &'static [MethodDescriptor],
    associated_types: Box<[AssociatedTypeDescriptor]>,
    associated_consts: Box<[AssociatedConstDescriptor]>,
}

impl TraitDescriptor {
    /// Starts an applied trait builder for `definition`.
    pub fn builder(definition: &'static TraitDefinitionDescriptor) -> TraitDescriptorBuilder {
        TraitDescriptorBuilder::new(definition)
    }

    /// Returns the declaration-level descriptor shared by every application.
    pub const fn definition(&self) -> &'static TraitDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected marker or external identity.
    pub const fn trait_id(&self) -> &AppliedTraitId {
        &self.trait_id
    }

    /// Returns concrete generic arguments in declaration order.
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Returns the Rust declaration name.
    pub const fn rust_name(&self) -> &'static str {
        self.definition.rust_name()
    }

    /// Returns the diagnostic fully qualified Rust path.
    pub const fn rust_path(&self) -> &'static str {
        self.definition.rust_path()
    }

    /// Returns the lookup name.
    pub const fn query_name(&self) -> &'static str {
        self.definition.query_name()
    }

    /// Returns whether this descriptor contains a complete declaration.
    pub const fn completeness(&self) -> TraitCompleteness {
        self.definition.completeness()
    }

    /// Returns direct supertraits in source declaration order.
    pub const fn direct_supertraits(&self) -> &[TraitDescriptorRef] {
        &self.direct_supertraits
    }

    /// Returns the sorted, duplicate-free, transitive supertrait closure.
    pub const fn all_supertraits(&self) -> SupertraitClosure<'_> {
        SupertraitClosure {
            descriptors: &self.all_supertraits,
        }
    }

    /// Returns method declarations in source order.
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a method by query name.
    ///
    /// `None` means this applied trait has no method with the requested name.
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods.iter().find(|method| method.query_name() == name)
    }

    /// Returns associated type declarations in source order.
    pub const fn associated_types(&self) -> &[AssociatedTypeDescriptor] {
        &self.associated_types
    }

    /// Finds an associated type by query name.
    ///
    /// `None` means no associated type has the requested name.
    pub fn associated_type(&self, name: &str) -> Option<&AssociatedTypeDescriptor> {
        self.associated_types.iter().find(|item| item.query_name() == name)
    }

    /// Returns associated constant declarations in source order.
    pub const fn associated_consts(&self) -> &[AssociatedConstDescriptor] {
        &self.associated_consts
    }

    /// Finds an associated constant by query name.
    ///
    /// `None` means no associated constant has the requested name.
    pub fn associated_const(&self, name: &str) -> Option<&AssociatedConstDescriptor> {
        self.associated_consts.iter().find(|item| item.query_name() == name)
    }

    /// Returns whether two descriptors are the same concrete trait application.
    pub fn same_application(&self, other: &Self) -> bool {
        self.trait_id == other.trait_id
    }
}

impl fmt::Debug for TraitDescriptor {
    /// Formats local facts without recursively expanding supertraits.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TraitDescriptor")
            .field("definition", &self.definition)
            .field("arguments", &self.arguments)
            .field("direct_supertrait_count", &self.direct_supertraits.len())
            .field("all_supertrait_count", &self.all_supertraits.len())
            .field("method_count", &self.methods.len())
            .field("associated_type_count", &self.associated_types.len())
            .field("associated_const_count", &self.associated_consts.len())
            .finish()
    }
}

/// An invalid applied trait graph or incomplete external declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraitDescriptorBuildError {
    /// A supertrait resolves to the application currently being built.
    RecursiveSupertrait {
        /// Diagnostic Rust path of the recursive application.
        rust_path: &'static str,
    },
    /// An external incomplete trait attempted to claim unobservable facts.
    ExternalTraitHasUnprovenFacts,
    /// The number of concrete type/const arguments does not match the
    /// definition.
    GenericArgumentCount {
        /// Number of runtime identity arguments required by the definition.
        expected: usize,
        /// Number of arguments supplied by the applied descriptor.
        actual: usize,
    },
    /// An argument kind does not match its type or const parameter.
    GenericArgumentKind {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// An argument still contains a symbolic type or const parameter.
    NonConcreteGenericArgument {
        /// Zero-based runtime identity argument index.
        index: usize,
    },
    /// A method declaration belongs to another trait or an impl.
    ForeignMethod,
}

impl fmt::Display for TraitDescriptorBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecursiveSupertrait { rust_path } => {
                write!(formatter, "recursive supertrait application: {rust_path}")
            }
            Self::ExternalTraitHasUnprovenFacts => {
                formatter.write_str("an external incomplete trait cannot claim supertraits or associated items")
            }
            Self::GenericArgumentCount { expected, actual } => write!(
                formatter,
                "trait application requires {expected} concrete arguments but received {actual}"
            ),
            Self::GenericArgumentKind { index } => {
                write!(formatter, "trait argument {index} has the wrong generic kind")
            }
            Self::NonConcreteGenericArgument { index } => {
                write!(formatter, "trait argument {index} is not concrete")
            }
            Self::ForeignMethod => formatter.write_str("applied trait contains a foreign method declaration"),
        }
    }
}

impl std::error::Error for TraitDescriptorBuildError {}

/// Builds one applied trait and validates its supertrait closure.
#[derive(Debug)]
pub struct TraitDescriptorBuilder {
    definition: &'static TraitDefinitionDescriptor,
    arguments: Vec<GenericArgument>,
    direct_supertraits: Vec<TraitDescriptorRef>,
    methods: &'static [MethodDescriptor],
    associated_types: Vec<AssociatedTypeDescriptor>,
    associated_consts: Vec<AssociatedConstDescriptor>,
}

impl TraitDescriptorBuilder {
    /// Creates an empty applied view for `definition`.
    fn new(definition: &'static TraitDefinitionDescriptor) -> Self {
        Self {
            definition,
            arguments: Vec::new(),
            direct_supertraits: Vec::new(),
            methods: &[],
            associated_types: Vec::new(),
            associated_consts: Vec::new(),
        }
    }

    /// Sets concrete generic arguments in declaration order.
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Sets direct supertraits in source declaration order.
    pub fn direct_supertraits<const N: usize>(mut self, direct_supertraits: [&'static TraitDescriptor; N]) -> Self {
        self.direct_supertraits = direct_supertraits.into_iter().map(TraitDescriptorRef::new).collect();
        self
    }

    /// Sets applied method declarations in source order.
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets applied associated types in source order.
    pub fn associated_types(mut self, associated_types: Vec<AssociatedTypeDescriptor>) -> Self {
        self.associated_types = associated_types;
        self
    }

    /// Sets applied associated constants in source order.
    pub fn associated_consts(mut self, associated_consts: Vec<AssociatedConstDescriptor>) -> Self {
        self.associated_consts = associated_consts;
        self
    }

    /// Validates and builds the applied trait descriptor.
    ///
    /// Returns [`TraitDescriptorBuildError`] for recursive supertraits or when
    /// an incomplete external trait claims supertraits or associated items.
    pub fn build(self) -> Result<TraitDescriptor, TraitDescriptorBuildError> {
        self.validate_arguments()?;
        if self.methods.iter().any(|method| {
            !method
                .declaring_trait()
                .is_some_and(|owner| std::ptr::eq(owner, self.definition))
        }) {
            return Err(TraitDescriptorBuildError::ForeignMethod);
        }
        if self.definition.completeness() == TraitCompleteness::ExternalIncomplete
            && (!self.direct_supertraits.is_empty()
                || !self.associated_types.is_empty()
                || !self.associated_consts.is_empty())
        {
            return Err(TraitDescriptorBuildError::ExternalTraitHasUnprovenFacts);
        }

        let mut all_supertraits = Vec::new();
        for direct in &self.direct_supertraits {
            self.collect_supertrait(direct.descriptor(), &mut all_supertraits)?;
        }
        all_supertraits.sort_by(|left, right| {
            left.rust_path()
                .cmp(right.rust_path())
                .then_with(|| left.query_name().cmp(right.query_name()))
        });

        let trait_id = AppliedTraitId {
            definition: self.definition.trait_id().clone(),
            arguments: self.arguments.clone().into_boxed_slice(),
        };
        Ok(TraitDescriptor {
            definition: self.definition,
            trait_id,
            arguments: self.arguments.into_boxed_slice(),
            direct_supertraits: self.direct_supertraits.into_boxed_slice(),
            all_supertraits: all_supertraits.into_boxed_slice(),
            methods: self.methods,
            associated_types: self.associated_types.into_boxed_slice(),
            associated_consts: self.associated_consts.into_boxed_slice(),
        })
    }

    /// Adds `candidate` and its direct ancestors while rejecting recursion and
    /// duplicate applied identities.
    fn collect_supertrait(
        &self,
        candidate: &'static TraitDescriptor,
        closure: &mut Vec<TraitDescriptorRef>,
    ) -> Result<(), TraitDescriptorBuildError> {
        if candidate.trait_id().definition() == self.definition.trait_id()
            && candidate.trait_id().arguments() == self.arguments
        {
            return Err(TraitDescriptorBuildError::RecursiveSupertrait {
                rust_path: self.definition.rust_path(),
            });
        }
        if closure.iter().any(|existing| existing.same_application(candidate)) {
            return Ok(());
        }
        closure.push(TraitDescriptorRef::new(candidate));
        for ancestor in candidate.direct_supertraits() {
            self.collect_supertrait(ancestor.descriptor(), closure)?;
        }
        Ok(())
    }

    /// Verifies every runtime identity parameter has one concrete argument of
    /// the matching generic kind.
    fn validate_arguments(&self) -> Result<(), TraitDescriptorBuildError> {
        if self.definition.completeness() == TraitCompleteness::ExternalIncomplete {
            for (index, argument) in self.arguments.iter().enumerate() {
                if !generic_argument_is_concrete(argument) {
                    return Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index });
                }
            }
            return Ok(());
        }
        let parameters: Vec<_> = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .collect();
        if parameters.len() != self.arguments.len() {
            return Err(TraitDescriptorBuildError::GenericArgumentCount {
                expected: parameters.len(),
                actual: self.arguments.len(),
            });
        }
        for (index, (parameter, argument)) in parameters.into_iter().zip(&self.arguments).enumerate() {
            let kind_matches = matches!(
                (parameter, argument),
                (GenericParameterDescriptor::Type { .. }, GenericArgument::Type(_))
                    | (GenericParameterDescriptor::Const { .. }, GenericArgument::Const(_))
            );
            if !kind_matches {
                return Err(TraitDescriptorBuildError::GenericArgumentKind { index });
            }
            if !generic_argument_is_concrete(argument) {
                return Err(TraitDescriptorBuildError::NonConcreteGenericArgument { index });
            }
        }
        Ok(())
    }
}

/// Returns whether an argument contains only concrete runtime identity facts.
pub(super) fn generic_argument_is_concrete(argument: &GenericArgument) -> bool {
    match argument {
        GenericArgument::Type(expression) => type_expression_is_concrete(expression),
        GenericArgument::Const(argument) => !matches!(argument.value, ConstExpression::Parameter(_)),
        GenericArgument::Lifetime(_) => true,
        GenericArgument::AssociatedType { value, .. } => type_expression_is_concrete(value),
        GenericArgument::AssociatedTypeBound { .. } => false,
    }
}

/// Returns whether a substituted type expression contains no symbolic type.
fn type_expression_is_concrete(expression: &TypeExpression) -> bool {
    match expression {
        TypeExpression::Concrete(concrete) => concrete.arguments.iter().all(generic_argument_is_concrete),
        TypeExpression::Reference(reference) => type_expression_is_concrete(&reference.target),
        TypeExpression::RawPointer(pointer) => type_expression_is_concrete(&pointer.target),
        TypeExpression::Slice(element) => type_expression_is_concrete(element),
        TypeExpression::Array(array) => {
            type_expression_is_concrete(&array.element) && !matches!(array.length, ConstExpression::Parameter(_))
        }
        TypeExpression::Tuple(elements) => elements.iter().all(type_expression_is_concrete),
        TypeExpression::FunctionPointer(function) => {
            function.parameters.iter().all(type_expression_is_concrete)
                && type_expression_is_concrete(&function.return_type)
        }
        TypeExpression::TraitObject(_) | TypeExpression::Never => true,
        TypeExpression::Parameter(_)
        | TypeExpression::SelfType
        | TypeExpression::Associated(_)
        | TypeExpression::Opaque(_) => false,
    }
}
