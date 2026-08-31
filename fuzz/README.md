# Bounded fuzz smoke

Build and smoke-test the fuzz workspace with a nightly toolchain on Linux:

```bash
cargo +nightly fuzz build --fuzz-dir fuzz

mkdir -p /tmp/qubit-reflect-fuzz/{corpus,artifacts}/{id_parser,type_expression,registry_model}

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
```

The first corpus path is writable and temporary, so bounded CI discovery does
not modify the checked-in seed corpus. `-s none` is suitable for this bounded
logic smoke; the preceding build still validates the default AddressSanitizer
configuration. Scheduled or manual long-running jobs should omit `-s none`
and retain crash artifacts.
