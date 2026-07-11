# Gate 0.5 Taffy Comparison

**Outcome:** Adopt Taffy as the layout-solver direction for the Phase 1 design.
**Prototype version:** Taffy 0.12.1
**Recorded:** 2026-07-10
**Base commit:** `dc231156a90a161164765f5af142dbaf4d40b90e`

This outcome selects Taffy's layout algorithms; it does not yet select the
current-frame tree architecture. The Gate 0.5 current-frame prototype remains a
required input to `ADR-001-frame-lifecycle.md` and will determine whether Esox
uses `TaffyTree` directly or adapts its own tree to Taffy's low-level traits.

## Question

Can Taffy replace the in-house flex/grid solver without losing current Esox
behavior, while adding constraint-aware intrinsic measurement needed for a
correct current-frame layout?

Taffy's documented model matches the needed pipeline: construct a styled tree,
compute layout with an optional leaf-measure function, then consume resolved
geometry. Its high-level API owns the tree and cache; its low-level API is
specifically intended for frameworks that already own a node tree.

References:

- [Taffy crate documentation](https://docs.rs/taffy/0.12.1/taffy/)
- [`TaffyTree` and measure-function examples](https://docs.rs/taffy/0.12.1/taffy/tree/struct.TaffyTree.html)
- [Taffy source repository](https://github.com/DioxusLabs/taffy)

## Prototype

The isolated harness is in `spikes/taffy_compare`. It depends on the existing
`esox_ui` crate and a pinned Taffy 0.12.1, but it is not a member of the Esox
workspace and introduces no production dependency.

Run it with:

```sh
cargo run --manifest-path spikes/taffy_compare/Cargo.toml
```

The harness compares unrounded logical geometry. Taffy's normal `layout()`
accessor applies pixel rounding, while Esox currently retains floating-point
logical coordinates; an integration must choose its physical-pixel snapping
boundary explicitly.

## Results

| Case | Result | Notes |
| --- | --- | --- |
| Flex grow | Match | Two 50 px items in 200 px distribute free space 1:3 to 75 px and 125 px. |
| Flex shrink | Match | 200 px and 100 px items shrink proportionally to 133.33 px and 66.67 px. |
| Flex wrap | Match | Three 80×30 items in 200 px place at `(0,0)`, `(80,0)`, and `(0,30)`. |
| Grid | Match | Fixed/`fr`/auto track starts agree; the auto track resolves from a 70 px intrinsic child. |
| Scroll sizing | Compatible | Taffy keeps a 100 px viewport and reports at least 300 px of content; offset, clipping, and interaction remain framework responsibilities. |
| Intrinsic text | Taffy adds required behavior | A 200×20 natural text leaf is remeasured under an 80 px constraint to 80×60. Current Esox retains the fixed 200×20 intrinsic size. |

The Taffy crate declares Rust 1.71 as its minimum version, below Esox's pinned
Rust 1.95.0. With default features, its normal dependency subtree in this
prototype is `arrayvec`, `grid`, and `slotmap`.

## Decision

Adopt Taffy for the Phase 1 layout solver, subject to the current-frame
prototype confirming the integration shape. Do not extend the in-house solver
with another constraint-aware text-measurement design in parallel.

The integration contract should be:

- Esox owns widget identity, frame reconciliation, scroll offsets, clipping,
  hit testing, focus, semantics, and display-list generation.
- Taffy owns flex, grid, constraint propagation, and leaf measurement queries.
- Text measurement is supplied through a GPU-independent callback and may be
  invoked more than once with different known dimensions or available space.
- Layout is consumed as unrounded logical geometry. Logical-to-physical
  conversion and pixel snapping happen at an explicit renderer boundary.
- An adapter maps Esox's zero-based grid placement and non-CSS defaults to
  Taffy's CSS-derived styles rather than leaking Taffy types directly into the
  public widget API.

## Risks and follow-up

- The current Esox layout defaults are not all CSS defaults. Alignment,
  shrinking, minimum sizing, overflow, and grid-line indexing require explicit
  mapping and broader contract tests.
- Scroll compatibility here covers geometry only. Nested scrolling, scrollbar
  layout, clipping, hit testing, and offset changes need current-frame contract
  tests owned by Esox.
- Taffy's measurement callback solves constrained intrinsic sizing, but the
  callback cannot depend on the GPU atlas. The Phase 2 text/layout boundary must
  preserve that separation.
- The prototype covers representative cases, not CSS conformance. Esox should
  rely on Taffy's upstream conformance suite and keep smaller adapter contract
  tests for framework-specific semantics.
- `ADR-001-frame-lifecycle.md` remains blocked on the separate current-frame
  display-list/element-tree prototype. No production integration should begin
  before that outcome is recorded.
