// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Opaque descriptors for opt-in Qubit value types.

use qubit_datatype::DataType;
use qubit_id::Id;

use crate::builtin::internal::reflected_opaque;

reflected_opaque!(
    Id,
    ID_DESCRIPTOR,
    concat!(stringify!(qubit_id), "::", stringify!(Id))
);
reflected_opaque!(
    DataType,
    DATA_TYPE_DESCRIPTOR,
    concat!(stringify!(qubit_datatype), "::", stringify!(DataType))
);
