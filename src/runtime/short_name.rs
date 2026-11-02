// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

pub(crate) fn short_name(name: &str) -> &str {
    if is_composite_type_name(name) {
        return name;
    }
    let base = name.split('<').next().unwrap_or(name);
    base.rsplit("::").next().unwrap_or(base)
}

/// Preserves the historical final-segment key used by context lookup indexes.
pub(crate) fn index_short_name(name: &str) -> &str {
    let base = name.split('<').next().unwrap_or(name);
    base.rsplit("::").next().unwrap_or(base)
}

fn is_composite_type_name(name: &str) -> bool {
    name.starts_with('&')
        || name.starts_with("*const ")
        || name.starts_with("*mut ")
        || name.starts_with('[')
        || name.starts_with('(')
        || name.starts_with("fn(")
        || name.starts_with("unsafe fn(")
        || name.starts_with("for<")
        || name.starts_with("extern ")
        || name.starts_with("dyn ")
        || name.starts_with("impl ")
        || name.starts_with('<')
        || name == "!"
}

#[cfg(test)]
mod tests {
    use super::index_short_name;
    use super::short_name;

    #[test]
    fn composite_names_remain_complete() {
        for name in [
            "for<'a> fn(&'a alloc::string::String) -> u32",
            "dyn core::fmt::Display",
            "<T as core::fmt::Display>::Output",
        ] {
            assert_eq!(short_name(name), name);
        }
    }

    #[test]
    fn composite_index_aliases_keep_the_previous_final_segment() {
        assert_eq!(index_short_name("dyn std::fmt::Display"), "Display");
    }
}
