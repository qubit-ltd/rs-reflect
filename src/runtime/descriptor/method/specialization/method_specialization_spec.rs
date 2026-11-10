// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// A concrete specialization declaration with typed arguments and adapters.
#[derive(Clone, Copy)]
pub struct MethodSpecializationSpec {
    /// Generic arguments supplied when selecting a method specialization.
    arguments: &'static [MethodSpecializationArgument],
    /// Invocation availability after generic arguments are checked.
    availability: InvocationAvailability,
    /// Generated adapters for supported value modes and invocation variants.
    adapters: InvocationAdapters,
}

impl MethodSpecializationSpec {
    /// Creates a callable specialization declaration.
    ///
    /// # Parameters
    ///
    /// - `arguments`: Generic arguments or invocation arguments supplied by the
    ///   caller.
    /// - `adapters`: Generated invocation adapters registered for the method.
    pub const fn new(arguments: &'static [MethodSpecializationArgument], adapters: InvocationAdapters) -> Self {
        Self {
            arguments,
            availability: InvocationAvailability::Callable,
            adapters,
        }
    }

    /// Marks this specialization unavailable with typed reasons.
    ///
    /// # Parameters
    ///
    /// - `reasons`: Structured reasons that prevent this operation.
    pub const fn with_unavailable(mut self, reasons: &'static [InvocationUnavailableReason]) -> Self {
        self.availability = InvocationAvailability::Unavailable { reasons };
        self.adapters = NO_ADAPTERS;
        self
    }

    /// Matches requested specialization arguments against the registered
    /// candidates. # Parameters
    ///
    /// - `parameters`: Source-ordered metadata for non-receiver arguments.
    /// # Parameters
    ///
    /// - `parameters`: Source-ordered parameter metadata used to validate
    ///   invocation arguments.
    pub(super) fn resolve(
        self,
        parameters: &'static [GenericParameterMetadata],
    ) -> Result<ResolvedMethodSpecialization, &'static str> {
        if self.arguments.len() != parameters.len() {
            return Err("method specialization argument count does not match its generic parameters");
        }
        let mut identities = Vec::with_capacity(self.arguments.len());
        let mut selectors = Vec::with_capacity(self.arguments.len());
        let mut metadata = Vec::with_capacity(self.arguments.len());
        for (argument, parameter) in self.arguments.iter().zip(parameters) {
            let (identity, selector, metadata_value) = match argument.value {
                MethodSpecializationArgumentValue::Type(type_metadata) => {
                    let GenericParameterMetadataKind::Type { .. } = parameter.kind else {
                        return Err("type specialization argument does not target a type parameter");
                    };
                    let concrete_type = type_metadata();
                    (
                        GenericArgumentIdentity::Type(concrete_type.rust_name().into()),
                        GenericArgumentSelector::Type(concrete_type.type_id()),
                        Some(GenericArgumentMetadata::concrete_type(parameter, type_metadata)),
                    )
                }
                MethodSpecializationArgumentValue::Lifetime(name) => (
                    GenericArgumentIdentity::Lifetime(name.into()),
                    GenericArgumentSelector::Lifetime(name.into()),
                    None,
                ),
                MethodSpecializationArgumentValue::ConstSigned { kind, value } => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("{}:{value}", kind.type_name()).into()),
                        integer_selector(kind, value.to_string()),
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::SignedInteger(value),
                        )),
                    )
                }
                MethodSpecializationArgumentValue::ConstUnsigned { kind, value } => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("{}:{value}", kind.type_name()).into()),
                        integer_selector(kind, value.to_string()),
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::UnsignedInteger(value),
                        )),
                    )
                }
                MethodSpecializationArgumentValue::ConstIntegerLiteral { negative, magnitude } => {
                    let GenericParameterMetadataKind::Const { ty, .. } = parameter.kind else {
                        return Err("integer specialization argument does not target a const parameter");
                    };
                    let kind = integer_kind_for_expression(ty)
                        .ok_or("unsuffixed integer specialization requires a primitive integer const type")?;
                    let value = kind
                        .normalize(negative, magnitude)
                        .ok_or("integer specialization literal is out of range for its const parameter")?;
                    let expression = match kind {
                        ConstIntegerKind::I8
                        | ConstIntegerKind::I16
                        | ConstIntegerKind::I32
                        | ConstIntegerKind::I64
                        | ConstIntegerKind::I128
                        | ConstIntegerKind::Isize => ConstExpression::SignedInteger(
                            value.parse().map_err(|_| "invalid normalized signed integer")?,
                        ),
                        _ => ConstExpression::UnsignedInteger(
                            value.parse().map_err(|_| "invalid normalized unsigned integer")?,
                        ),
                    };
                    (
                        GenericArgumentIdentity::Const(format!("{}:{value}", kind.type_name()).into()),
                        integer_selector(kind, value.clone()),
                        Some(GenericArgumentMetadata::const_argument(parameter, expression)),
                    )
                }
                MethodSpecializationArgumentValue::ConstBoolean(value) => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("bool:{value}").into()),
                        GenericArgumentSelector::Const {
                            type_id: TypeId::of::<bool>(),
                            value: value.to_string().into(),
                        },
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::Boolean(value),
                        )),
                    )
                }
                MethodSpecializationArgumentValue::ConstCharacter(value) => {
                    let GenericParameterMetadataKind::Const { .. } = parameter.kind else {
                        return Err("const specialization argument does not target a const parameter");
                    };
                    (
                        GenericArgumentIdentity::Const(format!("char:{value:?}").into()),
                        GenericArgumentSelector::Const {
                            type_id: TypeId::of::<char>(),
                            value: u32::from(value).to_string().into(),
                        },
                        Some(GenericArgumentMetadata::const_argument(
                            parameter,
                            ConstExpression::Character(value),
                        )),
                    )
                }
            };
            identities.push(identity);
            selectors.push(selector);
            if let Some(value) = metadata_value {
                metadata.push(value);
            }
        }
        Ok(ResolvedMethodSpecialization {
            arguments: ResolvedMethodArguments::Owned {
                display: identities.into_boxed_slice(),
                selectors: selectors.into_boxed_slice(),
            },
            argument_metadata: Some(metadata.into()),
            availability: self.availability,
            adapters: self.adapters,
            parameters: None,
        })
    }
}

/// Infers the integer representation from a source type expression.
/// # Parameters
///
/// - `expression`: Source type expression retained for context-bound
///   resolution.
pub(super) fn integer_kind_for_expression(expression: &'static ExpressionData) -> Option<ConstIntegerKind> {
    let ExpressionData::Named {
        path,
        definition,
        arguments,
    } = expression
    else {
        return None;
    };
    if definition.is_some() || !arguments.is_empty() {
        return None;
    }
    match *path {
        "i8" | "core::primitive::i8" | "std::primitive::i8" => Some(ConstIntegerKind::I8),
        "i16" | "core::primitive::i16" | "std::primitive::i16" => Some(ConstIntegerKind::I16),
        "i32" | "core::primitive::i32" | "std::primitive::i32" => Some(ConstIntegerKind::I32),
        "i64" | "core::primitive::i64" | "std::primitive::i64" => Some(ConstIntegerKind::I64),
        "i128" | "core::primitive::i128" | "std::primitive::i128" => Some(ConstIntegerKind::I128),
        "isize" | "core::primitive::isize" | "std::primitive::isize" => Some(ConstIntegerKind::Isize),
        "u8" | "core::primitive::u8" | "std::primitive::u8" => Some(ConstIntegerKind::U8),
        "u16" | "core::primitive::u16" | "std::primitive::u16" => Some(ConstIntegerKind::U16),
        "u32" | "core::primitive::u32" | "std::primitive::u32" => Some(ConstIntegerKind::U32),
        "u64" | "core::primitive::u64" | "std::primitive::u64" => Some(ConstIntegerKind::U64),
        "u128" | "core::primitive::u128" | "std::primitive::u128" => Some(ConstIntegerKind::U128),
        "usize" | "core::primitive::usize" | "std::primitive::usize" => Some(ConstIntegerKind::Usize),
        _ => None,
    }
}

impl ConstIntegerKind {
    /// Returns the source-level type name used for a specialization selector.
    pub(super) fn type_name(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::Isize => "isize",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::Usize => "usize",
        }
    }

    /// Normalizes a source expression before comparing specialization
    /// selectors. # Parameters
    ///
    /// - `negative`: Whether the integer literal has a leading minus sign.
    /// - `magnitude`: Absolute integer literal magnitude before applying its
    ///   sign.
    /// # Parameters
    ///
    /// - `negative`: Whether the integer selector has a leading minus sign.
    /// - `magnitude`: Absolute magnitude of the integer selector before
    ///   applying its sign.
    pub(super) fn normalize(self, negative: bool, magnitude: u128) -> Option<String> {
        let (signed, bits) = match self {
            Self::I8 => (true, 8),
            Self::I16 => (true, 16),
            Self::I32 => (true, 32),
            Self::I64 => (true, 64),
            Self::I128 => (true, 128),
            Self::Isize => (true, usize::BITS),
            Self::U8 => (false, 8),
            Self::U16 => (false, 16),
            Self::U32 => (false, 32),
            Self::U64 => (false, 64),
            Self::U128 => (false, 128),
            Self::Usize => (false, usize::BITS),
        };
        if signed {
            let negative_limit = 1_u128 << (bits - 1);
            let positive_limit = negative_limit - 1;
            if (negative && magnitude > negative_limit) || (!negative && magnitude > positive_limit) {
                return None;
            }
            let value = if negative {
                if magnitude == (1_u128 << 127) {
                    i128::MIN
                } else {
                    -(magnitude as i128)
                }
            } else {
                magnitude as i128
            };
            Some(value.to_string())
        } else if negative || (bits < 128 && magnitude >= (1_u128 << bits)) {
            None
        } else {
            Some(magnitude.to_string())
        }
    }
}

/// Extracts an integer specialization selector from its source expression.
/// # Parameters
///
/// - `kind`: Receiver form, return category, or constant type being recorded.
/// - `value`: Owned value retained for validation and construction.
fn integer_selector(kind: ConstIntegerKind, value: String) -> GenericArgumentSelector {
    let type_id = match kind {
        ConstIntegerKind::I8 => TypeId::of::<i8>(),
        ConstIntegerKind::I16 => TypeId::of::<i16>(),
        ConstIntegerKind::I32 => TypeId::of::<i32>(),
        ConstIntegerKind::I64 => TypeId::of::<i64>(),
        ConstIntegerKind::I128 => TypeId::of::<i128>(),
        ConstIntegerKind::Isize => TypeId::of::<isize>(),
        ConstIntegerKind::U8 => TypeId::of::<u8>(),
        ConstIntegerKind::U16 => TypeId::of::<u16>(),
        ConstIntegerKind::U32 => TypeId::of::<u32>(),
        ConstIntegerKind::U64 => TypeId::of::<u64>(),
        ConstIntegerKind::U128 => TypeId::of::<u128>(),
        ConstIntegerKind::Usize => TypeId::of::<usize>(),
    };
    GenericArgumentSelector::Const {
        type_id,
        value: value.into_boxed_str(),
    }
}
