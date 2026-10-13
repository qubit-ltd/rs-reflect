// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use crate::access::FieldAccessError;
use crate::access::FieldAccessPolicy;
use crate::access::FieldGetAdapter;
use crate::access::FieldGetMutAdapter;
use crate::descriptor::FieldDescriptor;
use crate::descriptor::TypeDescriptor;
use crate::descriptor::TypeDescriptorResolver;
use crate::descriptor::TypeRef;
use crate::identity::Visibility;
use crate::value::DynamicMut;
use crate::value::DynamicRef;
use crate::value::Local;

fn root_descriptor() -> &'static TypeDescriptor {
    TypeDescriptor::of::<u8>()
}

fn read_u8<'a>(target: DynamicRef<'a, Local>) -> Result<DynamicRef<'a, Local>, FieldAccessError> {
    let value = target
        .downcast::<u8>()
        .unwrap_or_else(|_| panic!("the descriptor validates the target type before the adapter"));
    Ok(DynamicRef::<Local>::new(value))
}

fn read_mut_u8<'a>(target: DynamicMut<'a, Local>) -> Result<DynamicMut<'a, Local>, FieldAccessError> {
    let value = target
        .downcast::<u8>()
        .unwrap_or_else(|_| panic!("the descriptor validates the target type before the adapter"));
    Ok(DynamicMut::<Local>::new(value))
}

#[test]
#[allow(unsafe_code)]
fn test_private_field_requires_explicit_unsafe_access() {
    let field_type = Box::leak(Box::new(TypeRef::of::<u8>()));
    let declaring_type: TypeDescriptorResolver = root_descriptor;
    let field = FieldDescriptor::new(
        declaring_type,
        0,
        Some("value"),
        Some("value"),
        field_type,
        Visibility::Private,
    )
    .with_access(
        FieldAccessPolicy::ReadWrite,
        Some(read_u8 as FieldGetAdapter),
        Some(read_mut_u8 as FieldGetMutAdapter),
        None,
    );
    let target = 42_u8;

    assert!(matches!(
        field.get::<u8, u8>(&target),
        Err(FieldAccessError::NotPublic { .. }),
    ));

    // SAFETY: the test owns the target and explicitly verifies the private access
    // boundary.
    let value = unsafe { field.get_unsafe::<u8, u8>(&target) }.expect("unsafe access uses the adapter");
    assert_eq!(*value, 42);

    assert!(matches!(
        field.get_mut::<u8, u8>(&mut 42),
        Err(FieldAccessError::NotPublic { .. }),
    ));
    let mut target = 42_u8;
    // SAFETY: the test owns the target and explicitly verifies the private access
    // boundary.
    let value = unsafe { field.get_mut_unsafe::<u8, u8>(&mut target) }.expect("unsafe mutable access uses the adapter");
    *value = 43;
    assert_eq!(target, 43);
}
