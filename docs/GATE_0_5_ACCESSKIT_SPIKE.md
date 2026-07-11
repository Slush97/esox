# Gate 0.5 AccessKit Spike

**Outcome:** Adopt AccessKit as the platform accessibility adapter over an
Esox-owned serializable semantic tree.
**Rejected:** AccessKit types as the semantic source of truth, and a direct
AT-SPI bridge maintained by Esox.
**Prototype version:** AccessKit 0.24.1
**Recorded:** 2026-07-11
**Base commit:** `20f8159`

## Question and prototype

Can AccessKit represent Esox's required semantics while preserving headless
snapshots, stable identity, action routing, multiple windows, and a later
AT-SPI implementation?

The isolated `spikes/accesskit_compare` crate builds an Esox-owned snapshot and
adapts it to an AccessKit `TreeUpdate`. It covers:

- a button with click and focus actions;
- editable text with grapheme lengths, cursor, selection, replacement, and
  selection actions;
- a ranged slider with increment, decrement, and set-value actions;
- table, row, and cell hierarchy and indices;
- a modal dialog and an explicit focus transition;
- a clipped, scrolled child with current visible bounds; and
- two windows with independent `(window_id, node_id)` identity spaces.

Run the five contract tests and summary with:

```sh
cargo test --manifest-path spikes/accesskit_compare/Cargo.toml
cargo run --manifest-path spikes/accesskit_compare/Cargo.toml
```

The snapshot round-trips through deterministic JSON, all twelve representative
nodes map without semantic loss, and an AccessKit `ActionRequest` routes back to
the owning window and stable Esox node. AccessKit itself provides per-tree
stable node IDs, atomic updates, focus, action requests, text selections,
numeric values, modal state, clipping, and subtree/tree IDs.

References:

- [AccessKit 0.24.1 core types](https://docs.rs/accesskit/0.24.1/accesskit/)
- [AccessKit Unix 0.22.0](https://docs.rs/accesskit_unix/0.22.0/accesskit_unix/)

## Decision

Architecture review question 7 is answered: **AccessKit is an adapter over an
Esox-owned semantic model, not the semantic source of truth.**

The Phase 2 boundary should expose an owned `SemanticSnapshot` per committed
window scene. The model owns backend-neutral roles, state, relationships,
logical identity, current resolved/visible geometry, text offsets, supported
actions, and focus. It must be serializable without AccessKit or a display
server so headless tests can compare semantic snapshots directly.

An AccessKit adapter converts complete or changed snapshots to `TreeUpdate` and
routes `ActionRequest` back through `(window_id, stable_node_id)`. Each native
window owns an AccessKit tree and an independent Esox ID namespace. Text layout
supplies AccessKit character lengths and positions; the semantic layer does not
invent cursor boundaries.

This separates the durable core contract from AccessKit's incremental update
protocol and permits other consumers without duplicating widget semantics.
AccessKit remains the only planned platform adapter: its Unix adapter exposes
the tree through AT-SPI, replacing the current incomplete direct bridge and its
unverified numeric role mapping when Phase 4 integration begins.

## Risks and follow-up

- The prototype validates representation and routing, not Orca behavior. Phase
  4 still needs AccessKit Unix integration and manual Orca/AT-SPI tests.
- Bounds passed to AccessKit ultimately need window-relative physical pixels.
  The core snapshot should retain resolved logical geometry and let the window
  adapter apply scale and origin transforms.
- Virtualized table/tree children, live regions, rich text runs, and platform
  event diffing remain Phase 4 contract cases.
- Stable IDs must come from the core identity system; vector positions and
  previous-frame geometry are not valid identity or semantic sources.
