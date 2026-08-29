# qubit-reflect

[![Rust CI](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-reflect/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-reflect/coverage-badge.json)](https://qubit-ltd.github.io/rs-reflect/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-reflect.svg?color=blue)](https://crates.io/crates/qubit-reflect)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-reflect` is a requirements-first Rust crate for macro-generated structural and executable reflection. It is intended to provide shared type, field, method, trait, implementation, variant, access, invocation, and construction metadata for higher-level frameworks.

The repository is currently an initial scaffold. No public reflection API has been implemented or stabilized.

## Intended Users

The planned crate is intended for framework and library authors who need checked runtime access to Rust structure without parsing source files or depending on compiler-private APIs.

## Installation

The crate is not published and is not ready for use as a dependency. Installation instructions will be added after the first public API is implemented.

## Current Status

The current repository contains CI infrastructure, crate metadata, and the initial Chinese requirements specification. It does not yet provide the planned derive or attribute macros.

## Planned Scope

The target design covers structural descriptors, checked field access, method invocation, trait and implementation metadata, and dynamic construction of structs and enum variants. Exact APIs remain subject to requirements review.

## Learn More

- [Simplified Chinese requirements](doc/2026-08-28-qubit-reflect-requirements.zh_CN.md)
- [简体中文 README](README.zh_CN.md)

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-reflect](https://github.com/qubit-ltd/rs-reflect)
