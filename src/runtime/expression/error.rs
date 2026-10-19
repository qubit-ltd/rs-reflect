use std::fmt;

/// The semantic kind of a concrete generic argument.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GenericArgumentKind {
    Type,
    Const,
}

/// A request to view a generic argument through the wrong kind-specific API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GenericArgumentError {
    expected: GenericArgumentKind,
    actual: GenericArgumentKind,
}

impl GenericArgumentError {
    pub(crate) const fn new(expected: GenericArgumentKind, actual: GenericArgumentKind) -> Self {
        Self { expected, actual }
    }

    pub(crate) const fn expected(self) -> GenericArgumentKind {
        self.expected
    }

    pub(crate) const fn actual(self) -> GenericArgumentKind {
        self.actual
    }
}

impl fmt::Display for GenericArgumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "expected {:?} generic argument, found {:?}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for GenericArgumentError {}

/// A type expression that cannot provide a requested concrete descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypeExpressionError {
    ConcreteDescriptorUnavailable { parameter: &'static str },
}

impl fmt::Display for TypeExpressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConcreteDescriptorUnavailable { parameter } => {
                write!(
                    formatter,
                    "generic type parameter {parameter} has no reflected descriptor"
                )
            }
        }
    }
}

impl std::error::Error for TypeExpressionError {}
