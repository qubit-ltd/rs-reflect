// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Compile-time handshake between generated code and its runtime.
//!
//! Derive output should reference this constant through
//! `qubit_reflect::__private::codegen_v4::protocol`. A macro can compare the
//! expected value in a const context so incompatible generated code fails at
//! compile time.

/// Version of the generated-code/runtime protocol.
pub const PROTOCOL_VERSION: u32 = 4;

/// Checks that generated code and the runtime use the same protocol version.
///
/// Generated code calls this function from a constant item so a mismatch is
/// reported while compiling the caller crate.
pub const fn assert_protocol_version(expected: u32) {
    assert!(
        expected == PROTOCOL_VERSION,
        "generated code and runtime protocol versions do not match",
    );
}

#[cfg(test)]
mod tests {
    use super::PROTOCOL_VERSION;
    use super::assert_protocol_version;

    #[test]
    fn accepts_the_current_protocol_version_at_runtime() {
        assert_protocol_version(PROTOCOL_VERSION);
    }

    #[test]
    #[should_panic(expected = "generated code and runtime protocol versions do not match")]
    fn rejects_a_mismatched_protocol_version_at_runtime() {
        assert_protocol_version(PROTOCOL_VERSION - 1);
    }
}
