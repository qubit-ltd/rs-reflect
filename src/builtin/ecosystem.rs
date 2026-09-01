// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Opaque descriptors for opt-in ecosystem value types.

use bigdecimal::BigDecimal;
use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveTime;
use chrono::Utc;
use uuid::Uuid;

use crate::builtin::internal::reflected_opaque;

reflected_opaque!(
    BigDecimal,
    BIG_DECIMAL_DESCRIPTOR,
    concat!(stringify!(bigdecimal), "::", stringify!(BigDecimal))
);
reflected_opaque!(
    DateTime<Utc>,
    UTC_DATE_TIME_DESCRIPTOR,
    concat!(
        stringify!(chrono),
        "::DateTime<",
        stringify!(chrono),
        "::",
        stringify!(Utc),
        ">"
    )
);
reflected_opaque!(
    NaiveDate,
    NAIVE_DATE_DESCRIPTOR,
    concat!(stringify!(chrono), "::", stringify!(NaiveDate))
);
reflected_opaque!(
    NaiveTime,
    NAIVE_TIME_DESCRIPTOR,
    concat!(stringify!(chrono), "::", stringify!(NaiveTime))
);
reflected_opaque!(Uuid, UUID_DESCRIPTOR, concat!(stringify!(uuid), "::", stringify!(Uuid)));
