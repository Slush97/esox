# Phase 0 Source Inventory

Captured on 2026-07-10 from the uncommitted Phase 0 working tree based on
commit `dc231156a90a161164765f5af142dbaf4d40b90e`. This is a mechanical inventory,
not an API-stability decision. Public items remain unclassified until Phase 7.

## Public surface

The counts below cover public source lines in each library crate. A declaration
is a `struct`, `enum`, `trait`, function, type alias, constant, static, or module.
The "other" column is primarily public fields and methods.

| Crate | Public source lines | Declarations | Re-exports | Other |
| --- | ---: | ---: | ---: | ---: |
| `esox_font` | 104 | 67 | 6 | 31 |
| `esox_gfx` | 901 | 525 | 31 | 345 |
| `esox_input` | 23 | 17 | 1 | 5 |
| `esox_markup` | 25 | 14 | 2 | 9 |
| `esox_platform` | 131 | 69 | 1 | 61 |
| `esox_ui` | 1,097 | 672 | 20 | 405 |
| **Total** | **2,281** | **1,364** | **61** | **856** |

These are lexical source counts rather than a semver API report. They do not
expand macros, count implicitly public enum variants separately, or resolve
whether a public item is reachable from a crate root. The inventory is intended
to size the review and expose the current concentration of public surface; a
reachable-API report should be generated when Phase 7 begins.

Reproduce the first column with:

```sh
rg -n '^\s*pub\s+' crates/*/src -g '*.rs'
```

The declaration and re-export subsets use:

```sh
rg -n '^\s*pub\s+(async\s+)?(struct|enum|trait|fn|type|const|static|mod)\b' \
  crates/*/src -g '*.rs'
rg -n '^\s*pub\s+use\b' crates/*/src -g '*.rs'
```

## Panic and unsafe-site inventory

These raw lexical counts intentionally include inline unit-test modules. That
keeps the capture reproducible and prevents test-only sites from silently
falling outside the inventory. Each match includes a filename and line number
when reproduced with the command below.

| Crate | `unwrap` | `expect` | `panic!` | `todo!` | `unsafe` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `esox_font` | 41 | 5 | 0 | 0 | 0 |
| `esox_gfx` | 96 | 15 | 7 | 0 | 0 |
| `esox_input` | 0 | 0 | 0 | 0 | 0 |
| `esox_markup` | 54 | 0 | 0 | 0 | 0 |
| `esox_platform` | 63 | 9 | 0 | 0 | 9 |
| `esox_ui` | 28 | 13 | 1 | 0 | 1 |
| **Total** | **282** | **42** | **8** | **0** | **10** |

Reproduce the complete site list with:

```sh
rg -n '\.unwrap\(|\.expect\(|panic!\(|todo!\(|\bunsafe\b' \
  crates/*/src -g '*.rs'
```

The ten `unsafe` matches are concentrated in four areas:

- one unchecked icon-codepoint conversion in `esox_ui`;
- one WGPU surface lifetime bridge and one `libc::sysconf` call in
  `esox_platform`;
- seven environment mutations in `esox_platform` XDG tests, required to be
  `unsafe` by the Rust 2024 environment-variable API.

This phase records rather than removes the sites. Production-path panic and
unsafe contracts need explicit classification before the affected public APIs
are stabilized.

## Accessibility follow-up

The optional `a11y` feature now compiles, but the AT-SPI bridge still discards
semantic snapshots and is not functional. Its numeric role mapping should be
treated as unverified; several constants appear suspect and must be checked
against the selected accessibility stack during the Gate 0.5 AccessKit spike.
