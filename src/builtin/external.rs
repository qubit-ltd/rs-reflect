// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Opaque descriptors for Qubit and ecosystem value types.

use bigdecimal::BigDecimal;
use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveTime;
use chrono::Utc;
use qubit_datatype::DataType;
use qubit_id::Id;
use uuid::Uuid;

use crate::descriptor::Reflect;
use crate::descriptor::TypeDescriptor;

macro_rules! reflected_opaque {
    ($type:ty, $descriptor:ident, $name:expr) => {
        static $descriptor: TypeDescriptor = crate::__private::descriptor::opaque_root::<$type>($name);

        impl Reflect for $type {
            /// Returns the static opaque descriptor for this external value
            /// type.
            fn type_descriptor() -> &'static TypeDescriptor {
                &$descriptor
            }
        }
    };
}

reflected_opaque!(Id, ID_DESCRIPTOR, concat!(stringify!(qubit_id), "::", stringify!(Id)));
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
reflected_opaque!(
    DataType,
    DATA_TYPE_DESCRIPTOR,
    concat!(stringify!(qubit_datatype), "::", stringify!(DataType))
);
reflected_opaque!(Uuid, UUID_DESCRIPTOR, concat!(stringify!(uuid), "::", stringify!(Uuid)));

#[cfg(test)]
mod tests {
    use super::BigDecimal;
    use super::DataType;
    use super::DateTime;
    use super::Id;
    use super::NaiveDate;
    use super::NaiveTime;
    use super::TypeDescriptor;
    use super::Utc;
    use super::Uuid;
    use crate::descriptor::TypeKind;

    #[test]
    fn external_values_expose_opaque_reflection_roots() {
        for descriptor in [
            TypeDescriptor::of::<Id>(),
            TypeDescriptor::of::<BigDecimal>(),
            TypeDescriptor::of::<DateTime<Utc>>(),
            TypeDescriptor::of::<NaiveDate>(),
            TypeDescriptor::of::<NaiveTime>(),
            TypeDescriptor::of::<DataType>(),
            TypeDescriptor::of::<Uuid>(),
        ] {
            assert_eq!(descriptor.kind(), TypeKind::Opaque);
        }
    }
}
