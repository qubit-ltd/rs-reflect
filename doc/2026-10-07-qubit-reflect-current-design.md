# Current Design Notes (2026-10-07)

This document records the current design of `qubit-reflect` 0.2.0 after the
reflection-reference review. The original [2026-09-03 design](2026-09-03-qubit-reflect-design.md)
describes the 0.1 design and remains unchanged; this note supplements it.

## Type references and identity

`TypeRef::of<T: Reflect + ?Sized>()` creates an owned `TypeRef::Resolved`
value around `TypeDescriptor::of::<T>()`. Constructing the enum value adds no
allocation. Initializing `T`'s root descriptor may allocate, and the descriptor
checks that a manual `Reflect` implementation reports the same Rust `TypeId`
as `T`. A mismatched implementation panics, matching direct use of
`TypeDescriptor::of`. The method does not promise process-wide interning of
the `TypeRef` value itself; identity belongs to the root descriptor.

`LazyTypeRef` uses this same constructor for first resolution. Its per-slot
`OnceLock` publishes one successful result to concurrent readers. If the
resolver panics, the slot remains uninitialized and a later call retries.

## Model metadata boundary

Model metadata needs `&'static TypeRef`, while the reflection API returns an
owned value. The model v7 adapter creates and leaks one `TypeRef` at that
static metadata boundary. Each helper call may allocate; generated metadata
providers cache their result, so this is initialization work rather than a
per-property-read allocation. The helper does not claim global interning.

## Field-name lookup

`TypeDescriptor::field` currently scans the descriptor's fields linearly by
name. The Criterion microbenchmark below uses a derived record with 16 `u8`
fields and measures only `field(name)` after obtaining the descriptor. The
custom benchmark `main` also performs its existing 20-process cold-start
measurement before Criterion applies the field benchmark filter.

Command: `cargo bench -p qubit-reflect --bench descriptor_lookup -- 'field_lookup/'`

Environment: rustc 1.94.0, x86_64 Intel Core i5-9600K at 3.70 GHz, six cores,
Cargo optimized bench profile. Criterion used 100 samples, a 3-second warmup,
and approximately 5 seconds of measurement per case.

| Lookup | Estimate | Criterion interval | Outliers |
| --- | ---: | ---: | ---: |
| First field (`f00`) | 5.2645 ns | 5.0341–5.5676 ns | 17/100 (17%) |
| Last field (`f15`) | 42.574 ns | 40.660–44.756 ns | 16/100 (16%) |
| Missing (`absent`) | 12.718 ns | 12.490–12.938 ns | 28/100 (28%) |

These figures describe one synthetic 16-field lookup on one machine. They do
not predict end-to-end reflection or model-metadata throughput. The outlier
counts, especially for the missing-field case, also counsel against reading
small differences as stable product performance. Keep the simple linear scan
for now: this measurement does not establish a representative application
benefit that would justify an index and its storage/build cost. Reassess with
real downstream model shapes and lookup frequency, and compare both latency
and descriptor memory before changing the data structure.

## Further reading

- [Historical 0.1 design](2026-09-03-qubit-reflect-design.md)
- [User guide](user_guide.md)
- [Derive contract matrix](derive-contract-matrix.md)
