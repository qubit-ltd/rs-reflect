# Bounded fuzz smoke

The regular CI builds all five fuzz targets. The weekly **Fuzz smoke** workflow
runs each target for 30 seconds with the default sanitizer; it can also be
started with `workflow_dispatch`. A failure uploads `fuzz/artifacts/` when
libFuzzer produces a crash input. Run `./scripts/cargo-fuzz-check.sh` with
`RS_CI_FUZZ_MODE=smoke` to reproduce the bounded check locally.

Build and smoke-test the fuzz workspace with a nightly toolchain on Linux:

```bash
cargo +nightly fuzz build --fuzz-dir fuzz

mkdir -p /tmp/qubit-reflect-fuzz/{corpus,artifacts}/{id_parser,type_expression,registry_model,registry_snapshot,dynamic_operations}

cargo +nightly fuzz run --fuzz-dir fuzz -s none id_parser \
  /tmp/qubit-reflect-fuzz/corpus/id_parser fuzz/corpus/id_parser -- \
  -runs=1000 \
  -artifact_prefix=/tmp/qubit-reflect-fuzz/artifacts/id_parser/ \
  -dict=fuzz/dictionaries/id_parser.dict

cargo +nightly fuzz run --fuzz-dir fuzz -s none type_expression \
  /tmp/qubit-reflect-fuzz/corpus/type_expression fuzz/corpus/type_expression -- \
  -runs=1000 \
  -artifact_prefix=/tmp/qubit-reflect-fuzz/artifacts/type_expression/

cargo +nightly fuzz run --fuzz-dir fuzz -s none registry_model \
  /tmp/qubit-reflect-fuzz/corpus/registry_model fuzz/corpus/registry_model -- \
  -runs=1000 \
  -artifact_prefix=/tmp/qubit-reflect-fuzz/artifacts/registry_model/ \
  -dict=fuzz/dictionaries/registry_model.dict

cargo +nightly fuzz run --fuzz-dir fuzz -s none registry_snapshot \
  /tmp/qubit-reflect-fuzz/corpus/registry_snapshot -- \
  -runs=1000 \
  -artifact_prefix=/tmp/qubit-reflect-fuzz/artifacts/registry_snapshot/

cargo +nightly fuzz run --fuzz-dir fuzz -s none dynamic_operations \
  /tmp/qubit-reflect-fuzz/corpus/dynamic_operations -- \
  -runs=1000 \
  -artifact_prefix=/tmp/qubit-reflect-fuzz/artifacts/dynamic_operations/
```

The first corpus path is writable and temporary, so bounded CI discovery does
not modify the checked-in seed corpus. `-s none` is suitable for this bounded
logic smoke; the preceding build still validates the default AddressSanitizer
configuration. Scheduled or manual long-running jobs should omit `-s none`
and retain crash artifacts.
