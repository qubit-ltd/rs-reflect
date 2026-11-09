// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Returns a concise lookup key for a source-level type name.
///
/// Ordinary paths are reduced to their final segment. Composite type syntax is
/// kept intact because stripping its path components would discard structure.
///
/// # Parameters
///
/// * `name` — the source-level type name to shorten.
pub(crate) fn short_name(name: &str) -> &str {
    if is_composite_type_name(name) {
        return name;
    }
    let base = name.split('<').next().unwrap_or(name);
    base.rsplit("::").next().unwrap_or(base)
}

/// Preserves the final-segment key used by existing context lookup indexes.
///
/// Unlike [`short_name`], this intentionally shortens composite names so old
/// index entries remain addressable.
///
/// # Parameters
///
/// * `name` — the source-level type name indexed by a context.
pub(crate) fn index_short_name(name: &str) -> &str {
    let base = name.split('<').next().unwrap_or(name);
    base.rsplit("::").next().unwrap_or(base)
}

/// Reports whether shortening a type name would discard composite syntax.
///
/// # Parameters
///
/// * `name` — the source-level type name to classify.
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
    fn test_composite_names_remain_complete() {
        for name in [
            "for<'a> fn(&'a alloc::string::String) -> u32",
            "dyn core::fmt::Display",
            "<T as core::fmt::Display>::Output",
        ] {
            assert_eq!(short_name(name), name);
        }
    }

    #[test]
    fn test_composite_index_aliases_keep_the_previous_final_segment() {
        assert_eq!(index_short_name("dyn std::fmt::Display"), "Display");
    }
}
