# Reflection and downstream model performance

Measured on 2026-09-05 with Rust 1.94.0, x86_64 Linux, Intel Core i5-9600K
(six physical cores, nominal 3.7 GHz). These are diagnostic measurements,
not performance thresholds or an application throughput guarantee.

## Descriptor lookup

Run `cargo +1.94.0 bench --bench descriptor_lookup -- --quick` in rs-reflect.

| Operation | Observed time |
| --- | ---: |
| First `Vec<Option<String>>` descriptor initialization | 2,938 ns median |
| Warm nested descriptor lookup | 58.99 ns |
| Warm builtin pair | 30.64 ns per pair |
| Concurrent warm lookup, one worker | 47.41 ns |
| Concurrent warm lookup, four workers | 278.19 ns |
| Concurrent warm lookup, eight workers | 508.80 ns |

First initialization uses 20 fresh child processes. Each child times only
descriptor initialization, excluding process startup. Criterion's repeated
nested-shape case measures warm lookup; it is no longer labelled cold.
Concurrent cases warm every worker before a barrier and report the slowest
worker's batch duration divided by that worker's iteration count. They exclude
thread creation and are not aggregate throughput measurements. Eight workers
also exceed this machine's physical core count. Quick-mode estimates require
longer runs before drawing small percentage comparisons.

## Real platform model graph

Run `cargo +1.94.0 bench -p qubit-platform-testkit --bench model_registry`
in rs-platform. The executable links 131 actual platform models and validates
their relationships. Allocation instrumentation delegates to the system
allocator and counts allocation, zeroed-allocation, and reallocation requests.

| Operation | Requests per operation | Requested bytes per operation |
| --- | ---: | ---: |
| Cold reflection linking and model projection | 10,504 | 1,979,709 |
| Warm model projection | 1,908 | 224,476 |
| Relationship validation | 3,630 | 391,662 |

Cold linking is measured once per process. The two warm operations are measured
100 times, including destruction of temporary results. Byte counts describe
cumulative requested allocation sizes, including reallocations, rather than
retained heap size. The counting allocator itself adds atomic-operation costs.

Concurrent CI builds made wall-clock times unstable: observed cold runs ranged
from about 1.6 to 10.1 ms, warm projection from 0.45 to 4.87 ms, and relationship
validation from 0.40 to 5.40 ms. Allocation counts were stable across these runs.
Do not use these loaded-machine timings as a regression baseline; repeat on an
idle controlled runner before setting latency budgets.

## Decision

Keep the existing descriptor interner for now. Contention is measurable, but
there is no demonstrated downstream latency or throughput requirement that
justifies a thread-local cache and its additional memory and consistency costs.
The generic capability correctness fix uses concrete `TypeId` keys, while
registry-dependent trait links and model properties remain snapshot-local.
Future optimization should compare end-to-end model workloads and allocation
costs, not only a synthetic repeated descriptor lookup.
