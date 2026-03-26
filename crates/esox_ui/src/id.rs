//! Widget ID utilities — FNV-1a hashing for zero-allocation u64 IDs.

/// FNV-1a 64-bit hash. const fn — computable at compile time.
pub const fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x00000100000001b3);
        i += 1;
    }
    hash
}

/// Runtime version for dynamically-constructed strings.
pub fn fnv1a_runtime(s: &str) -> u64 {
    fnv1a(s)
}

/// Mix a u64 (e.g. job_id) into an existing hash seed.
/// Use: `fnv1a_mix(id!("open_"), job_id)` for per-job button IDs.
pub fn fnv1a_mix(seed: u64, val: u64) -> u64 {
    let mut h = seed;
    for byte in val.to_le_bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x00000100000001b3);
    }
    h
}

/// XOR salt used to derive hover-animation IDs from widget IDs.
/// Chosen so collisions with any plausible widget ID string are negligible.
pub const HOVER_SALT: u64 = 0x9e3779b97f4a7c15;

/// XOR salt used to derive a distinct layout-tree key for the scroll content
/// container, avoiding collision with the viewport leaf that shares the same
/// user-provided widget ID.
pub const SCROLL_CONTENT_SALT: u64 = 0x6a09e667f3bcc908;

/// XOR salt for press-state animation IDs.
pub const PRESS_SALT: u64 = 0xd1b54a32d192ed03;

/// XOR salt for checkbox check animation IDs.
pub const CHECK_SALT: u64 = 0xa4e7c3f1b8d20956;

/// XOR salt for tab indicator slide animation IDs.
pub const TAB_SLIDE_SALT: u64 = 0x7b3f9e15c4a8d062;

/// XOR salt for focus border animation IDs.
pub const FOCUS_SALT: u64 = 0xe8b42d6f93a5c178;

/// XOR salt for number-input inline-edit animation IDs.
pub const EDIT_SALT: u64 = 0xED17_FACE;

/// Compile-time widget ID from a string literal.
/// `id!("my_widget")` → a `u64` constant, zero runtime cost.
#[macro_export]
macro_rules! id {
    ($s:literal) => {{
        const _ID: u64 = $crate::id::fnv1a($s);
        _ID
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_empty_string_is_offset_basis() {
        assert_eq!(fnv1a(""), 0xcbf29ce484222325);
    }

    #[test]
    fn fnv1a_known_value_a() {
        // FNV-1a of "a": offset_basis ^ 0x61, then *prime
        let expected = (0xcbf29ce484222325_u64 ^ 0x61).wrapping_mul(0x00000100000001b3);
        assert_eq!(fnv1a("a"), expected);
    }

    #[test]
    fn fnv1a_known_value_foobar() {
        // Compute step-by-step for "foobar"
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in b"foobar" {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x00000100000001b3);
        }
        assert_eq!(fnv1a("foobar"), hash);
    }

    #[test]
    fn fnv1a_different_strings_differ() {
        assert_ne!(fnv1a("button_ok"), fnv1a("button_cancel"));
    }

    #[test]
    fn fnv1a_runtime_matches_const() {
        assert_eq!(fnv1a_runtime("hello"), fnv1a("hello"));
        assert_eq!(fnv1a_runtime(""), fnv1a(""));
        assert_eq!(fnv1a_runtime("widget_123"), fnv1a("widget_123"));
    }

    #[test]
    fn fnv1a_mix_deterministic() {
        let seed = fnv1a("prefix_");
        let a = fnv1a_mix(seed, 42);
        let b = fnv1a_mix(seed, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn fnv1a_mix_different_vals_differ() {
        let seed = fnv1a("prefix_");
        assert_ne!(fnv1a_mix(seed, 1), fnv1a_mix(seed, 2));
        assert_ne!(fnv1a_mix(seed, 0), fnv1a_mix(seed, u64::MAX));
    }

    #[test]
    fn hover_salt_produces_distinct_ids() {
        let ids = [
            fnv1a("btn"),
            fnv1a("slider"),
            fnv1a("input"),
            0,
            1,
            u64::MAX,
        ];
        for id in ids {
            assert_ne!(
                id ^ HOVER_SALT,
                id,
                "HOVER_SALT should flip bits for id={id}"
            );
        }
    }

    #[test]
    fn id_macro_matches_fnv1a() {
        let macro_val = crate::id!("my_widget");
        assert_eq!(macro_val, fnv1a("my_widget"));

        let macro_val2 = crate::id!("other");
        assert_eq!(macro_val2, fnv1a("other"));
    }

    #[test]
    fn id_macro_is_const() {
        // This compiles because id! produces a const — that's the test.
        const ID: u64 = crate::id!("compile_time");
        assert_eq!(ID, fnv1a("compile_time"));
    }
}
