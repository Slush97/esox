# Gate 0.5 Current-Frame Prototype

**Outcome:** Adopt a lightweight current-frame element tree with a dedicated
paint traversal for the Phase 1 design.
**Rejected for Phase 1:** A recorded command stream and a reconciled render
tree.
**Prototype version:** Taffy 0.12.1
**Recorded:** 2026-07-11
**Base commit:** `9b987f4cde57e280d97545d83ddfa29d3f7dc46b`

This is the second Gate 0.5 decision spike required before
`ADR-001-frame-lifecycle.md`. It selects a prototype direction; it does not
change the production frame architecture or settle the lifecycle details that
belong in the ADR.

## Question

Which Phase 1 candidate can turn immediate-mode declarations into correct
first-frame paint, hit-test, clip, damage, and semantic geometry after Taffy
layout, without replaying application closures?

The candidates from `ARCHITECTURE_REVIEW_PLAN.md` section 9 were implemented in
miniature:

1. recorded widget and paint commands, materialized after declaration;
2. a lightweight owned element tree followed by layout and paint traversal;
3. a persistent keyed tree reconciled behind the same immediate-mode facade.

## Prototype

The isolated harness is in `spikes/current_frame_compare`. It is not a workspace
member and does not introduce a production dependency. Run it with:

```sh
cargo test --manifest-path spikes/current_frame_compare/Cargo.toml
cargo run --manifest-path spikes/current_frame_compare/Cargo.toml
```

Each candidate consumes one owned declaration per frame. The harness counts
declaration calls and fails if any layout or paint phase asks the application to
declare the scene again. A shared null-renderer traversal then makes the
architectural results directly comparable.

The representative scene contains:

- a flex row containing a nested flex column;
- a two-column grid with constrained, callback-measured text;
- a scrolling subtree with a current-frame transform and clip;
- an absolutely positioned blocking overlay painted last;
- interactive nodes resolved by reverse-paint-order hit testing; and
- semantic, hit, clip, paint, and damage bounds derived from resolved layout.

The structural-change frame resizes the viewport, changes text metrics and the
scroll offset, inserts one keyed row, removes another, hides a grid child, and
reorders two retained rows.

## Results

| Candidate | First frame | Structural change | Closure replay | Finding |
| --- | --- | --- | --- | --- |
| Recorded commands | Correct | Correct | None | The stream has to encode balanced hierarchy and be materialized as a tree before Taffy can solve it. |
| Lightweight element tree | Correct | Correct | None | The declaration result is already the exact owned hierarchy needed by layout and paint traversal. |
| Reconciled tree | Correct | Correct | None | Keyed reuse works, but adds persistent-tree mutation and removal/move accounting without being needed for correctness. |

For every candidate:

- the first snapshot equals the unchanged second snapshot, so there is no
  warm-up frame;
- the post-change snapshot contains no stale removed, hidden, or reordered
  geometry;
- text measurement and viewport resizing affect the same frame's geometry;
- a scroll offset immediately affects paint and hit coordinates, while the
  viewport clip prevents hits on the clipped portion of a child;
- the overlay wins hit testing over content in its blocking region; and
- paint, hit, semantic, clip, and damage records are produced from one resolved
  traversal.

The candidates produce identical initial and structural-change snapshots. On
the structural-change frame, the reconciled prototype reports one insertion,
twelve reused nodes, one removed node, and one moved node. Those figures
demonstrate reconciliation, but not a correctness advantage.

## Decision

Adopt the lightweight current-frame element tree as the Phase 1 starting point:

1. Run the application-facing immediate-mode declaration once.
2. Store an owned, lightweight element hierarchy for the current frame.
3. Build or adapt that hierarchy to Taffy and perform constraint-aware layout.
4. Traverse resolved nodes once to produce display-list, clip, hit-test,
   semantic, and damage data.
5. Commit the resolved scene for rendering and subsequent input dispatch as
   specified later by ADR-001.

This shape matches Taffy's hierarchical input directly and makes current-frame
geometry the only source for downstream outputs. It also keeps identity and
state ownership in Esox, consistent with the Taffy spike, without requiring the
render hierarchy itself to persist across frames.

Reject the recorded-command design for the Phase 1 foundation. A flat stream is
not actually flat once nested layout, clipping, scrolling, overlays, semantics,
and hit testing are represented. The prototype reconstructs an element tree
from balanced begin/end commands before solving; retaining the stream adds an
intermediate representation without removing the tree.

Reject a reconciled render tree as a Phase 1 correctness requirement. It makes
insertion, removal, reordering, invalidation, and cache lifetime part of the
critical path before current-frame correctness exists. Keyed persistence may be
introduced later as a measured optimization or for narrowly scoped widget
state, provided it cannot become a previous-frame geometry source.

## ADR-001 input and remaining questions

ADR-001 may now begin. It should use the lightweight tree direction and the
previous Taffy adoption outcome, but must still specify matters this spike does
not decide:

- whether input targets the last committed scene or the scene under
  construction;
- when responses become observable and which frame owns their geometry;
- focus and capture behavior for inserted, removed, hidden, and overlay nodes;
- animation, invalidation, multi-window ownership, and scene commit boundaries;
- stable identity/state storage outside the ephemeral element hierarchy; and
- the exact logical-to-physical snapping boundary.

The prototype intentionally omits production integration, GPU rendering,
incremental caching, and public API design. Its deterministic text measurer and
null output records prove the frame shape, not a complete headless backend.
