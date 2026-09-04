# qubit-reflect Requirements Traceability Matrix

This matrix maps all 284 unique `REQ-*` identifiers in the final requirements
specification. Every row names the historical implementation tasks, at least
one current implementation path, and at least one executable verification
path. Paths were rechecked against the repository after the 2026-09-03
breaking boundary redesign.

`T01` through `T26` are historical task identifiers retained as decision
provenance; they do not imply unfinished work. The generated-code protocol is
`__private::codegen_v2`, model metadata uses ABI v4, and the sole registry
builder is `src/registry/registry_builder.rs`.

## Verification

```bash
rg -o 'REQ-[A-Z]+-[0-9]+' doc/2026-09-03-qubit-reflect-requirements.md | sort -u > /tmp/reflect-requirements
rg '^\| REQ-[A-Z]+-[0-9]+ \|' doc/2026-09-03-qubit-reflect-requirements-traceability.md \
  | sed -E 's/^\| (REQ-[A-Z]+-[0-9]+).*/\1/' > /tmp/reflect-traceability-rows
sort -u /tmp/reflect-traceability-rows > /tmp/reflect-traceability
test "$(wc -l < /tmp/reflect-requirements)" -eq 284
test "$(wc -l < /tmp/reflect-traceability-rows)" -eq 284
test "$(wc -l < /tmp/reflect-traceability)" -eq 284
diff -u /tmp/reflect-requirements /tmp/reflect-traceability
```

## Requirement-to-Implementation Mapping

| Requirement | Historical tasks | Implementation | Verification |
| --- | --- | --- | --- |
| REQ-ACC-001 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-002 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-003 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-004 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-005 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-006 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-007 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACC-008 | T08, T14, T15, T22 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs` |
| REQ-ACCPT-001 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `derive/src/lib.rs`, `derive/src/validate/declaration.rs` | `derive/tests/parser_tests.rs`, `tests/ui_tests.rs` |
| REQ-ACCPT-002 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `derive/src/expand/structs.rs`, `derive/src/expand/enums.rs`, `derive/src/expand/construction.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-ACCPT-003 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/access/field_adapter.rs`, `src/descriptor/field_descriptor.rs` | `tests/access/field_tests.rs`, `tests/ui/fail/thread_safe_field_tests.rs` |
| REQ-ACCPT-004 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/receiver.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ACCPT-005 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/private/registration.rs`, `src/registry/registry_builder.rs` | `tests/registry/cross_crate_tests.rs` |
| REQ-ACCPT-006 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/registry/effective_type_view.rs`, `src/descriptor/impl_descriptor.rs` | `tests/descriptor/type_descriptor_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ACCPT-007 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/value/mode.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/ui/fail/thread_safe_borrow_bound_tests.rs`, `tests/ui/fail/thread_safe_field_tests.rs` |
| REQ-ACCPT-008 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/invocation.rs`, `src/construct/validated.rs`, `src/access/field_adapter.rs` | `tests/invoke/runtime_tests.rs`, `tests/construct/runtime_tests.rs`, `tests/access/field_tests.rs` |
| REQ-ACCPT-009 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/private/lazy_type_ref.rs`, `src/registry/interner.rs` | `tests/descriptor/derive_struct_tests.rs` |
| REQ-ACCPT-010 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `test-crates/model-facade-runtime/src/lib.rs`, `src/descriptor/field_descriptor.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-ACCPT-011 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/builtin/option.rs`, `src/builtin/sequence.rs`, `src/descriptor/type_descriptor.rs` | `tests/descriptor/builtin_tests.rs` |
| REQ-ACCPT-012 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ACCPT-013 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/recovery.rs`, `src/construct/recovery.rs` | `tests/invoke/runtime_tests.rs`, `tests/construct/runtime_tests.rs` |
| REQ-ACCPT-014 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `test-crates/model-facade-runtime/src/lib.rs`, `src/descriptor/type_descriptor.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-ACCPT-015 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/builtin/map.rs`, `src/builtin/set.rs` | `tests/descriptor/builtin_tests.rs` |
| REQ-ACCPT-016 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/type_ref.rs`, `derive/src/expand/structs.rs` | `tests/descriptor/derive_struct_tests.rs` |
| REQ-ACCPT-017 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/value/mode.rs`, `src/value/dynamic_owned.rs` | `tests/value/thread_mode_tests.rs`, `tests/ui/fail/thread_safe_field_tests.rs` |
| REQ-ACCPT-018 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/builtin/tuple.rs`, `src/builtin/reference.rs`, `src/builtin/slice.rs`, `src/builtin/raw_pointer.rs`, `src/builtin/function.rs`, `src/builtin/trait_object.rs` | `tests/descriptor/builtin_tests.rs` |
| REQ-ACCPT-019 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/value/dynamic_owned.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/value/dynamic_owned_tests.rs`, `tests/value/dynamic_ref_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-ACCPT-020 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs` |
| REQ-ACCPT-021 | T07, T14, T22, T26 | `src/capability/registration.rs`, `src/capability/descriptor.rs`, `derive/src/expand/structs.rs` | `src/capability/registration.rs`, `tests/descriptor/capability_tests.rs` |
| REQ-ACCPT-022 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/value/mode.rs`, `src/capability/builtin.rs` | `tests/value/thread_mode_tests.rs`, `tests/descriptor/capability_tests.rs` |
| REQ-ACCPT-023 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/type_kind.rs`, `src/descriptor/typed_view.rs` | `tests/descriptor/builtin_tests.rs`, `tests/descriptor/type_descriptor_tests.rs` |
| REQ-ACCPT-024 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/trait_descriptor.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_trait_tests.rs` |
| REQ-ACCPT-025 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/impl_descriptor.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ACCPT-026 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs` | `tests/descriptor/associated_item_impl_tests.rs`, `tests/descriptor/trait_tests.rs` |
| REQ-ACCPT-027 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/expression/generic_definition.rs`, `src/descriptor/generic_descriptor.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/generic_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ACCPT-028 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `derive/src/parse/attributes.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/access/policy_tests.rs` |
| REQ-ACCPT-029 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/registry/registry_builder.rs`, `src/private/registration.rs` | `tests/registry/cross_crate_tests.rs`, `tests/registry/runtime_tests.rs` |
| REQ-ACCPT-030 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/construct/validated.rs`, `src/construct/update.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-ACCPT-031 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/adapter.rs`, `derive/src/expand/impls.rs` | `tests/invocation_macro_contract_tests.rs`, `tests/panic_abort_invocation_tests.rs`, `tests/fixtures/panic_abort_invocation.rs`, `project-ci-check.sh` |
| REQ-ACCPT-032 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/future.rs`, `derive/src/expand/impls.rs` | `tests/invocation_macro_contract_tests.rs` |
| REQ-ACCPT-033 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/receiver.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ACCPT-034 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/invocation.rs`, `src/invoke/recovery.rs` | `tests/invoke/runtime_tests.rs`, `tests/ui/fail/thread_safe_borrow_bound_tests.rs` |
| REQ-ACCPT-035 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ACCPT-036 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/registry/registry_builder.rs`, `derive/src/expand/impls.rs` | `tests/registry/runtime_tests.rs`, `tests/generic_impl_trait_link_tests.rs` |
| REQ-ACCPT-037 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `derive/src/lib.rs`, `derive/src/expand/mod.rs` | `derive/tests/parser_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-ACCPT-038 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/registry/indexes.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-ACCPT-039 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/builtin/slice.rs`, `src/builtin/trait_object.rs`, `src/value/dynamic_ref.rs` | `tests/descriptor/builtin_tests.rs`, `tests/value/str_tests.rs` |
| REQ-ACCPT-040 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/builtin/mod.rs`, `src/descriptor/typed_view.rs` | `tests/descriptor/builtin_tests.rs` |
| REQ-ACCPT-041 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/registry/interner.rs`, `src/private/lazy_type_ref.rs` | `tests/descriptor/generic_tests.rs`, `tests/registry/stress_tests.rs` |
| REQ-ACCPT-042 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/descriptor/trait_descriptor.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_trait_tests.rs`, `tests/descriptor/trait_tests.rs` |
| REQ-ACCPT-043 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/registry/interner.rs`, `src/registry/registry.rs` | `tests/builtin_registry_freeze_tests.rs` |
| REQ-ACCPT-044 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `derive/src/expand/structs.rs`, `src/descriptor/type_ref.rs` | `tests/ui/pass/opaque_generic_tests.rs`, `tests/descriptor/generic_tests.rs` |
| REQ-ACCPT-045 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/invoke/adapter.rs`, `derive/src/expand/impls.rs` | `tests/invocation_macro_contract_tests.rs`, `tests/ui/pass/catch_unwind_thread_safe_tests.rs` |
| REQ-ACCPT-046 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `derive/src/expand/generics.rs`, `derive/src/expand/structs.rs` | `tests/ui/pass/lifetime_generic_tests.rs`, `tests/descriptor/generic_tests.rs` |
| REQ-ACCPT-047 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/private/codegen_v2/mod.rs`, `derive/src/expand/`, `test-crates/model-facade-derive/src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `derive/tests/codegen_protocol_tests.rs`, `test-crates/model-facade-app/src/lib.rs`, `tests/registry/model_facade_tests.rs` |
| REQ-ACCPT-048 | T14, T15, T17, T18, T19, T21, T22, T24, T25, T26 | `src/value/aliases.rs`, `src/value/mode.rs` | `tests/value/thread_mode_tests.rs` |
| REQ-AGG-001 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-002 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-003 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-004 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-005 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-006 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-007 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-008 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-009 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-010 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-011 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-012 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-013 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-014 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-AGG-015 | T12, T17, T20, T21, T26 | `src/registry/registry_builder.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/cross_crate_tests.rs` |
| REQ-CON-001 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-002 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-003 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-004 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-005 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-006 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-007 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-008 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-009 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-010 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-011 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-012 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-013 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-CON-014 | T10, T15, T19, T22 | `src/construct/validated.rs`, `derive/src/expand/construction.rs` | `tests/construct/runtime_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-DESC-001 | T05, T11, T25 | `src/descriptor/type_descriptor.rs`, `src/descriptor/typed_view.rs`, `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs` | `tests/descriptor/type_descriptor_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-DESC-002 | T05, T11, T12 | `src/descriptor/type_ref.rs`, `src/private/lazy_type_ref.rs`, `src/registry/interner.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/builtin_tests.rs`, `tests/registry/stress_tests.rs` |
| REQ-DESC-003 | T02, T05, T11 | `src/identity/member_id.rs`, `src/identity/fragment_id.rs`, `src/descriptor/method_descriptor.rs` | `tests/descriptor/identity_tests.rs`, `tests/descriptor/type_descriptor_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-DESC-004 | T02, T05, T11 | `src/descriptor/type_descriptor.rs`, `src/descriptor/field_descriptor.rs`, `src/descriptor/method_descriptor.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-DESC-005 | T05, T11 | `src/descriptor/method_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/type_descriptor_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-DESC-006 | T02, T05, T11, T12 | `src/identity/member_id.rs`, `src/identity/fragment_id.rs`, `src/registry/indexes.rs` | `tests/descriptor/identity_tests.rs`, `tests/registry/runtime_tests.rs` |
| REQ-DESC-007 | T03, T05, T13 | `derive/src/parse/attributes.rs`, `derive/src/ir/attribute.rs`, `src/expression/type_expression.rs` | `derive/src/parse/tests.rs`, `derive/tests/parser_tests.rs`, `tests/descriptor/expression_tests.rs` |
| REQ-DESC-008 | T02, T11, T12 | `src/identity/fragment_id.rs`, `src/private/registration.rs`, `src/registry/registry_builder.rs` | `tests/registry/runtime_tests.rs`, `tests/registry/stress_tests.rs` |
| REQ-DESC-009 | T02, T05, T11, T13 | `src/identity/visibility.rs`, `derive/src/parse/declaration.rs`, `derive/src/expand/traits.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/identity_tests.rs`, `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/reflect_trait_tests.rs` |
| REQ-DESC-010 | T03, T13 | `src/expression/type_expression.rs`, `src/expression/generic_argument.rs`, `src/expression/lifetime_expression.rs`, `derive/src/parse/type_ir.rs` | `tests/descriptor/expression_tests.rs`, `tests/descriptor/reflect_trait_tests.rs` |
| REQ-DESC-011 | T02, T05, T11 | `src/descriptor/type_descriptor.rs`, `src/descriptor/trait_descriptor.rs`, `src/descriptor/field_descriptor.rs`, `src/descriptor/variant_descriptor.rs`, `src/descriptor/method_descriptor.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/derive_enum_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-ERR-001 | T02, T08, T09, T10, T12, T18, T19, T22 | `derive/src/parse/declaration.rs`, `derive/src/validate/declaration.rs` | `tests/ui_tests.rs`, `tests/ui/fail/derive_union_tests.rs` |
| REQ-ERR-002 | T02, T08, T09, T10, T12, T18, T19, T22 | `derive/src/parse/attributes.rs`, `derive/src/validate/declaration.rs` | `tests/ui_tests.rs`, `tests/ui/fail/thread_safe_type_not_send_sync_tests.rs` |
| REQ-ERR-003 | T02, T08, T09, T10, T12, T18, T19, T22 | `derive/src/lib.rs`, `derive/src/parse/declaration.rs` | `tests/ui_tests.rs`, `derive/tests/parser_tests.rs` |
| REQ-ERR-004 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-005 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-006 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-007 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-008 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-009 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-010 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-011 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-ERR-012 | T02, T08, T09, T10, T12, T18, T19, T22 | `src/error/registry_error.rs`, `src/invoke/error.rs` | `tests/registry/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-FLD-001 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-002 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-003 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-004 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-005 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-006 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-007 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-008 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-009 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-010 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-FLD-011 | T05, T08, T14, T22 | `src/descriptor/field_descriptor.rs`, `src/access/field_adapter.rs` | `tests/access/field_tests.rs` |
| REQ-GEN-001 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-002 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-003 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-004 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-005 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-006 | T03, T06, T14, T20, T22 | `src/builtin/array.rs`, `src/builtin/tuple.rs`, `src/builtin/function.rs`, `src/descriptor/typed_view.rs` | `tests/descriptor/builtin_tests.rs`, `tests/descriptor/generic_tests.rs` |
| REQ-GEN-007 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-008 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-009 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-010 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-011 | T03, T06, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `derive/src/expand/generics.rs` | `tests/descriptor/generic_tests.rs` |
| REQ-GEN-012 | T03, T06, T07, T12, T14, T20, T22 | `src/descriptor/generic_descriptor.rs`, `src/descriptor/type_definition_descriptor.rs`, `src/descriptor/type_descriptor.rs`, `derive/src/expand/generics.rs`, `src/registry/registry.rs` | `tests/descriptor/generic_tests.rs`, `tests/public_type_definition_registry_tests.rs`, `tests/public_capability_registry_tests.rs`, `tests/capability_key_allocation_tests.rs` |
| REQ-INT-001 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-002 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-003 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-004 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-005 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-006 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-007 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-008 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-009 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-010 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-011 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INT-012 | T23, T25, T26 | `src/lib.rs`, `test-crates/model-facade-runtime/src/lib.rs` | `tests/registry/model_facade_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-INV-001 | T09, T17, T18, T22 | `derive/src/expand/impls.rs`, `src/invoke/receiver.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-INV-002 | T09, T18, T22 | `src/invoke/invocation.rs`, `src/invoke/error.rs` | `tests/invoke/runtime_tests.rs`, `tests/invoke/adapter_tests.rs` |
| REQ-INV-003 | T09, T18, T22 | `src/invoke/receiver.rs`, `src/invoke/invocation.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-INV-004 | T09, T17, T18, T22 | `src/invoke/output.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-INV-005 | T09, T17, T18, T22 | `src/invoke/error.rs`, `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs`, `derive/src/validate/declaration.rs` | `tests/invocation_macro_contract_tests.rs`, `tests/invocation_capability_tests.rs`, `tests/panic_abort_invocation_tests.rs`, `tests/fixtures/panic_abort_invocation.rs`, `tests/ui/fail/catch_unwind_receiver_not_ref_unwind_safe_tests.rs`, `tests/ui/fail/catch_unwind_mutable_parameter_not_unwind_safe_tests.rs` |
| REQ-INV-006 | T09, T17, T18, T22 | `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs`, `src/descriptor/method_descriptor.rs`, `src/lib.rs`, `derive/src/lib.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs` |
| REQ-INV-007 | T09, T17, T18, T22 | `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-INV-008 | T09, T17, T18, T22 | `src/invoke/future.rs`, `src/invoke/output.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/invocation_macro_contract_tests.rs`, `tests/invoke/runtime_tests.rs`, `tests/ui/pass/local_non_send_async_tests.rs`, `tests/ui/fail/thread_safe_future_not_send_tests.rs` |
| REQ-INV-009 | T09, T17, T18, T20, T22 | `derive/src/expand/generics.rs`, `derive/src/expand/impls.rs`, `src/descriptor/generic_descriptor.rs`, `src/descriptor/method_descriptor.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/ui/pass/generic_impl_specialization_tests.rs`, `tests/ui/fail/generic_impl_unsatisfied_specialization_tests.rs` |
| REQ-INV-010 | T09, T17, T18, T22 | `src/invoke/receiver.rs`, `src/invoke/pinned.rs`, `src/invoke/invocation.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs` |
| REQ-INV-011 | T05, T11, T21, T22 | `src/descriptor/impl_descriptor.rs`, `src/descriptor/type_descriptor.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/type_descriptor_tests.rs` |
| REQ-INV-012 | T09, T17, T18, T22 | `src/invoke/adapter.rs`, `src/invoke/invocation.rs`, `src/invoke/output.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/invoke/runtime_tests.rs`, `tests/descriptor/reflect_impl_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs` |
| REQ-INV-013 | T09, T18, T22 | `src/invoke/recovery.rs`, `src/invoke/invocation.rs`, `src/invoke/pinned.rs` | `tests/invoke/runtime_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-INV-014 | T09, T17, T18, T22 | `src/invoke/argument.rs`, `src/invoke/invocation.rs`, `src/descriptor/method_descriptor.rs` | `tests/invoke/runtime_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs` |
| REQ-INV-015 | T09, T17, T18, T22 | `src/invoke/output.rs`, `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-INV-016 | T09, T17, T18, T22 | `src/invoke/future.rs`, `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/invocation_macro_contract_tests.rs`, `tests/descriptor/default_trait_invocation_tests.rs`, `tests/ui/pass/thread_safe_adapter_tests.rs`, `tests/ui/fail/thread_safe_receiver_not_sync_tests.rs`, `tests/ui/fail/thread_safe_owned_parameter_not_send_sync_tests.rs`, `tests/ui/fail/thread_safe_owned_output_not_send_sync_tests.rs`, `tests/ui/fail/thread_safe_future_not_send_tests.rs` |
| REQ-MAC-001 | T13, T14, T15, T16, T17, T22 | `derive/src/lib.rs`, `derive/src/expand/structs.rs`, `derive/src/expand/enums.rs` | `tests/ui/pass/derive_struct_tests.rs`, `tests/ui/pass/generic_enum_tests.rs` |
| REQ-MAC-002 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/structs.rs`, `src/descriptor/field_descriptor.rs` | `tests/descriptor/derive_struct_tests.rs` |
| REQ-MAC-003 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/structs.rs`, `src/identity/visibility.rs` | `tests/descriptor/derive_struct_tests.rs` |
| REQ-MAC-004 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/structs.rs`, `src/descriptor/type_descriptor.rs` | `tests/descriptor/derive_struct_tests.rs` |
| REQ-MAC-005 | T13, T14, T15, T16, T17, T22 | `derive/src/validate/declaration.rs` | `tests/ui/fail/derive_union_tests.rs` |
| REQ-MAC-006 | T13, T14, T15, T16, T17, T22 | `derive/src/lib.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_trait_tests.rs` |
| REQ-MAC-007 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/traits.rs`, `src/descriptor/trait_descriptor.rs` | `tests/descriptor/reflect_trait_tests.rs` |
| REQ-MAC-008 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/traits.rs`, `src/descriptor/trait_descriptor.rs` | `tests/descriptor/associated_item_impl_tests.rs`, `tests/descriptor/reflect_trait_tests.rs` |
| REQ-MAC-009 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/traits.rs` | `tests/ui/pass/non_dyn_compatible_trait_declaration_tests.rs`, `tests/descriptor/reflect_trait_tests.rs` |
| REQ-MAC-010 | T13, T14, T15, T16, T17, T22 | `derive/src/lib.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-011 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/impls.rs`, `src/descriptor/impl_descriptor.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-012 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/impls.rs`, `src/private/registration.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-013 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/impls.rs`, `src/registry/registry_builder.rs` | `tests/registry/cross_crate_tests.rs` |
| REQ-MAC-014 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/impls.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-015 | T13, T14, T15, T16, T17, T22 | `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-016 | T13, T14, T15, T16, T17, T22 | `derive/src/lib.rs` | `derive/tests/parser_tests.rs` |
| REQ-MAC-017 | T13, T14, T15, T16, T17, T22 | `derive/src/lib.rs`, `derive/src/expand/mod.rs`, `src/private/codegen_v2/mod.rs` | `derive/tests/codegen_protocol_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-MAC-018 | T13, T14, T15, T16, T17, T22 | `derive/src/parse/attributes.rs`, `derive/src/expand/impls.rs` | `tests/ui/fail/external_trait_id_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-019 | T13, T14, T15, T16, T17, T22 | `derive/src/parse/attributes.rs`, `derive/src/ir/attribute.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-020 | T13, T14, T15, T16, T17, T20, T22 | `derive/src/expand/impls.rs`, `src/descriptor/impl_descriptor.rs` | `tests/ui/pass/generic_impl_specialization_tests.rs`, `tests/descriptor/reflect_impl_tests.rs` |
| REQ-MAC-021 | T13, T14, T15, T16, T17, T22 | `derive/src/parse/attributes.rs`, `derive/src/expand/traits.rs` | `tests/descriptor/reflect_trait_tests.rs`, `tests/ui/fail/invalid_dyn_compatible_proof_tests.rs` |
| REQ-MTH-001 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-002 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-003 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-004 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-005 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-006 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-007 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-008 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-009 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-010 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-011 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-012 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-MTH-013 | T09, T11, T17, T18, T20 | `src/descriptor/method_descriptor.rs`, `src/invoke/invocation.rs` | `tests/descriptor/reflect_impl_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-OUT-001 | T01, T05, T06, T07, T12, T23, T26 | `derive/src/lib.rs`, `src/registry/fragment.rs` | `tests/registry/runtime_tests.rs` |
| REQ-OUT-002 | T01, T05, T06, T07, T12, T23, T26 | `src/private/registration.rs`, `src/registry/registry_builder.rs` | `tests/registry/cross_crate_tests.rs` |
| REQ-OUT-003 | T01, T05, T06, T07, T12, T23, T26 | `src/value/storage.rs`, `src/access/field_adapter.rs` | `tests/ui/fail/thread_safe_borrow_bound_tests.rs`, `tests/access/field_tests.rs` |
| REQ-OUT-004 | T01, T05, T06, T07, T12, T23, T26 | `src/descriptor/type_descriptor.rs`, `src/identity/fragment_id.rs` | `tests/descriptor/identity_tests.rs`, `tests/descriptor/type_descriptor_tests.rs` |
| REQ-OUT-005 | T01, T05, T06, T07, T12, T23, T26 | `src/private/registration.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-OUT-006 | T01, T05, T06, T07, T12, T23, T26 | `src/descriptor/method_descriptor.rs`, `derive/src/expand/impls.rs` | `tests/descriptor/reflect_impl_tests.rs` |
| REQ-OUT-007 | T01, T05, T06, T07, T12, T23, T26 | `src/lib.rs`, `src/descriptor/type_descriptor.rs` | `tests/registry/model_facade_tests.rs` |
| REQ-OUT-008 | T01, T05, T06, T07, T12, T23, T26 | `src/construct/validated.rs`, `src/construct/struct_constructor.rs` | `tests/construct/runtime_tests.rs` |
| REQ-OUT-009 | T01, T26 | `Cargo.toml`, `src/lib.rs` | `.rs-ci-cargo-matrix.json`, `.rs-ci/cargo-feature-check.sh` |
| REQ-OUT-010 | T01, T05, T06, T07, T12, T23, T26 | `src/construct/struct_construction_descriptor.rs`, `src/descriptor/type_kind.rs` | `tests/construct/adapter_tests.rs` |
| REQ-SYS-001 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-002 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-003 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-004 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-005 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-006 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-007 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-008 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs`, `scripts/check-markdown-examples.sh` | `tests/registry/runtime_tests.rs`, `project-ci-check.sh` |
| REQ-SYS-009 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-010 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-011 | T01, T26 | `Cargo.toml`, `src/lib.rs` | `.rs-ci-cargo-matrix.json`, `.rs-ci/cargo-feature-check.sh` |
| REQ-SYS-012 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-SYS-013 | T01, T04, T05, T06, T12, T23, T26 | `src/lib.rs`, `src/registry/registry.rs` | `tests/registry/runtime_tests.rs` |
| REQ-TRT-001 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-002 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-003 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-004 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-005 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-006 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-007 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-008 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-009 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-010 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-011 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-012 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-013 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-014 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TRT-015 | T11, T16, T17, T20, T21 | `src/descriptor/trait_descriptor.rs`, `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs` | `tests/descriptor/trait_tests.rs`, `tests/descriptor/reflect_trait_tests.rs`, `test-crates/registry-app/src/lib.rs` |
| REQ-TYPE-001 | T05, T06, T14, T15 | `src/descriptor/type_descriptor.rs`, `src/registry/interner.rs` | `tests/descriptor/type_descriptor_tests.rs`, `tests/descriptor/builtin_tests.rs` |
| REQ-TYPE-002 | T05, T14, T15 | `src/descriptor/type_descriptor.rs`, `derive/src/expand/structs.rs` | `tests/descriptor/type_descriptor_tests.rs` |
| REQ-TYPE-003 | T02, T05 | `src/descriptor/type_descriptor.rs`, `src/descriptor/type_ref.rs` | `tests/descriptor/type_descriptor_tests.rs` |
| REQ-TYPE-004 | T02, T05 | `src/descriptor/type_descriptor.rs` | `tests/descriptor/type_descriptor_tests.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-TYPE-005 | T05, T14, T15 | `src/descriptor/type_descriptor.rs`, `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/descriptor/derive_enum_tests.rs` |
| REQ-TYPE-006 | T05 | `src/descriptor/type_descriptor.rs`, `src/descriptor/typed_view.rs` | `tests/descriptor/type_descriptor_tests.rs` |
| REQ-TYPE-007 | T06 | `src/builtin/mod.rs`, `src/builtin/scalar.rs`, `src/builtin/sequence.rs`, `src/builtin/map.rs`, `src/builtin/pointer.rs` | `tests/descriptor/builtin_tests.rs`, `tests/builtin_registry_freeze_tests.rs` |
| REQ-TYPE-008 | T05, T11, T12, T13 | `src/descriptor/impl_descriptor.rs`, `src/registry/effective_type_view.rs`, `derive/src/validate/declaration.rs` | `tests/descriptor/type_descriptor_tests.rs`, `tests/ui/fail/duplicate_rename_tests.rs` |
| REQ-TYPE-009 | T05, T23 | `src/descriptor/type_descriptor.rs`, `src/lib.rs` | `test-crates/model-facade-runtime/src/lib.rs`, `test-crates/model-facade-app/src/lib.rs` |
| REQ-TYPE-010 | T06, T14, T15 | `src/builtin/mod.rs`, `derive/src/expand/generics.rs`, `derive/src/expand/structs.rs` | `tests/descriptor/builtin_tests.rs`, `tests/descriptor/generic_tests.rs`, `tests/ui/pass/opaque_generic_tests.rs` |
| REQ-TYPE-011 | T05, T14 | `src/descriptor/type_kind.rs`, `src/descriptor/typed_view.rs`, `derive/src/expand/structs.rs` | `tests/descriptor/derive_struct_tests.rs` |
| REQ-TYPE-012 | T05, T12 | `src/descriptor/type_descriptor.rs`, `src/registry/registry.rs`, `src/registry/indexes.rs` | `tests/registry/runtime_tests.rs`, `tests/builtin_registry_freeze_tests.rs` |
| REQ-TYPE-013 | T05, T06 | `src/descriptor/type_kind.rs`, `src/descriptor/typed_view.rs`, `src/builtin/tuple.rs`, `src/builtin/function.rs` | `tests/descriptor/builtin_tests.rs`, `tests/descriptor/type_descriptor_tests.rs` |
| REQ-TYPE-014 | T07 | `src/capability/key.rs`, `src/capability/descriptor.rs`, `src/capability/set.rs` | `tests/descriptor/capability_tests.rs` |
| REQ-TYPE-015 | T05, T08, T09, T10, T14, T15 | `src/descriptor/type_ref.rs`, `src/descriptor/field_descriptor.rs`, `src/construct/validated.rs`, `src/invoke/argument.rs` | `tests/access/field_tests.rs`, `tests/construct/runtime_tests.rs`, `tests/invoke/runtime_tests.rs` |
| REQ-TYPE-016 | T02, T07 | `src/identity/capability_id.rs`, `src/capability/key.rs` | `tests/descriptor/identity_tests.rs` |
| REQ-TYPE-017 | T06, T07, T14, T15 | `src/capability/builtin.rs`, `src/capability/registration.rs`, `derive/src/expand/structs.rs` | `tests/descriptor/capability_tests.rs`, `src/capability/registration.rs` |
| REQ-TYPE-018 | T04, T07 | `src/capability/builtin.rs`, `src/capability/descriptor.rs`, `src/value/dynamic_owned.rs` | `tests/descriptor/capability_tests.rs` |
| REQ-TYPE-019 | T07, T12 | `src/capability/set.rs`, `src/capability/registration.rs`, `src/registry/registry_builder.rs` | `tests/descriptor/capability_tests.rs`, `tests/registry/runtime_tests.rs` |
| REQ-TYPE-020 | T05, T06 | `src/descriptor/type_descriptor.rs`, `src/descriptor/typed_view.rs` | `tests/descriptor/builtin_tests.rs`, `tests/descriptor/type_descriptor_tests.rs` |
| REQ-TYPE-021 | T06, T12, T21 | `src/registry/interner.rs`, `src/registry/registry.rs`, `src/descriptor/type_descriptor.rs` | `tests/builtin_registry_freeze_tests.rs`, `tests/type_descriptor_registry_failure_tests.rs` |
| REQ-TYPE-022 | T04, T05, T06, T16 | `src/descriptor/type_descriptor.rs`, `src/builtin/slice.rs`, `src/builtin/trait_object.rs`, `src/value/dynamic_owned.rs` | `tests/descriptor/builtin_tests.rs`, `tests/ui/fail/slice_dynamic_value_tests.rs`, `tests/ui/fail/dyn_trait_dynamic_value_tests.rs` |
| REQ-TYPE-023 | T05, T06 | `src/descriptor/type_kind.rs`, `src/builtin/scalar.rs` | `tests/descriptor/builtin_tests.rs`, `tests/builtin_registry_freeze_tests.rs` |
| REQ-TYPE-024 | T05, T06 | `src/descriptor/typed_view.rs`, `src/builtin/map.rs`, `src/builtin/pointer.rs`, `src/builtin/function.rs` | `tests/descriptor/builtin_tests.rs` |
| REQ-TYPE-025 | T12, T13, T21 | `src/registry/indexes.rs`, `src/registry/registry.rs`, `derive/src/validate/declaration.rs` | `tests/registry/runtime_tests.rs`, `tests/ui/fail/duplicate_rename_tests.rs` |
| REQ-TYPE-026 | T04, T07 | `src/capability/set.rs`, `src/capability/registration.rs`, `src/value/dynamic_owned.rs` | `tests/descriptor/capability_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-TYPE-027 | T06, T12 | `src/registry/interner.rs`, `src/registry/registry.rs`, `src/registry/registry_builder.rs` | `tests/builtin_registry_freeze_tests.rs`, `tests/descriptor/builtin_tests.rs` |
| REQ-TYPE-028 | T05, T14, T15, T20 | `src/descriptor/type_ref.rs`, `src/private/lazy_type_ref.rs`, `derive/src/expand/structs.rs`, `src/construct/validated.rs` | `tests/descriptor/type_descriptor_tests.rs`, `tests/descriptor/derive_struct_tests.rs`, `tests/access/field_tests.rs` |
| REQ-TYPE-029 | T05, T07, T14, T15, T19 | `derive/src/expand/structs.rs`, `derive/src/expand/enums.rs`, `src/private/descriptor.rs` | `tests/descriptor/derive_struct_tests.rs`, `tests/ui/pass/opaque_generic_tests.rs`, `tests/descriptor/capability_tests.rs` |
| REQ-VAL-001 | T04, T22 | `src/value/dynamic_owned.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/value/dynamic_owned_tests.rs`, `tests/value/dynamic_ref_tests.rs` |
| REQ-VAL-002 | T04, T22 | `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `src/value/mode.rs`, `tests/value/dynamic_ref_tests.rs` |
| REQ-VAL-003 | T04, T22 | `src/value/dynamic_owned.rs` | `tests/value/dynamic_owned_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-VAL-004 | T04, T22 | `src/value/storage.rs`, `src/value/dynamic_owned.rs` | `tests/value/dynamic_owned_tests.rs` |
| REQ-VAL-005 | T04, T22 | `src/value/mode.rs`, `src/value/storage.rs` | `src/value/mode.rs`, `tests/value/thread_mode_tests.rs`, `tests/ui/fail/thread_safe_borrow_bound_tests.rs` |
| REQ-VAL-006 | T04, T22 | `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs`, `src/value/dynamic_owned.rs` | `src/lib.rs`, `tests/value/dynamic_owned_tests.rs`, `tests/value/dynamic_ref_tests.rs` |
| REQ-VAL-007 | T04, T22 | `src/value/storage.rs`, `src/value/dynamic_owned.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/value/dynamic_owned_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-VAL-008 | T04, T22 | `src/value/dynamic_owned.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/value/dynamic_owned_tests.rs`, `tests/value/dynamic_ref_tests.rs` |
| REQ-VAL-009 | T04, T22 | `src/value/mode.rs`, `src/value/storage.rs` | `src/value/mode.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-VAL-010 | T04, T22 | `src/value/dynamic_owned.rs` | `src/value/mode.rs`, `tests/value/dynamic_owned_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-VAL-011 | T04, T22 | `src/value/mode.rs`, `src/value/aliases.rs` | `tests/value/thread_mode_tests.rs` |
| REQ-VAL-012 | T04, T22 | `src/value/dynamic_owned.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/value/dynamic_owned_tests.rs`, `tests/value/dynamic_ref_tests.rs`, `tests/value/str_tests.rs` |
| REQ-VAL-013 | T04, T22 | `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `src/value/mode.rs`, `tests/ui/fail/thread_safe_borrow_bound_tests.rs` |
| REQ-VAL-014 | T04, T22 | `src/value/storage.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `tests/value/str_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-VAL-015 | T04, T22 | `src/value/dynamic_owned.rs`, `src/value/dynamic_ref.rs`, `src/value/dynamic_mut.rs` | `src/value/mode.rs`, `tests/value/dynamic_owned_tests.rs`, `tests/value/thread_mode_tests.rs` |
| REQ-VAR-001 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-002 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-003 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-004 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-005 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-006 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-007 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-008 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
| REQ-VAR-009 | T08, T15, T19 | `src/descriptor/variant_descriptor.rs`, `derive/src/expand/enums.rs` | `tests/descriptor/derive_enum_tests.rs`, `tests/construct/adapter_tests.rs` |
