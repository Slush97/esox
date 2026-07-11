# Gate 0.5 Unicode Text-Stack Spike

**Outcome:** Adopt cosmic-text as the Phase 2 shaping/layout engine behind an
Esox-owned text-layout and editing interface.
**Rejected:** The current rustybuzz/swash direction as a complete layout stack,
and Parley 0.11.0 as the editor navigation contract.
**Prototype versions:** cosmic-text 0.19.0, Parley 0.11.0, rustybuzz 0.20.1
**Recorded:** 2026-07-11
**Base commit:** `20f8159`

## Question and prototype

Which stack provides GPU-independent measurement and the Unicode data required
by Phase 2 for bidirectional layout, line breaking, fallback, cursor mapping,
selection, and IME integration?

The isolated `spikes/text_stack_compare` crate shapes the same mixed Latin,
Arabic, Japanese, emoji-ZWJ, combining-mark, and Devanagari samples through all
three candidates. It runs with cosmic-text's `swash` feature disabled and does
not create an atlas or GPU resource. Six tests cover Unicode grapheme and line
boundaries, bidi, wrapping, mixed-script fallback, shaped-cluster positions,
cursor motion, selection, and IME-facing state.

Run it with:

```sh
cargo test --manifest-path spikes/text_stack_compare/Cargo.toml
cargo run --manifest-path spikes/text_stack_compare/Cargo.toml
```

On the representative wrapped paragraph, both Parley and cosmic-text produced
three lines, one RTL run, and four fallback fonts. The rustybuzz-only path
produced one shaped run with one font; it has no paragraph line breaker, bidi
run resolver, fallback manager, or editing model.

The editor sample contains `e` plus a combining mark and a family emoji ZWJ
sequence. cosmic-text moved through byte offsets `0, 1, 4, 5, 30, 31, 34`, all
extended-grapheme boundaries, and produced a selection. Parley moved through
`0, 1, 2, 4, 5, 9, 12`; offsets 2, 9, and 12 split the required grapheme units.
Parley has the stronger built-in IME composition API and direct AccessKit text
support, but that does not offset incorrect behavior for the stated grapheme
navigation contract.

References:

- [cosmic-text 0.19.0 buffer and cursor API](https://docs.rs/cosmic-text/0.19.0/cosmic_text/struct.Buffer.html)
- [Parley 0.11.0 editing cursor API](https://docs.rs/parley/0.11.0/parley/editing/struct.Cursor.html)
- [rustybuzz 0.20.1](https://docs.rs/rustybuzz/0.20.1/rustybuzz/)

## Decision and Phase 2 interface

Adopt cosmic-text for advanced shaping, bidi paragraph resolution, fallback,
line breaking, glyph/cluster layout, hit testing, and cursor motion. Do not
expose `Buffer`, `Editor`, font database IDs, glyph cache keys, or renderer
types across the core boundary.

Phase 2 should define an Esox-owned interface with these inputs and outputs:

- input text, style spans, locale/direction hints, scale, and width/height
  constraints;
- logical size, lines, bidi runs, fallback font handles, positioned glyphs,
  grapheme-safe cursor stops, hit-test results, caret geometry, and selection
  rectangles;
- an editing state with UTF-8 byte ranges guaranteed to fall on extended
  grapheme boundaries; and
- explicit IME preedit text, preedit selection, replacement range, caret area,
  commit, and cancel operations owned by Esox and projected into cosmic-text
  buffers for each measurement.

The interface must be usable by Taffy's measurement callback and headless
tests without WGPU, a glyph atlas, or rasterization. Rasterization remains a
downstream concern. Phase 2 may adapt cosmic-text font data/glyph IDs to the
existing swash rasterizer or replace that rasterizer later; neither choice may
affect measurement or cursor results.

## Risks and follow-up

- cosmic-text has no integrated preedit transaction matching Parley's API, so
  the Esox editing layer must own composition and receive focused IME contract
  tests before production use.
- The spike used installed system fonts. CI needs a small licensed multi-script
  font fixture set so fallback snapshots are deterministic.
- Locale-specific tailoring, vertical text, normalization policy, and exact
  first-release language coverage remain unanswered by review question 8.
- Phase 2 should pin the chosen version and retain conformance tests around the
  owned interface because cursor and fallback behavior can change upstream.
