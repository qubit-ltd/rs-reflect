//! Reflected inherent and trait implementation descriptors.

use std::fmt;

use super::trait_descriptor::generic_argument_is_concrete;
use crate::descriptor::{
    AssociatedConstDescriptor, AssociatedTypeDescriptor, MethodDescriptor,
    MethodInstanceDescriptor, TraitDefinitionDescriptor, TraitDescriptor, TypeDescriptor,
    TypeDescriptorResolver,
};
use crate::expression::{
    GenericArgument, GenericDefinitionDescriptor, GenericParameterDescriptor, TypeExpression,
};
use crate::identity::FragmentIdentity;
use crate::value::ReflectedOwned;

/// Whether an implementation is inherent or implements a trait.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplKind {
    /// An inherent implementation block.
    Inherent,
    /// A trait implementation block.
    Trait,
}

/// Declaration facts for a generic, blanket, or concrete impl block.
#[derive(Debug)]
pub struct ImplDefinitionDescriptor {
    fragment_identity: FragmentIdentity,
    target_type: TypeExpression,
    kind: ImplKind,
    implemented_trait: Option<&'static TraitDefinitionDescriptor>,
    generic_definition: &'static GenericDefinitionDescriptor,
}

impl ImplDefinitionDescriptor {
    /// Creates an impl definition without claiming a concrete target instance.
    ///
    /// Returns [`ImplDescriptorBuildError`] when `kind` and
    /// `implemented_trait` disagree.
    #[doc(hidden)]
    pub fn new(
        fragment_identity: FragmentIdentity,
        target_type: TypeExpression,
        kind: ImplKind,
        implemented_trait: Option<&'static TraitDefinitionDescriptor>,
        generic_definition: &'static GenericDefinitionDescriptor,
    ) -> Result<Self, ImplDescriptorBuildError> {
        validate_kind(kind, implemented_trait.is_some())?;
        Ok(Self {
            fragment_identity,
            target_type,
            kind,
            implemented_trait,
            generic_definition,
        })
    }

    /// Returns the source/content identity of this impl fragment.
    pub const fn fragment_identity(&self) -> &FragmentIdentity {
        &self.fragment_identity
    }

    /// Returns the possibly symbolic target type expression.
    pub const fn target_type(&self) -> &TypeExpression {
        &self.target_type
    }

    /// Returns whether this definition is inherent or implements a trait.
    pub const fn kind(&self) -> ImplKind {
        self.kind
    }

    /// Returns the implemented trait definition.
    ///
    /// `None` identifies an inherent impl definition.
    pub const fn implemented_trait(&self) -> Option<&'static TraitDefinitionDescriptor> {
        self.implemented_trait
    }

    /// Returns generic parameters and predicates in source order.
    pub const fn generic_definition(&self) -> &'static GenericDefinitionDescriptor {
        self.generic_definition
    }
}

/// The effective source of an associated constant value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AssociatedConstImplementationSource {
    /// The implementation uses the trait declaration's default value.
    Defaulted,
    /// The implementation explicitly overrides the constant.
    Overridden,
}

/// A safe reader for one concrete associated constant value.
pub struct AssociatedConstReader {
    read: fn() -> ReflectedOwned,
}

impl AssociatedConstReader {
    /// Creates a reader from generated safe adapter code.
    #[doc(hidden)]
    pub const fn new(read: fn() -> ReflectedOwned) -> Self {
        Self { read }
    }

    /// Reads a fresh owned reflected value.
    pub fn read(&self) -> ReflectedOwned {
        (self.read)()
    }
}

impl fmt::Debug for AssociatedConstReader {
    /// Formats adapter availability without exposing a process address.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AssociatedConstReader(..)")
    }
}

/// One associated type binding contributed by a concrete impl.
#[derive(Clone, Debug)]
pub struct AssociatedTypeBindingDescriptor {
    declaration: &'static AssociatedTypeDescriptor,
    value: TypeExpression,
    concrete_type: Option<TypeDescriptorResolver>,
}

impl AssociatedTypeBindingDescriptor {
    /// Creates an associated type binding.
    ///
    /// `concrete_type` is present only when `value` resolves to an exact root.
    #[doc(hidden)]
    pub const fn new(
        declaration: &'static AssociatedTypeDescriptor,
        value: TypeExpression,
        concrete_type: Option<TypeDescriptorResolver>,
    ) -> Self {
        Self {
            declaration,
            value,
            concrete_type,
        }
    }

    /// Returns the trait declaration being bound.
    pub const fn declaration(&self) -> &'static AssociatedTypeDescriptor {
        self.declaration
    }

    /// Returns the concrete or still-symbolic binding expression.
    pub const fn value(&self) -> &TypeExpression {
        &self.value
    }

    /// Returns the exact reflected binding when it is known.
    ///
    /// `None` means the expression remains symbolic or unresolved.
    pub fn concrete_type(&self) -> Option<&'static TypeDescriptor> {
        self.concrete_type.map(|resolver| resolver())
    }
}

/// One associated constant binding contributed by a concrete impl.
#[derive(Clone, Debug)]
pub struct AssociatedConstBindingDescriptor {
    declaration: &'static AssociatedConstDescriptor,
    implementation_source: AssociatedConstImplementationSource,
    reader: Option<&'static AssociatedConstReader>,
}

impl AssociatedConstBindingDescriptor {
    /// Creates associated constant binding facts.
    #[doc(hidden)]
    pub const fn new(
        declaration: &'static AssociatedConstDescriptor,
        implementation_source: AssociatedConstImplementationSource,
        reader: Option<&'static AssociatedConstReader>,
    ) -> Self {
        Self {
            declaration,
            implementation_source,
            reader,
        }
    }

    /// Returns the trait declaration being implemented.
    pub const fn declaration(&self) -> &'static AssociatedConstDescriptor {
        self.declaration
    }

    /// Returns whether the value is defaulted or explicitly overridden.
    pub const fn implementation_source(&self) -> AssociatedConstImplementationSource {
        self.implementation_source
    }

    /// Returns whether a safe owned-value reader is available.
    pub const fn is_readable(&self) -> bool {
        self.reader.is_some()
    }

    /// Reads the associated constant through its safe adapter.
    ///
    /// `None` means the declared type cannot cross the owned dynamic boundary.
    pub fn read(&self) -> Option<ReflectedOwned> {
        self.reader.map(AssociatedConstReader::read)
    }
}

/// An invalid impl definition or concrete application.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImplDescriptorBuildError {
    /// An inherent impl attempted to name an implemented trait.
    InherentImplHasTrait,
    /// A trait impl omitted its trait link.
    TraitImplMissingTrait,
    /// Concrete arguments do not match the impl definition.
    GenericArgumentsDoNotMatchDefinition,
    /// The applied trait does not originate from the definition's trait.
    ImplementedTraitDefinitionMismatch,
    /// A method or associated binding belongs to another descriptor graph.
    ForeignMember,
}

impl fmt::Display for ImplDescriptorBuildError {
    /// Formats a stable diagnostic message.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InherentImplHasTrait => {
                formatter.write_str("an inherent impl cannot name a trait")
            }
            Self::TraitImplMissingTrait => formatter.write_str("a trait impl must name a trait"),
            Self::GenericArgumentsDoNotMatchDefinition => {
                formatter.write_str("concrete impl arguments do not match the definition")
            }
            Self::ImplementedTraitDefinitionMismatch => {
                formatter.write_str("applied trait does not match the impl definition")
            }
            Self::ForeignMember => formatter.write_str("impl descriptor contains a foreign member"),
        }
    }
}

impl std::error::Error for ImplDescriptorBuildError {}

/// A qualifier used to resolve methods across implementation namespaces.
#[derive(Clone, Copy, Debug)]
pub enum MethodQualifier<'a> {
    /// Search inherent and every trait namespace.
    Any,
    /// Search only inherent implementations.
    Inherent,
    /// Search one concrete applied trait namespace.
    Trait(&'a TraitDescriptor),
}

/// The result of a method lookup across implementation namespaces.
#[derive(Clone, Copy, Debug)]
pub enum MethodLookup<'a> {
    /// No matching concrete method instance exists.
    Missing,
    /// Exactly one concrete method instance matches.
    Unique(&'a MethodInstanceDescriptor),
    /// Multiple namespaces or fragments match the query.
    Ambiguous,
}

/// One explicitly registered concrete instance of an impl definition.
pub struct ImplDescriptor {
    definition: &'static ImplDefinitionDescriptor,
    target_type: TypeDescriptorResolver,
    implemented_trait: Option<&'static TraitDescriptor>,
    methods: &'static [MethodDescriptor],
    method_instances: Box<[MethodInstanceDescriptor]>,
    associated_types: Box<[AssociatedTypeBindingDescriptor]>,
    associated_consts: Box<[AssociatedConstBindingDescriptor]>,
    arguments: Box<[GenericArgument]>,
}

impl ImplDescriptor {
    /// Starts a concrete impl builder for `definition` and `target_type`.
    pub fn builder(
        definition: &'static ImplDefinitionDescriptor,
        target_type: TypeDescriptorResolver,
    ) -> ImplDescriptorBuilder {
        ImplDescriptorBuilder::new(definition, target_type)
    }

    /// Returns the generic or blanket impl definition.
    pub const fn definition(&self) -> &'static ImplDefinitionDescriptor {
        self.definition
    }

    /// Returns the reflected root targeted by this concrete impl.
    pub fn target_type(&self) -> &'static TypeDescriptor {
        (self.target_type)()
    }

    /// Returns whether this is an inherent or trait implementation.
    pub const fn kind(&self) -> ImplKind {
        self.definition.kind()
    }

    /// Returns the concrete applied trait, or `None` for an inherent impl.
    pub const fn implemented_trait(&self) -> Option<&'static TraitDescriptor> {
        self.implemented_trait
    }

    /// Returns methods explicitly declared by this impl definition.
    pub const fn methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Returns methods explicitly declared by this impl definition.
    pub const fn implementation_methods(&self) -> &[MethodDescriptor] {
        self.methods
    }

    /// Finds a trait declaration by query name.
    pub fn method(&self, name: &str) -> Option<&MethodDescriptor> {
        self.methods
            .iter()
            .find(|method| method.query_name() == name)
    }

    /// Returns concrete effective instances, including defaulted methods.
    pub const fn method_instances(&self) -> &[MethodInstanceDescriptor] {
        &self.method_instances
    }

    /// Returns associated type bindings in declaration order.
    pub const fn associated_types(&self) -> &[AssociatedTypeBindingDescriptor] {
        &self.associated_types
    }

    /// Returns associated constant bindings in declaration order.
    pub const fn associated_consts(&self) -> &[AssociatedConstBindingDescriptor] {
        &self.associated_consts
    }

    /// Returns concrete impl arguments in definition parameter order.
    pub const fn arguments(&self) -> &[GenericArgument] {
        &self.arguments
    }

    /// Looks up one effective method across impl namespaces.
    pub fn lookup_method<'a>(
        implementations: &'a [&'a ImplDescriptor],
        qualifier: MethodQualifier<'_>,
        name: &str,
    ) -> MethodLookup<'a> {
        let mut found = None;
        for implementation in implementations {
            if !matches_qualifier(implementation, qualifier) {
                continue;
            }
            for instance in implementation.method_instances() {
                if instance.declaration().query_name() != name {
                    continue;
                }
                if found.is_some() {
                    return MethodLookup::Ambiguous;
                }
                found = Some(instance);
            }
        }
        found.map_or(MethodLookup::Missing, MethodLookup::Unique)
    }
}

impl fmt::Debug for ImplDescriptor {
    /// Formats local facts without recursively expanding graph roots.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImplDescriptor")
            .field("fragment", self.definition.fragment_identity())
            .field("kind", &self.kind())
            .field("target_type", &"<resolver>")
            .field("method_instance_count", &self.method_instances.len())
            .field("arguments", &self.arguments)
            .finish()
    }
}

/// Builds a concrete impl while preserving source order.
#[derive(Debug)]
pub struct ImplDescriptorBuilder {
    definition: &'static ImplDefinitionDescriptor,
    target_type: TypeDescriptorResolver,
    implemented_trait: Option<&'static TraitDescriptor>,
    methods: &'static [MethodDescriptor],
    method_instances: Vec<MethodInstanceDescriptor>,
    associated_types: Vec<AssociatedTypeBindingDescriptor>,
    associated_consts: Vec<AssociatedConstBindingDescriptor>,
    arguments: Vec<GenericArgument>,
}

impl ImplDescriptorBuilder {
    /// Creates an empty concrete instance builder.
    fn new(
        definition: &'static ImplDefinitionDescriptor,
        target_type: TypeDescriptorResolver,
    ) -> Self {
        Self {
            definition,
            target_type,
            implemented_trait: None,
            methods: &[],
            method_instances: Vec::new(),
            associated_types: Vec::new(),
            associated_consts: Vec::new(),
            arguments: Vec::new(),
        }
    }

    /// Sets the applied trait implemented by this instance.
    pub fn implemented_trait(mut self, implemented_trait: &'static TraitDescriptor) -> Self {
        self.implemented_trait = Some(implemented_trait);
        self
    }

    /// Sets methods explicitly declared by the impl definition.
    pub fn methods(mut self, methods: &'static [MethodDescriptor]) -> Self {
        self.methods = methods;
        self
    }

    /// Sets concrete effective method instances.
    pub fn method_instances(mut self, instances: Vec<MethodInstanceDescriptor>) -> Self {
        self.method_instances = instances;
        self
    }

    /// Sets associated type bindings in declaration order.
    pub fn associated_types(mut self, bindings: Vec<AssociatedTypeBindingDescriptor>) -> Self {
        self.associated_types = bindings;
        self
    }

    /// Sets associated constant bindings in declaration order.
    pub fn associated_consts(mut self, bindings: Vec<AssociatedConstBindingDescriptor>) -> Self {
        self.associated_consts = bindings;
        self
    }

    /// Sets concrete impl arguments in definition parameter order.
    pub fn arguments(mut self, arguments: Vec<GenericArgument>) -> Self {
        self.arguments = arguments;
        self
    }

    /// Validates and builds the concrete impl descriptor.
    ///
    /// Returns [`ImplDescriptorBuildError`] for inconsistent trait, generic,
    /// method, or associated-item relationships.
    pub fn build(self) -> Result<ImplDescriptor, ImplDescriptorBuildError> {
        validate_kind(self.definition.kind(), self.implemented_trait.is_some())?;
        let expected_arguments = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .count();
        if expected_arguments != self.arguments.len()
            || self
                .arguments
                .iter()
                .any(|argument| !generic_argument_is_concrete(argument))
        {
            return Err(ImplDescriptorBuildError::GenericArgumentsDoNotMatchDefinition);
        }
        let kinds_match = self
            .definition
            .generic_definition()
            .parameters
            .iter()
            .filter(|parameter| !matches!(parameter, GenericParameterDescriptor::Lifetime { .. }))
            .zip(&self.arguments)
            .all(|(parameter, argument)| {
                matches!(
                    (parameter, argument),
                    (
                        GenericParameterDescriptor::Type { .. },
                        GenericArgument::Type(_)
                    ) | (
                        GenericParameterDescriptor::Const { .. },
                        GenericArgument::Const(_)
                    )
                )
            });
        if !kinds_match {
            return Err(ImplDescriptorBuildError::GenericArgumentsDoNotMatchDefinition);
        }
        if let (Some(expected), Some(actual)) =
            (self.definition.implemented_trait(), self.implemented_trait)
            && actual.definition().trait_id() != expected.trait_id()
        {
            return Err(ImplDescriptorBuildError::ImplementedTraitDefinitionMismatch);
        }
        if self.methods.iter().any(|method| {
            !method
                .declaring_impl()
                .is_some_and(|owner| std::ptr::eq(owner, self.definition))
        }) {
            return Err(ImplDescriptorBuildError::ForeignMember);
        }
        if let Some(applied_trait) = self.implemented_trait {
            let foreign_method = self.method_instances.iter().any(|instance| {
                !applied_trait
                    .methods()
                    .iter()
                    .any(|method| std::ptr::eq(method, instance.declaration()))
                    || instance.implementation_method().is_some_and(|method| {
                        !self
                            .methods
                            .iter()
                            .any(|candidate| std::ptr::eq(candidate, method))
                    })
            });
            let foreign_type = self.associated_types.iter().any(|binding| {
                !applied_trait
                    .associated_types()
                    .iter()
                    .any(|item| std::ptr::eq(item, binding.declaration()))
            });
            let foreign_const = self.associated_consts.iter().any(|binding| {
                !applied_trait
                    .associated_consts()
                    .iter()
                    .any(|item| std::ptr::eq(item, binding.declaration()))
            });
            if foreign_method || foreign_type || foreign_const {
                return Err(ImplDescriptorBuildError::ForeignMember);
            }
        } else if self.method_instances.iter().any(|instance| {
            instance.implementation_source()
                != crate::descriptor::MethodImplementationSource::Declared
                || !self
                    .methods
                    .iter()
                    .any(|method| std::ptr::eq(method, instance.declaration()))
                || instance.implementation_method().is_some()
        }) {
            return Err(ImplDescriptorBuildError::ForeignMember);
        }
        Ok(ImplDescriptor {
            definition: self.definition,
            target_type: self.target_type,
            implemented_trait: self.implemented_trait,
            methods: self.methods,
            method_instances: self.method_instances.into_boxed_slice(),
            associated_types: self.associated_types.into_boxed_slice(),
            associated_consts: self.associated_consts.into_boxed_slice(),
            arguments: self.arguments.into_boxed_slice(),
        })
    }
}

/// Validates the invariant shared by impl definitions and instances.
fn validate_kind(kind: ImplKind, has_trait: bool) -> Result<(), ImplDescriptorBuildError> {
    match (kind, has_trait) {
        (ImplKind::Inherent, true) => Err(ImplDescriptorBuildError::InherentImplHasTrait),
        (ImplKind::Trait, false) => Err(ImplDescriptorBuildError::TraitImplMissingTrait),
        _ => Ok(()),
    }
}

/// Returns whether `implementation` belongs to the requested namespace.
fn matches_qualifier(implementation: &ImplDescriptor, qualifier: MethodQualifier<'_>) -> bool {
    match qualifier {
        MethodQualifier::Any => true,
        MethodQualifier::Inherent => implementation.kind() == ImplKind::Inherent,
        MethodQualifier::Trait(expected) => implementation
            .implemented_trait()
            .is_some_and(|actual| actual.same_application(expected)),
    }
}
